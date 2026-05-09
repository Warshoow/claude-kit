<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import { ArrowUpCircle, Check, Download, RefreshCw, Search, X } from "lucide-vue-next";
import { useAppStore } from "@/stores/app";
import { useMarketplaceStore } from "@/stores/marketplace";
import type { MarketplaceSource, Plugin, PluginSource } from "@/lib/types";
import { pluginImportStatus, type PluginImportStatus } from "@/lib/origins";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

interface PluginEntry {
  plugin: Plugin;
  source: MarketplaceSource;
}

const store = useAppStore();
const marketplaceStore = useMarketplaceStore();
const router = useRouter();
const { library, hooks, mcp } = storeToRefs(store);
const {
  allPlugins,
  catalogs,
  marketplaceLoading,
  marketplaceError,
  importingPlugin,
} = storeToRefs(marketplaceStore);

function statusOf(entry: PluginEntry): PluginImportStatus {
  return pluginImportStatus(
    library.value,
    entry.source.name,
    entry.plugin,
    hooks.value,
    mcp.value
  );
}

function statusTitle(entry: PluginEntry): string | undefined {
  const s = statusOf(entry);
  if (s.kind === "update") {
    return `Imported v${s.importedVersion} → marketplace has v${s.currentVersion}`;
  }
  if (s.kind === "unknown") {
    return "Imported before version tracking — can't tell if updates exist";
  }
  return undefined;
}

function viewInLibrary(p: Plugin) {
  router.push({
    name: "browse-library-plugin",
    params: { plugin: p.name },
  });
}

const query = ref("");
const selectedCategory = ref<string>("all");
const selectedSource = ref<string>("all");

const showSourceBadges = computed(() => catalogs.value.length > 1);

const entries = computed<PluginEntry[]>(() => allPlugins.value);

const categories = computed<string[]>(() => {
  const set = new Set<string>();
  for (const e of entries.value) if (e.plugin.category) set.add(e.plugin.category);
  return Array.from(set).sort();
});

const sourceNames = computed<string[]>(() =>
  catalogs.value.map((c) => c.source.name)
);

const filtered = computed<PluginEntry[]>(() => {
  const q = query.value.trim().toLowerCase();
  const cat = selectedCategory.value;
  const src = selectedSource.value;
  return entries.value.filter((e) => {
    const p = e.plugin;
    if (src !== "all" && e.source.name !== src) return false;
    if (cat !== "all" && p.category !== cat) return false;
    if (!q) return true;
    if (p.name.toLowerCase().includes(q)) return true;
    if (p.description.toLowerCase().includes(q)) return true;
    if (p.author?.name.toLowerCase().includes(q)) return true;
    return false;
  });
});

const hasActiveFilter = computed(
  () =>
    query.value.length > 0 ||
    selectedCategory.value !== "all" ||
    selectedSource.value !== "all"
);

function clearFilters() {
  query.value = "";
  selectedCategory.value = "all";
  selectedSource.value = "all";
}

function openPlugin(p: Plugin) {
  router.push({ name: "plugin-detail", params: { name: p.name } });
}

function sourceLabel(src: PluginSource): string {
  if (typeof src === "string") return src;
  if (src.source === "url") return shortRepo(src.url);
  if (src.source === "git-subdir") return `${shortRepo(src.url)} · ${src.path}`;
  if (src.source === "github") return src.repo;
  return "";
}

function shortRepo(url: string): string {
  return url
    .replace(/^https?:\/\//, "")
    .replace(/^github\.com\//, "")
    .replace(/\.git$/, "");
}

// Lazy fetch on first visit; cached afterwards.
onMounted(() => marketplaceStore.loadMarketplace());
</script>

<template>
  <div class="flex h-full flex-col overflow-hidden">
    <!-- Toolbar: count + search + filter + refresh -->
    <div class="flex flex-wrap items-center gap-2 border-b px-4 py-2">
      <span class="text-xs text-muted-foreground">
        <template v-if="catalogs.length">
          {{ filtered.length
          }}<span v-if="filtered.length !== entries.length"
            >/{{ entries.length }}</span
          >
          plugin{{ filtered.length === 1 ? "" : "s" }}
          <span v-if="catalogs.length > 1" class="opacity-50">
            · {{ catalogs.length }} marketplaces
          </span>
        </template>
      </span>

      <div class="relative flex-1 max-w-md">
        <Search
          class="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground"
        />
        <Input
          v-model="query"
          type="text"
          placeholder="Search name, description, author…"
          class="h-8 pl-8"
        />
      </div>

      <Select v-if="catalogs.length > 1" v-model="selectedSource">
        <SelectTrigger class="h-8 w-[180px]">
          <SelectValue placeholder="All marketplaces" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="all">All marketplaces</SelectItem>
          <SelectItem v-for="s in sourceNames" :key="s" :value="s">
            {{ s }}
          </SelectItem>
        </SelectContent>
      </Select>

      <Select v-model="selectedCategory">
        <SelectTrigger class="h-8 w-[180px]">
          <SelectValue placeholder="All categories" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="all">All categories</SelectItem>
          <SelectItem v-for="c in categories" :key="c" :value="c">
            {{ c }}
          </SelectItem>
        </SelectContent>
      </Select>

      <Button
        v-if="hasActiveFilter"
        variant="ghost"
        size="sm"
        @click="clearFilters"
      >
        <X />
        Clear
      </Button>

      <div class="flex-1" />

      <Button
        variant="outline"
        size="sm"
        :disabled="marketplaceLoading"
        @click="marketplaceStore.loadMarketplace(true)"
      >
        <RefreshCw :class="marketplaceLoading ? 'animate-spin' : ''" />
        {{ marketplaceLoading ? "Loading…" : "Refresh" }}
      </Button>
    </div>

    <!-- Body -->
    <ScrollArea class="flex-1 min-h-0">
      <div
        v-if="marketplaceLoading && !catalogs.length"
        class="px-6 py-16 text-center text-sm text-muted-foreground"
      >
        Fetching marketplaces…
      </div>

      <div
        v-else-if="marketplaceError && !entries.length"
        class="m-6 rounded-lg border border-destructive/30 bg-destructive/10 p-4"
      >
        <div class="text-sm font-medium text-destructive">
          Failed to load marketplaces
        </div>
        <code class="mt-2 block break-words text-xs text-muted-foreground">{{ marketplaceError }}</code>
        <Button
          class="mt-3"
          size="sm"
          variant="outline"
          @click="marketplaceStore.loadMarketplace(true)"
        >Retry</Button>
      </div>

      <template v-else>
        <!-- Partial-failure banner: some sources OK, some not -->
        <div
          v-if="marketplaceError"
          class="m-3 rounded-md border border-amber-500/40 bg-amber-500/10 p-2.5 text-[11px] text-amber-700 dark:text-amber-300"
        >
          Some marketplaces couldn't load: <code class="font-mono">{{ marketplaceError }}</code>
        </div>

        <div
          v-if="filtered.length === 0"
          class="px-6 py-16 text-center text-sm text-muted-foreground"
        >
          <template v-if="entries.length === 0">No plugins.</template>
          <template v-else>No match for the current filters.</template>
        </div>

        <div
          v-else
          class="grid gap-4 p-6"
          style="grid-template-columns: repeat(auto-fill, minmax(340px, 1fr))"
        >
          <article
            v-for="e in filtered"
            :key="`${e.source.name}/${e.plugin.name}`"
            class="group flex cursor-pointer flex-col rounded-xl border bg-card p-4 transition-all hover:-translate-y-0.5 hover:border-primary/40 hover:shadow-md focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
            tabindex="0"
            @click="openPlugin(e.plugin)"
            @keydown.enter="openPlugin(e.plugin)"
            @keydown.space.prevent="openPlugin(e.plugin)"
          >
            <header class="flex items-start justify-between gap-2">
              <h3 class="break-words text-sm font-semibold leading-snug">
                {{ e.plugin.name }}
              </h3>
              <div class="flex shrink-0 items-center gap-1.5">
                <template v-if="statusOf(e).kind === 'update'">
                  <Badge
                    variant="outline"
                    class="gap-1 border-amber-500/50 text-[10px] uppercase tracking-wider text-amber-600 dark:text-amber-400"
                    :title="statusTitle(e)"
                  >
                    <ArrowUpCircle class="size-2.5" />
                    Update
                  </Badge>
                </template>
                <template v-else-if="statusOf(e).kind === 'current' || statusOf(e).kind === 'unknown'">
                  <Badge
                    variant="outline"
                    class="gap-1 border-primary/40 text-[10px] uppercase tracking-wider text-primary"
                    :title="statusTitle(e)"
                  >
                    <Check class="size-2.5" />
                    In library
                  </Badge>
                </template>
                <Badge
                  v-if="e.plugin.category"
                  variant="secondary"
                  class="text-[10px] uppercase tracking-wider"
                >{{ e.plugin.category }}</Badge>
              </div>
            </header>

            <p class="mt-2 line-clamp-3 text-xs leading-relaxed text-muted-foreground">
              {{ e.plugin.description }}
            </p>

            <div class="mt-3 flex flex-wrap items-center gap-1.5 text-[11px] text-muted-foreground">
              <span v-if="e.plugin.author">{{ e.plugin.author.name }}</span>
              <span v-if="e.plugin.author" class="opacity-50">·</span>
              <span
                class="truncate font-mono"
                :title="sourceLabel(e.plugin.source)"
              >{{ sourceLabel(e.plugin.source) }}</span>
              <Badge
                v-if="showSourceBadges"
                variant="outline"
                class="ml-auto text-[10px] uppercase tracking-wider"
                :title="`from ${e.source.name}`"
              >{{ e.source.name }}</Badge>
            </div>

            <div class="mt-4 flex items-center gap-2 pt-2">
              <template v-if="statusOf(e).kind === 'update'">
                <Button
                  size="sm"
                  title="Review the upstream changes hunk-by-hunk before overwriting your local copy."
                  @click.stop="router.push({ name: 'plugin-update', params: { name: e.plugin.name } })"
                >
                  <ArrowUpCircle />
                  Update…
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  @click.stop="viewInLibrary(e.plugin)"
                >View in library</Button>
              </template>
              <template v-else-if="statusOf(e).kind === 'current' || statusOf(e).kind === 'unknown'">
                <Button
                  size="sm"
                  variant="outline"
                  @click.stop="viewInLibrary(e.plugin)"
                >
                  <Check />
                  View in library
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  :disabled="!!importingPlugin"
                  title="Re-pull the plugin (any new files since last import will be added; existing files are kept)."
                  @click.stop="marketplaceStore.importMarketplacePlugin(e.plugin, e.source.name)"
                >
                  {{ importingPlugin === e.plugin.name ? "Re-importing…" : "Re-import" }}
                </Button>
              </template>
              <template v-else>
                <Button
                  size="sm"
                  :disabled="!!importingPlugin"
                  @click.stop="marketplaceStore.importMarketplacePlugin(e.plugin, e.source.name)"
                >
                  <Download />
                  {{ importingPlugin === e.plugin.name ? "Importing…" : "Import" }}
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  @click.stop="openPlugin(e.plugin)"
                >Details</Button>
              </template>
            </div>
          </article>
        </div>
      </template>
    </ScrollArea>
  </div>
</template>
