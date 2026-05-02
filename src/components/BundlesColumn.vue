<script setup lang="ts">
import { ref, computed } from "vue";
import { Plus, Trash2, X } from "lucide-vue-next";
import type { Asset, Bundle, BundleRef } from "@/lib/types";
import { assetKey } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";

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
  <div
    class="flex h-full flex-col overflow-hidden"
    @dragover.prevent
    @drop="onDropOutside"
  >
    <!-- Header -->
    <div class="flex items-center gap-2 border-b bg-card/40 px-4 py-2.5">
      <span class="text-sm font-semibold">Bundles</span>
      <span class="text-xs text-muted-foreground">{{ bundles.length }}</span>
      <div class="flex-1" />
      <Button variant="outline" size="sm" @click="creating = !creating">
        <Plus />
        New bundle
      </Button>
    </div>

    <!-- Body -->
    <ScrollArea class="flex-1 min-h-0">
      <div class="space-y-2 p-3">
        <!-- Inline create form -->
        <div
          v-if="creating"
          class="space-y-2 rounded-lg border bg-card p-3"
        >
          <Input
            v-model="newName"
            type="text"
            placeholder="Bundle name (e.g. python-backend)"
            @keyup.enter="submitCreate"
          />
          <Input
            v-model="newDesc"
            type="text"
            placeholder="Description (optional)"
            @keyup.enter="submitCreate"
          />
          <div class="flex gap-2 pt-1">
            <Button size="sm" @click="submitCreate">Create</Button>
            <Button variant="ghost" size="sm" @click="creating = false">
              Cancel
            </Button>
          </div>
        </div>

        <div
          v-if="bundles.length === 0 && !creating"
          class="rounded-lg border border-dashed bg-card/40 px-4 py-8 text-center text-xs text-muted-foreground"
        >
          No bundles yet.<br />Click <b class="text-foreground">+ New bundle</b> to create one.<br /><br />
          Tip: drop assets from the Library onto a bundle to add them.
        </div>

        <!-- Bundle cards -->
        <div
          v-for="b in bundles"
          :key="b.name"
          class="cursor-pointer rounded-lg border bg-card p-3 transition-all hover:border-foreground/20"
          :class="{
            'ring-2 ring-primary/40 ring-offset-2 ring-offset-background':
              b.name === selected,
            'border-primary bg-primary/10':
              dragOver === b.name,
          }"
          @click="emit('select', b.name)"
          @dragover="onDragOverBundle($event, b.name)"
          @dragleave="onDragLeaveBundle(b.name)"
          @drop="onDropOnBundle($event, b)"
        >
          <div class="flex items-center justify-between gap-2">
            <span class="text-sm font-semibold">{{ b.name }}</span>
            <span class="text-xs text-muted-foreground">
              {{ b.assets.length }} asset{{ b.assets.length === 1 ? "" : "s" }}
            </span>
          </div>
          <p
            v-if="b.description"
            class="mt-1 text-xs leading-snug text-muted-foreground"
          >{{ b.description }}</p>

          <!-- Expanded editor for the selected bundle -->
          <template v-if="b.name === selected">
            <Separator class="my-3" />

            <div class="flex flex-wrap gap-1.5">
              <Badge
                v-for="a in b.assets"
                :key="assetKey(a)"
                variant="default"
                class="cursor-pointer gap-1 font-mono text-[11px]"
                draggable="true"
                title="Click to remove, or drag out of the column"
                @dragstart="onDragStartChip($event, b, a)"
                @click.stop="toggleAssetInBundle(b, a)"
              >
                {{ a.kind.charAt(0) }}/{{ a.name }}
                <X class="size-3 opacity-70" />
              </Badge>
              <div
                v-if="b.assets.length === 0"
                class="text-xs italic text-muted-foreground"
              >
                Drop assets here from the Library.
              </div>
            </div>

            <details class="mt-3 group">
              <summary
                class="cursor-pointer select-none text-xs text-muted-foreground hover:text-foreground"
              >Add from library…</summary>
              <div class="mt-2 flex flex-wrap gap-1.5">
                <Badge
                  v-for="a in library.filter((x) => !isInBundle(b, x))"
                  :key="assetKey(a)"
                  variant="outline"
                  class="cursor-pointer font-mono text-[11px] hover:bg-accent"
                  @click.stop="
                    toggleAssetInBundle(b, { kind: a.kind, name: a.name })
                  "
                >+ {{ a.kind.charAt(0) }}/{{ a.name }}</Badge>
              </div>
            </details>

            <div class="mt-4 flex gap-2">
              <Button
                size="sm"
                :disabled="!canApply"
                @click.stop="emit('apply', b.name, false)"
              >Apply to project</Button>
              <Button
                variant="outline"
                size="sm"
                :disabled="!canApply"
                @click.stop="emit('apply', b.name, true)"
              >Apply (replace)</Button>
              <div class="flex-1" />
              <Button
                variant="ghost"
                size="sm"
                class="text-destructive hover:bg-destructive/10 hover:text-destructive"
                @click.stop="emit('delete', b.name)"
              >
                <Trash2 />
              </Button>
            </div>
          </template>
        </div>
      </div>
    </ScrollArea>
  </div>
</template>
