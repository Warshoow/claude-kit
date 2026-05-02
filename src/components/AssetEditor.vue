<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Loader2 } from "lucide-vue-next";
import type { Asset } from "@/lib/types";
import { api } from "@/lib/api";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";

const props = defineProps<{ asset: Asset | null }>();
const emit = defineEmits<{
  close: [];
  saved: [];
}>();

const content = ref("");
const original = ref("");
const loading = ref(false);
const saving = ref(false);
const error = ref<string | null>(null);

const open = computed(() => props.asset !== null);
const dirty = computed(() => content.value !== original.value);

watch(
  () => props.asset,
  async (a) => {
    error.value = null;
    content.value = "";
    original.value = "";
    if (!a) return;
    loading.value = true;
    try {
      const text = await api.readAsset(a.kind, a.name);
      content.value = text;
      original.value = text;
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  },
  { immediate: true }
);

async function save() {
  if (!props.asset) return;
  saving.value = true;
  error.value = null;
  try {
    await api.writeAsset(props.asset.kind, props.asset.name, content.value);
    original.value = content.value;
    emit("saved");
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}

function onOpenChange(value: boolean) {
  if (value) return;
  if (dirty.value && !confirm("Discard unsaved changes?")) return;
  emit("close");
}

function onKey(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === "s") {
    e.preventDefault();
    save();
  }
}
</script>

<template>
  <Dialog :open="open" @update:open="onOpenChange">
    <DialogContent
      class="flex h-[min(720px,88vh)] w-[min(900px,90vw)] max-w-none flex-col gap-0 p-0 sm:max-w-none"
    >
      <DialogHeader class="border-b px-6 pt-6 pb-4">
        <DialogTitle class="font-mono text-sm font-semibold">
          <template v-if="asset">{{ asset.kind }} / {{ asset.name }}</template>
        </DialogTitle>
        <DialogDescription
          class="truncate font-mono text-[11px] text-muted-foreground"
          :title="asset?.path"
        >{{ asset?.path }}</DialogDescription>
      </DialogHeader>

      <div class="relative flex-1 overflow-hidden">
        <div
          v-if="loading"
          class="flex h-full items-center justify-center text-sm text-muted-foreground"
        >
          <Loader2 class="mr-2 size-4 animate-spin" />
          Loading…
        </div>
        <textarea
          v-else
          v-model="content"
          class="size-full resize-none border-0 bg-background p-4 font-mono text-[13px] leading-relaxed outline-none"
          spellcheck="false"
          @keydown="onKey"
        />
      </div>

      <div
        v-if="error"
        class="border-t border-destructive/30 bg-destructive/10 px-6 py-2 text-xs text-destructive"
      >{{ error }}</div>

      <DialogFooter class="flex flex-row items-center gap-3 border-t bg-card/40 px-6 py-3 sm:justify-start">
        <span class="text-[11px] text-muted-foreground">
          <span v-if="dirty" class="text-primary">● unsaved</span>
          <span v-else>saved</span>
          · Ctrl+S to save · Esc to close
        </span>
        <div class="flex-1" />
        <Button
          :disabled="saving || loading || !dirty"
          @click="save"
        >
          <Loader2 v-if="saving" class="animate-spin" />
          {{ saving ? "Saving…" : "Save" }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
