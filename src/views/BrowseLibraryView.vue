<script setup lang="ts">
import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import {
  Boxes,
  ChevronRight,
  FolderOpen,
  Package,
  Plus,
  Search,
  Upload,
  X,
} from "lucide-vue-next";
import { useAppStore } from "@/stores/app";
import type { Asset, AssetKind } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import NewAssetDialog from "@/components/NewAssetDialog.vue";

const LOCAL_KEY = "__local__";

interface PluginGroup {
  /** Identifier used in the route — `__local__` for the local bucket. */
  key: string;
  /** Display name. */
  name: string;
  /** Marketplace this plugin came from, if any. */
  marketplace: string | null;
  assets: Asset[];
  counts: Record<AssetKind, number>;
  isLocal: boolean;
}

const store = useAppStore();
const router = useRouter();
const { library } = storeToRefs(store);

const query = ref("");
const newAssetOpen = ref(false);

const groups = computed<PluginGroup[]>(() => {
  const buckets = new Map<string, PluginGroup>();

  for (const a of library.value) {
    const key = a.origin?.plugin ?? LOCAL_KEY;
    let g = buckets.get(key);
    if (!g) {
      g = {
        key,
        name: a.origin?.plugin ?? "Local",
        marketplace: a.origin?.marketplace ?? null,
        assets: [],
        counts: { skills: 0, commands: 0, agents: 0 },
        isLocal: !a.origin,
      };
      buckets.set(key, g);
    }
    g.assets.push(a);
    g.counts[a.kind]++;
  }

  return Array.from(buckets.values()).sort((a, b) => {
    // Local always first, then alphabetical.
    if (a.isLocal) return -1;
    if (b.isLocal) return 1;
    return a.name.localeCompare(b.name);
  });
});

const filtered = computed<PluginGroup[]>(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return groups.value;
  return groups.value.filter((g) => {
    if (g.name.toLowerCase().includes(q)) return true;
    if (g.marketplace?.toLowerCase().includes(q)) return true;
    // also match if any asset inside the plugin matches — useful when searching
    // for a specific skill/command name without remembering its plugin.
    return g.assets.some(
      (a) =>
        a.name.toLowerCase().includes(q) ||
        a.description?.toLowerCase().includes(q) ||
        a.tags?.some((t) => t.toLowerCase().includes(q))
    );
  });
});

const totalAssetCount = computed(() => library.value.length);

function openPlugin(g: PluginGroup) {
  router.push({
    name: "browse-library-plugin",
    params: { plugin: g.key },
  });
}
</script>

<template>
  <TooltipProvider :delay-duration="200">
    <div class="flex h-full flex-col overflow-hidden">
      <!-- Toolbar: count + search + create + import -->
      <div class="flex items-center gap-3 border-b px-4 py-2">
        <span class="text-xs text-muted-foreground">
          {{ filtered.length
          }}<span v-if="filtered.length !== groups.length">/{{ groups.length }}</span>
          plugin{{ filtered.length === 1 ? "" : "s" }}
          <span class="opacity-50">·</span>
          {{ totalAssetCount }}
          asset{{ totalAssetCount === 1 ? "" : "s" }}
        </span>

        <div class="relative flex-1 max-w-md">
          <Search
            class="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground"
          />
          <Input
            v-model="query"
            type="text"
            placeholder="Search plugin or asset…"
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

        <div class="flex-1" />

        <Tooltip>
          <TooltipTrigger as-child>
            <Button size="sm" @click="newAssetOpen = true">
              <Plus />
              New
            </Button>
          </TooltipTrigger>
          <TooltipContent>Create a new skill, command or agent</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger as-child>
            <Button variant="outline" size="sm" @click="store.importLocalPlugin">
              <Upload />
              Import
            </Button>
          </TooltipTrigger>
          <TooltipContent>Import assets from a plugin folder</TooltipContent>
        </Tooltip>
      </div>

      <!-- Body -->
      <ScrollArea class="flex-1 min-h-0">
        <template v-if="library.length === 0">
          <div
            class="flex h-full min-h-[400px] flex-col items-center justify-center px-6 py-16 text-center"
          >
            <div class="rounded-full bg-muted p-4">
              <Boxes class="size-8 text-muted-foreground" />
            </div>
            <h3 class="mt-4 text-base font-semibold">Empty library</h3>
            <p class="mt-1 max-w-sm text-sm text-muted-foreground">
              Create a skill, command or agent from scratch, or import from
              an existing plugin folder or the marketplace.
            </p>
            <div class="mt-5 flex gap-2">
              <Button @click="newAssetOpen = true">
                <Plus />
                New asset
              </Button>
              <Button variant="outline" @click="store.importLocalPlugin">
                <Upload />
                Import folder
              </Button>
            </div>
          </div>
        </template>

        <template v-else-if="filtered.length === 0">
          <div class="px-6 py-12 text-center text-xs text-muted-foreground">
            No match for "{{ query }}".
          </div>
        </template>

        <template v-else>
          <div
            class="grid gap-4 p-6"
            style="grid-template-columns: repeat(auto-fill, minmax(280px, 1fr))"
          >
            <article
              v-for="g in filtered"
              :key="g.key"
              class="group flex cursor-pointer flex-col rounded-xl border bg-card p-4 transition-all hover:-translate-y-0.5 hover:border-primary/40 hover:shadow-md focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
              :class="g.isLocal ? 'border-dashed' : ''"
              tabindex="0"
              @click="openPlugin(g)"
              @keydown.enter="openPlugin(g)"
              @keydown.space.prevent="openPlugin(g)"
            >
              <header class="flex items-start justify-between gap-2">
                <div class="flex min-w-0 items-center gap-2">
                  <FolderOpen
                    v-if="g.isLocal"
                    class="size-4 shrink-0 text-muted-foreground"
                  />
                  <Package
                    v-else
                    class="size-4 shrink-0 text-muted-foreground"
                  />
                  <h3 class="truncate text-sm font-semibold">{{ g.name }}</h3>
                </div>
                <ChevronRight
                  class="size-4 shrink-0 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100"
                />
              </header>

              <p
                v-if="g.isLocal"
                class="mt-2 text-xs leading-relaxed text-muted-foreground"
              >Hand-crafted assets, not tied to any imported plugin.</p>
              <p
                v-else-if="g.marketplace"
                class="mt-2 text-xs leading-relaxed text-muted-foreground"
              >
                from <span class="font-mono">{{ g.marketplace }}</span>
              </p>

              <div class="mt-3 flex flex-wrap items-center gap-1.5">
                <Badge
                  v-if="g.counts.skills > 0"
                  variant="secondary"
                  class="font-mono text-[10px]"
                >{{ g.counts.skills }} skill{{ g.counts.skills === 1 ? "" : "s" }}</Badge>
                <Badge
                  v-if="g.counts.commands > 0"
                  variant="secondary"
                  class="font-mono text-[10px]"
                >{{ g.counts.commands }} cmd{{ g.counts.commands === 1 ? "" : "s" }}</Badge>
                <Badge
                  v-if="g.counts.agents > 0"
                  variant="secondary"
                  class="font-mono text-[10px]"
                >{{ g.counts.agents }} agent{{ g.counts.agents === 1 ? "" : "s" }}</Badge>
              </div>
            </article>
          </div>
        </template>
      </ScrollArea>

      <NewAssetDialog v-model:open="newAssetOpen" />
    </div>
  </TooltipProvider>
</template>
