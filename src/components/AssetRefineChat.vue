<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { storeToRefs } from "pinia";
import {
  AlertCircle,
  ArrowUp,
  ChevronDown,
  Loader2,
  MessageSquare,
  Sparkles,
  Undo2,
  X,
} from "lucide-vue-next";
import { toast } from "vue-sonner";
import { api } from "@/lib/api";
import { useAiStore } from "@/stores/ai";
import type { AssetKind, ChatMessage } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";

const props = defineProps<{
  kind: AssetKind;
  currentContent: string;
  open: boolean;
}>();

const emit = defineEmits<{
  "update:currentContent": [value: string];
  "update:open": [value: boolean];
}>();

const aiStore = useAiStore();
const { status: aiStatus } = storeToRefs(aiStore);

const aiReady = computed(
  () => aiStatus.value !== null && aiStatus.value.mode !== "none",
);

// Chat state. Messages alternate user → assistant. Assistant turns
// hold the *full asset content* the model produced — we'll surface a
// collapsed view in the bubble.
const messages = ref<ChatMessage[]>([]);
const input = ref("");
const streaming = ref(false);
const streamingBuffer = ref("");
const expanded = ref<Set<number>>(new Set());

// One snapshot per user turn taken BEFORE the AI reply lands. Pressing
// Undo restores both the editor content and the messages list to the
// state right before that turn — auto-apply with proper rollback.
type Snapshot = { messages: ChatMessage[]; content: string };
const undoStack = ref<Snapshot[]>([]);

const scrollHost = ref<HTMLElement | null>(null);
async function scrollToBottom() {
  await nextTick();
  if (scrollHost.value) {
    scrollHost.value.scrollTop = scrollHost.value.scrollHeight;
  }
}

watch([messages, streamingBuffer], scrollToBottom, { deep: true });
watch(
  () => props.open,
  (v) => {
    if (v) scrollToBottom();
  },
);

async function send() {
  const text = input.value.trim();
  if (!text || streaming.value || !aiReady.value) return;

  // Snapshot the world before this turn so Undo can rewind it.
  undoStack.value.push({
    messages: messages.value.map((m) => ({ ...m })),
    content: props.currentContent,
  });

  const userMsg: ChatMessage = { role: "user", content: text };
  messages.value.push(userMsg);
  input.value = "";

  // Build history exclusive of the just-pushed user message — the
  // backend appends it from `userMessage` separately.
  const history = messages.value.slice(0, -1);

  streaming.value = true;
  streamingBuffer.value = "";

  try {
    await api.refineAssetChat(
      props.kind,
      props.currentContent,
      history,
      text,
      (ev) => {
        if (ev.event === "token") {
          streamingBuffer.value += ev.data.delta;
        } else if (ev.event === "done") {
          const finalContent = stripFences(ev.data.content);
          messages.value.push({ role: "assistant", content: finalContent });
          emit("update:currentContent", finalContent);
          streamingBuffer.value = "";
          streaming.value = false;
        } else if (ev.event === "error") {
          streaming.value = false;
          streamingBuffer.value = "";
          // Roll back the user message we optimistically appended +
          // discard the snapshot — nothing happened, no need to undo.
          messages.value.pop();
          undoStack.value.pop();
          toast.error("Refine failed", { description: ev.data.message });
        }
      },
    );
  } catch (e) {
    streaming.value = false;
    streamingBuffer.value = "";
    messages.value.pop();
    undoStack.value.pop();
    toast.error("Refine failed", { description: String(e) });
  }
}

function undo() {
  const snap = undoStack.value.pop();
  if (!snap) return;
  messages.value = snap.messages;
  emit("update:currentContent", snap.content);
  // Drop any expansion state pointing past the new length.
  expanded.value = new Set(
    [...expanded.value].filter((i) => i < messages.value.length),
  );
}

function clearChat() {
  if (streaming.value) return;
  if (
    messages.value.length > 0 &&
    !confirm("Clear the chat? The editor content stays — only the history is reset.")
  )
    return;
  messages.value = [];
  undoStack.value = [];
  expanded.value = new Set();
}

function toggleExpand(idx: number) {
  const next = new Set(expanded.value);
  if (next.has(idx)) next.delete(idx);
  else next.add(idx);
  expanded.value = next;
}

function lineCount(s: string): number {
  return s.split("\n").length;
}

function firstNonEmptyLine(s: string): string {
  for (const line of s.split("\n")) {
    const t = line.trim();
    if (t) return t;
  }
  return "(empty)";
}

// Some models keep wrapping output in ```...``` even when told not to.
// Mirror the backend's strip_code_fences so the editor never inherits
// a stray fence when the user pops the bubble open and copies content.
function stripFences(s: string): string {
  const trimmed = s.trim();
  if (trimmed.startsWith("```")) {
    const firstNL = trimmed.indexOf("\n");
    if (firstNL > 0) {
      const body = trimmed.slice(firstNL + 1);
      const endFence = body.lastIndexOf("```");
      if (endFence >= 0) return body.slice(0, endFence).trimEnd();
    }
  }
  return trimmed;
}

function close() {
  emit("update:open", false);
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
    e.preventDefault();
    send();
  }
}

const canUndo = computed(() => undoStack.value.length > 0 && !streaming.value);
</script>

<style scoped>
.chat-pre {
  font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
  font-size: 11.5px;
  line-height: 1.55;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>

<template>
  <aside
    v-if="open"
    class="flex h-full w-[380px] shrink-0 flex-col border-l bg-card/30"
  >
    <!-- Header -->
    <div class="flex items-center gap-2 border-b px-3 py-2.5">
      <MessageSquare class="size-3.5 text-primary" />
      <span class="text-sm font-medium">Refine with AI</span>
      <span class="text-[10px] uppercase tracking-wider text-muted-foreground">
        chat
      </span>
      <div class="flex-1" />
      <Button
        v-if="messages.length > 0"
        size="sm"
        variant="ghost"
        class="h-7 px-2 text-[11px]"
        :disabled="streaming"
        @click="clearChat"
      >Clear</Button>
      <Button
        size="sm"
        variant="ghost"
        class="size-7 p-0"
        @click="close"
        title="Close panel"
      >
        <X class="size-3.5" />
      </Button>
    </div>

    <!-- Messages -->
    <div
      ref="scrollHost"
      class="flex-1 space-y-3 overflow-y-auto px-3 py-3"
    >
      <!-- Empty state -->
      <div
        v-if="messages.length === 0 && !streaming"
        class="rounded-lg border border-dashed border-border/60 bg-background/60 p-3 text-[12px] text-muted-foreground"
      >
        <p class="mb-1.5 font-medium text-foreground">
          Describe how to refine this asset.
        </p>
        <ul class="space-y-1 text-[11.5px]">
          <li>· "Make the description more specific."</li>
          <li>· "Add a section on common edge cases."</li>
          <li>· "Shorten the body to 5 steps."</li>
        </ul>
        <p class="mt-2 text-[10.5px] opacity-70">
          Each reply auto-applies to the editor — use Undo to rewind a turn.
        </p>
      </div>

      <!-- Existing turns -->
      <template v-for="(msg, idx) in messages" :key="idx">
        <!-- User bubble -->
        <div v-if="msg.role === 'user'" class="flex justify-end">
          <div
            class="max-w-[90%] rounded-lg rounded-tr-sm bg-primary/10 px-3 py-2 text-[12.5px] leading-snug"
          >{{ msg.content }}</div>
        </div>

        <!-- Assistant bubble: collapsed view of the produced asset -->
        <div v-else class="space-y-1">
          <div
            class="flex items-center gap-1.5 text-[10.5px] uppercase tracking-wider text-muted-foreground"
          >
            <Sparkles class="size-3 text-primary" />
            <span>Updated · {{ lineCount(msg.content) }} lines</span>
          </div>
          <div
            class="rounded-lg border bg-background/60 px-2.5 py-2"
          >
            <button
              class="flex w-full items-center gap-1.5 text-left text-[11.5px] hover:text-primary"
              @click="toggleExpand(idx)"
            >
              <ChevronDown
                class="size-3 shrink-0 transition-transform"
                :class="expanded.has(idx) ? '' : '-rotate-90'"
              />
              <span class="truncate font-mono opacity-80">{{
                firstNonEmptyLine(msg.content)
              }}</span>
            </button>
            <pre
              v-if="expanded.has(idx)"
              class="chat-pre mt-2 max-h-[260px] overflow-auto rounded bg-muted/40 p-2"
            >{{ msg.content }}</pre>
          </div>
        </div>
      </template>

      <!-- Live streaming bubble -->
      <div v-if="streaming" class="space-y-1">
        <div
          class="flex items-center gap-1.5 text-[10.5px] uppercase tracking-wider text-muted-foreground"
        >
          <Loader2 class="size-3 animate-spin text-primary" />
          <span>Generating…</span>
        </div>
        <pre
          class="chat-pre max-h-[280px] overflow-auto rounded-lg border bg-background/60 px-2.5 py-2"
        >{{ streamingBuffer || "…" }}</pre>
      </div>
    </div>

    <!-- AI not configured warning -->
    <div
      v-if="!aiReady"
      class="mx-3 mb-2 flex items-start gap-2 rounded-md border border-amber-500/40 bg-amber-500/10 p-2 text-[11px] text-amber-700 dark:text-amber-300"
    >
      <AlertCircle class="mt-0.5 size-3.5 shrink-0" />
      <div class="flex-1">
        {{ aiStatus?.message ?? "AI backend not detected." }}
      </div>
    </div>

    <!-- Input -->
    <div class="border-t bg-card/40 p-2.5">
      <Textarea
        v-model="input"
        rows="3"
        placeholder="Describe the change… (⌘/Ctrl+Enter to send)"
        class="min-h-[64px] resize-none text-[12.5px]"
        :disabled="streaming || !aiReady"
        @keydown="handleKeydown"
      />
      <div class="mt-2 flex items-center gap-2">
        <Button
          size="sm"
          variant="outline"
          class="h-7 px-2 text-[11px]"
          :disabled="!canUndo"
          @click="undo"
          title="Undo the last turn"
        >
          <Undo2 class="size-3" />
          Undo turn
        </Button>
        <div class="flex-1" />
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
</template>
