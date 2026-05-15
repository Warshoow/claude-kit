<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  AlertCircle,
  Loader2,
  Sparkles,
  Webhook,
} from "lucide-vue-next";
import { toast } from "vue-sonner";
import { api } from "@/lib/api";
import { useAiStore } from "@/stores/ai";
import { storeToRefs } from "pinia";
import { HOOK_EVENTS, type HookEvent } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

const props = defineProps<{
  open: boolean;
  /** When set, edits an existing local hook by filename. Filename
   *  input is then locked. */
  editName?: string;
}>();

const emit = defineEmits<{
  "update:open": [value: boolean];
  saved: [filename: string];
}>();

const aiStore = useAiStore();
const { status: aiStatus } = storeToRefs(aiStore);
const aiReady = computed(
  () => aiStatus.value !== null && aiStatus.value.mode !== "none",
);

const filename = ref("");
const event = ref<HookEvent>("PreToolUse");
const matcher = ref("");
const description = ref("");
const aiPrompt = ref("");
const script = ref("");
const generating = ref(false);
const submitting = ref(false);
const error = ref<string | null>(null);

const isEdit = computed(() => !!props.editName);

watch(
  () => props.open,
  async (v) => {
    if (!v) return;
    error.value = null;
    submitting.value = false;
    aiStore.refreshStatus();
    if (isEdit.value) {
      filename.value = props.editName!;
      try {
        const got = await api.readLocalHook(props.editName!);
        if (got) {
          event.value = (got.meta.event as HookEvent) ?? "PreToolUse";
          matcher.value = got.meta.matcher ?? "";
          description.value = got.meta.description ?? "";
          script.value = got.script;
          aiPrompt.value = "";
        }
      } catch (e) {
        error.value = `Couldn't load existing hook: ${String(e)}`;
      }
    } else {
      filename.value = "";
      event.value = "PreToolUse";
      matcher.value = "";
      description.value = "";
      aiPrompt.value = "";
      script.value = "#!/bin/bash\n# Write your hook script here.\nexit 0\n";
    }
  },
  { immediate: true },
);

function validate(): string | null {
  if (!isEdit.value) {
    const f = filename.value.trim();
    if (!f) return "Filename is required.";
    if (!/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(f)) {
      return "Filename must start with a letter or digit; allowed: letters, digits, '-', '_', '.'.";
    }
    if (f.length > 96) return "Filename too long (max 96).";
    if (f.endsWith(".meta.json")) return "'.meta.json' suffix is reserved.";
  }
  if (!event.value) return "Event is required.";
  if (!script.value.trim()) return "Script body is empty.";
  return null;
}

async function generate() {
  if (!aiReady.value || generating.value) return;
  if (!aiPrompt.value.trim()) {
    error.value = "Describe what the hook should do first.";
    return;
  }
  generating.value = true;
  error.value = null;
  try {
    const out = await api.aiGenerateHook(
      event.value,
      matcher.value.trim(),
      aiPrompt.value.trim(),
    );
    script.value = out;
    if (!description.value.trim()) {
      description.value = aiPrompt.value.trim();
    }
    toast.success("Script generated", {
      description: "Review and edit before saving.",
    });
  } catch (e) {
    error.value = String(e);
    toast.error("AI generation failed", { description: String(e) });
  } finally {
    generating.value = false;
  }
}

async function submit() {
  error.value = null;
  const v = validate();
  if (v) {
    error.value = v;
    return;
  }
  const desc = description.value.trim() || undefined;
  submitting.value = true;
  try {
    if (isEdit.value) {
      await api.updateLocalHook(
        props.editName!,
        event.value,
        matcher.value.trim(),
        desc,
        script.value,
      );
      toast.success(`Updated hook "${props.editName}"`);
      emit("saved", props.editName!);
    } else {
      const f = filename.value.trim();
      await api.createLocalHook(
        f,
        event.value,
        matcher.value.trim(),
        desc,
        script.value,
      );
      toast.success(`Created hook "${f}"`);
      emit("saved", f);
    }
    emit("update:open", false);
  } catch (e) {
    error.value = String(e);
  } finally {
    submitting.value = false;
  }
}

function cancel() {
  emit("update:open", false);
}
</script>

<style scoped>
.code-editor {
  font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
  font-size: 12.5px;
  line-height: 1.55;
  min-height: 360px;
}
</style>

<template>
  <Dialog :open="open" @update:open="emit('update:open', $event)">
    <DialogContent class="max-h-[88vh] overflow-y-auto sm:max-w-3xl">
      <DialogHeader>
        <DialogTitle class="flex items-center gap-2">
          <Webhook class="size-4" />
          {{ isEdit ? `Edit hook "${editName}"` : "New hook" }}
        </DialogTitle>
        <DialogDescription>
          Hooks are scripts Claude Code runs on a specific event. We auto-
          register them in the project's
          <code class="text-[11px]">.claude/settings.json</code> on apply, so
          you don't have to edit it by hand.
        </DialogDescription>
      </DialogHeader>

      <form class="space-y-4" @submit.prevent="submit">
        <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <div class="space-y-1.5">
            <Label for="hook-filename">Filename</Label>
            <Input
              id="hook-filename"
              v-model="filename"
              placeholder="e.g. block-rm.sh"
              :disabled="isEdit"
              autocomplete="off"
            />
            <p class="text-[10.5px] text-muted-foreground">
              Include extension (<code>.sh</code>, <code>.py</code>, etc.).
            </p>
          </div>
          <div class="space-y-1.5">
            <Label for="hook-event">Event</Label>
            <Select
              :model-value="event"
              @update:model-value="(v) => v && (event = v as HookEvent)"
            >
              <SelectTrigger id="hook-event">
                <SelectValue placeholder="Pick an event" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="e in HOOK_EVENTS" :key="e" :value="e">
                  {{ e }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <div class="space-y-1.5">
            <Label for="hook-matcher">Tool matcher</Label>
            <Input
              id="hook-matcher"
              v-model="matcher"
              placeholder="Bash, Edit, * — empty for any tool"
              autocomplete="off"
            />
          </div>
          <div class="space-y-1.5">
            <Label for="hook-desc">Description (optional)</Label>
            <Input
              id="hook-desc"
              v-model="description"
              placeholder="One-line summary"
              autocomplete="off"
            />
          </div>
        </div>

        <div class="space-y-1.5">
          <div class="flex items-center justify-between">
            <Label for="hook-ai">Generate script with AI</Label>
            <span
              v-if="!aiReady"
              class="text-[10.5px] text-amber-700 dark:text-amber-300"
            >AI backend not configured</span>
          </div>
          <Textarea
            id="hook-ai"
            v-model="aiPrompt"
            rows="2"
            placeholder="e.g. Block any rm -rf command from running"
            class="text-[12.5px]"
            :disabled="!aiReady || generating"
          />
          <div class="flex justify-end">
            <Button
              type="button"
              size="sm"
              variant="outline"
              :disabled="!aiReady || generating || !aiPrompt.trim()"
              @click="generate"
            >
              <Loader2 v-if="generating" class="size-3 animate-spin" />
              <Sparkles v-else class="size-3" />
              {{ generating ? "Generating…" : "Generate script" }}
            </Button>
          </div>
        </div>

        <div class="space-y-1.5">
          <Label for="hook-script">Script</Label>
          <Textarea
            id="hook-script"
            v-model="script"
            class="code-editor"
            spellcheck="false"
          />
        </div>

        <div
          v-if="error"
          class="flex items-start gap-2 rounded-md border border-destructive/40 bg-destructive/10 p-2.5 text-[11.5px] text-destructive"
        >
          <AlertCircle class="mt-0.5 size-3.5 shrink-0" />
          <div class="flex-1 break-words">{{ error }}</div>
        </div>
      </form>

      <DialogFooter>
        <Button variant="outline" :disabled="submitting" @click="cancel">
          Cancel
        </Button>
        <Button :disabled="submitting" @click="submit">
          <Loader2 v-if="submitting" class="animate-spin" />
          <Webhook v-else />
          {{ isEdit ? "Save" : "Create" }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
