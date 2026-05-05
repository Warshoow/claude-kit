import type { Asset, HookEntry, McpEntry, Origin, Plugin } from "./types";

/**
 * Find the most-recent Origin record (by `imported_at`) for a given
 * marketplace+plugin pair, scanning every asset in the library. Returns
 * null if the plugin has no assets currently materialized.
 *
 * Why "most recent": assets from the same plugin are usually imported in
 * one batch and share an Origin, but a user can re-import after deleting
 * some files — the latest batch is what we want to compare versions
 * against.
 */
export function findLatestOrigin(
  library: Asset[],
  marketplaceName: string,
  pluginName: string
): Origin | null {
  let latest: Origin | null = null;
  for (const a of library) {
    const o = a.origin;
    if (!o) continue;
    if (o.marketplace !== marketplaceName || o.plugin !== pluginName) continue;
    if (!latest || o.imported_at > latest.imported_at) latest = o;
  }
  return latest;
}

export type PluginImportStatus =
  | { kind: "not-imported" }
  | { kind: "current"; importedVersion?: string }
  | { kind: "update"; importedVersion: string; currentVersion: string }
  | { kind: "unknown" };

/**
 * Decide what state to show for a marketplace plugin:
 * - `not-imported` — not in library
 * - `current`      — versions match (or the marketplace plugin has no version
 *                    field, so there's nothing to compare against)
 * - `update`       — stored version differs from the current marketplace one
 * - `unknown`      — imported, but the stored Origin predates version tracking
 *                    (legacy data) so we can't tell either way
 */
export function pluginImportStatus(
  library: Asset[],
  marketplaceName: string | undefined,
  plugin: Plugin,
  hooks: HookEntry[] = [],
  mcp: McpEntry[] = [],
): PluginImportStatus {
  if (!marketplaceName) return { kind: "not-imported" };

  const o = findLatestOrigin(library, marketplaceName, plugin.name);
  if (o) {
    if (!o.version) return { kind: "unknown" };
    if (!plugin.version) return { kind: "current", importedVersion: o.version };
    return o.version === plugin.version
      ? { kind: "current", importedVersion: o.version }
      : { kind: "update", importedVersion: o.version, currentVersion: plugin.version };
  }

  // No tracked assets — but hooks or mcp entries from this plugin count too.
  const hasHooks = hooks.some((h) => h.plugin === plugin.name);
  const hasMcp = mcp.some((m) => m.plugin === plugin.name);
  if (hasHooks || hasMcp) {
    // Hooks/MCP have no version tracking, treat like a version-less import.
    return { kind: "current" };
  }

  return { kind: "not-imported" };
}
