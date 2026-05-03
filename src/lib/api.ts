import { invoke } from "@tauri-apps/api/core";
import type {
  Asset,
  AssetKind,
  ApplyResult,
  Bundle,
  BundleRef,
  ImportResult,
  InstalledAsset,
  Marketplace,
  Plugin,
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
  importMarketplacePlugin: (plugin: Plugin) =>
    invoke<ImportResult>("import_marketplace_plugin", { plugin }),
  fetchPluginReadme: (plugin: Plugin) =>
    invoke<string | null>("fetch_plugin_readme", { plugin }),
};
