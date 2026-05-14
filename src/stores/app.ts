import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "vue-sonner";
import { api } from "@/lib/api";
import type {
  Asset,
  Bundle,
  HookEntry,
  InstalledAsset,
  InstalledHook,
  McpEntry,
} from "@/lib/types";
import { assetKey } from "@/lib/types";

export const useAppStore = defineStore("app", () => {
  // ── Library / bundles / project ─────────────────────────────────
  const library = ref<Asset[]>([]);
  const bundles = ref<Bundle[]>([]);
  const installed = ref<InstalledAsset[]>([]);
  const hooks = ref<HookEntry[]>([]);
  const mcp = ref<McpEntry[]>([]);
  const installedHooks = ref<InstalledHook[]>([]);
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
      installedHooks.value = [];
      return;
    }
    [installed.value, installedHooks.value] = await Promise.all([
      api.listInstalled(projectPath.value),
      api.listInstalledHooks(projectPath.value),
    ]);
  }

  async function refreshHooksMcp() {
    [hooks.value, mcp.value] = await Promise.all([
      api.listHooks(),
      api.listMcp(),
    ]);
  }

  async function refreshAll() {
    await Promise.all([refreshLibrary(), refreshBundles(), refreshInstalled(), refreshHooksMcp()]);
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

    if (res.errors.length === 0) {
      toast.success(
        `Applied ${res.ok.length} asset${res.ok.length === 1 ? "" : "s"}`
      );
      return;
    }

    // Surface the actual error message rather than just a count — users
    // hitting Windows symlink permission errors otherwise have no idea
    // what to do. We show the first error verbatim (ours include the fix
    // instructions inline) plus a count if multiple files failed.
    const firstError = res.errors[0];
    const overflow =
      res.errors.length > 1
        ? ` (and ${res.errors.length - 1} more file${res.errors.length === 2 ? "" : "s"})`
        : "";
    const variant = res.ok.length === 0 ? "error" : "warning";
    const title =
      res.ok.length === 0
        ? `Apply failed${overflow}`
        : `Applied ${res.ok.length}, ${res.errors.length} failed`;
    toast[variant](title, { description: `${firstError}${overflow}` });
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

  async function toggleHook(plugin: string, filename: string) {
    if (!projectPath.value) {
      toast.warning("Pick a project first");
      return;
    }
    const isInstalled = installedHooks.value.some(
      (h) => h.plugin === plugin && h.filename === filename
    );
    if (isInstalled) {
      await api.removeHook(projectPath.value, filename);
    } else {
      await api.applyHook(projectPath.value, plugin, filename);
    }
    await refreshInstalled();
  }

  async function applyMcp(plugin: string) {
    if (!projectPath.value) {
      toast.warning("Pick a project first");
      return;
    }
    await api.applyMcp(projectPath.value, plugin);
    toast.success(`MCP servers from "${plugin}" merged into project`);
  }

  async function applyMcpLocal(name: string) {
    if (!projectPath.value) {
      toast.warning("Pick a project first");
      return;
    }
    await api.applyMcpLocal(projectPath.value, name);
    toast.success(`MCP "${name}" merged into project`);
  }

  async function removePlugin(pluginName: string) {
    await api.removePlugin(pluginName);
    await Promise.all([refreshLibrary(), refreshHooksMcp()]);
    toast.success(`Plugin "${pluginName}" removed from library`);
  }

  return {
    // state
    library,
    bundles,
    installed,
    hooks,
    mcp,
    installedHooks,
    projectPath,
    // computed
    installedKeys,
    // actions
    refreshLibrary,
    refreshBundles,
    refreshInstalled,
    refreshHooksMcp,
    refreshAll,
    pickProject,
    applyBundle,
    toggleAsset,
    cleanProject,
    toggleHook,
    applyMcp,
    applyMcpLocal,
    removePlugin,
  };
});
