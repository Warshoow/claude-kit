import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "vue-sonner";
import { api } from "@/lib/api";
import type {
  Asset,
  Bundle,
  InstalledAsset,
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


  return {
    // state
    library,
    bundles,
    installed,
    projectPath,
    // computed
    installedKeys,
    // actions
    refreshLibrary,
    refreshBundles,
    refreshInstalled,
    refreshAll,
    pickProject,
    applyBundle,
    toggleAsset,
    cleanProject,
  };
});
