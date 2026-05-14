<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import {
  ArrowLeft,
  ChevronLeft,
  ClipboardCheck,
  Copy,
  Plus,
  Share2,
  Sparkles,
  Trash2,
  Check,
  Search,
  X,
} from "lucide-vue-next";
import { useAppStore } from "@/stores/app";
import { useBundleStore } from "@/stores/bundle";
import { assetKey, bundleEntryKey, isAssetRef, LOCAL_PLUGIN } from "@/lib/types";
import type { Asset, BundleEntryKind, BundleRef, McpEntry } from "@/lib/types";
import NewMcpDialog from "@/components/NewMcpDialog.vue";
import {
  groupAssets,
  loadGroupByPreference,
  saveGroupByPreference,
  type AssetGroupBy,
} from "@/lib/grouping";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { Checkbox } from "@/components/ui/checkbox";
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
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";

const props = defineProps<{ name: string }>();

const store = useAppStore();
const bundleStore = useBundleStore();
const router = useRouter();
const { bundles, library, projectPath, installedKeys, mcp } = storeToRefs(store);

const bundle = computed(() =>
  bundles.value.find((b) => b.name === props.name) ?? null
);

const bundleAssetKeys = computed(
  () => new Set(bundle.value?.assets.map(bundleEntryKey) ?? [])
);

const isFullyApplied = computed(() => {
  if (!bundle.value || bundle.value.assets.length === 0) return false;
  // Step 1: installed-tracking covers skills/commands/agents only.
  // Hook + MCP installation tracking will land alongside their UI;
  // until then any hook/mcp ref keeps the bundle "not fully applied".
  return bundle.value.assets.every(
    (a) => isAssetRef(a) && installedKeys.value.has(assetKey(a)),
  );
});

// ── Add asset dialog ──────────────────────────────────────────────
const addOpen = ref(false);
const addQuery = ref("");
const pickedKeys = ref<Set<string>>(new Set());

const candidateAssets = computed<Asset[]>(() => {
  const q = addQuery.value.trim().toLowerCase();
  return library.value
    .filter((a) => !bundleAssetKeys.value.has(assetKey(a)))
    .filter((a) => {
      if (!q) return true;
      if (a.name.toLowerCase().includes(q)) return true;
      if (a.description?.toLowerCase().includes(q)) return true;
      if (a.kind.toLowerCase().includes(q)) return true;
      return false;
    });
});

const addGroupBy = ref<AssetGroupBy>(loadGroupByPreference());
watch(addGroupBy, saveGroupByPreference);

const candidateGroups = computed(() =>
  groupAssets(candidateAssets.value, addGroupBy.value)
);

function openAdd() {
  addQuery.value = "";
  pickedKeys.value = new Set();
  addOpen.value = true;
}

function togglePick(a: Asset) {
  const k = assetKey(a);
  const next = new Set(pickedKeys.value);
  if (next.has(k)) next.delete(k);
  else next.add(k);
  pickedKeys.value = next;
}

async function commitAdd() {
  if (!bundle.value || pickedKeys.value.size === 0) return;
  const newRefs: BundleRef[] = library.value
    .filter((a) => pickedKeys.value.has(assetKey(a)))
    .map((a) => ({ kind: a.kind, name: a.name }));
  await bundleStore.updateBundle({
    ...bundle.value,
    assets: [...bundle.value.assets, ...newRefs],
  });
  addOpen.value = false;
}

// ── Add MCP dialog ────────────────────────────────────────────────
//
// Lists every MCP entry from the library (plugin-shipped under their
// own bucket, locally-created entries grouped under "Local"). The
// "+ Create new MCP" button below opens NewMcpDialog inline — once the
// user saves, the new entry shows up in the list and is auto-picked.

const addMcpOpen = ref(false);
const mcpPickedKeys = ref<Set<string>>(new Set());
const newMcpOpen = ref(false);

// Stable key for selection — matches the bundle ref shape so we can
// reuse `bundleEntryKey` for de-dup against what's already in the bundle.
function mcpRefFromEntry(entry: McpEntry): BundleRef {
  return entry.plugin === LOCAL_PLUGIN
    ? { kind: "mcp", name: entry.name }
    : { kind: "mcp", name: entry.name, plugin: entry.plugin };
}

const candidateMcps = computed<McpEntry[]>(() =>
  mcp.value.filter((e) => !bundleAssetKeys.value.has(bundleEntryKey(mcpRefFromEntry(e)))),
);

// Group by plugin (Local bucket gets a friendly label).
const candidateMcpsByPlugin = computed(() => {
  const groups = new Map<string, McpEntry[]>();
  for (const e of candidateMcps.value) {
    const key = e.plugin === LOCAL_PLUGIN ? "Local" : e.plugin;
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key)!.push(e);
  }
  return Array.from(groups.entries()).sort(([a], [b]) => {
    // Local bucket at the bottom for grouping consistency with browse-library.
    if (a === "Local") return 1;
    if (b === "Local") return -1;
    return a.localeCompare(b);
  });
});

function openAddMcp() {
  mcpPickedKeys.value = new Set();
  addMcpOpen.value = true;
  store.refreshHooksMcp();
}

function toggleMcpPick(entry: McpEntry) {
  const k = bundleEntryKey(mcpRefFromEntry(entry));
  const next = new Set(mcpPickedKeys.value);
  if (next.has(k)) next.delete(k);
  else next.add(k);
  mcpPickedKeys.value = next;
}

async function commitAddMcp() {
  if (!bundle.value || mcpPickedKeys.value.size === 0) return;
  const newRefs: BundleRef[] = candidateMcps.value
    .filter((e) => mcpPickedKeys.value.has(bundleEntryKey(mcpRefFromEntry(e))))
    .map(mcpRefFromEntry);
  await bundleStore.updateBundle({
    ...bundle.value,
    assets: [...bundle.value.assets, ...newRefs],
  });
  addMcpOpen.value = false;
}

async function onMcpSaved(name: string) {
  // Refresh the listing then auto-pick the entry the user just made
  // so committing the picker now adds it to the bundle in one click.
  await store.refreshHooksMcp();
  mcpPickedKeys.value = new Set([
    ...mcpPickedKeys.value,
    bundleEntryKey({ kind: "mcp", name }),
  ]);
}

/** Short single-line description of an MCP entry for the picker list. */
function mcpPreview(entry: McpEntry): string {
  const inner =
    (entry.servers as Record<string, unknown> | undefined)?.mcpServers ??
    entry.servers;
  if (inner && typeof inner === "object") {
    const obj = inner as Record<string, Record<string, unknown>>;
    const first = obj[entry.name] ?? Object.values(obj)[0];
    if (first && typeof first === "object") {
      const cmd = first.command;
      const args = Array.isArray(first.args)
        ? ` ${(first.args as unknown[]).join(" ")}`
        : "";
      if (typeof cmd === "string") return `${cmd}${args}`;
    }
  }
  return entry.path;
}

// ── Remove asset from bundle ──────────────────────────────────────
async function removeAsset(ref: BundleRef) {
  if (!bundle.value) return;
  const assets = bundle.value.assets.filter(
    (a) => !(a.kind === ref.kind && a.name === ref.name)
  );
  await bundleStore.updateBundle({ ...bundle.value, assets });
}

// ── Delete bundle ─────────────────────────────────────────────────
const deleteOpen = ref(false);

async function confirmDelete() {
  if (!bundle.value) return;
  await bundleStore.deleteBundle(bundle.value.name);
  deleteOpen.value = false;
  router.push({ name: "bundles" });
}

// ── Share bundle ──────────────────────────────────────────────────
const shareOpen = ref(false);
const shareCode = ref("");
const shareLoading = ref(false);
const shareCopied = ref(false);

async function openShare() {
  if (!bundle.value) return;
  shareCode.value = "";
  shareLoading.value = true;
  shareOpen.value = true;
  try {
    shareCode.value = await bundleStore.encodeBundleShare(bundle.value.name);
  } finally {
    shareLoading.value = false;
  }
}

async function copyShareCode() {
  await navigator.clipboard.writeText(shareCode.value);
  shareCopied.value = true;
  setTimeout(() => { shareCopied.value = false; }, 2000);
}

// Decorate an asset reference with its full library entry (for description, origin).
// Hook/MCP refs have no markdown counterpart in `library`, so we return undefined
// for them — the table just won't show description/origin badges for those rows.
function findAsset(ref: BundleRef): Asset | undefined {
  if (!isAssetRef(ref)) return undefined;
  return library.value.find(
    (a) => a.kind === ref.kind && a.name === ref.name
  );
}

function openAsset(ref: BundleRef) {
  // Only the markdown kinds have a `/library/:kind/:name` editor view.
  // Hook/MCP detail pages will arrive with their dedicated UIs.
  if (!isAssetRef(ref)) return;
  router.push({
    name: "asset-detail",
    params: { kind: ref.kind, name: ref.name },
  });
}

function kindLabel(kind: BundleEntryKind): string {
  switch (kind) {
    case "skills": return "Skill";
    case "commands": return "Command";
    case "agents": return "Agent";
    case "hooks": return "Hook";
    case "mcp": return "MCP";
  }
}
</script>

<template>
  <div class="flex h-full flex-col overflow-hidden">
    <!-- Not found state -->
    <div
      v-if="!bundle"
      class="flex h-full flex-col items-center justify-center gap-4 px-6 text-center"
    >
      <h3 class="text-base font-semibold">Bundle not found</h3>
      <p class="text-sm text-muted-foreground">
        No bundle named <code class="rounded bg-muted px-1 py-0.5 font-mono text-xs">{{ name }}</code> exists.
      </p>
      <Button variant="outline" @click="router.push({ name: 'bundles' })">
        <ArrowLeft />
        Back to bundles
      </Button>
    </div>

    <template v-else>
      <!-- Header -->
      <div class="border-b bg-card/40 px-6 py-4">
        <button
          class="mb-2 inline-flex items-center gap-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
          @click="router.push({ name: 'bundles' })"
        >
          <ChevronLeft class="size-3" />
          All bundles
        </button>

        <div class="flex items-start justify-between gap-4">
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <h1 class="truncate text-lg font-semibold">{{ bundle.name }}</h1>
              <Badge
                v-if="isFullyApplied"
                variant="outline"
                class="text-[10px] font-medium uppercase tracking-wider text-primary"
              >Applied</Badge>
            </div>
            <p
              v-if="bundle.description"
              class="mt-1 text-sm text-muted-foreground"
            >{{ bundle.description }}</p>
          </div>

          <div class="flex shrink-0 items-center gap-2">
            <Button
              size="sm"
              :disabled="!projectPath || bundle.assets.length === 0"
              @click="store.applyBundle(bundle.name, false)"
            >
              <Check />
              Apply to project
            </Button>
            <Button
              variant="outline"
              size="sm"
              :disabled="!projectPath || bundle.assets.length === 0"
              @click="store.applyBundle(bundle.name, true)"
            >Apply (replace)</Button>
            <Button
              variant="outline"
              size="sm"
              :disabled="bundle.assets.length === 0"
              title="Rewrite the bundle's assets via AI to make tone, terminology and structure consistent"
              @click="router.push({ name: 'bundle-harmonize', params: { name: bundle.name } })"
            >
              <Sparkles />
              Harmonize
            </Button>
            <Button
              variant="outline"
              size="sm"
              :disabled="bundle.assets.length === 0"
              title="Share this bundle with someone"
              @click="openShare"
            >
              <Share2 />
              Share
            </Button>
            <Button
              variant="ghost"
              size="sm"
              class="text-destructive hover:bg-destructive/10 hover:text-destructive"
              @click="deleteOpen = true"
            >
              <Trash2 />
            </Button>
          </div>
        </div>
      </div>

      <!-- Content -->
      <ScrollArea class="flex-1 min-h-0">
        <div class="px-6 py-5">
          <div class="mb-3 flex items-center gap-2">
            <h2 class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
              Content
            </h2>
            <Badge variant="secondary" class="font-mono text-[10px]">
              {{ bundle.assets.length }} asset{{ bundle.assets.length === 1 ? "" : "s" }}
            </Badge>
            <div class="flex-1" />
            <Button size="sm" variant="outline" @click="openAddMcp">
              <Plus />
              Add MCP
            </Button>
            <Button size="sm" variant="outline" @click="openAdd">
              <Plus />
              Add asset
            </Button>
          </div>

          <div
            v-if="bundle.assets.length === 0"
            class="rounded-lg border border-dashed bg-card/40 px-4 py-10 text-center"
          >
            <p class="text-sm text-muted-foreground">
              This bundle is empty.
            </p>
            <Button class="mt-4" size="sm" variant="outline" @click="openAdd">
              <Plus />
              Add your first asset
            </Button>
          </div>

          <div v-else class="overflow-hidden rounded-lg border bg-card">
            <table class="w-full text-sm">
              <thead>
                <tr class="border-b text-left text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
                  <th class="px-4 py-2.5">Kind</th>
                  <th class="px-4 py-2.5">Name</th>
                  <th class="px-4 py-2.5">Origin</th>
                  <th class="w-10 px-4 py-2.5"></th>
                </tr>
              </thead>
              <tbody class="divide-y">
                <tr
                  v-for="ref in bundle.assets"
                  :key="`${ref.kind}:${ref.name}`"
                  class="group cursor-pointer transition-colors hover:bg-accent/40"
                  @click="openAsset(ref)"
                >
                  <td class="px-4 py-2.5 align-top">
                    <Badge variant="outline" class="text-[10px]">
                      {{ kindLabel(ref.kind) }}
                    </Badge>
                  </td>
                  <td class="px-4 py-2.5 align-top">
                    <div class="font-medium">{{ ref.name }}</div>
                    <div
                      v-if="findAsset(ref)?.description"
                      class="mt-0.5 line-clamp-1 text-xs text-muted-foreground"
                    >{{ findAsset(ref)?.description }}</div>
                  </td>
                  <td class="px-4 py-2.5 align-top">
                    <Badge
                      v-if="findAsset(ref)?.origin"
                      variant="secondary"
                      class="text-[10px]"
                    >from {{ findAsset(ref)?.origin?.plugin }}</Badge>
                    <span v-else class="text-xs italic text-muted-foreground/60">local</span>
                  </td>
                  <td class="px-4 py-2.5 align-top text-right">
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      class="opacity-0 transition-opacity group-hover:opacity-100 hover:text-destructive"
                      title="Remove from bundle"
                      @click.stop="removeAsset(ref)"
                    >
                      <X />
                    </Button>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </ScrollArea>
    </template>

    <!-- Add asset dialog -->
    <Dialog v-model:open="addOpen">
      <DialogContent class="flex max-h-[80vh] flex-col gap-0 sm:max-w-2xl p-0">
        <DialogHeader class="border-b px-6 pt-6 pb-4">
          <DialogTitle>Add assets to {{ bundle?.name }}</DialogTitle>
          <DialogDescription>
            Pick assets from your library to include in this bundle.
          </DialogDescription>
        </DialogHeader>

        <div class="flex items-center gap-2 border-b px-6 py-3">
          <div class="relative flex-1">
            <Search class="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
            <Input
              v-model="addQuery"
              placeholder="Search by name, description or kind…"
              class="h-9 pl-8"
            />
          </div>
          <ToggleGroup
            type="single"
            :model-value="addGroupBy"
            @update:model-value="(v) => v && (addGroupBy = v as AssetGroupBy)"
            variant="outline"
            size="sm"
          >
            <ToggleGroupItem value="kind">Kind</ToggleGroupItem>
            <ToggleGroupItem value="plugin">Plugin</ToggleGroupItem>
          </ToggleGroup>
        </div>

        <div class="flex-1 overflow-y-auto px-6 py-4">
          <div
            v-if="library.length === 0"
            class="py-10 text-center text-sm text-muted-foreground"
          >
            Your library is empty.<br />
            Import assets from a plugin folder or the marketplace first.
          </div>
          <div
            v-else-if="candidateAssets.length === 0"
            class="py-10 text-center text-sm text-muted-foreground"
          >
            <template v-if="addQuery">No match for "{{ addQuery }}".</template>
            <template v-else>All your library assets are already in this bundle.</template>
          </div>
          <div v-else class="space-y-4">
            <div v-for="g in candidateGroups" :key="g.key">
              <div class="mb-1.5 px-1 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                {{ g.label }}
                <span class="font-normal opacity-70">{{ g.items.length }}</span>
              </div>
              <div class="space-y-0.5">
                <label
                  v-for="a in g.items"
                  :key="assetKey(a)"
                  class="flex cursor-pointer items-start gap-3 rounded-md px-2.5 py-2 transition-colors hover:bg-accent/60"
                >
                  <Checkbox
                    :model-value="pickedKeys.has(assetKey(a))"
                    class="mt-0.5"
                    @update:model-value="togglePick(a)"
                  />
                  <div class="min-w-0 flex-1">
                    <div class="flex items-center gap-1.5">
                      <Badge
                        v-if="addGroupBy === 'plugin'"
                        variant="outline"
                        class="h-4 px-1.5 text-[9px] uppercase"
                      >{{ a.kind }}</Badge>
                      <span class="text-sm font-medium leading-none">{{ a.name }}</span>
                    </div>
                    <p
                      v-if="a.description"
                      class="mt-1 line-clamp-1 text-xs text-muted-foreground"
                    >{{ a.description }}</p>
                  </div>
                </label>
              </div>
            </div>
          </div>
        </div>

        <Separator />
        <DialogFooter class="px-6 py-3 sm:justify-between">
          <span class="text-xs text-muted-foreground">
            {{ pickedKeys.size }} selected
          </span>
          <div class="flex gap-2">
            <Button variant="outline" @click="addOpen = false">Cancel</Button>
            <Button :disabled="pickedKeys.size === 0" @click="commitAdd">
              Add {{ pickedKeys.size > 0 ? `(${pickedKeys.size})` : "" }}
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <!-- Add MCP dialog -->
    <Dialog v-model:open="addMcpOpen">
      <DialogContent class="overflow-hidden p-0 sm:max-w-2xl">
        <DialogHeader class="border-b px-6 py-4">
          <DialogTitle>Add MCP servers to {{ bundle?.name }}</DialogTitle>
          <DialogDescription>
            Pick MCP entries to include. Plugin-shipped servers come grouped by
            their source; entries you built in-app live under <strong>Local</strong>.
          </DialogDescription>
        </DialogHeader>

        <div class="border-b bg-card/30 px-6 py-2.5">
          <Button
            size="sm"
            variant="outline"
            @click="newMcpOpen = true"
          >
            <Plus />
            Create new MCP
          </Button>
        </div>

        <ScrollArea class="max-h-[60vh]">
          <div class="px-6 py-4">
            <div
              v-if="mcp.length === 0"
              class="rounded-lg border border-dashed px-4 py-10 text-center text-sm text-muted-foreground"
            >
              No MCP entries yet. Use <strong>Create new MCP</strong> above or
              import a plugin that ships one.
            </div>

            <div
              v-else-if="candidateMcps.length === 0"
              class="rounded-lg border border-dashed px-4 py-10 text-center text-sm text-muted-foreground"
            >
              All available MCP entries are already in this bundle.
            </div>

            <div v-else class="space-y-4">
              <div
                v-for="[groupName, entries] in candidateMcpsByPlugin"
                :key="groupName"
                class="space-y-2"
              >
                <div class="flex items-center gap-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                  <span>{{ groupName }}</span>
                  <span class="font-mono">{{ entries.length }}</span>
                </div>
                <label
                  v-for="entry in entries"
                  :key="bundleEntryKey(mcpRefFromEntry(entry))"
                  class="flex cursor-pointer items-start gap-3 rounded-md border bg-card/40 px-3 py-2 transition-colors hover:bg-card/60"
                >
                  <Checkbox
                    :model-value="mcpPickedKeys.has(bundleEntryKey(mcpRefFromEntry(entry)))"
                    @update:model-value="() => toggleMcpPick(entry)"
                    class="mt-0.5"
                  />
                  <div class="min-w-0 flex-1">
                    <div class="flex items-center gap-2">
                      <Badge variant="outline" class="text-[10px]">MCP</Badge>
                      <span class="truncate font-mono text-[13px] font-medium">
                        {{ entry.name }}
                      </span>
                    </div>
                    <p class="mt-0.5 line-clamp-1 font-mono text-[11px] text-muted-foreground">
                      {{ mcpPreview(entry) }}
                    </p>
                  </div>
                </label>
              </div>
            </div>
          </div>
        </ScrollArea>

        <Separator />
        <DialogFooter class="px-6 py-3 sm:justify-between">
          <span class="text-xs text-muted-foreground">
            {{ mcpPickedKeys.size }} selected
          </span>
          <div class="flex gap-2">
            <Button variant="outline" @click="addMcpOpen = false">Cancel</Button>
            <Button :disabled="mcpPickedKeys.size === 0" @click="commitAddMcp">
              Add {{ mcpPickedKeys.size > 0 ? `(${mcpPickedKeys.size})` : "" }}
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <!-- Create MCP dialog (nested entry-point from the picker) -->
    <NewMcpDialog v-model:open="newMcpOpen" @saved="onMcpSaved" />

    <!-- Share dialog -->
    <Dialog v-model:open="shareOpen">
      <DialogContent class="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Share "{{ bundle?.name }}"</DialogTitle>
          <DialogDescription>
            Send this code to anyone. Importing it recreates the bundle with
            all your assets — including any edits or AI harmonizations.
          </DialogDescription>
        </DialogHeader>
        <div class="space-y-3">
          <div
            v-if="shareLoading"
            class="rounded-md border bg-muted/40 px-3 py-6 text-center text-xs text-muted-foreground"
          >
            Generating share code…
          </div>
          <div v-else class="relative">
            <textarea
              readonly
              :value="shareCode"
              class="h-28 w-full resize-none rounded-md border bg-muted/40 px-3 py-2 font-mono text-[11px] leading-relaxed text-muted-foreground focus:outline-none"
              @click="($event.target as HTMLTextAreaElement).select()"
            />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" @click="shareOpen = false">Close</Button>
          <Button :disabled="shareLoading || !shareCode" @click="copyShareCode">
            <ClipboardCheck v-if="shareCopied" />
            <Copy v-else />
            {{ shareCopied ? "Copied!" : "Copy code" }}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <!-- Delete confirm -->
    <AlertDialog v-model:open="deleteOpen">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete this bundle?</AlertDialogTitle>
          <AlertDialogDescription>
            <span class="block">
              This removes <code class="rounded bg-muted px-1 py-0.5 font-mono text-xs">{{ bundle?.name }}</code> from your bundles list.
            </span>
            <span class="mt-2 block text-xs">
              The library assets it referenced stay untouched, and any project where it was applied keeps the symlinks until you Clean.
            </span>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            class="bg-destructive text-white hover:bg-destructive/90"
            @click="confirmDelete"
          >Delete bundle</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>
