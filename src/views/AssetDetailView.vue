<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { onBeforeRouteLeave, useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import { marked } from "marked";
import DOMPurify from "dompurify";
import {
  AlertCircle,
  ChevronLeft,
  Eye,
  Loader2,
  Pencil,
  Save,
  Sparkles,
} from "lucide-vue-next";
import { toast } from "vue-sonner";
import { EditorView, keymap } from "@codemirror/view";
import { EditorState } from "@codemirror/state";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { markdown } from "@codemirror/lang-markdown";
import { oneDark } from "@codemirror/theme-one-dark";
import { useAppStore } from "@/stores/app";
import { useAiStore } from "@/stores/ai";
import { api } from "@/lib/api";
import type { AssetKind } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import {
  ToggleGroup,
  ToggleGroupItem,
} from "@/components/ui/toggle-group";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

const props = defineProps<{ kind: string; name: string }>();

const router = useRouter();
const store = useAppStore();
const aiStore = useAiStore();
const { library } = storeToRefs(store);
const { status: aiStatus, generating } = storeToRefs(aiStore);

// Validate the route param against the known asset kinds.
const VALID_KINDS = ["skills", "commands", "agents"] as const;
const kindSafe = computed<AssetKind | null>(() =>
  (VALID_KINDS as readonly string[]).includes(props.kind)
    ? (props.kind as AssetKind)
    : null
);

// Optional metadata enrichment: if the library scan exposes this asset,
// reuse its description / origin for the header.
const libraryAsset = computed(() =>
  kindSafe.value
    ? library.value.find(
        (a) => a.kind === kindSafe.value && a.name === props.name
      ) ?? null
    : null
);

type ViewMode = "preview" | "edit";
const viewMode = ref<ViewMode>("preview");

const content = ref("");
const original = ref("");
const loading = ref(false);
const saving = ref(false);
const loadError = ref<string | null>(null);
const saveError = ref<string | null>(null);

const dirty = computed(() => content.value !== original.value);

const renderedMarkdown = computed<string>(() => {
  if (!content.value) return "";
  const out = marked.parse(content.value, {
    gfm: true,
    breaks: false,
    async: false,
  });
  return DOMPurify.sanitize(typeof out === "string" ? out : "");
});

async function load() {
  if (!kindSafe.value) {
    loadError.value = `Invalid asset kind "${props.kind}"`;
    return;
  }
  loading.value = true;
  loadError.value = null;
  try {
    const text = await api.readAsset(kindSafe.value, props.name);
    content.value = text;
    original.value = text;
  } catch (e) {
    loadError.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function save() {
  if (!kindSafe.value || !dirty.value || saving.value) return;
  saving.value = true;
  saveError.value = null;
  try {
    await api.writeAsset(kindSafe.value, props.name, content.value);
    original.value = content.value;
    await store.refreshLibrary();
    toast.success("Saved");
  } catch (e) {
    saveError.value = String(e);
    toast.error("Save failed", { description: String(e) });
  } finally {
    saving.value = false;
  }
}

// ── CodeMirror editor lifecycle ───────────────────────────────────
const editorContainer = ref<HTMLElement | null>(null);
let editorView: EditorView | null = null;

function initEditor() {
  if (!editorContainer.value || editorView) return;
  editorView = new EditorView({
    state: EditorState.create({
      doc: content.value,
      extensions: [
        history(),
        keymap.of([
          {
            key: "Mod-s",
            preventDefault: true,
            run: () => {
              save();
              return true;
            },
          },
          indentWithTab,
          ...defaultKeymap,
          ...historyKeymap,
        ]),
        markdown(),
        oneDark,
        EditorView.lineWrapping,
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            content.value = update.state.doc.toString();
          }
        }),
      ],
    }),
    parent: editorContainer.value,
  });
}

function destroyEditor() {
  editorView?.destroy();
  editorView = null;
}

watch(viewMode, async (mode) => {
  if (mode === "edit") {
    await nextTick();
    initEditor();
  } else {
    destroyEditor();
  }
});

// Keep the editor in sync if content is replaced externally
// (eg. after re-loading on route change).
watch(content, (val) => {
  if (!editorView) return;
  const current = editorView.state.doc.toString();
  if (val !== current) {
    editorView.dispatch({
      changes: { from: 0, to: current.length, insert: val },
    });
  }
});

onBeforeRouteLeave(() => {
  if (dirty.value && !confirm("Discard unsaved changes?")) {
    return false;
  }
});

onMounted(async () => {
  await load();
  if (viewMode.value === "edit") {
    await nextTick();
    initEditor();
  }
});

onBeforeUnmount(() => destroyEditor());

watch(
  () => [props.kind, props.name],
  () => {
    content.value = "";
    original.value = "";
    load();
  }
);

function kindLabel(k: string): string {
  if (k === "skills") return "Skill";
  if (k === "commands") return "Command";
  if (k === "agents") return "Agent";
  return k;
}

// ── AI generation modal ──────────────────────────────────────────
type GenerateMode = "refine" | "replace";

const generateOpen = ref(false);
const generatePrompt = ref("");
const generateMode = ref<GenerateMode>("refine");

const aiReady = computed(
  () => aiStatus.value !== null && aiStatus.value.mode !== "none"
);

// "Substantial" = anything beyond the empty frontmatter scaffold we ship
// for new assets. Below this threshold we default to Replace because
// there's effectively nothing worth refining.
const hasSubstantialContent = computed(() => {
  const stripped = content.value
    .replace(/^---[\s\S]*?---\s*/m, "")
    .replace(/^#\s+\S.*$/m, "")
    .trim();
  return stripped.length > 20;
});

function openGenerate() {
  generatePrompt.value = "";
  generateMode.value = hasSubstantialContent.value ? "refine" : "replace";
  generateOpen.value = true;
  aiStore.refreshStatus();
}

async function submitGenerate() {
  if (!kindSafe.value || !aiReady.value || generating.value) return;
  const prompt = generatePrompt.value.trim();
  if (!prompt) return;

  const context = generateMode.value === "refine" ? content.value : undefined;
  const result = await aiStore.generateAsset(kindSafe.value, prompt, context);
  if (!result) return;

  // Drop into edit mode so the user can review what landed before saving.
  // The CodeMirror history makes Cmd-Z available if the result is wrong.
  content.value = result;
  if (viewMode.value !== "edit") viewMode.value = "edit";
  generateOpen.value = false;
  toast.success("Content generated", {
    description: "Review the diff in the editor before saving.",
  });
}

function goToSettingsFromDialog() {
  generateOpen.value = false;
  router.push({ name: "settings" });
}
</script>

<style scoped>
/* CodeMirror — fill the host and align its frame with the page chrome */
.cm-host :deep(.cm-editor) {
  height: 100%;
  font-size: 13px;
}
.cm-host :deep(.cm-scroller) {
  font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
  line-height: 1.55;
  padding: 8px 4px;
}
.cm-host :deep(.cm-content) {
  padding: 12px 12px 32px;
}
.cm-host :deep(.cm-gutters) {
  background: transparent;
  border-right: 1px solid var(--border);
}
.cm-host :deep(.cm-focused) {
  outline: none;
}
</style>

<template>
  <div class="flex h-full flex-col overflow-hidden">
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
            Library / <span class="font-mono">{{ kind }}</span>
          </div>
          <div class="flex items-center gap-2">
            <Badge variant="outline" class="text-[10px]">
              {{ kindLabel(kind) }}
            </Badge>
            <h1 class="truncate text-lg font-semibold">{{ name }}</h1>
            <Badge
              v-if="libraryAsset?.origin"
              variant="secondary"
              class="text-[10px] uppercase tracking-wider"
              :title="`from ${libraryAsset.origin.marketplace} · imported ${libraryAsset.origin.imported_at}`"
            >from {{ libraryAsset.origin.plugin }}</Badge>
          </div>
          <p
            v-if="libraryAsset?.description"
            class="mt-1 text-sm text-muted-foreground"
          >{{ libraryAsset.description }}</p>
        </div>

        <div class="flex shrink-0 items-center gap-2">
          <Button size="sm" variant="outline" @click="openGenerate">
            <Sparkles />
            Generate with AI
          </Button>
          <ToggleGroup
            type="single"
            :model-value="viewMode"
            @update:model-value="(v) => v && (viewMode = v as ViewMode)"
            variant="outline"
            size="sm"
          >
            <ToggleGroupItem value="preview">
              <Eye class="size-3.5" />
              Preview
            </ToggleGroupItem>
            <ToggleGroupItem value="edit">
              <Pencil class="size-3.5" />
              Edit
            </ToggleGroupItem>
          </ToggleGroup>
        </div>
      </div>
    </div>

    <!-- Body -->
    <div class="relative flex-1 min-h-0 overflow-hidden">
      <div
        v-if="loading"
        class="flex h-full items-center justify-center text-sm text-muted-foreground"
      >
        <Loader2 class="mr-2 size-4 animate-spin" />
        Loading…
      </div>

      <div
        v-else-if="loadError"
        class="m-6 rounded-lg border border-destructive/30 bg-destructive/10 p-4"
      >
        <div class="text-sm font-medium text-destructive">
          Couldn't load this asset
        </div>
        <code class="mt-2 block break-words text-xs text-muted-foreground">{{ loadError }}</code>
        <Button class="mt-3" size="sm" variant="outline" @click="router.back()">
          Back
        </Button>
      </div>

      <ScrollArea v-else-if="viewMode === 'preview'" class="h-full">
        <div class="markdown px-6 py-5" v-html="renderedMarkdown" />
      </ScrollArea>

      <div
        v-else
        ref="editorContainer"
        class="cm-host h-full overflow-hidden"
      />
    </div>

    <!-- Footer (edit mode only) -->
    <div
      v-if="viewMode === 'edit' && !loading && !loadError"
      class="flex items-center gap-3 border-t bg-card/40 px-6 py-2.5"
    >
      <span class="text-xs text-muted-foreground">
        <span v-if="dirty" class="text-primary">● Unsaved changes</span>
        <span v-else>Saved</span>
        · Ctrl+S to save
      </span>
      <span
        v-if="saveError"
        class="text-xs text-destructive"
      >{{ saveError }}</span>
      <div class="flex-1" />
      <Button
        size="sm"
        :disabled="!dirty || saving"
        @click="save"
      >
        <Loader2 v-if="saving" class="animate-spin" />
        <Save v-else />
        {{ saving ? "Saving…" : "Save" }}
      </Button>
    </div>

    <!-- AI generation dialog -->
    <Dialog v-model:open="generateOpen">
      <DialogContent class="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle class="flex items-center gap-2">
            <Sparkles class="size-4" />
            Generate with AI
          </DialogTitle>
          <DialogDescription>
            Describe what you want. The result lands in the editor — nothing
            is saved until you click Save.
          </DialogDescription>
        </DialogHeader>

        <form class="space-y-4" @submit.prevent="submitGenerate">
          <div class="space-y-1.5">
            <Label>Strategy</Label>
            <ToggleGroup
              type="single"
              :model-value="generateMode"
              @update:model-value="(v) => v && (generateMode = v as GenerateMode)"
              variant="outline"
              class="w-full"
              :disabled="!hasSubstantialContent"
            >
              <ToggleGroupItem
                value="refine"
                class="flex-1"
                :disabled="!hasSubstantialContent"
              >Refine current content</ToggleGroupItem>
              <ToggleGroupItem value="replace" class="flex-1">
                Replace from scratch
              </ToggleGroupItem>
            </ToggleGroup>
            <p class="text-[11px] text-muted-foreground">
              <template v-if="generateMode === 'refine'">
                The current content is sent as context. Use this to tweak,
                expand, or restructure what's there.
              </template>
              <template v-else>
                Ignores the current content and writes from scratch based
                on your prompt only.
              </template>
            </p>
          </div>

          <div class="space-y-1.5">
            <Label for="ai-prompt">Prompt</Label>
            <Textarea
              id="ai-prompt"
              v-model="generatePrompt"
              rows="5"
              :placeholder="
                generateMode === 'refine'
                  ? 'e.g. Make it more concise, add a section on edge cases.'
                  : 'e.g. A skill that helps refactor Python code to follow PEP 8.'
              "
              autofocus
            />
          </div>

          <div
            v-if="!aiReady"
            class="flex items-start gap-2.5 rounded-md border border-amber-500/40 bg-amber-500/10 p-2.5 text-[11px] text-amber-700 dark:text-amber-300"
          >
            <AlertCircle class="mt-0.5 size-3.5 shrink-0" />
            <div class="flex-1 space-y-1">
              <div>{{ aiStatus?.message ?? "AI backend not detected." }}</div>
              <button
                type="button"
                class="font-medium underline underline-offset-2"
                @click="goToSettingsFromDialog"
              >Open Settings</button>
            </div>
          </div>
          <p v-else class="text-[11px] text-muted-foreground">
            {{ aiStatus?.message }}
          </p>
        </form>

        <DialogFooter>
          <Button
            variant="outline"
            :disabled="generating"
            @click="generateOpen = false"
          >Cancel</Button>
          <Button
            :disabled="!aiReady || !generatePrompt.trim() || generating"
            @click="submitGenerate"
          >
            <Loader2 v-if="generating" class="animate-spin" />
            <Sparkles v-else />
            {{ generating ? "Generating…" : "Generate" }}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </div>
</template>
