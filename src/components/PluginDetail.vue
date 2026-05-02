<script setup lang="ts">
import { computed, onMounted, onUnmounted, watch } from "vue";
import { marked } from "marked";
import DOMPurify from "dompurify";
import type { Plugin, PluginSource } from "../lib/types";

const props = defineProps<{
  plugin: Plugin | null;
  readme: string | null;
  readmeLoading: boolean;
  readmeError: string | null;
  importing: boolean;
}>();

const emit = defineEmits<{
  close: [];
  import: [plugin: Plugin];
}>();

const open = computed(() => props.plugin !== null);

const renderedReadme = computed<string>(() => {
  if (!props.readme) return "";
  const out = marked.parse(props.readme, { gfm: true, breaks: false, async: false });
  return DOMPurify.sanitize(typeof out === "string" ? out : "");
});

interface SourceLine {
  label: string;
  value: string;
  mono?: boolean;
}

function sourceLines(src: PluginSource): SourceLine[] {
  if (typeof src === "string") {
    return [
      { label: "Type", value: "inline (marketplace repo)" },
      { label: "Path", value: src, mono: true },
    ];
  }
  if (src.source === "url") {
    const lines: SourceLine[] = [
      { label: "Type", value: "url" },
      { label: "URL", value: src.url, mono: true },
    ];
    if (src.sha) lines.push({ label: "Commit", value: src.sha, mono: true });
    return lines;
  }
  if (src.source === "git-subdir") {
    const lines: SourceLine[] = [
      { label: "Type", value: "git-subdir" },
      { label: "URL", value: src.url, mono: true },
      { label: "Path", value: src.path, mono: true },
    ];
    if (src.sha) lines.push({ label: "Commit", value: src.sha, mono: true });
    else if (src.ref) lines.push({ label: "Ref", value: src.ref, mono: true });
    else if (src.branch) lines.push({ label: "Branch", value: src.branch, mono: true });
    return lines;
  }
  if (src.source === "github") {
    const lines: SourceLine[] = [
      { label: "Type", value: "github" },
      { label: "Repo", value: src.repo, mono: true },
    ];
    if (src.commit) lines.push({ label: "Commit", value: src.commit, mono: true });
    return lines;
  }
  return [];
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape" && open.value) emit("close");
}

onMounted(() => window.addEventListener("keydown", onKeydown));
onUnmounted(() => window.removeEventListener("keydown", onKeydown));

// Lock body scroll when open
watch(open, (isOpen) => {
  document.body.style.overflow = isOpen ? "hidden" : "";
});
</script>

<template>
  <Transition name="drawer">
    <div v-if="plugin" class="drawer-backdrop" @click="emit('close')">
      <aside class="drawer" @click.stop>
        <header class="drawer-header">
          <div class="drawer-title-row">
            <h2 class="drawer-title">{{ plugin.name }}</h2>
            <span v-if="plugin.category" class="drawer-cat">{{ plugin.category }}</span>
          </div>
          <button class="ghost drawer-close" @click="emit('close')" title="Close (Esc)">×</button>
        </header>

        <div class="drawer-meta-row">
          <div class="drawer-meta-info">
            <span v-if="plugin.author" class="drawer-author">{{ plugin.author.name }}</span>
            <a
              v-if="plugin.homepage"
              :href="plugin.homepage"
              target="_blank"
              rel="noopener"
              class="drawer-homepage"
            >Homepage ↗</a>
          </div>
          <button
            class="primary"
            :disabled="importing"
            @click="emit('import', plugin)"
          >
            {{ importing ? "Importing…" : "Import to library" }}
          </button>
        </div>

        <div class="drawer-body">
          <section class="drawer-section">
            <h3 class="drawer-section-title">Description</h3>
            <p class="drawer-description">{{ plugin.description }}</p>
          </section>

          <section class="drawer-section">
            <h3 class="drawer-section-title">Source</h3>
            <dl class="drawer-source">
              <template v-for="line in sourceLines(plugin.source)" :key="line.label">
                <dt>{{ line.label }}</dt>
                <dd :class="{ mono: line.mono }">{{ line.value }}</dd>
              </template>
            </dl>
          </section>

          <section class="drawer-section">
            <h3 class="drawer-section-title">Readme</h3>
            <div v-if="readmeLoading" class="drawer-readme-status">Loading readme…</div>
            <div v-else-if="readmeError" class="drawer-readme-status error">
              Couldn't load readme: {{ readmeError }}
            </div>
            <div v-else-if="!readme" class="drawer-readme-status muted">
              No readme found in this plugin's source.
            </div>
            <div v-else class="drawer-readme markdown" v-html="renderedReadme" />
          </section>
        </div>
      </aside>
    </div>
  </Transition>
</template>

<style scoped>
.drawer-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(2px);
  display: flex;
  justify-content: flex-end;
  z-index: 60;
}

.drawer {
  width: min(560px, 92vw);
  height: 100%;
  background: var(--bg);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  box-shadow: var(--shadow-lg);
  overflow: hidden;
}

.drawer-header {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 18px 20px 14px;
  background: var(--bg-elev);
  border-bottom: 1px solid var(--border);
}

.drawer-title-row {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.drawer-title {
  margin: 0;
  font-size: 17px;
  font-weight: 600;
  letter-spacing: -0.01em;
  word-break: break-word;
}

.drawer-cat {
  padding: 2px 8px;
  border-radius: var(--radius-sm);
  background: var(--accent-soft);
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--accent);
  border: 1px solid color-mix(in srgb, var(--accent) 25%, transparent);
  font-weight: 500;
  flex-shrink: 0;
}

.drawer-close {
  font-size: 22px;
  line-height: 1;
  padding: 2px 10px;
  margin-top: -2px;
}

.drawer-meta-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 20px;
  background: var(--bg-elev);
  border-bottom: 1px solid var(--border);
}

.drawer-meta-info {
  display: flex;
  align-items: center;
  gap: 14px;
  color: var(--text-dim);
  font-size: 12px;
}

.drawer-author {
  color: var(--text-dim);
}

.drawer-homepage {
  color: var(--accent);
  text-decoration: none;
  font-weight: 500;
}

.drawer-homepage:hover {
  text-decoration: underline;
}

.drawer-body {
  flex: 1;
  overflow-y: auto;
  padding: 18px 20px 24px;
}

.drawer-section {
  margin-bottom: 22px;
}

.drawer-section:last-child {
  margin-bottom: 0;
}

.drawer-section-title {
  margin: 0 0 8px;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.6px;
  color: var(--text-faint);
  font-weight: 600;
}

.drawer-description {
  margin: 0;
  color: var(--text);
  font-size: 13px;
  line-height: 1.6;
}

.drawer-source {
  display: grid;
  grid-template-columns: 80px 1fr;
  gap: 6px 14px;
  margin: 0;
  font-size: 12px;
}

.drawer-source dt {
  color: var(--text-faint);
  text-transform: uppercase;
  font-size: 10px;
  letter-spacing: 0.5px;
  align-self: center;
}

.drawer-source dd {
  margin: 0;
  color: var(--text-dim);
  word-break: break-all;
}

.drawer-source dd.mono {
  font-family: ui-monospace, "SF Mono", Menlo, monospace;
  font-size: 11.5px;
  color: var(--text);
}

.drawer-readme-status {
  padding: 12px;
  background: var(--bg-elev);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  color: var(--text-dim);
  font-size: 12px;
}

.drawer-readme-status.error {
  color: var(--danger);
  border-color: color-mix(in srgb, var(--danger) 30%, transparent);
  background: color-mix(in srgb, var(--danger) 10%, transparent);
}

.drawer-readme-status.muted {
  color: var(--text-faint);
  font-style: italic;
}

.drawer-readme {
  font-size: 13px;
  line-height: 1.6;
}

/* Markdown content styling — generic, applies to anything inside .markdown */
.markdown :deep(h1),
.markdown :deep(h2),
.markdown :deep(h3),
.markdown :deep(h4) {
  margin: 18px 0 8px;
  font-weight: 600;
  letter-spacing: -0.01em;
}
.markdown :deep(h1) { font-size: 18px; }
.markdown :deep(h2) { font-size: 15px; }
.markdown :deep(h3) { font-size: 14px; }
.markdown :deep(h4) { font-size: 13px; color: var(--text-dim); }

.markdown :deep(p) {
  margin: 0 0 12px;
  color: var(--text);
}

.markdown :deep(a) {
  color: var(--accent);
  text-decoration: none;
}
.markdown :deep(a:hover) {
  text-decoration: underline;
}

.markdown :deep(code) {
  font-family: ui-monospace, "SF Mono", Menlo, monospace;
  font-size: 11.5px;
  background: var(--bg-elev);
  padding: 1px 5px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
}

.markdown :deep(pre) {
  background: var(--bg-elev);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 10px 12px;
  overflow-x: auto;
  margin: 12px 0;
}

.markdown :deep(pre code) {
  background: none;
  border: none;
  padding: 0;
  font-size: 11.5px;
  line-height: 1.5;
}

.markdown :deep(ul),
.markdown :deep(ol) {
  margin: 0 0 12px;
  padding-left: 22px;
}

.markdown :deep(li) {
  margin-bottom: 4px;
}

.markdown :deep(blockquote) {
  margin: 12px 0;
  padding: 8px 14px;
  border-left: 3px solid var(--accent);
  background: var(--accent-soft);
  color: var(--text-dim);
  border-radius: 0 var(--radius) var(--radius) 0;
}

.markdown :deep(hr) {
  border: none;
  border-top: 1px solid var(--border);
  margin: 18px 0;
}

.markdown :deep(table) {
  border-collapse: collapse;
  margin: 12px 0;
  font-size: 12px;
}

.markdown :deep(th),
.markdown :deep(td) {
  border: 1px solid var(--border);
  padding: 6px 10px;
  text-align: left;
}

.markdown :deep(th) {
  background: var(--bg-elev);
  font-weight: 600;
}

.markdown :deep(img) {
  max-width: 100%;
  border-radius: var(--radius);
}

/* Drawer slide-in / fade-out */
.drawer-enter-active,
.drawer-leave-active {
  transition: background 200ms ease;
}
.drawer-enter-active .drawer,
.drawer-leave-active .drawer {
  transition: transform 240ms cubic-bezier(0.4, 0, 0.2, 1);
}
.drawer-enter-from,
.drawer-leave-to {
  background: transparent;
}
.drawer-enter-from .drawer,
.drawer-leave-to .drawer {
  transform: translateX(100%);
}
</style>
