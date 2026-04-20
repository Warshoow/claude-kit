<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "./lib/api";
import type { Asset, Bundle, InstalledAsset } from "./lib/types";
import { assetKey } from "./lib/types";
import LibraryColumn from "./components/LibraryColumn.vue";
import BundlesColumn from "./components/BundlesColumn.vue";
import ProjectColumn from "./components/ProjectColumn.vue";
import AssetEditor from "./components/AssetEditor.vue";

const library = ref<Asset[]>([]);
const bundles = ref<Bundle[]>([]);
const projectPath = ref<string | null>(
  localStorage.getItem("claude-kit:last-project")
);
const installed = ref<InstalledAsset[]>([]);
const selectedBundle = ref<string | null>(null);
const toast = ref<string | null>(null);
const editing = ref<Asset | null>(null);

const installedKeys = computed(() => new Set(installed.value.map(assetKey)));

async function refreshLibrary() {
  library.value = await api.listLibrary();
}

async function refreshBundles() {
  bundles.value = await api.listBundles();
}

async function refreshInstalled() {
  if (!projectPath.value) {
    installed.value = [];
    return;
  }
  installed.value = await api.listInstalled(projectPath.value);
}

async function refreshAll() {
  await Promise.all([refreshLibrary(), refreshBundles(), refreshInstalled()]);
}

async function pickProject() {
  const selected = await open({ directory: true, multiple: false });
  if (typeof selected === "string") {
    projectPath.value = selected;
    localStorage.setItem("claude-kit:last-project", selected);
    await refreshInstalled();
  }
}

function flash(msg: string) {
  toast.value = msg;
  setTimeout(() => (toast.value = null), 2500);
}

async function onApplyBundle(name: string, replace: boolean) {
  if (!projectPath.value) {
    flash("Pick a project first");
    return;
  }
  const res = await api.applyBundle(projectPath.value, name, replace);
  await refreshInstalled();
  flash(`Applied ${res.ok.length} asset(s)${res.errors.length ? `, ${res.errors.length} errors` : ""}`);
}

async function onToggleAsset(a: Asset) {
  if (!projectPath.value) {
    flash("Pick a project first");
    return;
  }
  const key = assetKey(a);
  if (installedKeys.value.has(key)) {
    await api.removeSingle(projectPath.value, a.kind, a.name);
  } else {
    await api.applySingle(projectPath.value, a.kind, a.name);
  }
  await refreshInstalled();
}

async function onCreateBundle(name: string, description?: string) {
  await api.createBundle(name, description);
  await refreshBundles();
  selectedBundle.value = name;
}

async function onDeleteBundle(name: string) {
  await api.deleteBundle(name);
  if (selectedBundle.value === name) selectedBundle.value = null;
  await refreshBundles();
}

async function onUpdateBundle(bundle: Bundle) {
  await api.setBundleAssets(bundle.name, bundle.assets);
  await refreshBundles();
}

async function onCleanProject() {
  if (!projectPath.value) return;
  const n = await api.cleanProject(projectPath.value);
  await refreshInstalled();
  flash(`Removed ${n} symlink(s)`);
}

async function onImportPlugin() {
  const selected = await open({ directory: true, multiple: false, title: "Pick a plugin folder" });
  if (typeof selected !== "string") return;
  try {
    const res = await api.importPlugin(selected);
    await refreshLibrary();
    flash(
      `Imported ${res.imported.length}${res.skipped.length ? `, skipped ${res.skipped.length} (already exist)` : ""}`
    );
  } catch (e) {
    flash(`Import failed: ${e}`);
  }
}

function onEditAsset(a: Asset) {
  editing.value = a;
}

function onEditorClose() {
  editing.value = null;
}

async function onEditorSaved() {
  await refreshLibrary();
  flash("Saved");
}

onMounted(refreshAll);
</script>

<template>
  <div class="app">
    <div class="topbar">
      <h1>claude-kit</h1>
      <div class="spacer" />
      <div class="project-selector">
        <span class="project-path">{{ projectPath ?? "No project selected" }}</span>
        <button @click="pickProject">
          {{ projectPath ? "Change…" : "Pick project…" }}
        </button>
      </div>
    </div>

    <div class="columns">
      <LibraryColumn
        :library="library"
        :installed-keys="installedKeys"
        :can-install="!!projectPath"
        @toggle="onToggleAsset"
        @edit="onEditAsset"
        @import="onImportPlugin"
      />
      <BundlesColumn
        :bundles="bundles"
        :library="library"
        :selected="selectedBundle"
        :can-apply="!!projectPath"
        @select="(n) => (selectedBundle = n)"
        @create="onCreateBundle"
        @delete="onDeleteBundle"
        @update="onUpdateBundle"
        @apply="onApplyBundle"
      />
      <ProjectColumn
        :project-path="projectPath"
        :installed="installed"
        :bundles="bundles"
        @clean="onCleanProject"
      />
    </div>

    <AssetEditor
      :asset="editing"
      @close="onEditorClose"
      @saved="onEditorSaved"
    />

    <div v-if="toast" class="toast">{{ toast }}</div>
  </div>
</template>
