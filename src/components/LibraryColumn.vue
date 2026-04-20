<script setup lang="ts">
import { computed, ref } from "vue";
import type { Asset, AssetKind } from "../lib/types";
import { assetKey } from "../lib/types";

const props = defineProps<{
  library: Asset[];
  installedKeys: Set<string>;
  canInstall: boolean;
}>();

const emit = defineEmits<{
  toggle: [asset: Asset];
  edit: [asset: Asset];
  import: [];
}>();

const query = ref("");

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return props.library;
  return props.library.filter((a) => {
    if (a.name.toLowerCase().includes(q)) return true;
    if (a.description?.toLowerCase().includes(q)) return true;
    if (a.tags?.some((t) => t.toLowerCase().includes(q))) return true;
    if (a.kind.toLowerCase().includes(q)) return true;
    return false;
  });
});

const grouped = computed(() => {
  const g: Record<AssetKind, Asset[]> = { skills: [], commands: [], agents: [] };
  for (const a of filtered.value) g[a.kind].push(a);
  return g;
});

function onDragStart(e: DragEvent, a: Asset) {
  if (!e.dataTransfer) return;
  e.dataTransfer.effectAllowed = "copy";
  e.dataTransfer.setData(
    "application/claude-asset",
    JSON.stringify({ kind: a.kind, name: a.name })
  );
  e.dataTransfer.setData("text/plain", `${a.kind}/${a.name}`);
}
</script>

<template>
  <div class="column">
    <div class="col-header">
      Library
      <span class="count">{{ filtered.length }}<span v-if="filtered.length !== library.length">/{{ library.length }}</span></span>
      <div style="flex:1" />
      <button @click="emit('import')" title="Import assets from a plugin folder">Import…</button>
    </div>
    <div class="col-search">
      <input
        v-model="query"
        type="text"
        placeholder="Search name, description, tag…"
      />
      <button v-if="query" class="clear-btn" @click="query = ''" title="Clear">×</button>
    </div>
    <div class="col-body">
      <template v-if="library.length === 0">
        <div class="empty">
          Empty library.<br />
          Add files to <code>~/.claude-assets/library/</code> or import a plugin.
        </div>
      </template>
      <template v-else-if="filtered.length === 0">
        <div class="empty">No match for "{{ query }}".</div>
      </template>
      <template v-else>
        <div v-for="kind in (['skills', 'commands', 'agents'] as AssetKind[])" :key="kind" class="kind-group">
          <div v-if="grouped[kind].length > 0" class="kind-label">
            {{ kind }} ({{ grouped[kind].length }})
          </div>
          <div
            v-for="a in grouped[kind]"
            :key="assetKey(a)"
            class="asset-item"
            :class="{ installed: installedKeys.has(assetKey(a)) }"
            draggable="true"
            @dragstart="onDragStart($event, a)"
            @click="canInstall && emit('toggle', a)"
          >
            <input
              type="checkbox"
              :checked="installedKeys.has(assetKey(a))"
              :disabled="!canInstall"
              @click.stop="canInstall && emit('toggle', a)"
            />
            <div class="asset-body">
              <div class="asset-name">{{ a.name }}</div>
              <div v-if="a.description" class="asset-desc">{{ a.description }}</div>
            </div>
            <button
              class="icon-btn"
              title="Edit content"
              @click.stop="emit('edit', a)"
            >✎</button>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>
