export type AssetKind = "skills" | "commands" | "agents";

export interface Asset {
  kind: AssetKind;
  name: string;
  path: string;
  description?: string;
  tags?: string[];
  origin?: Origin;
  /** ISO 8601 timestamp of the most recent harmonize-apply. */
  harmonized_at?: string;
}

export interface Origin {
  marketplace: string;
  plugin: string;
  imported_at: string;
  /** Plugin version at import time (from marketplace.json), if it had one. */
  version?: string;
  /** Resolved git ref we actually pulled (branch / tag / sha). */
  git_ref?: string;
}

export interface BundleRef {
  kind: AssetKind;
  name: string;
}

export interface Bundle {
  name: string;
  description?: string;
  assets: BundleRef[];
}

export interface InstalledAsset {
  kind: AssetKind;
  name: string;
}

export interface HookEntry {
  plugin: string;
  filename: string;
  path: string;
  content: string;
}

export interface McpEntry {
  plugin: string;
  path: string;
  servers: Record<string, unknown>;
}

export interface InstalledHook {
  plugin: string;
  filename: string;
}

export interface ApplyResult {
  ok: string[];
  errors: string[];
}

export interface ImportResult {
  imported: string[];
  skipped: string[];
}

export function assetKey(a: { kind: AssetKind; name: string }): string {
  return `${a.kind}:${a.name}`;
}

// Marketplace types — mirror src-tauri/src/marketplace.rs
export interface Marketplace {
  name: string;
  description?: string;
  owner?: { name: string; email?: string };
  plugins: Plugin[];
}

export interface Plugin {
  name: string;
  description: string;
  source: PluginSource;
  category?: string;
  author?: { name: string; email?: string };
  homepage?: string;
  version?: string;
}

// `source` is polymorphic in the upstream JSON: a string ("./plugins/foo")
// or a tagged object. We mirror the Rust enum here.
export type PluginSource = string | PluginSourceObject;

export type PluginSourceObject =
  | { source: "url"; url: string; sha?: string }
  | { source: "git-subdir"; url: string; path: string; ref?: string; sha?: string; branch?: string }
  | { source: "github"; repo: string; commit?: string };

// AI / settings — mirror src-tauri/src/{ai,settings}.rs
export type AiMode = "auto" | "claude-cli" | "api";

export type AiBackendMode = "claude-cli" | "api" | "none";

export interface AiSettings {
  mode: AiMode;
  api_base_url?: string;
  api_key?: string;
  api_model?: string;
}

export interface MarketplaceSource {
  name: string;
  url: string;
  builtin: boolean;
}

export interface Settings {
  ai: AiSettings;
  marketplaces: MarketplaceSource[];
}

export interface AiStatus {
  /** Effective backend in use right now: "claude-cli" | "api" | "none". */
  mode: AiBackendMode;
  claude_cli_path?: string;
  api_configured: boolean;
  /** One-line user-facing description of the current state. */
  message: string;
}

/**
 * One asset's before/after pair returned by the bundle harmonizer.
 * `proposed` may equal `original` when the model decided no changes
 * were needed.
 */
export interface HarmonizationResult {
  kind: AssetKind;
  name: string;
  original: string;
  proposed: string;
}

// Bundle recommender — mirrors src-tauri/src/recommend.rs
export interface RecommendedPlugin {
  name: string;
  already_imported: boolean;
  reason: string;
  /** Re-attached so the frontend can call importMarketplacePlugin without re-fetching. */
  plugin?: Plugin;
}

export interface RecommendedAsset {
  kind: AssetKind;
  name: string;
  plugin: string;
  already_in_library: boolean;
  reason: string;
}

export interface RecommendationResult {
  bundle_name: string;
  bundle_description?: string;
  rationale: string;
  plugins_to_import: RecommendedPlugin[];
  assets: RecommendedAsset[];
}

// Plugin update — mirrors src-tauri/src/update.rs
export interface AssetPair {
  kind: AssetKind;
  name: string;
  original: string;
  proposed: string;
}

export interface NewAssetEntry {
  kind: AssetKind;
  name: string;
  content: string;
}

export interface UpdatePreview {
  plugin_name: string;
  current_version?: string;
  new_version?: string;
  current_git_ref?: string;
  new_git_ref: string;
  modified: AssetPair[];
  added: NewAssetEntry[];
}

export interface AssetWrite {
  kind: AssetKind;
  name: string;
  content: string;
}

export interface ApplyUpdateResult {
  written: number;
  origins_refreshed: number;
}

export interface ImportShareResult {
  bundle_name: string;
  imported: string[];
  skipped: string[];
}
