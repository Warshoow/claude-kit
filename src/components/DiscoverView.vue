<script setup lang="ts">
import { computed, ref } from "vue";
import type { Marketplace, Plugin, PluginSource } from "../lib/types";

const props = defineProps<{
  marketplace: Marketplace | null;
  loading: boolean;
  error: string | null;
  importingPlugin: string | null;
}>();

defineEmits<{ refresh: []; import: [plugin: Plugin] }>();

const query = ref("");
const selectedCategory = ref<string>("");

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
    if (cat && p.category !== cat) return false;
    if (!q) return true;
    if (p.name.toLowerCase().includes(q)) return true;
    if (p.description.toLowerCase().includes(q)) return true;
    if (p.author?.name.toLowerCase().includes(q)) return true;
    return false;
  });
});

function sourceLabel(src: PluginSource): string {
  if (typeof src === "string") return src;
  if (src.source === "url") return shortRepo(src.url);
  if (src.source === "git-subdir") return `${shortRepo(src.url)} · ${src.path}`;
  if (src.source === "github") return src.repo;
  return "";
}

function shortRepo(url: string): string {
  // strip protocol, trailing .git, github.com/ prefix
  return url
    .replace(/^https?:\/\//, "")
    .replace(/^github\.com\//, "")
    .replace(/\.git$/, "");
}
</script>

<template>
  <div class="discover">
    <div class="discover-header">
      <div class="discover-title">
        <span class="discover-name">{{ marketplace?.name ?? "Discover" }}</span>
        <span v-if="marketplace" class="discover-count">
          {{ filtered.length }}<span v-if="filtered.length !== plugins.length">/{{ plugins.length }}</span> plugins
        </span>
      </div>
      <div style="flex:1" />
      <button @click="$emit('refresh')" :disabled="loading" title="Re-fetch marketplace">
        {{ loading ? "Loading…" : "Refresh" }}
      </button>
    </div>

    <div v-if="marketplace" class="discover-filters">
      <input
        v-model="query"
        type="text"
        placeholder="Search name, description, author…"
      />
      <select v-model="selectedCategory" class="cat-select">
        <option value="">All categories</option>
        <option v-for="c in categories" :key="c" :value="c">{{ c }}</option>
      </select>
      <button v-if="query || selectedCategory" @click="query = ''; selectedCategory = ''">Clear</button>
    </div>

    <div class="discover-body">
      <div v-if="loading && !marketplace" class="empty">Fetching marketplace…</div>
      <div v-else-if="error" class="discover-error">
        Failed to load marketplace.<br />
        <code>{{ error }}</code>
        <div style="margin-top: 12px"><button @click="$emit('refresh')">Retry</button></div>
      </div>
      <div v-else-if="filtered.length === 0" class="empty">
        <template v-if="plugins.length === 0">No plugins.</template>
        <template v-else>No match.</template>
      </div>
      <div v-else class="plugin-grid">
        <div v-for="p in filtered" :key="p.name" class="plugin-card">
          <div class="plugin-head">
            <div class="plugin-name">{{ p.name }}</div>
            <span v-if="p.category" class="plugin-cat">{{ p.category }}</span>
          </div>
          <div class="plugin-desc">{{ p.description }}</div>
          <div class="plugin-meta">
            <span v-if="p.author" class="plugin-author">by {{ p.author.name }}</span>
            <span class="plugin-source" :title="sourceLabel(p.source)">{{ sourceLabel(p.source) }}</span>
          </div>
          <div class="plugin-actions">
            <button
              class="primary"
              :disabled="!!importingPlugin"
              @click="$emit('import', p)"
            >
              {{ importingPlugin === p.name ? "Importing…" : "Import" }}
            </button>
            <a v-if="p.homepage" :href="p.homepage" target="_blank" rel="noopener" class="plugin-home">Homepage ↗</a>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.discover {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
}

.discover-header {
  display: flex;
  align-items: center;
  padding: 10px 14px;
  background: var(--bg-elev);
  border-bottom: 1px solid var(--border);
  gap: 12px;
}

.discover-title {
  display: flex;
  align-items: baseline;
  gap: 10px;
}

.discover-name {
  font-weight: 600;
}

.discover-count {
  color: var(--text-faint);
  font-size: 12px;
}

.discover-filters {
  display: flex;
  gap: 8px;
  padding: 8px 14px;
  border-bottom: 1px solid var(--border);
  align-items: center;
}

.discover-filters input[type="text"] {
  flex: 1;
}

.cat-select {
  font-family: inherit;
  font-size: inherit;
  padding: 6px 10px;
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 6px;
  outline: none;
  min-width: 180px;
}

.cat-select:focus {
  border-color: var(--accent);
}

.discover-body {
  flex: 1;
  overflow-y: auto;
  padding: 14px;
}

.plugin-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 12px;
}

.plugin-card {
  display: flex;
  flex-direction: column;
  padding: 12px 14px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg-elev);
  gap: 8px;
}

.plugin-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.plugin-name {
  font-weight: 600;
  word-break: break-word;
}

.plugin-cat {
  padding: 2px 8px;
  border-radius: 4px;
  background: var(--bg);
  font-size: 11px;
  color: var(--text-dim);
  border: 1px solid var(--border);
  flex-shrink: 0;
}

.plugin-desc {
  color: var(--text-dim);
  font-size: 12px;
  line-height: 1.45;
  display: -webkit-box;
  -webkit-line-clamp: 4;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.plugin-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  font-size: 11px;
  color: var(--text-faint);
}

.plugin-author {
  font-style: italic;
}

.plugin-source {
  font-family: ui-monospace, "SF Mono", Menlo, monospace;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 100%;
}

.plugin-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: auto;
  padding-top: 4px;
}

.plugin-home {
  color: var(--text-dim);
  font-size: 12px;
  text-decoration: none;
}

.plugin-home:hover {
  color: var(--accent);
}

.discover-error {
  padding: 20px;
  color: var(--danger);
  font-size: 13px;
  background: color-mix(in srgb, var(--danger) 10%, transparent);
  border: 1px solid color-mix(in srgb, var(--danger) 30%, transparent);
  border-radius: 6px;
}

.discover-error code {
  display: block;
  margin-top: 6px;
  font-size: 11px;
  color: var(--text-dim);
  word-break: break-word;
}
</style>
