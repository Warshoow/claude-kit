<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import { ArrowUpCircle, Check, Download, RefreshCw, Search, X } from "lucide-vue-next";
import { useAppStore } from "@/stores/app";
import type { Plugin, PluginSource } from "@/lib/types";
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

const store = useAppStore();
const router = useRouter();
const {
  marketplace,
  marketplaceLoading,
  marketplaceError,
  importingPlugin,
  library,
} = storeToRefs(store);

function statusOf(p: Plugin): PluginImportStatus {
  return pluginImportStatus(library.value, marketplace.value?.name, p);
}

function statusTitle(p: Plugin): string | undefined {
  const s = statusOf(p);
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

const plugins = computed<Plugin[]>(() => marketplace.value?.plugins ?? []);

const categories = computed<string[]>(() => {
  const set = new Set<string>();
  for (const p of plugins.value) if (p.category) set.add(p.category);
  return Array.from(set).sort();
});

const filtered = computed<Plugin[]>(() => {
  const q = query.value.trim().toLowerCase();
  const cat = selectedCategory.value;
  return plugins.value.filter((p) => {
    if (cat !== "all" && p.category !== cat) return false;
    if (!q) return true;
    if (p.name.toLowerCase().includes(q)) return true;
    if (p.description.toLowerCase().includes(q)) return true;
    if (p.author?.name.toLowerCase().includes(q)) return true;
    return false;
  });
});

const hasActiveFilter = computed(
  () => query.value.length > 0 || selectedCategory.value !== "all"
);

function clearFilters() {
  query.value = "";
  selectedCategory.value = "all";
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
onMounted(() => store.loadMarketplace());
</script>

<template>
  <div class="flex h-full flex-col overflow-hidden">
    <!-- Toolbar: count + search + filter + refresh -->
    <div class="flex flex-wrap items-center gap-2 border-b px-4 py-2">
      <span class="text-xs text-muted-foreground">
        <template v-if="marketplace">
          {{ marketplace.name }} ·
          {{ filtered.length
          }}<span v-if="filtered.length !== plugins.length"
            >/{{ plugins.length }}</span
          >
          plugin{{ filtered.length === 1 ? "" : "s" }}
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
        @click="store.loadMarketplace(true)"
      >
        <RefreshCw :class="marketplaceLoading ? 'animate-spin' : ''" />
        {{ marketplaceLoading ? "Loading…" : "Refresh" }}
      </Button>
    </div>

    <!-- Body -->
    <ScrollArea class="flex-1 min-h-0">
      <div
        v-if="marketplaceLoading && !marketplace"
        class="px-6 py-16 text-center text-sm text-muted-foreground"
      >
        Fetching marketplace…
      </div>

      <div
        v-else-if="marketplaceError"
        class="m-6 rounded-lg border border-destructive/30 bg-destructive/10 p-4"
      >
        <div class="text-sm font-medium text-destructive">
          Failed to load marketplace
        </div>
        <code class="mt-2 block break-words text-xs text-muted-foreground">{{ marketplaceError }}</code>
        <Button
          class="mt-3"
          size="sm"
          variant="outline"
          @click="store.loadMarketplace(true)"
        >Retry</Button>
      </div>

      <div
        v-else-if="filtered.length === 0"
        class="px-6 py-16 text-center text-sm text-muted-foreground"
      >
        <template v-if="plugins.length === 0">No plugins.</template>
        <template v-else>No match for the current filters.</template>
      </div>

      <div
        v-else
        class="grid gap-4 p-6"
        style="grid-template-columns: repeat(auto-fill, minmax(340px, 1fr))"
      >
        <article
          v-for="p in filtered"
          :key="p.name"
          class="group flex cursor-pointer flex-col rounded-xl border bg-card p-4 transition-all hover:-translate-y-0.5 hover:border-primary/40 hover:shadow-md focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
          tabindex="0"
          @click="openPlugin(p)"
          @keydown.enter="openPlugin(p)"
          @keydown.space.prevent="openPlugin(p)"
        >
          <header class="flex items-start justify-between gap-2">
            <h3 class="break-words text-sm font-semibold leading-snug">
              {{ p.name }}
            </h3>
            <div class="flex shrink-0 items-center gap-1.5">
              <template v-if="statusOf(p).kind === 'update'">
                <Badge
                  variant="outline"
                  class="gap-1 border-amber-500/50 text-[10px] uppercase tracking-wider text-amber-600 dark:text-amber-400"
                  :title="statusTitle(p)"
                >
                  <ArrowUpCircle class="size-2.5" />
                  Update
                </Badge>
              </template>
              <template v-else-if="statusOf(p).kind === 'current' || statusOf(p).kind === 'unknown'">
                <Badge
                  variant="outline"
                  class="gap-1 border-primary/40 text-[10px] uppercase tracking-wider text-primary"
                  :title="statusTitle(p)"
                >
                  <Check class="size-2.5" />
                  In library
                </Badge>
              </template>
              <Badge
                v-if="p.category"
                variant="secondary"
                class="text-[10px] uppercase tracking-wider"
              >{{ p.category }}</Badge>
            </div>
          </header>

          <p class="mt-2 line-clamp-3 text-xs leading-relaxed text-muted-foreground">
            {{ p.description }}
          </p>

          <div class="mt-3 flex flex-wrap items-center gap-1.5 text-[11px] text-muted-foreground">
            <span v-if="p.author">{{ p.author.name }}</span>
            <span v-if="p.author" class="opacity-50">·</span>
            <span
              class="truncate font-mono"
              :title="sourceLabel(p.source)"
            >{{ sourceLabel(p.source) }}</span>
          </div>

          <div class="mt-4 flex items-center gap-2 pt-2">
            <template v-if="statusOf(p).kind === 'update'">
              <Button
                size="sm"
                :disabled="!!importingPlugin"
                title="Re-pulls the plugin. New files since last import are added; existing files are NOT overwritten yet (coming in a later version)."
                @click.stop="store.importMarketplacePlugin(p)"
              >
                <ArrowUpCircle />
                {{ importingPlugin === p.name ? "Updating…" : "Update" }}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                @click.stop="viewInLibrary(p)"
              >View in library</Button>
            </template>
            <template v-else-if="statusOf(p).kind === 'current' || statusOf(p).kind === 'unknown'">
              <Button
                size="sm"
                variant="outline"
                @click.stop="viewInLibrary(p)"
              >
                <Check />
                View in library
              </Button>
              <Button
                variant="ghost"
                size="sm"
                :disabled="!!importingPlugin"
                title="Re-pull the plugin (any new files since last import will be added; existing files are kept)."
                @click.stop="store.importMarketplacePlugin(p)"
              >
                {{ importingPlugin === p.name ? "Re-importing…" : "Re-import" }}
              </Button>
            </template>
            <template v-else>
              <Button
                size="sm"
                :disabled="!!importingPlugin"
                @click.stop="store.importMarketplacePlugin(p)"
              >
                <Download />
                {{ importingPlugin === p.name ? "Importing…" : "Import" }}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                @click.stop="openPlugin(p)"
              >Details</Button>
            </template>
          </div>
        </article>
      </div>
    </ScrollArea>
  </div>
</template>
