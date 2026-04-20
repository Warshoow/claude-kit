export type AssetKind = "skills" | "commands" | "agents";

export interface Asset {
  kind: AssetKind;
  name: string;
  path: string;
  description?: string;
  tags?: string[];
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
