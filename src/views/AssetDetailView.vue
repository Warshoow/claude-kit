<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { onBeforeRouteLeave, useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import { marked } from "marked";
import DOMPurify from "dompurify";
import {
  ChevronLeft,
  Eye,
  Loader2,
  Pencil,
  Save,
} from "lucide-vue-next";
import { toast } from "vue-sonner";
import { EditorView, keymap } from "@codemirror/view";
import { EditorState } from "@codemirror/state";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { markdown } from "@codemirror/lang-markdown";
import { oneDark } from "@codemirror/theme-one-dark";
import { useAppStore } from "@/stores/app";
import { api } from "@/lib/api";
import type { AssetKind } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  ToggleGroup,
  ToggleGroupItem,
} from "@/components/ui/toggle-group";

const props = defineProps<{ kind: string; name: string }>();

const router = useRouter();
const store = useAppStore();
const { library } = storeToRefs(store);

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
  </div>
</template>
