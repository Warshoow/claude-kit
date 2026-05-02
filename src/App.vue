<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "vue-sonner";
import { Folder } from "lucide-vue-next";
import { api } from "@/lib/api";
import type { Asset, Bundle, InstalledAsset, Marketplace, Plugin } from "@/lib/types";
import { assetKey } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { Toaster } from "@/components/ui/sonner";
import LibraryColumn from "@/components/LibraryColumn.vue";
import BundlesColumn from "@/components/BundlesColumn.vue";
import ProjectColumn from "@/components/ProjectColumn.vue";
import AssetEditor from "@/components/AssetEditor.vue";
import DiscoverView from "@/components/DiscoverView.vue";
import PluginDetail from "@/components/PluginDetail.vue";

type View = "manage" | "discover";

const library = ref<Asset[]>([]);
const bundles = ref<Bundle[]>([]);
const projectPath = ref<string | null>(
  localStorage.getItem("claude-kit:last-project")
);
const installed = ref<InstalledAsset[]>([]);
const selectedBundle = ref<string | null>(null);
const editing = ref<Asset | null>(null);
const currentView = ref<View>("manage");
const marketplace = ref<Marketplace | null>(null);
const marketplaceLoading = ref(false);
const marketplaceError = ref<string | null>(null);
const importingPlugin = ref<string | null>(null);
const selectedPlugin = ref<Plugin | null>(null);
const readme = ref<string | null>(null);
const readmeLoading = ref(false);
const readmeError = ref<string | null>(null);

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

async function onApplyBundle(name: string, replace: boolean) {
  if (!projectPath.value) {
    toast.warning("Pick a project first");
    return;
  }
  const res = await api.applyBundle(projectPath.value, name, replace);
  await refreshInstalled();
  toast.success(
    `Applied ${res.ok.length} asset${res.ok.length === 1 ? "" : "s"}`,
    res.errors.length ? { description: `${res.errors.length} errors` } : undefined
  );
}

async function onToggleAsset(a: Asset) {
  if (!projectPath.value) {
    toast.warning("Pick a project first");
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
  toast.success(`Removed ${n} symlink${n === 1 ? "" : "s"}`);
}

async function onImportPlugin() {
  const selected = await open({ directory: true, multiple: false, title: "Pick a plugin folder" });
  if (typeof selected !== "string") return;
  try {
    const res = await api.importPlugin(selected);
    await refreshLibrary();
    toast.success(
      `Imported ${res.imported.length}`,
      res.skipped.length ? { description: `Skipped ${res.skipped.length} (already exist)` } : undefined
    );
  } catch (e) {
    toast.error("Import failed", { description: String(e) });
  }
}

async function loadMarketplace(force = false) {
  if (marketplace.value && !force) return;
  marketplaceLoading.value = true;
  marketplaceError.value = null;
  try {
    marketplace.value = await api.listMarketplacePlugins();
  } catch (e) {
    marketplaceError.value = String(e);
  } finally {
    marketplaceLoading.value = false;
  }
}

async function onSwitchView(v: View | undefined) {
  if (!v) return;
  currentView.value = v;
  if (v === "discover") await loadMarketplace();
}

async function onSelectPlugin(plugin: Plugin) {
  selectedPlugin.value = plugin;
  readme.value = null;
  readmeError.value = null;
  readmeLoading.value = true;
  try {
    readme.value = await api.fetchPluginReadme(plugin);
  } catch (e) {
    readmeError.value = String(e);
  } finally {
    readmeLoading.value = false;
  }
}

function onCloseDetail() {
  selectedPlugin.value = null;
}

async function onImportMarketplacePlugin(plugin: Plugin) {
  if (importingPlugin.value) return;
  importingPlugin.value = plugin.name;
  try {
    const res = await api.importMarketplacePlugin(plugin);
    await refreshLibrary();
    toast.success(
      `Imported ${res.imported.length} from ${plugin.name}`,
      res.skipped.length
        ? { description: `Skipped ${res.skipped.length} (already in library)` }
        : undefined
    );
  } catch (e) {
    toast.error("Import failed", { description: String(e) });
  } finally {
    importingPlugin.value = null;
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
  toast.success("Saved");
}

onMounted(refreshAll);
</script>

<template>
  <div class="flex h-screen flex-col">
    <header
      class="flex items-center gap-4 border-b bg-card/50 px-4 py-2.5 backdrop-blur"
    >
      <h1 class="text-sm font-semibold tracking-tight">claude-kit</h1>

      <ToggleGroup
        type="single"
        :model-value="currentView"
        @update:model-value="onSwitchView($event as View | undefined)"
        variant="outline"
        size="sm"
      >
        <ToggleGroupItem value="manage">Manage</ToggleGroupItem>
        <ToggleGroupItem value="discover">Discover</ToggleGroupItem>
      </ToggleGroup>

      <div class="flex-1" />

      <span
        v-if="projectPath"
        class="max-w-[420px] truncate font-mono text-xs text-muted-foreground"
        :title="projectPath"
      >{{ projectPath }}</span>
      <span v-else class="text-xs text-muted-foreground italic">No project selected</span>
      <Button variant="outline" size="sm" @click="pickProject">
        <Folder />
        {{ projectPath ? "Change" : "Pick project" }}
      </Button>
    </header>

    <main class="flex-1 overflow-hidden">
      <div v-if="currentView === 'manage'" class="grid h-full grid-cols-3 divide-x">
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

      <DiscoverView
        v-else
        :marketplace="marketplace"
        :loading="marketplaceLoading"
        :error="marketplaceError"
        :importing-plugin="importingPlugin"
        @refresh="loadMarketplace(true)"
        @import="onImportMarketplacePlugin"
        @select="onSelectPlugin"
      />
    </main>

    <PluginDetail
      :plugin="selectedPlugin"
      :readme="readme"
      :readme-loading="readmeLoading"
      :readme-error="readmeError"
      :importing="!!selectedPlugin && importingPlugin === selectedPlugin.name"
      @close="onCloseDetail"
      @import="onImportMarketplacePlugin"
    />

    <AssetEditor
      :asset="editing"
      @close="onEditorClose"
      @saved="onEditorSaved"
    />

    <Toaster position="bottom-right" rich-colors />
  </div>
</template>
