<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import { marked } from "marked";
import DOMPurify from "dompurify";
import {
  ArrowUpCircle,
  Check,
  ChevronLeft,
  Download,
  ExternalLink,
  Loader2,
} from "lucide-vue-next";
import { useAppStore } from "@/stores/app";
import { useMarketplaceStore } from "@/stores/marketplace";
import { api } from "@/lib/api";
import type { Plugin, PluginSource } from "@/lib/types";
import { pluginImportStatus } from "@/lib/origins";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";

const props = defineProps<{ name: string }>();

const router = useRouter();
const store = useAppStore();
const marketplaceStore = useMarketplaceStore();
const { library, hooks, mcp } = storeToRefs(store);
const {
  marketplace,
  marketplaceLoading,
  importingPlugin,
} = storeToRefs(marketplaceStore);

const plugin = computed<Plugin | null>(
  () =>
    marketplace.value?.plugins.find((p) => p.name === props.name) ?? null
);

const isImporting = computed(() => importingPlugin.value === props.name);

const status = computed(() =>
  plugin.value
    ? pluginImportStatus(library.value, marketplace.value?.name, plugin.value, hooks.value, mcp.value)
    : { kind: "not-imported" as const }
);

const isImported = computed(() => status.value.kind !== "not-imported");

// Latest import timestamp across all assets that originated from this plugin —
// surfaced in the header so the user knows when they last pulled.
const importedAt = computed<string | null>(() => {
  let latest: string | null = null;
  for (const a of library.value) {
    if (a.origin?.plugin === props.name && a.origin.marketplace === marketplace.value?.name) {
      if (!latest || a.origin.imported_at > latest) latest = a.origin.imported_at;
    }
  }
  return latest;
});

function formatImportedAt(iso: string): string {
  try {
    return new Date(iso).toLocaleString(undefined, {
      dateStyle: "medium",
      timeStyle: "short",
    });
  } catch {
    return iso;
  }
}

function viewInLibrary() {
  router.push({
    name: "browse-library-plugin",
    params: { plugin: props.name },
  });
}

// ── Readme fetch (per-mount, no global cache) ─────────────────────
const readme = ref<string | null>(null);
const readmeLoading = ref(false);
const readmeError = ref<string | null>(null);

async function loadReadme() {
  if (!plugin.value) return;
  readme.value = null;
  readmeError.value = null;
  readmeLoading.value = true;
  try {
    readme.value = await api.fetchPluginReadme(plugin.value);
  } catch (e) {
    readmeError.value = String(e);
  } finally {
    readmeLoading.value = false;
  }
}

const renderedReadme = computed<string>(() => {
  if (!readme.value) return "";
  const out = marked.parse(readme.value, {
    gfm: true,
    breaks: false,
    async: false,
  });
  return DOMPurify.sanitize(typeof out === "string" ? out : "");
});

// Make sure the marketplace data is in memory; if the user lands here via a
// direct URL refresh, marketplaceStore.loadMarketplace fetches it once.
onMounted(async () => {
  if (!marketplace.value) await marketplaceStore.loadMarketplace();
  await loadReadme();
});

watch(
  () => props.name,
  () => loadReadme()
);

// ── Source display helpers (same shape as the previous Sheet view) ─
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
  <div class="flex h-full flex-col overflow-hidden">
    <!-- Loading marketplace state -->
    <div
      v-if="!marketplace && marketplaceLoading"
      class="flex h-full items-center justify-center text-sm text-muted-foreground"
    >
      <Loader2 class="mr-2 size-4 animate-spin" />
      Loading marketplace…
    </div>

    <!-- Not found -->
    <div
      v-else-if="!plugin"
      class="flex h-full flex-col items-center justify-center gap-4 px-6 text-center"
    >
      <h3 class="text-base font-semibold">Plugin not found</h3>
      <p class="text-sm text-muted-foreground">
        No plugin named
        <code class="rounded bg-muted px-1 py-0.5 font-mono text-xs">{{ name }}</code>
        in the marketplace.
      </p>
      <Button variant="outline" @click="router.push({ name: 'browse' })">
        <ChevronLeft />
        Back to browse
      </Button>
    </div>

    <template v-else>
      <!-- Header -->
      <div class="border-b bg-card/40 px-6 py-4">
        <button
          class="mb-2 inline-flex items-center gap-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
          @click="router.back()"
        >
          <ChevronLeft class="size-3" />
          Back
        </button>

        <div class="flex items-start justify-between gap-4">
          <div class="min-w-0 flex-1">
            <div class="mb-1 text-[11px] font-medium text-muted-foreground">
              Marketplace · <span class="font-mono">{{ marketplace?.name }}</span>
            </div>
            <div class="flex items-center gap-2">
              <h1 class="truncate text-lg font-semibold">{{ plugin.name }}</h1>
              <Badge
                v-if="plugin.category"
                variant="secondary"
                class="text-[10px] uppercase tracking-wider"
              >{{ plugin.category }}</Badge>
              <Badge
                v-if="status.kind === 'update'"
                variant="outline"
                class="gap-1 border-amber-500/50 text-[10px] uppercase tracking-wider text-amber-600 dark:text-amber-400"
              >
                <ArrowUpCircle class="size-2.5" />
                Update available
              </Badge>
              <Badge
                v-else-if="isImported"
                variant="outline"
                class="gap-1 border-primary/40 text-[10px] uppercase tracking-wider text-primary"
              >
                <Check class="size-2.5" />
                In library
              </Badge>
            </div>
            <p
              v-if="plugin.author"
              class="mt-1 text-sm text-muted-foreground"
            >by {{ plugin.author.name }}</p>
            <p
              v-if="status.kind === 'update'"
              class="mt-1 text-xs text-amber-600 dark:text-amber-400"
            >
              Imported v{{ status.importedVersion }} ·
              marketplace has v{{ status.currentVersion }}
            </p>
            <p
              v-else-if="status.kind === 'unknown'"
              class="mt-1 text-xs text-muted-foreground"
            >
              Imported before version tracking — can't tell if updates exist.
              <span v-if="importedAt">({{ formatImportedAt(importedAt) }})</span>
            </p>
            <p
              v-else-if="isImported && importedAt"
              class="mt-1 text-xs text-muted-foreground"
            >Imported on {{ formatImportedAt(importedAt) }}<span
              v-if="status.kind === 'current' && status.importedVersion"
            > · v{{ status.importedVersion }}</span></p>
          </div>

          <div class="flex shrink-0 items-center gap-2">
            <template v-if="status.kind === 'update'">
              <Button
                size="sm"
                title="Review the upstream changes hunk-by-hunk before overwriting your local copy."
                @click="router.push({ name: 'plugin-update', params: { name: plugin.name } })"
              >
                <ArrowUpCircle />
                Update…
              </Button>
              <Button size="sm" variant="ghost" @click="viewInLibrary">
                View in library
              </Button>
            </template>
            <template v-else-if="isImported">
              <Button size="sm" variant="outline" @click="viewInLibrary">
                <Check />
                View in library
              </Button>
              <Button
                size="sm"
                variant="ghost"
                :disabled="isImporting"
                title="Re-pull the plugin (any new files since last import will be added; existing files are kept)."
                @click="marketplaceStore.importMarketplacePlugin(plugin)"
              >
                <Loader2 v-if="isImporting" class="animate-spin" />
                <Download v-else />
                {{ isImporting ? "Re-importing…" : "Re-import" }}
              </Button>
            </template>
            <Button
              v-else
              size="sm"
              :disabled="isImporting"
              @click="marketplaceStore.importMarketplacePlugin(plugin)"
            >
              <Loader2 v-if="isImporting" class="animate-spin" />
              <Download v-else />
              {{ isImporting ? "Importing…" : "Import to library" }}
            </Button>
            <a
              v-if="plugin.homepage"
              :href="plugin.homepage"
              target="_blank"
              rel="noopener"
              class="inline-flex items-center gap-1.5 px-2 text-xs text-muted-foreground transition-colors hover:text-foreground"
            >
              Homepage
              <ExternalLink class="size-3" />
            </a>
          </div>
        </div>
      </div>

      <!-- Body -->
      <ScrollArea class="flex-1 min-h-0">
        <div class="space-y-6 px-6 py-5">
          <!-- Description -->
          <section>
            <h3
              class="mb-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground"
            >Description</h3>
            <p class="text-sm leading-relaxed">{{ plugin.description }}</p>
          </section>

          <Separator />

          <!-- Source -->
          <section>
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
                  class="self-center text-[10px] font-medium uppercase tracking-wider text-muted-foreground"
                >{{ line.label }}</dt>
                <dd
                  class="m-0 break-all"
                  :class="
                    line.mono
                      ? 'font-mono text-[11.5px]'
                      : 'text-muted-foreground'
                  "
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
            <div v-else class="markdown" v-html="renderedReadme" />
          </section>
        </div>
      </ScrollArea>
    </template>
  </div>
</template>
