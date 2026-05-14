<script setup lang="ts">
import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import {
  BookOpen,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  FolderOpen,
  Package,
  Search,
  Pencil,
  Plug,
  Plus,
  Server,
  Sparkles,
  Terminal,
  Trash2,
  X,
} from "lucide-vue-next";
import { toast } from "vue-sonner";
import { api } from "@/lib/api";
import { useAppStore } from "@/stores/app";
import type { Asset, HookEntry, McpEntry } from "@/lib/types";
import { assetKey } from "@/lib/types";
import NewMcpDialog from "@/components/NewMcpDialog.vue";
import NewHookDialog from "@/components/NewHookDialog.vue";
import { groupAssets } from "@/lib/grouping";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
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

const LOCAL_KEY = "__local__";

const props = defineProps<{ plugin: string }>();

const store = useAppStore();
const router = useRouter();
const { library, hooks, mcp, installedKeys, installedHooks, projectPath } = storeToRefs(store);

const canInstall = computed(() => !!projectPath.value);
const isLocal = computed(() => props.plugin === LOCAL_KEY);

const pluginAssets = computed<Asset[]>(() => {
  if (isLocal.value) {
    return library.value.filter((a) => !a.origin);
  }
  return library.value.filter((a) => a.origin?.plugin === props.plugin);
});

const marketplace = computed<string | null>(() => {
  return pluginAssets.value[0]?.origin?.marketplace ?? null;
});

// True when this non-local plugin exists in the marketplace catalog.
// We determine this by checking if it has assets with an origin, or
// hooks/mcp (all imported from the marketplace).
const hasMarketplaceDocs = computed(
  () => !isLocal.value && (marketplace.value !== null || pluginHooks.value.length > 0 || !!pluginMcp.value)
);

const pluginHooks = computed(() =>
  isLocal.value ? [] : hooks.value.filter((h) => h.plugin === props.plugin)
);

const pluginMcp = computed(() =>
  isLocal.value ? null : mcp.value.find((m) => m.plugin === props.plugin) ?? null
);

/** All MCP entries living under the Local bucket — used only on the
 *  `__local__` detail page where each entry is one server per file
 *  (the in-app create flow). Returns empty otherwise. */
const localMcps = computed<McpEntry[]>(() =>
  isLocal.value ? mcp.value.filter((m) => m.plugin === LOCAL_KEY) : [],
);

/** Local hooks — the AI-generated and manual ones, with their
 *  event/matcher metadata for the card. */
const localHooks = computed<HookEntry[]>(() =>
  isLocal.value ? hooks.value.filter((h) => h.plugin === LOCAL_KEY) : [],
);

const mcpServersJson = computed(() => {
  if (!pluginMcp.value) return "";
  return JSON.stringify(pluginMcp.value.servers, null, 2);
});

// ── Local MCP CRUD wired through NewMcpDialog + the delete confirm ──
const newMcpOpen = ref(false);
const editMcpName = ref<string | undefined>(undefined);
const deleteMcpName = ref<string | null>(null);

// ── Local hook CRUD wired through NewHookDialog + the delete confirm ─
const newHookOpen = ref(false);
const editHookName = ref<string | undefined>(undefined);
const deleteHookName = ref<string | null>(null);

function openCreateHook() {
  editHookName.value = undefined;
  newHookOpen.value = true;
}
function openEditHook(name: string) {
  editHookName.value = name;
  newHookOpen.value = true;
}
async function onHookSaved() {
  await store.refreshHooksMcp();
}
async function confirmDeleteHook() {
  if (!deleteHookName.value) return;
  try {
    await api.deleteLocalHook(deleteHookName.value);
    toast.success(`Deleted hook "${deleteHookName.value}"`);
    await store.refreshHooksMcp();
  } catch (e) {
    toast.error("Delete failed", { description: String(e) });
  } finally {
    deleteHookName.value = null;
  }
}

function hookCardLabel(entry: HookEntry): string {
  if (!entry.meta) return "(no metadata)";
  const matcher = entry.meta.matcher && entry.meta.matcher !== "*"
    ? entry.meta.matcher
    : "any tool";
  return `${entry.meta.event} · ${matcher}`;
}

function openCreateMcp() {
  editMcpName.value = undefined;
  newMcpOpen.value = true;
}
function openEditMcp(name: string) {
  editMcpName.value = name;
  newMcpOpen.value = true;
}
async function onMcpSaved() {
  await store.refreshHooksMcp();
}
async function confirmDeleteMcp() {
  if (!deleteMcpName.value) return;
  try {
    await api.deleteLocalMcp(deleteMcpName.value);
    toast.success(`Deleted MCP "${deleteMcpName.value}"`);
    await store.refreshHooksMcp();
  } catch (e) {
    toast.error("Delete failed", { description: String(e) });
  } finally {
    deleteMcpName.value = null;
  }
}

/** Short command preview for the local-MCP card. */
function mcpCardPreview(entry: McpEntry): string {
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
  return "(no command)";
}

const query = ref("");

const filtered = computed<Asset[]>(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return pluginAssets.value;
  return pluginAssets.value.filter((a) => {
    if (a.name.toLowerCase().includes(q)) return true;
    if (a.description?.toLowerCase().includes(q)) return true;
    if (a.tags?.some((t) => t.toLowerCase().includes(q))) return true;
    if (a.kind.toLowerCase().includes(q)) return true;
    return false;
  });
});

const groups = computed(() => groupAssets(filtered.value, "kind"));

const title = computed(() => (isLocal.value ? "Local" : props.plugin));

// Per-hook expanded state
const expandedHooks = ref(new Set<string>());
function toggleHookExpand(filename: string) {
  if (expandedHooks.value.has(filename)) {
    expandedHooks.value.delete(filename);
  } else {
    expandedHooks.value.add(filename);
  }
}

const deleteOpen = ref(false);

function openAsset(a: Asset) {
  router.push({
    name: "asset-detail",
    params: { kind: a.kind, name: a.name },
  });
}

function onCheckboxChange(a: Asset) {
  if (!canInstall.value) return;
  store.toggleAsset(a);
}

function isHookInstalled(plugin: string, filename: string): boolean {
  return installedHooks.value.some((h) => h.plugin === plugin && h.filename === filename);
}

function viewDocs() {
  router.push({ name: "plugin-detail", params: { name: props.plugin } });
}

async function confirmDelete() {
  await store.removePlugin(props.plugin);
  deleteOpen.value = false;
  router.push({ name: "browse-library" });
}

function back() {
  router.push({ name: "browse-library" });
}
</script>

<template>
  <TooltipProvider :delay-duration="200">
    <div class="flex h-full flex-col overflow-hidden">
      <!-- Header -->
      <div class="border-b bg-card/40 px-6 py-4">
        <button
          class="mb-2 inline-flex items-center gap-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
          @click="back"
        >
          <ChevronLeft class="size-3" />
          Back to library
        </button>

        <div class="flex items-start justify-between gap-3">
          <div class="flex min-w-0 items-center gap-2">
            <FolderOpen v-if="isLocal" class="size-5 shrink-0 text-muted-foreground" />
            <Package v-else class="size-5 shrink-0 text-muted-foreground" />
            <h1 class="truncate text-lg font-semibold">{{ title }}</h1>
            <Badge v-if="marketplace" variant="secondary" class="text-[10px] uppercase tracking-wider">
              from {{ marketplace }}
            </Badge>
          </div>

          <!-- Actions: docs + delete -->
          <div v-if="!isLocal" class="flex shrink-0 items-center gap-2">
            <Tooltip>
              <TooltipTrigger as-child>
                <Button variant="outline" size="sm" @click="viewDocs">
                  <BookOpen class="size-3.5" />
                  Docs
                </Button>
              </TooltipTrigger>
              <TooltipContent>View README and marketplace details</TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger as-child>
                <Button
                  variant="ghost"
                  size="sm"
                  class="text-destructive hover:bg-destructive/10 hover:text-destructive"
                  @click="deleteOpen = true"
                >
                  <Trash2 class="size-3.5" />
                  Remove
                </Button>
              </TooltipTrigger>
              <TooltipContent>Remove this plugin from your library</TooltipContent>
            </Tooltip>
          </div>
        </div>

        <p v-if="isLocal" class="mt-1 text-xs text-muted-foreground">
          Assets you created here, not imported from any plugin.
        </p>
      </div>

      <!-- Toolbar -->
      <div class="flex items-center gap-3 border-b px-4 py-2">
        <span class="text-xs text-muted-foreground">
          {{ filtered.length
          }}<span v-if="filtered.length !== pluginAssets.length">/{{ pluginAssets.length }}</span>
          asset{{ filtered.length === 1 ? "" : "s" }}
        </span>

        <div class="relative flex-1 max-w-md">
          <Search class="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground" />
          <Input
            v-model="query"
            type="text"
            placeholder="Search name, description, tag…"
            class="h-8 pl-8 pr-8 text-sm"
          />
          <button
            v-if="query"
            type="button"
            class="absolute right-1.5 top-1/2 grid size-5 -translate-y-1/2 place-items-center rounded-sm text-muted-foreground hover:bg-accent hover:text-foreground"
            @click="query = ''"
          >
            <X class="size-3" />
          </button>
        </div>
      </div>

      <!-- Body -->
      <ScrollArea class="flex-1 min-h-0">
        <div class="space-y-0">
          <!-- Assets section -->
          <template v-if="pluginAssets.length === 0 && pluginHooks.length === 0 && !pluginMcp">
            <div class="px-6 py-10 text-center text-xs text-muted-foreground">
              <template v-if="isLocal">No local assets yet.</template>
              <template v-else>This plugin has no assets in your library.</template>
            </div>
          </template>
          <template v-else-if="pluginAssets.length > 0 && filtered.length === 0">
            <div class="px-6 py-10 text-center text-xs text-muted-foreground">
              No match for "{{ query }}".
            </div>
          </template>
          <template v-else-if="filtered.length > 0">
            <div class="space-y-3 p-3">
              <div v-for="g in groups" :key="g.key">
                <div class="px-2.5 pt-1 pb-1.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                  {{ g.label }}
                  <span class="font-normal opacity-70">{{ g.items.length }}</span>
                </div>
                <div class="space-y-0.5">
                  <div
                    v-for="a in g.items"
                    :key="assetKey(a)"
                    class="group flex cursor-pointer items-start gap-2.5 rounded-md px-2.5 py-2 transition-colors hover:bg-accent/60"
                    :class="installedKeys.has(assetKey(a)) ? 'bg-primary/10 ring-1 ring-inset ring-primary/30' : ''"
                    tabindex="0"
                    @click="openAsset(a)"
                    @keydown.enter="openAsset(a)"
                    @keydown.space.prevent="openAsset(a)"
                  >
                    <Tooltip>
                      <TooltipTrigger as-child>
                        <Checkbox
                          :model-value="installedKeys.has(assetKey(a))"
                          :disabled="!canInstall"
                          class="mt-0.5"
                          @update:model-value="onCheckboxChange(a)"
                          @click.stop
                        />
                      </TooltipTrigger>
                      <TooltipContent>
                        {{ canInstall ? "Toggle install in project" : "Pick a project to install" }}
                      </TooltipContent>
                    </Tooltip>
                    <div class="min-w-0 flex-1">
                      <div class="flex items-center gap-1.5">
                        <span class="text-sm font-medium leading-none">{{ a.name }}</span>
                        <Tooltip v-if="a.harmonized_at">
                          <TooltipTrigger as-child>
                            <Badge
                              variant="outline"
                              class="gap-1 border-primary/40 px-1.5 py-0 text-[9px] font-medium uppercase tracking-wider text-primary"
                            >
                              <Sparkles class="size-2.5" />
                              Harmonized
                            </Badge>
                          </TooltipTrigger>
                          <TooltipContent>Last harmonized {{ a.harmonized_at }}</TooltipContent>
                        </Tooltip>
                      </div>
                      <p v-if="a.description" class="mt-1 line-clamp-2 text-xs leading-snug text-muted-foreground">
                        {{ a.description }}
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </template>

          <!-- Hooks section -->
          <template v-if="pluginHooks.length > 0">
            <Separator class="my-1" />
            <div class="p-3">
              <div class="px-2.5 pt-1 pb-1.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                Hooks <span class="font-normal opacity-70">{{ pluginHooks.length }}</span>
              </div>
              <div class="space-y-1">
                <div
                  v-for="h in pluginHooks"
                  :key="h.filename"
                  class="rounded-md border"
                  :class="isHookInstalled(h.plugin, h.filename) ? 'border-primary/30 bg-primary/5' : 'bg-card'"
                >
                  <!-- Hook row -->
                  <div class="flex items-center gap-2.5 px-2.5 py-2">
                    <Tooltip>
                      <TooltipTrigger as-child>
                        <Checkbox
                          :model-value="isHookInstalled(h.plugin, h.filename)"
                          :disabled="!canInstall"
                          @update:model-value="store.toggleHook(h.plugin, h.filename)"
                        />
                      </TooltipTrigger>
                      <TooltipContent>
                        {{ canInstall ? "Toggle install in project" : "Pick a project to install" }}
                      </TooltipContent>
                    </Tooltip>
                    <Terminal class="size-3.5 shrink-0 text-muted-foreground" />
                    <span class="flex-1 truncate font-mono text-sm">{{ h.filename }}</span>
                    <!-- Expand toggle -->
                    <button
                      type="button"
                      class="ml-auto grid size-6 shrink-0 place-items-center rounded text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                      :title="expandedHooks.has(h.filename) ? 'Collapse' : 'View content'"
                      @click="toggleHookExpand(h.filename)"
                    >
                      <ChevronDown
                        class="size-3.5 transition-transform"
                        :class="expandedHooks.has(h.filename) ? 'rotate-180' : ''"
                      />
                    </button>
                  </div>
                  <!-- Hook content -->
                  <div v-if="expandedHooks.has(h.filename)" class="border-t px-3 py-2">
                    <pre class="overflow-x-auto font-mono text-[11px] leading-relaxed text-muted-foreground whitespace-pre-wrap break-all">{{ h.content || '(empty file)' }}</pre>
                  </div>
                </div>
              </div>
            </div>
          </template>

          <!-- Local hooks section — only on the __local__ bucket. -->
          <template v-if="isLocal">
            <Separator class="my-1" />
            <div class="p-3">
              <div class="flex items-center justify-between px-2.5 pt-1 pb-2">
                <div class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                  Local Hooks
                </div>
                <Button
                  size="sm"
                  variant="outline"
                  class="h-6 px-2 text-xs"
                  @click="openCreateHook"
                >
                  <Plus class="size-3" />
                  New hook
                </Button>
              </div>
              <div
                v-if="localHooks.length === 0"
                class="rounded-md border border-dashed px-3 py-6 text-center text-xs text-muted-foreground"
              >
                No local hooks yet — click <strong>New hook</strong> to
                generate one with AI, or add one straight from a bundle's
                <strong>Add hook</strong> picker.
              </div>
              <div v-else class="space-y-2">
                <div
                  v-for="entry in localHooks"
                  :key="entry.filename"
                  class="flex items-center gap-3 rounded-md border bg-card/40 px-3 py-2"
                >
                  <Terminal class="size-3.5 shrink-0 text-muted-foreground" />
                  <div class="min-w-0 flex-1">
                    <div class="font-mono text-[12.5px] font-medium">
                      {{ entry.filename }}
                    </div>
                    <div class="mt-0.5 truncate text-[11px] text-muted-foreground">
                      {{ hookCardLabel(entry) }}
                      <template v-if="entry.meta?.description">
                        — {{ entry.meta.description }}
                      </template>
                    </div>
                  </div>
                  <Tooltip>
                    <TooltipTrigger as-child>
                      <Button
                        size="sm"
                        variant="outline"
                        class="h-7 px-2 text-xs"
                        :disabled="!canInstall"
                        @click="store.applyHookLocal(entry.filename)"
                      >
                        <Server class="size-3" />
                        Install
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent>
                      {{ canInstall ? "Symlink + register in settings.json" : "Pick a project first" }}
                    </TooltipContent>
                  </Tooltip>
                  <Button
                    size="sm"
                    variant="ghost"
                    class="h-7 px-2 text-xs"
                    @click="openEditHook(entry.filename)"
                  >
                    <Pencil class="size-3" />
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    class="h-7 px-2 text-xs text-destructive hover:text-destructive"
                    @click="deleteHookName = entry.filename"
                  >
                    <Trash2 class="size-3" />
                  </Button>
                </div>
              </div>
            </div>
          </template>

          <!-- Local MCP section — only on the __local__ bucket. Lists
               each in-app-created server with edit/delete actions. -->
          <template v-if="isLocal">
            <Separator class="my-1" />
            <div class="p-3">
              <div class="flex items-center justify-between px-2.5 pt-1 pb-2">
                <div class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                  Local MCP Servers
                </div>
                <Button
                  size="sm"
                  variant="outline"
                  class="h-6 px-2 text-xs"
                  @click="openCreateMcp"
                >
                  <Plus class="size-3" />
                  New MCP
                </Button>
              </div>
              <div
                v-if="localMcps.length === 0"
                class="rounded-md border border-dashed px-3 py-6 text-center text-xs text-muted-foreground"
              >
                No local MCP servers yet — click <strong>New MCP</strong> to
                define one, or add one straight from a bundle's
                <strong>Add MCP</strong> picker.
              </div>
              <div v-else class="space-y-2">
                <div
                  v-for="entry in localMcps"
                  :key="entry.name"
                  class="flex items-center gap-3 rounded-md border bg-card/40 px-3 py-2"
                >
                  <Plug class="size-3.5 shrink-0 text-muted-foreground" />
                  <div class="min-w-0 flex-1">
                    <div class="font-mono text-[12.5px] font-medium">
                      {{ entry.name }}
                    </div>
                    <div class="mt-0.5 truncate font-mono text-[11px] text-muted-foreground">
                      {{ mcpCardPreview(entry) }}
                    </div>
                  </div>
                  <Tooltip>
                    <TooltipTrigger as-child>
                      <Button
                        size="sm"
                        variant="outline"
                        class="h-7 px-2 text-xs"
                        :disabled="!canInstall"
                        @click="store.applyMcpLocal(entry.name)"
                      >
                        <Server class="size-3" />
                        Merge
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent>
                      {{ canInstall ? "Merge into .claude/mcp.json" : "Pick a project first" }}
                    </TooltipContent>
                  </Tooltip>
                  <Button
                    size="sm"
                    variant="ghost"
                    class="h-7 px-2 text-xs"
                    @click="openEditMcp(entry.name)"
                  >
                    <Pencil class="size-3" />
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    class="h-7 px-2 text-xs text-destructive hover:text-destructive"
                    @click="deleteMcpName = entry.name"
                  >
                    <Trash2 class="size-3" />
                  </Button>
                </div>
              </div>
            </div>
          </template>

          <!-- MCP section -->
          <template v-if="pluginMcp">
            <Separator class="my-1" />
            <div class="p-3">
              <div class="flex items-center justify-between px-2.5 pt-1 pb-2">
                <div class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                  MCP Servers
                </div>
                <Tooltip>
                  <TooltipTrigger as-child>
                    <Button
                      size="sm"
                      variant="outline"
                      class="h-6 px-2 text-xs"
                      :disabled="!canInstall"
                      @click="store.applyMcp(pluginMcp!.plugin)"
                    >
                      <Server class="size-3" />
                      Merge into project
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>
                    {{ canInstall ? "Merge MCP servers into .claude/mcp.json" : "Pick a project first" }}
                  </TooltipContent>
                </Tooltip>
              </div>
              <pre class="overflow-x-auto rounded-md border bg-muted/40 px-3 py-2 font-mono text-[11px] leading-relaxed text-muted-foreground">{{ mcpServersJson }}</pre>
            </div>
          </template>
        </div>
      </ScrollArea>
    </div>

    <!-- Delete confirmation -->
    <AlertDialog v-model:open="deleteOpen">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Remove "{{ props.plugin }}"?</AlertDialogTitle>
          <AlertDialogDescription>
            <span class="block">
              This deletes all assets, hooks, and MCP config imported from this plugin.
            </span>
            <span class="mt-2 block text-xs">
              Assets already applied to projects (as symlinks) will become broken links — use
              <code class="rounded bg-muted px-1 py-0.5 font-mono text-[11px]">Clean all</code>
              in Project view afterward to remove them.
            </span>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            class="bg-destructive text-white hover:bg-destructive/90"
            @click="confirmDelete"
          >Remove plugin</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>

    <!-- Local hook: create / edit -->
    <NewHookDialog
      v-model:open="newHookOpen"
      :edit-name="editHookName"
      @saved="onHookSaved"
    />

    <!-- Local hook: delete confirmation -->
    <AlertDialog
      :open="!!deleteHookName"
      @update:open="(v) => !v && (deleteHookName = null)"
    >
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete hook "{{ deleteHookName }}"?</AlertDialogTitle>
          <AlertDialogDescription>
            Removes the script and its metadata sidecar under
            <code class="rounded bg-muted px-1 py-0.5 font-mono text-[11px]">library/hooks/__local__/</code>.
            Bundles that reference it will fail to apply this hook until you
            re-create or remove the reference.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            class="bg-destructive text-white hover:bg-destructive/90"
            @click="confirmDeleteHook"
          >Delete</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>

    <!-- Local MCP: create / edit -->
    <NewMcpDialog
      v-model:open="newMcpOpen"
      :edit-name="editMcpName"
      @saved="onMcpSaved"
    />

    <!-- Local MCP: delete confirmation -->
    <AlertDialog
      :open="!!deleteMcpName"
      @update:open="(v) => !v && (deleteMcpName = null)"
    >
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete MCP "{{ deleteMcpName }}"?</AlertDialogTitle>
          <AlertDialogDescription>
            Removes the file under
            <code class="rounded bg-muted px-1 py-0.5 font-mono text-[11px]">library/mcp/__local__/</code>.
            Bundles that reference it will fail to apply this MCP until you
            re-create or remove the reference.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            class="bg-destructive text-white hover:bg-destructive/90"
            @click="confirmDeleteMcp"
          >Delete</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </TooltipProvider>
</template>
