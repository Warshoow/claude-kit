import type { Asset, AssetKind } from "./types";

export type AssetGroupBy = "kind" | "plugin";

export interface AssetGroup {
  /** Display label for the group header. */
  label: string;
  /** Sub-label rendered next to the count, optional. */
  hint?: string;
  /** Stable key for v-for. */
  key: string;
  items: Asset[];
}

const KIND_ORDER: AssetKind[] = ["skills", "commands", "agents"];

const LOCAL_KEY = "__local__";

/**
 * Group assets either by their kind (skills / commands / agents) or by their
 * plugin of origin (a "Local" bucket holds anything that wasn't imported from
 * the marketplace). Returned groups are sorted in a stable order.
 */
export function groupAssets(assets: Asset[], by: AssetGroupBy): AssetGroup[] {
  if (by === "kind") {
    const buckets: Record<AssetKind, Asset[]> = {
      skills: [],
      commands: [],
      agents: [],
    };
    for (const a of assets) buckets[a.kind].push(a);
    return KIND_ORDER
      .filter((k) => buckets[k].length > 0)
      .map((k) => ({ label: k, key: k, items: buckets[k] }));
  }

  // by plugin
  const buckets = new Map<string, Asset[]>();
  for (const a of assets) {
    const k = a.origin?.plugin ?? LOCAL_KEY;
    if (!buckets.has(k)) buckets.set(k, []);
    buckets.get(k)!.push(a);
  }

  return Array.from(buckets.entries())
    .sort(([a], [b]) => {
      // Local first, then alphabetical
      if (a === LOCAL_KEY) return -1;
      if (b === LOCAL_KEY) return 1;
      return a.localeCompare(b);
    })
    .map(([k, items]) => ({
      label: k === LOCAL_KEY ? "Local" : `from ${k}`,
      key: k,
      items,
    }));
}

const STORAGE_KEY = "claude-kit:asset-group-by";

export function loadGroupByPreference(): AssetGroupBy {
  return localStorage.getItem(STORAGE_KEY) === "plugin" ? "plugin" : "kind";
}

export function saveGroupByPreference(value: AssetGroupBy) {
  localStorage.setItem(STORAGE_KEY, value);
}
