use crate::library::{bundles_dir, library_dir, mark_harmonized, read_harmonized, read_origins,
                     set_origin, AssetKind, Origin};
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleRef {
    pub kind: AssetKind,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub assets: Vec<BundleRef>,
}

fn bundle_path(name: &str) -> PathBuf {
    bundles_dir().join(format!("{name}.json"))
}

pub fn list_bundles() -> Vec<Bundle> {
    let Ok(entries) = fs::read_dir(bundles_dir()) else {
        return vec![];
    };
    let mut out: Vec<Bundle> = entries
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .map(|x| x == "json")
                .unwrap_or(false)
        })
        .filter_map(|e| {
            let content = fs::read_to_string(e.path()).ok()?;
            serde_json::from_str::<Bundle>(&content).ok()
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

pub fn read_bundle(name: &str) -> Option<Bundle> {
    let content = fs::read_to_string(bundle_path(name)).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn write_bundle(bundle: &Bundle) -> Result<()> {
    let json = serde_json::to_string_pretty(bundle)?;
    fs::write(bundle_path(&bundle.name), json + "\n")
        .context("write bundle")?;
    Ok(())
}

pub fn create_bundle(name: &str, description: Option<String>) -> Result<Bundle> {
    if let Some(existing) = read_bundle(name) {
        return Ok(existing);
    }
    let b = Bundle {
        name: name.to_string(),
        description,
        assets: vec![],
    };
    write_bundle(&b)?;
    Ok(b)
}

pub fn delete_bundle(name: &str) -> Result<()> {
    let path = bundle_path(name);
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn set_bundle_assets(name: &str, assets: Vec<BundleRef>) -> Result<Bundle> {
    let mut b = read_bundle(name).unwrap_or(Bundle {
        name: name.to_string(),
        description: None,
        assets: vec![],
    });
    b.assets = assets;
    write_bundle(&b)?;
    Ok(b)
}

// ── Bundle sharing ──────────────────────────────────────────────────────────
//
// A share code is: "ck1:" + base64(gzip(JSON))
//
// The JSON payload (ShareManifest) embeds:
//   - the current content of every asset (so modifications are preserved)
//   - origin metadata (marketplace/plugin/version/git_ref) for provenance
//   - harmonized_at timestamp if the asset was AI-rewritten
//
// On import the recipient gets exactly what the sharer had, including any
// hand edits or AI harmonizations, with full provenance reconstructed.

#[derive(Debug, Serialize, Deserialize)]
pub struct SharedAsset {
    pub kind: AssetKind,
    pub name: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<Origin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harmonized_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShareManifest {
    pub v: u8,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub assets: Vec<SharedAsset>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportShareResult {
    pub bundle_name: String,
    pub imported: Vec<String>,
    pub skipped: Vec<String>,
}

/// Path to the readable content file for an asset (SKILL.md for skills,
/// <name>.md for commands/agents).
fn asset_content_path(kind: AssetKind, name: &str) -> PathBuf {
    let base = library_dir().join(kind.as_str());
    match kind {
        AssetKind::Skills => base.join(name).join("SKILL.md"),
        _ => base.join(format!("{name}.md")),
    }
}

pub fn encode_bundle_share(bundle_name: &str) -> Result<String> {
    let bundle =
        read_bundle(bundle_name).ok_or_else(|| anyhow!("bundle not found: {bundle_name}"))?;

    let origins = read_origins();
    let harmonized = read_harmonized();

    let mut assets = Vec::new();
    for r in &bundle.assets {
        let content_path = asset_content_path(r.kind, &r.name);
        let content = fs::read_to_string(&content_path)
            .with_context(|| format!("reading {}", content_path.display()))?;
        let key = format!("{}:{}", r.kind.as_str(), &r.name);
        assets.push(SharedAsset {
            kind: r.kind,
            name: r.name.clone(),
            content,
            origin: origins.get(&key).cloned(),
            harmonized_at: harmonized.get(&key).cloned(),
        });
    }

    let manifest = ShareManifest {
        v: 1,
        name: bundle.name,
        description: bundle.description,
        assets,
    };

    let json = serde_json::to_string(&manifest)?;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(json.as_bytes())?;
    let compressed = encoder.finish()?;
    Ok(format!("ck1:{}", STANDARD_NO_PAD.encode(&compressed)))
}

pub fn import_bundle_share(code: &str) -> Result<ImportShareResult> {
    let encoded = code
        .trim()
        .strip_prefix("ck1:")
        .ok_or_else(|| anyhow!("invalid share code: must start with ck1:"))?;

    let compressed = STANDARD_NO_PAD
        .decode(encoded)
        .map_err(|e| anyhow!("invalid share code: base64 decode failed: {e}"))?;

    let mut decoder = GzDecoder::new(&compressed[..]);
    let mut json = String::new();
    decoder
        .read_to_string(&mut json)
        .map_err(|e| anyhow!("invalid share code: decompression failed: {e}"))?;

    let manifest: ShareManifest = serde_json::from_str(&json)
        .map_err(|e| anyhow!("invalid share code: JSON parse failed: {e}"))?;

    let mut imported = Vec::new();
    let mut skipped = Vec::new();

    for asset in &manifest.assets {
        let content_path = asset_content_path(asset.kind, &asset.name);
        if content_path.exists() {
            skipped.push(format!("{}:{}", asset.kind.as_str(), &asset.name));
            continue;
        }
        fs::create_dir_all(content_path.parent().unwrap())?;
        fs::write(&content_path, &asset.content)?;
        if let Some(origin) = &asset.origin {
            set_origin(asset.kind, &asset.name, origin.clone())?;
        }
        if let Some(harmonized_at) = &asset.harmonized_at {
            mark_harmonized(asset.kind, &asset.name, harmonized_at)?;
        }
        imported.push(format!("{}:{}", asset.kind.as_str(), &asset.name));
    }

    // Deduplicate bundle name if one already exists locally.
    let bundle_name = unique_bundle_name(&manifest.name);
    let bundle = Bundle {
        name: bundle_name.clone(),
        description: manifest.description,
        assets: manifest
            .assets
            .iter()
            .map(|a| BundleRef { kind: a.kind, name: a.name.clone() })
            .collect(),
    };
    write_bundle(&bundle)?;

    Ok(ImportShareResult { bundle_name, imported, skipped })
}

fn unique_bundle_name(base: &str) -> String {
    if read_bundle(base).is_none() {
        return base.to_string();
    }
    for i in 2u32.. {
        let candidate = format!("{base}-{i}");
        if read_bundle(&candidate).is_none() {
            return candidate;
        }
    }
    unreachable!()
}
