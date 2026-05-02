<script setup lang="ts">
import { computed } from "vue";
import { marked } from "marked";
import DOMPurify from "dompurify";
import { Download, ExternalLink, Loader2 } from "lucide-vue-next";
import type { Plugin, PluginSource } from "@/lib/types";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { ScrollArea } from "@/components/ui/scroll-area";

const props = defineProps<{
  plugin: Plugin | null;
  readme: string | null;
  readmeLoading: boolean;
  readmeError: string | null;
  importing: boolean;
}>();

const emit = defineEmits<{
  close: [];
  import: [plugin: Plugin];
}>();

const open = computed(() => props.plugin !== null);

function onOpenChange(value: boolean) {
  if (!value) emit("close");
}

const renderedReadme = computed<string>(() => {
  if (!props.readme) return "";
  const out = marked.parse(props.readme, {
    gfm: true,
    breaks: false,
    async: false,
  });
  return DOMPurify.sanitize(typeof out === "string" ? out : "");
});

interface SourceLine {
  label: string;
  value: string;
  mono?: boolean;
}

function sourceLines(src: PluginSource): SourceLine[] {
  if (typeof src === "string") {
    return [
      { label: "Type", value: "inline (marketplace repo)" },
      { label: "Path", value: src, mono: true },
    ];
  }
  if (src.source === "url") {
    const lines: SourceLine[] = [
      { label: "Type", value: "url" },
      { label: "URL", value: src.url, mono: true },
    ];
    if (src.sha) lines.push({ label: "Commit", value: src.sha, mono: true });
    return lines;
  }
  if (src.source === "git-subdir") {
    const lines: SourceLine[] = [
      { label: "Type", value: "git-subdir" },
      { label: "URL", value: src.url, mono: true },
      { label: "Path", value: src.path, mono: true },
    ];
    if (src.sha) lines.push({ label: "Commit", value: src.sha, mono: true });
    else if (src.ref) lines.push({ label: "Ref", value: src.ref, mono: true });
    else if (src.branch) lines.push({ label: "Branch", value: src.branch, mono: true });
    return lines;
  }
  if (src.source === "github") {
    const lines: SourceLine[] = [
      { label: "Type", value: "github" },
      { label: "Repo", value: src.repo, mono: true },
    ];
    if (src.commit) lines.push({ label: "Commit", value: src.commit, mono: true });
    return lines;
  }
  return [];
}
</script>

<template>
  <Sheet :open="open" @update:open="onOpenChange">
    <SheetContent
      side="right"
      class="flex w-full flex-col gap-0 p-0 sm:max-w-xl"
    >
      <SheetHeader class="border-b px-6 pt-6 pb-4">
        <div class="flex items-center gap-2 pr-8">
          <SheetTitle class="text-base font-semibold">
            {{ plugin?.name }}
          </SheetTitle>
          <Badge
            v-if="plugin?.category"
            variant="secondary"
            class="text-[10px] uppercase tracking-wider"
          >{{ plugin.category }}</Badge>
        </div>
        <SheetDescription v-if="plugin?.author" class="text-xs">
          by {{ plugin.author.name }}
        </SheetDescription>
      </SheetHeader>

      <!-- Action bar -->
      <div class="flex items-center gap-2 border-b px-6 py-3">
        <Button
          size="sm"
          :disabled="importing"
          @click="plugin && emit('import', plugin)"
        >
          <Download v-if="!importing" />
          <Loader2 v-else class="animate-spin" />
          {{ importing ? "Importing…" : "Import to library" }}
        </Button>
        <a
          v-if="plugin?.homepage"
          :href="plugin.homepage"
          target="_blank"
          rel="noopener"
          class="inline-flex items-center gap-1.5 px-2 text-xs text-muted-foreground transition-colors hover:text-foreground"
        >
          Homepage
          <ExternalLink class="size-3" />
        </a>
      </div>

      <!-- Body -->
      <ScrollArea class="flex-1 min-h-0">
        <div class="space-y-6 px-6 py-5">
          <!-- Description -->
          <section v-if="plugin">
            <h3
              class="mb-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground"
            >Description</h3>
            <p class="text-sm leading-relaxed">{{ plugin.description }}</p>
          </section>

          <Separator />

          <!-- Source -->
          <section v-if="plugin">
            <h3
              class="mb-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground"
            >Source</h3>
            <dl
              class="grid gap-x-4 gap-y-1.5 text-xs"
              style="grid-template-columns: 80px 1fr"
            >
              <template
                v-for="line in sourceLines(plugin.source)"
                :key="line.label"
              >
                <dt
                  class="text-[10px] font-medium uppercase tracking-wider text-muted-foreground self-center"
                >{{ line.label }}</dt>
                <dd
                  class="m-0 break-all"
                  :class="line.mono ? 'font-mono text-[11.5px]' : 'text-muted-foreground'"
                >{{ line.value }}</dd>
              </template>
            </dl>
          </section>

          <Separator />

          <!-- Readme -->
          <section>
            <h3
              class="mb-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground"
            >Readme</h3>
            <div
              v-if="readmeLoading"
              class="flex items-center gap-2 rounded-md border bg-card/40 p-3 text-xs text-muted-foreground"
            >
              <Loader2 class="size-3.5 animate-spin" />
              Loading readme…
            </div>
            <div
              v-else-if="readmeError"
              class="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-xs text-destructive"
            >
              Couldn't load readme: {{ readmeError }}
            </div>
            <div
              v-else-if="!readme"
              class="rounded-md border border-dashed bg-card/40 p-3 text-xs italic text-muted-foreground"
            >
              No readme found in this plugin's source.
            </div>
            <div
              v-else
              class="markdown text-sm"
              v-html="renderedReadme"
            />
          </section>
        </div>
      </ScrollArea>
    </SheetContent>
  </Sheet>
</template>

<style scoped>
/* Markdown content styling */
.markdown :deep(h1),
.markdown :deep(h2),
.markdown :deep(h3),
.markdown :deep(h4) {
  font-weight: 600;
  letter-spacing: -0.01em;
  margin-top: 1.25rem;
  margin-bottom: 0.5rem;
}
.markdown :deep(h1) { font-size: 1.125rem; }
.markdown :deep(h2) { font-size: 1rem; }
.markdown :deep(h3) { font-size: 0.9rem; }
.markdown :deep(h4) {
  font-size: 0.85rem;
  color: var(--muted-foreground);
}

.markdown :deep(p) {
  margin: 0 0 0.75rem;
}

.markdown :deep(a) {
  color: var(--primary);
  text-decoration: none;
}
.markdown :deep(a:hover) {
  text-decoration: underline;
}

.markdown :deep(code) {
  font-family: ui-monospace, "SF Mono", Menlo, monospace;
  font-size: 0.78rem;
  padding: 0.1rem 0.35rem;
  border-radius: 0.25rem;
  background: var(--muted);
  border: 1px solid var(--border);
}

.markdown :deep(pre) {
  background: var(--muted);
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  padding: 0.75rem 0.875rem;
  overflow-x: auto;
  margin: 0.75rem 0;
}

.markdown :deep(pre code) {
  background: transparent;
  border: 0;
  padding: 0;
  font-size: 0.78rem;
  line-height: 1.5;
}

.markdown :deep(ul),
.markdown :deep(ol) {
  margin: 0 0 0.75rem;
  padding-left: 1.4rem;
}

.markdown :deep(li) {
  margin-bottom: 0.25rem;
}

.markdown :deep(blockquote) {
  margin: 0.75rem 0;
  padding: 0.5rem 0.875rem;
  border-left: 3px solid var(--primary);
  background: var(--muted);
  color: var(--muted-foreground);
  border-radius: 0 0.5rem 0.5rem 0;
}

.markdown :deep(hr) {
  border: 0;
  border-top: 1px solid var(--border);
  margin: 1.25rem 0;
}

.markdown :deep(table) {
  border-collapse: collapse;
  margin: 0.75rem 0;
  font-size: 0.78rem;
}

.markdown :deep(th),
.markdown :deep(td) {
  border: 1px solid var(--border);
  padding: 0.4rem 0.625rem;
  text-align: left;
}

.markdown :deep(th) {
  background: var(--muted);
  font-weight: 600;
}

.markdown :deep(img) {
  max-width: 100%;
  border-radius: 0.5rem;
}
</style>
