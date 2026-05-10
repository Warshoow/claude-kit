import { invoke } from "@tauri-apps/api/core";
import type {
  AiStatus,
  ApplyResult,
  ApplyUpdateResult,
  Asset,
  AssetKind,
  AssetWrite,
  Bundle,
  BundleRef,
  HarmonizationResult,
  HookEntry,
  ImportResult,
  InstalledAsset,
  InstalledHook,
  Marketplace,
  MarketplaceSource,
  McpEntry,
  Plugin,
  RecommendationResult,
  Settings,
  UpdatePreview,
} from "./types";

export const api = {
  initLibrary: () => invoke<string>("init_library"),
  listLibrary: () => invoke<Asset[]>("list_library"),
  listBundles: () => invoke<Bundle[]>("list_bundles_cmd"),
  createBundle: (name: string, description?: string) =>
    invoke<Bundle>("create_bundle", { name, description }),
  deleteBundle: (name: string) => invoke<void>("delete_bundle", { name }),
  setBundleAssets: (name: string, assets: BundleRef[]) =>
    invoke<Bundle>("set_bundle_assets", { name, assets }),
  applyBundle: (projectPath: string, bundleName: string, replace = false) =>
    invoke<ApplyResult>("apply_bundle", { projectPath, bundleName, replace }),
  applySingle: (projectPath: string, kind: AssetKind, name: string) =>
    invoke<void>("apply_single", { projectPath, kind, name }),
  removeSingle: (projectPath: string, kind: AssetKind, name: string) =>
    invoke<boolean>("remove_single", { projectPath, kind, name }),
  listInstalled: (projectPath: string) =>
    invoke<InstalledAsset[]>("list_installed_cmd", { projectPath }),
  cleanProject: (projectPath: string) =>
    invoke<number>("clean_project", { projectPath }),
  importPlugin: (sourcePath: string) =>
    invoke<ImportResult>("import_plugin", { sourcePath }),
  readAsset: (kind: AssetKind, name: string) =>
    invoke<string>("read_asset", { kind, name }),
  writeAsset: (kind: AssetKind, name: string, content: string) =>
    invoke<void>("write_asset", { kind, name, content }),
  createAsset: (kind: AssetKind, name: string, description?: string) =>
    invoke<void>("create_asset", { kind, name, description }),
  listMarketplacePlugins: (url?: string) =>
    invoke<Marketplace>("list_marketplace_plugins", { url }),
  importMarketplacePlugin: (plugin: Plugin, marketplaceName?: string) =>
    invoke<ImportResult>("import_marketplace_plugin", { plugin, marketplaceName }),
  fetchPluginReadme: (plugin: Plugin) =>
    invoke<string | null>("fetch_plugin_readme", { plugin }),
  listHooks: () => invoke<HookEntry[]>("list_hooks_cmd"),
  listMcp: () => invoke<McpEntry[]>("list_mcp_cmd"),
  applyHook: (projectPath: string, plugin: string, filename: string) =>
    invoke<void>("apply_hook_cmd", { projectPath, plugin, filename }),
  removeHook: (projectPath: string, filename: string) =>
    invoke<boolean>("remove_hook_cmd", { projectPath, filename }),
  listInstalledHooks: (projectPath: string) =>
    invoke<InstalledHook[]>("list_installed_hooks_cmd", { projectPath }),
  applyMcp: (projectPath: string, plugin: string) =>
    invoke<void>("apply_mcp_cmd", { projectPath, plugin }),
  removePlugin: (pluginName: string) =>
    invoke<number>("remove_plugin_cmd", { pluginName }),
  // ── AI / settings ─────────────────────────────────────────────
  aiStatus: () => invoke<AiStatus>("ai_status_cmd"),
  aiGenerate: (kind: AssetKind, prompt: string, context?: string) =>
    invoke<string>("ai_generate", { kind, prompt, context }),
  readSettings: () => invoke<Settings>("read_settings_cmd"),
  writeSettings: (settings: Settings) =>
    invoke<void>("write_settings_cmd", { settings }),
  harmonizeBundle: (bundleName: string, instruction?: string) =>
    invoke<HarmonizationResult[]>("harmonize_bundle_cmd", {
      bundleName,
      instruction,
    }),
  markAssetsHarmonized: (items: { kind: AssetKind; name: string }[]) =>
    invoke<number>("mark_assets_harmonized", { items }),
  recommendBundle: (userNeed: string, marketplaceUrl?: string) =>
    invoke<RecommendationResult>("recommend_bundle_cmd", {
      userNeed,
      marketplaceUrl,
    }),
  previewPluginUpdate: (plugin: Plugin, marketplaceName?: string) =>
    invoke<UpdatePreview>("preview_plugin_update_cmd", {
      plugin,
      marketplaceName,
    }),
  applyPluginUpdate: (
    pluginName: string,
    newGitRef: string,
    writes: AssetWrite[],
    newVersion?: string,
    marketplaceName?: string,
  ) =>
    invoke<ApplyUpdateResult>("apply_plugin_update_cmd", {
      pluginName,
      marketplaceName,
      newVersion,
      newGitRef,
      writes,
    }),
  // ── Marketplace sources (the user's list of marketplaces) ──────
  listMarketplaceSources: () =>
    invoke<MarketplaceSource[]>("list_marketplace_sources_cmd"),
  addMarketplaceSource: (url: string) =>
    invoke<MarketplaceSource>("add_marketplace_source_cmd", { url }),
  removeMarketplaceSource: (url: string) =>
    invoke<void>("remove_marketplace_source_cmd", { url }),
};
