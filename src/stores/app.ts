import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "vue-sonner";
import { api } from "@/lib/api";
import type {
  Asset,
  Bundle,
  InstalledAsset,
  Marketplace,
  Plugin,
} from "@/lib/types";
import { assetKey } from "@/lib/types";

export const useAppStore = defineStore("app", () => {
  // ── Library / bundles / project ─────────────────────────────────
  const library = ref<Asset[]>([]);
  const bundles = ref<Bundle[]>([]);
  const installed = ref<InstalledAsset[]>([]);
  const projectPath = ref<string | null>(
    localStorage.getItem("claude-kit:last-project")
  );

  const installedKeys = computed(
    () => new Set(installed.value.map(assetKey))
  );

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

  // ── Bundle CRUD ─────────────────────────────────────────────────
  async function createBundle(name: string, description?: string) {
    await api.createBundle(name, description);
    await refreshBundles();
  }

  async function deleteBundle(name: string) {
    await api.deleteBundle(name);
    await refreshBundles();
  }

  async function updateBundle(bundle: Bundle) {
    await api.setBundleAssets(bundle.name, bundle.assets);
    await refreshBundles();
  }

  // ── Apply / install ─────────────────────────────────────────────
  async function applyBundle(name: string, replace: boolean) {
    if (!projectPath.value) {
      toast.warning("Pick a project first");
      return;
    }
    const res = await api.applyBundle(projectPath.value, name, replace);
    await refreshInstalled();
    toast.success(
      `Applied ${res.ok.length} asset${res.ok.length === 1 ? "" : "s"}`,
      res.errors.length
        ? { description: `${res.errors.length} errors` }
        : undefined
    );
  }

  async function toggleAsset(a: Asset) {
    if (!projectPath.value) {
      toast.warning("Pick a project first");
      return;
    }
    if (installedKeys.value.has(assetKey(a))) {
      await api.removeSingle(projectPath.value, a.kind, a.name);
    } else {
      await api.applySingle(projectPath.value, a.kind, a.name);
    }
    await refreshInstalled();
  }

  async function cleanProject() {
    if (!projectPath.value) return;
    const n = await api.cleanProject(projectPath.value);
    await refreshInstalled();
    toast.success(`Removed ${n} symlink${n === 1 ? "" : "s"}`);
  }

  // ── Plugin import (local folder) ────────────────────────────────
  async function importLocalPlugin() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Pick a plugin folder",
    });
    if (typeof selected !== "string") return;
    try {
      const res = await api.importPlugin(selected);
      await refreshLibrary();
      toast.success(
        `Imported ${res.imported.length}`,
        res.skipped.length
          ? { description: `Skipped ${res.skipped.length} (already exist)` }
          : undefined
      );
    } catch (e) {
      toast.error("Import failed", { description: String(e) });
    }
  }

  // ── Marketplace ─────────────────────────────────────────────────
  const marketplace = ref<Marketplace | null>(null);
  const marketplaceLoading = ref(false);
  const marketplaceError = ref<string | null>(null);
  const importingPlugin = ref<string | null>(null);

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

  async function importMarketplacePlugin(plugin: Plugin) {
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

  return {
    // state
    library,
    bundles,
    installed,
    projectPath,
    marketplace,
    marketplaceLoading,
    marketplaceError,
    importingPlugin,
    // computed
    installedKeys,
    // actions
    refreshLibrary,
    refreshBundles,
    refreshInstalled,
    refreshAll,
    pickProject,
    createBundle,
    deleteBundle,
    updateBundle,
    applyBundle,
    toggleAsset,
    cleanProject,
    importLocalPlugin,
    loadMarketplace,
    importMarketplacePlugin,
  };
});
