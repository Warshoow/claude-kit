export type AssetKind = "skills" | "commands" | "agents";

/** Anything that can live inside a bundle. Wider than `AssetKind`:
 * also covers hooks (script files) and mcp (server config files). */
export type BundleEntryKind = AssetKind | "hooks" | "mcp";

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
  kind: BundleEntryKind;
  name: string;
  /** Plugin folder name for hooks/mcp refs that live under a plugin
   * (`library/hooks/<plugin>/<name>` or `library/mcp/<plugin>.json`).
   * Always absent for skills/commands/agents. */
  plugin?: string;
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
  /** Plugin folder name, or `"__local__"` for entries created in-app
   * via the manual MCP dialog. */
  plugin: string;
  /** File basename (without `.json`). Equals `plugin` for plugin-
   * shipped entries; for local entries it's the chosen server slug. */
  name: string;
  path: string;
  servers: Record<string, unknown>;
}

/** Sentinel matching `LOCAL_PLUGIN` in `src-tauri/src/library.rs`. */
export const LOCAL_PLUGIN = "__local__";

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

/** Stable key for any bundle entry — includes the plugin segment so
 * hooks from different plugins don't collide. Use this when working
 * with `BundleRef`; `assetKey` stays for narrow `AssetKind` callers. */
export function bundleEntryKey(a: {
  kind: BundleEntryKind;
  name: string;
  plugin?: string;
}): string {
  return a.plugin ? `${a.kind}:${a.plugin}/${a.name}` : `${a.kind}:${a.name}`;
}

/** Type guard narrowing a `BundleRef` down to the three markdown
 * asset kinds — useful at callsites that still expect `AssetKind`
 * (library lookups, the `asset-detail` route, the installed-keys
 * set built from `list_installed`). Hooks/mcp tracking is wired
 * separately and shouldn't pass through these paths. */
export function isAssetRef(
  r: BundleRef,
): r is BundleRef & { kind: AssetKind } {
  return r.kind === "skills" || r.kind === "commands" || r.kind === "agents";
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

// ── Refine-chat streaming ────────────────────────────────────────
// Backend tags the enum as `{ event, data }` (#[serde tag/content]).
// Mirrors `ai::StreamEvent` in `src-tauri/src/ai.rs`.

export interface ChatMessage {
  role: "user" | "assistant";
  content: string;
}

export type StreamEvent =
  | { event: "token"; data: { delta: string } }
  | { event: "done"; data: { content: string } }
  | { event: "error"; data: { message: string } };

// ── Bundle-chat (generate a bundle of assets via chat) ───────────
//
// Mirrors `bundle_chat::GeneratedAsset` and `BundleChatEvent` in
// `src-tauri/src/bundle_chat.rs`.

export interface GeneratedAsset {
  kind: AssetKind;
  name: string;
  content: string;
}

export type BundleChatEvent =
  | { event: "token"; data: { delta: string } }
  | {
      event: "done";
      data: {
        assets: GeneratedAsset[];
        bundle_name: string | null;
        bundle_description: string | null;
      };
    }
  | { event: "error"; data: { message: string } };

export interface MaterializeResult {
  bundle_name: string;
  asset_count: number;
  created: BundleRef[];
}
