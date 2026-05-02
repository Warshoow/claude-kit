<script setup lang="ts">
import { computed, ref } from "vue";
import { Download, RefreshCw, Search, X } from "lucide-vue-next";
import type { Marketplace, Plugin, PluginSource } from "@/lib/types";
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

const props = defineProps<{
  marketplace: Marketplace | null;
  loading: boolean;
  error: string | null;
  importingPlugin: string | null;
}>();

defineEmits<{
  refresh: [];
  import: [plugin: Plugin];
  select: [plugin: Plugin];
}>();

const query = ref("");
const selectedCategory = ref<string>("all");

const plugins = computed<Plugin[]>(() => props.marketplace?.plugins ?? []);

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
</script>

<template>
  <div class="flex h-full flex-col overflow-hidden">
    <!-- Header -->
    <div class="flex items-center gap-3 border-b bg-card/40 px-6 py-3">
      <div class="flex items-baseline gap-2">
        <span class="text-sm font-semibold">{{ marketplace?.name ?? "Discover" }}</span>
        <span v-if="marketplace" class="text-xs text-muted-foreground">
          {{ filtered.length
          }}<span v-if="filtered.length !== plugins.length"
            >/{{ plugins.length }}</span
          >
          plugins
        </span>
      </div>
      <div class="flex-1" />
      <Button
        variant="outline"
        size="sm"
        :disabled="loading"
        @click="$emit('refresh')"
      >
        <RefreshCw :class="loading ? 'animate-spin' : ''" />
        {{ loading ? "Loading…" : "Refresh" }}
      </Button>
    </div>

    <!-- Filters -->
    <div
      v-if="marketplace"
      class="flex items-center gap-2 border-b px-6 py-2.5"
    >
      <div class="relative flex-1 max-w-md">
        <Search
          class="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground"
        />
        <Input
          v-model="query"
          type="text"
          placeholder="Search name, description, author…"
          class="h-9 pl-8"
        />
      </div>
      <Select v-model="selectedCategory">
        <SelectTrigger class="h-9 w-[200px]">
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
    </div>

    <!-- Body -->
    <ScrollArea class="flex-1 min-h-0">
      <div v-if="loading && !marketplace" class="px-6 py-16 text-center text-sm text-muted-foreground">
        Fetching marketplace…
      </div>
      <div v-else-if="error" class="m-6 rounded-lg border border-destructive/30 bg-destructive/10 p-4">
        <div class="text-sm font-medium text-destructive">Failed to load marketplace</div>
        <code class="mt-2 block break-words text-xs text-muted-foreground">{{ error }}</code>
        <Button class="mt-3" size="sm" variant="outline" @click="$emit('refresh')">
          Retry
        </Button>
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
          @click="$emit('select', p)"
          @keydown.enter="$emit('select', p)"
          @keydown.space.prevent="$emit('select', p)"
        >
          <header class="flex items-start justify-between gap-2">
            <h3 class="break-words text-sm font-semibold leading-snug">
              {{ p.name }}
            </h3>
            <Badge
              v-if="p.category"
              variant="secondary"
              class="shrink-0 text-[10px] uppercase tracking-wider"
            >{{ p.category }}</Badge>
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
            <Button
              size="sm"
              :disabled="!!importingPlugin"
              @click.stop="$emit('import', p)"
            >
              <Download />
              {{ importingPlugin === p.name ? "Importing…" : "Import" }}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              @click.stop="$emit('select', p)"
            >Details</Button>
          </div>
        </article>
      </div>
    </ScrollArea>
  </div>
</template>
