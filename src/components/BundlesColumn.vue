<script setup lang="ts">
import { ref, computed } from "vue";
import type { Asset, Bundle, BundleRef } from "../lib/types";
import { assetKey } from "../lib/types";

const props = defineProps<{
  bundles: Bundle[];
  library: Asset[];
  selected: string | null;
  canApply: boolean;
}>();

const emit = defineEmits<{
  select: [name: string];
  create: [name: string, description?: string];
  delete: [name: string];
  update: [bundle: Bundle];
  apply: [name: string, replace: boolean];
}>();

const creating = ref(false);
const newName = ref("");
const newDesc = ref("");
const dragOver = ref<string | null>(null);

const libraryKeys = computed(
  () => new Set(props.library.map((a) => assetKey(a)))
);

function submitCreate() {
  const name = newName.value.trim();
  if (!name) return;
  emit("create", name, newDesc.value.trim() || undefined);
  newName.value = "";
  newDesc.value = "";
  creating.value = false;
}

function toggleAssetInBundle(b: Bundle, ref: BundleRef) {
  const idx = b.assets.findIndex(
    (x) => x.kind === ref.kind && x.name === ref.name
  );
  const assets =
    idx >= 0
      ? b.assets.filter((_, i) => i !== idx)
      : [...b.assets, ref];
  emit("update", { ...b, assets });
}

function isInBundle(b: Bundle, a: BundleRef): boolean {
  return b.assets.some((x) => x.kind === a.kind && x.name === a.name);
}

function readAssetRef(e: DragEvent): BundleRef | null {
  const raw = e.dataTransfer?.getData("application/claude-asset");
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as BundleRef;
    if (!parsed.kind || !parsed.name) return null;
    return parsed;
  } catch {
    return null;
  }
}

function onDragOverBundle(e: DragEvent, bundleName: string) {
  if (!e.dataTransfer?.types.includes("application/claude-asset")) return;
  e.preventDefault();
  e.dataTransfer.dropEffect = "copy";
  dragOver.value = bundleName;
}

function onDragLeaveBundle(bundleName: string) {
  if (dragOver.value === bundleName) dragOver.value = null;
}

function onDropOnBundle(e: DragEvent, b: Bundle) {
  e.preventDefault();
  dragOver.value = null;
  const ref = readAssetRef(e);
  if (!ref) return;
  if (!libraryKeys.value.has(assetKey(ref))) return;
  if (isInBundle(b, ref)) return;
  emit("update", { ...b, assets: [...b.assets, ref] });
  if (props.selected !== b.name) emit("select", b.name);
}

function onDragStartChip(e: DragEvent, b: Bundle, ref: BundleRef) {
  if (!e.dataTransfer) return;
  e.dataTransfer.effectAllowed = "move";
  e.dataTransfer.setData(
    "application/claude-bundle-chip",
    JSON.stringify({ bundle: b.name, ...ref })
  );
}

function onDropOutside(e: DragEvent) {
  const raw = e.dataTransfer?.getData("application/claude-bundle-chip");
  if (!raw) return;
  try {
    const parsed = JSON.parse(raw) as BundleRef & { bundle: string };
    const b = props.bundles.find((x) => x.name === parsed.bundle);
    if (!b) return;
    const assets = b.assets.filter(
      (x) => !(x.kind === parsed.kind && x.name === parsed.name)
    );
    emit("update", { ...b, assets });
  } catch {
    return;
  }
}
</script>

<template>
  <div class="column" @dragover.prevent @drop="onDropOutside">
    <div class="col-header">
      Bundles
      <span class="count">{{ bundles.length }}</span>
      <div style="flex:1" />
      <button @click="creating = !creating">+ New</button>
    </div>

    <div class="col-body">
      <div v-if="creating" style="padding: 10px; display: flex; flex-direction: column; gap: 6px;">
        <input v-model="newName" type="text" placeholder="Bundle name (e.g. python-backend)" @keyup.enter="submitCreate" />
        <input v-model="newDesc" type="text" placeholder="Description (optional)" @keyup.enter="submitCreate" />
        <div style="display:flex; gap:6px">
          <button class="primary" @click="submitCreate">Create</button>
          <button @click="creating = false">Cancel</button>
        </div>
      </div>

      <div v-if="bundles.length === 0 && !creating" class="empty">
        No bundles yet.<br />Click <b>+ New</b> to create one.<br /><br />
        Tip: drop assets from the Library onto a bundle to add them.
      </div>

      <div
        v-for="b in bundles"
        :key="b.name"
        class="bundle-item"
        :class="{ active: b.name === selected, 'drag-target': dragOver === b.name }"
        @click="emit('select', b.name)"
        @dragover="onDragOverBundle($event, b.name)"
        @dragleave="onDragLeaveBundle(b.name)"
        @drop="onDropOnBundle($event, b)"
      >
        <div class="bundle-title">
          <span class="bundle-name">{{ b.name }}</span>
          <span class="bundle-count">{{ b.assets.length }} assets</span>
        </div>
        <div v-if="b.description" class="bundle-desc">{{ b.description }}</div>

        <!-- Expanded editor for the selected bundle -->
        <template v-if="b.name === selected">
          <div class="bundle-assets">
            <div
              v-for="a in b.assets"
              :key="assetKey(a)"
              class="tag tag-active"
              draggable="true"
              title="Click to remove, or drag out"
              @dragstart="onDragStartChip($event, b, a)"
              @click.stop="toggleAssetInBundle(b, a)"
            >
              {{ a.kind.charAt(0) }}/{{ a.name }}
              <span class="tag-x">×</span>
            </div>
            <div v-if="b.assets.length === 0" class="empty" style="padding: 6px 0; width: 100%; text-align: left">
              Drop assets here from the Library.
            </div>
          </div>

          <details class="bundle-browse">
            <summary>Add from library…</summary>
            <div class="bundle-assets">
              <div
                v-for="a in library.filter(a => !isInBundle(b, a))"
                :key="assetKey(a)"
                class="tag"
                style="cursor: pointer"
                @click.stop="toggleAssetInBundle(b, { kind: a.kind, name: a.name })"
              >
                + {{ a.kind.charAt(0) }}/{{ a.name }}
              </div>
            </div>
          </details>

          <div style="display:flex; gap:6px; margin-top: 10px;">
            <button
              class="primary"
              :disabled="!canApply"
              @click.stop="emit('apply', b.name, false)"
            >
              Apply to project
            </button>
            <button
              :disabled="!canApply"
              @click.stop="emit('apply', b.name, true)"
            >
              Apply (replace)
            </button>
            <div style="flex:1" />
            <button class="danger" @click.stop="emit('delete', b.name)">Delete</button>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>
