export type AssetKind = "skills" | "commands" | "agents";

export interface Asset {
  kind: AssetKind;
  name: string;
  path: string;
  description?: string;
  tags?: string[];
  origin?: Origin;
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
