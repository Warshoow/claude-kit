<script setup lang="ts">
import { ref, watch } from "vue";
import type { Asset } from "../lib/types";
import { api } from "../lib/api";

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
    } catch (e: any) {
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
  } catch (e: any) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}

function onClose() {
  if (content.value !== original.value) {
    if (!confirm("Discard unsaved changes?")) return;
  }
  emit("close");
}

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") onClose();
  if ((e.ctrlKey || e.metaKey) && e.key === "s") {
    e.preventDefault();
    save();
  }
}
</script>

<template>
  <div v-if="asset" class="modal-backdrop" @click.self="onClose" @keydown="onKey" tabindex="-1">
    <div class="modal">
      <div class="modal-header">
        <div>
          <div class="modal-title">{{ asset.kind }} / {{ asset.name }}</div>
          <div class="modal-subtitle">{{ asset.path }}</div>
        </div>
        <button @click="onClose">Close</button>
      </div>
      <div class="modal-body">
        <div v-if="loading" class="empty">Loading…</div>
        <textarea
          v-else
          v-model="content"
          class="editor-textarea"
          spellcheck="false"
          @keydown="onKey"
        />
      </div>
      <div v-if="error" class="modal-error">{{ error }}</div>
      <div class="modal-footer">
        <span class="modal-hint">
          <span v-if="content !== original">● unsaved</span>
          <span v-else>saved</span>
          · Ctrl+S to save · Esc to close
        </span>
        <div style="flex:1" />
        <button
          class="primary"
          :disabled="saving || loading || content === original"
          @click="save"
        >
          {{ saving ? "Saving…" : "Save" }}
        </button>
      </div>
    </div>
  </div>
</template>
