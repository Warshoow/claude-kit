<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import {
  AlertCircle,
  ArrowUp,
  ChevronDown,
  ChevronLeft,
  Loader2,
  MessageSquare,
  PackagePlus,
  Sparkles,
  Trash2,
  Undo2,
} from "lucide-vue-next";
import { toast } from "vue-sonner";
import { api } from "@/lib/api";
import { useAiStore } from "@/stores/ai";
import { useAppStore } from "@/stores/app";
import type {
  AssetKind,
  ChatMessage,
  GeneratedAsset,
} from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";

const router = useRouter();
const aiStore = useAiStore();
const appStore = useAppStore();
const { status: aiStatus } = storeToRefs(aiStore);

const aiReady = computed(
  () => aiStatus.value !== null && aiStatus.value.mode !== "none",
);

// ── Chat state ────────────────────────────────────────────────
const messages = ref<ChatMessage[]>([]);
const input = ref("");
const streaming = ref(false);
const streamingBuffer = ref("");

// ── Generated state ───────────────────────────────────────────
// Assets and bundle metadata are derived from the latest assistant
// turn; bundleName / bundleDescription stay editable so the user can
// override the model's suggestion before clicking Create.
const generatedAssets = ref<GeneratedAsset[]>([]);
const bundleName = ref("");
const bundleDescription = ref("");
const userTouchedName = ref(false);
const userTouchedDesc = ref(false);
const expanded = ref<Set<string>>(new Set());

// ── Undo stack ────────────────────────────────────────────────
// One snapshot per user turn taken BEFORE the model replies.
type Snapshot = {
  messages: ChatMessage[];
  assets: GeneratedAsset[];
  bundleName: string;
  bundleDescription: string;
  userTouchedName: boolean;
  userTouchedDesc: boolean;
};
const undoStack = ref<Snapshot[]>([]);

// ── Create flow ───────────────────────────────────────────────
const creating = ref(false);
const createError = ref<string | null>(null);

const canCreate = computed(
  () =>
    !streaming.value &&
    !creating.value &&
    generatedAssets.value.length > 0 &&
    bundleName.value.trim().length > 0,
);

const scrollHost = ref<HTMLElement | null>(null);
async function scrollChatToBottom() {
  await nextTick();
  if (scrollHost.value) {
    scrollHost.value.scrollTop = scrollHost.value.scrollHeight;
  }
}

watch([messages, streamingBuffer], scrollChatToBottom, { deep: true });

onMounted(() => {
  aiStore.refreshStatus();
});

function assetKey(a: GeneratedAsset): string {
  return `${a.kind}:${a.name}`;
}

function toggleExpand(key: string) {
  const next = new Set(expanded.value);
  if (next.has(key)) next.delete(key);
  else next.add(key);
  expanded.value = next;
}

function kindLabel(k: AssetKind): string {
  if (k === "skills") return "Skill";
  if (k === "commands") return "Command";
  return "Agent";
}

function hasBundleBlocks(content: string): boolean {
  return content.includes("<<<ASSET-BEGIN:");
}

function assistantDisplayText(content: string): string {
  return content
    .replace(/BUNDLE-META:[\s\S]*?END-BUNDLE-META/g, "")
    .replace(/<<<ASSET-BEGIN:[\s\S]*?<<<ASSET-END>>>/g, "")
    .trim();
}

function descriptionFromContent(content: string): string {
  // Pull the `description:` line from the YAML frontmatter for a
  // quick preview without parsing the full document.
  const fm = content.match(/^---\s*([\s\S]*?)\s*---/);
  if (!fm) return "";
  const m = fm[1].match(/^\s*description:\s*(.+)$/m);
  return m ? m[1].trim() : "";
}

function bodyPreview(content: string): string {
  const stripped = content.replace(/^---[\s\S]*?---\s*/m, "").trim();
  return stripped.slice(0, 240);
}

async function send() {
  const text = input.value.trim();
  if (!text || streaming.value || !aiReady.value) return;

  // Snapshot the whole world before this turn — Undo restores it
  // wholesale (chat + generated state + form state).
  undoStack.value.push({
    messages: messages.value.map((m) => ({ ...m })),
    assets: generatedAssets.value.map((a) => ({ ...a })),
    bundleName: bundleName.value,
    bundleDescription: bundleDescription.value,
    userTouchedName: userTouchedName.value,
    userTouchedDesc: userTouchedDesc.value,
  });

  messages.value.push({ role: "user", content: text });
  input.value = "";
  const history = messages.value.slice(0, -1);

  streaming.value = true;
  streamingBuffer.value = "";
  createError.value = null;

  try {
    await api.generateBundleChat(history, text, (ev) => {
      if (ev.event === "token") {
        streamingBuffer.value += ev.data.delta;
      } else if (ev.event === "done") {
        // Store the raw model output as the assistant turn so the
        // backend can replay context on the next refinement.
        messages.value.push({
          role: "assistant",
          content: streamingBuffer.value,
        });
        // Only replace the asset list when the model actually emitted
        // bundle blocks. An empty assets array means a conversational
        // reply — keep whatever is already on the left.
        if (ev.data.assets.length > 0) {
          generatedAssets.value = ev.data.assets;
          expanded.value = new Set();
          if (!userTouchedName.value && ev.data.bundle_name) {
            bundleName.value = ev.data.bundle_name;
          }
          if (!userTouchedDesc.value && ev.data.bundle_description) {
            bundleDescription.value = ev.data.bundle_description;
          }
        }
        streamingBuffer.value = "";
        streaming.value = false;
      } else if (ev.event === "error") {
        streaming.value = false;
        streamingBuffer.value = "";
        // Roll back the optimistic user message + the snapshot —
        // nothing actually changed.
        messages.value.pop();
        undoStack.value.pop();
        toast.error("Generation failed", { description: ev.data.message });
      }
    });
  } catch (e) {
    streaming.value = false;
    streamingBuffer.value = "";
    messages.value.pop();
    undoStack.value.pop();
    toast.error("Generation failed", { description: String(e) });
  }
}

function undo() {
  const snap = undoStack.value.pop();
  if (!snap) return;
  messages.value = snap.messages;
  generatedAssets.value = snap.assets;
  bundleName.value = snap.bundleName;
  bundleDescription.value = snap.bundleDescription;
  userTouchedName.value = snap.userTouchedName;
  userTouchedDesc.value = snap.userTouchedDesc;
  expanded.value = new Set();
  createError.value = null;
}

function clearAll() {
  if (streaming.value) return;
  if (
    messages.value.length > 0 &&
    !confirm("Clear the chat and start over? Nothing has been saved yet.")
  )
    return;
  messages.value = [];
  generatedAssets.value = [];
  undoStack.value = [];
  bundleName.value = "";
  bundleDescription.value = "";
  userTouchedName.value = false;
  userTouchedDesc.value = false;
  expanded.value = new Set();
  createError.value = null;
}

async function create() {
  if (!canCreate.value) return;
  creating.value = true;
  createError.value = null;
  try {
    const res = await api.materializeGeneratedBundle(
      bundleName.value.trim(),
      bundleDescription.value.trim() || undefined,
      generatedAssets.value,
    );
    toast.success(`Bundle "${res.bundle_name}" created`, {
      description: `${res.asset_count} asset${res.asset_count === 1 ? "" : "s"} written`,
    });
    // Refresh so the new bundle + assets show up in the rest of the UI.
    await Promise.all([
      appStore.refreshBundles(),
      appStore.refreshLibrary(),
    ]);
    router.push({ name: "bundle-detail", params: { name: res.bundle_name } });
  } catch (e) {
    createError.value = String(e);
    toast.error("Couldn't create bundle", { description: String(e) });
  } finally {
    creating.value = false;
  }
}

function handleChatKey(e: KeyboardEvent) {
  if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
    e.preventDefault();
    send();
  }
}

const canUndo = computed(() => undoStack.value.length > 0 && !streaming.value);
</script>

<style scoped>
.asset-pre {
  font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
  font-size: 11.5px;
  line-height: 1.55;
  white-space: pre-wrap;
  word-break: break-word;
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
        <div class="min-w-0">
          <div class="mb-1 text-[11px] font-medium text-muted-foreground">
            Bundles / <span class="font-mono">generate</span>
          </div>
          <h1 class="text-lg font-semibold">Generate a bundle from chat</h1>
          <p class="mt-1 max-w-xl text-sm text-muted-foreground">
            Describe what you want and the model produces a coherent set of
            skills, commands and agents you can refine through conversation —
            nothing is written to disk until you click Create.
          </p>
        </div>

        <div class="flex shrink-0 items-center gap-2">
          <Button
            size="sm"
            variant="ghost"
            :disabled="streaming || creating"
            @click="clearAll"
          >
            <Trash2 />
            Clear
          </Button>
          <Button
            size="sm"
            :disabled="!canCreate"
            @click="create"
          >
            <Loader2 v-if="creating" class="animate-spin" />
            <PackagePlus v-else />
            Create bundle
          </Button>
        </div>
      </div>
    </div>

    <!-- Body: split layout -->
    <div class="flex flex-1 min-h-0 overflow-hidden">
      <!-- Left: generated assets + bundle metadata -->
      <div class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
        <div class="border-b bg-background/40 px-6 py-3">
          <div class="grid grid-cols-1 gap-3 md:grid-cols-2">
            <div class="space-y-1">
              <label class="text-[11px] font-medium text-muted-foreground">
                Bundle name
              </label>
              <Input
                v-model="bundleName"
                placeholder="e.g. python-backend"
                class="h-8 text-[13px]"
                @input="userTouchedName = true"
              />
            </div>
            <div class="space-y-1">
              <label class="text-[11px] font-medium text-muted-foreground">
                Description (optional)
              </label>
              <Input
                v-model="bundleDescription"
                placeholder="One-sentence summary"
                class="h-8 text-[13px]"
                @input="userTouchedDesc = true"
              />
            </div>
          </div>
          <p
            v-if="createError"
            class="mt-2 text-[11px] text-destructive"
          >{{ createError }}</p>
        </div>

        <div class="flex-1 min-h-0 overflow-y-auto">
          <div class="space-y-3 px-6 py-4">
            <!-- Empty state -->
            <div
              v-if="generatedAssets.length === 0 && !streaming"
              class="rounded-lg border border-dashed border-border/60 bg-background/40 p-5 text-sm text-muted-foreground"
            >
              <p class="mb-2 font-medium text-foreground">
                No assets yet. Start a conversation on the right.
              </p>
              <p class="text-[12.5px]">
                Try something like
                <em>"A Python backend bundle with pytest, ruff, and a
                run-tests command"</em> — the model will return a complete
                bundle you can iterate on.
              </p>
            </div>

            <!-- Asset list -->
            <template v-for="asset in generatedAssets" :key="assetKey(asset)">
              <div class="rounded-lg border bg-card/40">
                <button
                  class="flex w-full items-start gap-2 px-3 py-2 text-left transition-colors hover:bg-card/60"
                  @click="toggleExpand(assetKey(asset))"
                >
                  <ChevronDown
                    class="mt-0.5 size-3.5 shrink-0 transition-transform"
                    :class="expanded.has(assetKey(asset)) ? '' : '-rotate-90'"
                  />
                  <div class="min-w-0 flex-1">
                    <div class="flex items-center gap-2">
                      <Badge variant="outline" class="text-[10px]">
                        {{ kindLabel(asset.kind) }}
                      </Badge>
                      <span class="truncate font-mono text-[13px] font-medium">
                        {{ asset.name }}
                      </span>
                    </div>
                    <p
                      v-if="descriptionFromContent(asset.content)"
                      class="mt-1 line-clamp-2 text-[12px] text-muted-foreground"
                    >{{ descriptionFromContent(asset.content) }}</p>
                  </div>
                </button>
                <div v-if="expanded.has(assetKey(asset))" class="border-t bg-muted/20 px-3 py-2">
                  <pre class="asset-pre max-h-[420px] overflow-auto">{{ asset.content }}</pre>
                </div>
                <div
                  v-else
                  class="border-t bg-muted/10 px-3 py-1.5 text-[11px] text-muted-foreground"
                >{{ bodyPreview(asset.content) || "(empty body)" }}</div>
              </div>
            </template>

            <!-- Live stream preview while generating -->
            <div
              v-if="streaming"
              class="rounded-lg border bg-card/40 px-3 py-2"
            >
              <div
                class="mb-1.5 flex items-center gap-1.5 text-[10.5px] uppercase tracking-wider text-muted-foreground"
              >
                <Loader2 class="size-3 animate-spin text-primary" />
                <span>Generating…</span>
              </div>
              <pre
                class="asset-pre max-h-[340px] overflow-auto rounded bg-muted/30 px-2 py-2"
              >{{ streamingBuffer || "…" }}</pre>
            </div>
          </div>
        </div>
      </div>

      <!-- Right: chat panel -->
      <aside
        class="flex h-full w-[400px] shrink-0 flex-col border-l bg-card/30"
      >
        <div class="flex items-center gap-2 border-b px-3 py-2.5">
          <MessageSquare class="size-3.5 text-primary" />
          <span class="text-sm font-medium">Conversation</span>
          <div class="flex-1" />
          <Button
            size="sm"
            variant="outline"
            class="h-7 px-2 text-[11px]"
            :disabled="!canUndo"
            @click="undo"
            title="Undo the last turn"
          >
            <Undo2 class="size-3" />
            Undo
          </Button>
        </div>

        <div
          ref="scrollHost"
          class="flex-1 space-y-3 overflow-y-auto px-3 py-3"
        >
          <div
            v-if="messages.length === 0 && !streaming"
            class="rounded-lg border border-dashed border-border/60 bg-background/60 p-3 text-[12px] text-muted-foreground"
          >
            <p class="mb-1.5 font-medium text-foreground">
              Describe the bundle you want.
            </p>
            <ul class="space-y-1 text-[11.5px]">
              <li>· "Python backend with pytest + ruff"</li>
              <li>· "Bundle for working on Tauri apps"</li>
              <li>· "Code review helper with style + security checks"</li>
            </ul>
            <p class="mt-2 text-[10.5px] opacity-70">
              Each reply replaces the asset list on the left. Use Undo to
              rewind a turn before clicking Create.
            </p>
          </div>

          <template v-for="(msg, idx) in messages" :key="idx">
            <div v-if="msg.role === 'user'" class="flex justify-end">
              <div
                class="max-w-[90%] rounded-lg rounded-tr-sm bg-primary/10 px-3 py-2 text-[12.5px] leading-snug"
              >{{ msg.content }}</div>
            </div>
            <div v-else class="space-y-1.5">
              <!-- Natural-language reply text (stripped of bundle blocks) -->
              <div
                v-if="assistantDisplayText(msg.content)"
                class="rounded-lg border bg-background/60 px-2.5 py-2 text-[12px] leading-relaxed text-foreground whitespace-pre-wrap"
              >{{ assistantDisplayText(msg.content) }}</div>
              <!-- Bundle-updated pill — only when model emitted asset blocks -->
              <div
                v-if="hasBundleBlocks(msg.content)"
                class="flex items-center gap-1.5 text-[10.5px] uppercase tracking-wider text-muted-foreground"
              >
                <Sparkles class="size-3 text-primary" />
                <span>Bundle updated — see assets on the left</span>
              </div>
            </div>
          </template>

          <div v-if="streaming" class="space-y-1">
            <div
              class="flex items-center gap-1.5 text-[10.5px] uppercase tracking-wider text-muted-foreground"
            >
              <Loader2 class="size-3 animate-spin text-primary" />
              <span>Generating bundle…</span>
            </div>
          </div>
        </div>

        <div
          v-if="!aiReady"
          class="mx-3 mb-2 flex items-start gap-2 rounded-md border border-amber-500/40 bg-amber-500/10 p-2 text-[11px] text-amber-700 dark:text-amber-300"
        >
          <AlertCircle class="mt-0.5 size-3.5 shrink-0" />
          <div class="flex-1">
            {{ aiStatus?.message ?? "AI backend not detected." }}
          </div>
        </div>

        <div class="border-t bg-card/40 p-2.5">
          <Textarea
            v-model="input"
            rows="3"
            placeholder="Describe what you want… (⌘/Ctrl+Enter to send)"
            class="min-h-[64px] resize-none text-[12.5px]"
            :disabled="streaming || !aiReady"
            @keydown="handleChatKey"
          />
          <div class="mt-2 flex items-center justify-end">
            <Button
              size="sm"
              class="h-7 px-3 text-[11.5px]"
              :disabled="!input.trim() || streaming || !aiReady"
              @click="send"
            >
              <Loader2 v-if="streaming" class="size-3 animate-spin" />
              <ArrowUp v-else class="size-3" />
              Send
            </Button>
          </div>
        </div>
      </aside>
    </div>
  </div>
</template>
