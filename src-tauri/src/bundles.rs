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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::{ensure_layout, library_dir, AssetKind};

    fn with_temp_home<F: FnOnce()>(f: F) {
        let dir = tempfile::tempdir().unwrap();
        let _guard = crate::HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("CLAUDE_KIT_HOME", dir.path());
        ensure_layout().unwrap();
        f();
        std::env::remove_var("CLAUDE_KIT_HOME");
    }

    /// Encode a `ShareManifest` directly (avoids file I/O — handy for
    /// crafting edge-case payloads without a full bundle on disk).
    fn encode_manifest(m: &ShareManifest) -> String {
        use flate2::{write::GzEncoder, Compression};
        use std::io::Write;
        let json = serde_json::to_string(m).unwrap();
        let mut enc = GzEncoder::new(Vec::new(), Compression::best());
        enc.write_all(json.as_bytes()).unwrap();
        let compressed = enc.finish().unwrap();
        format!("ck1:{}", STANDARD_NO_PAD.encode(&compressed))
    }

    fn make_cmd_asset(name: &str) -> SharedAsset {
        SharedAsset {
            kind: AssetKind::Commands,
            name: name.to_string(),
            content: format!("---\ndescription: test\n---\n# {name}\n"),
            origin: None,
            harmonized_at: None,
        }
    }

    // ── ShareManifest serialisation ───────────────────────────────────

    #[test]
    fn share_manifest_json_roundtrip() {
        let manifest = ShareManifest {
            v: 1,
            name: "test-bundle".to_string(),
            description: Some("A test bundle".to_string()),
            assets: vec![make_cmd_asset("my-cmd")],
        };
        let json = serde_json::to_string(&manifest).unwrap();
        let back: ShareManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.v, 1);
        assert_eq!(back.name, "test-bundle");
        assert_eq!(back.description.as_deref(), Some("A test bundle"));
        assert_eq!(back.assets.len(), 1);
        assert_eq!(back.assets[0].name, "my-cmd");
    }

    #[test]
    fn share_manifest_optional_fields_omitted_when_none() {
        let asset = SharedAsset {
            kind: AssetKind::Commands,
            name: "cmd".to_string(),
            content: "body".to_string(),
            origin: None,
            harmonized_at: None,
        };
        let json = serde_json::to_string(&asset).unwrap();
        assert!(!json.contains("origin"), "None origin must be omitted");
        assert!(!json.contains("harmonized_at"), "None harmonized_at must be omitted");
    }

    // ── encode_bundle_share ───────────────────────────────────────────

    #[test]
    fn encode_produces_ck1_prefix() {
        with_temp_home(|| {
            create_bundle("my-bundle", None).unwrap();
            let cmd_path = library_dir().join("commands").join("hello.md");
            std::fs::write(&cmd_path, "# hello\n").unwrap();
            set_bundle_assets("my-bundle", vec![
                BundleRef { kind: AssetKind::Commands, name: "hello".to_string() },
            ]).unwrap();

            let code = encode_bundle_share("my-bundle").unwrap();
            assert!(code.starts_with("ck1:"), "share code must start with ck1:");
        });
    }

    #[test]
    fn encode_errors_on_missing_bundle() {
        with_temp_home(|| {
            let err = encode_bundle_share("nonexistent").unwrap_err();
            assert!(err.to_string().contains("bundle not found"));
        });
    }

    // ── import_bundle_share — error handling ──────────────────────────

    #[test]
    fn import_rejects_missing_prefix() {
        let err = import_bundle_share("notavalidcode").unwrap_err();
        assert!(err.to_string().contains("ck1:"));
    }

    #[test]
    fn import_rejects_invalid_base64() {
        let err = import_bundle_share("ck1:!!!not-base64!!!").unwrap_err();
        assert!(err.to_string().contains("base64"));
    }

    #[test]
    fn import_rejects_bad_gzip() {
        let bad = format!("ck1:{}", STANDARD_NO_PAD.encode(b"not gzip data at all"));
        let err = import_bundle_share(&bad).unwrap_err();
        assert!(
            err.to_string().contains("decompression") || err.to_string().contains("gzip"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn import_rejects_invalid_json_after_decompress() {
        use flate2::{write::GzEncoder, Compression};
        use std::io::Write;
        let mut enc = GzEncoder::new(Vec::new(), Compression::best());
        enc.write_all(b"not json {{{").unwrap();
        let compressed = enc.finish().unwrap();
        let code = format!("ck1:{}", STANDARD_NO_PAD.encode(&compressed));
        let err = import_bundle_share(&code).unwrap_err();
        assert!(err.to_string().contains("JSON"));
    }

    // ── full encode → import round-trip ──────────────────────────────

    #[test]
    fn encode_then_import_round_trip() {
        with_temp_home(|| {
            create_bundle("source-bundle", Some("A source bundle".to_string())).unwrap();
            let cmd_path = library_dir().join("commands").join("shared-cmd.md");
            std::fs::write(&cmd_path, "---\ndescription: shared\n---\n# shared-cmd\n").unwrap();
            set_bundle_assets("source-bundle", vec![
                BundleRef { kind: AssetKind::Commands, name: "shared-cmd".to_string() },
            ]).unwrap();

            let code = encode_bundle_share("source-bundle").unwrap();
            // Simulate a fresh recipient: remove the original bundle + asset.
            delete_bundle("source-bundle").unwrap();
            std::fs::remove_file(&cmd_path).unwrap();

            let result = import_bundle_share(&code).unwrap();
            assert_eq!(result.bundle_name, "source-bundle");
            assert!(result.imported.contains(&"commands:shared-cmd".to_string()));
            assert!(result.skipped.is_empty());
            assert!(cmd_path.exists(), "asset must be restored on disk");
        });
    }

    #[test]
    fn import_preserves_asset_content() {
        with_temp_home(|| {
            let manifest = ShareManifest {
                v: 1,
                name: "content-test".to_string(),
                description: None,
                assets: vec![SharedAsset {
                    kind: AssetKind::Commands,
                    name: "special-cmd".to_string(),
                    content: "---\ndescription: test\n---\n# special-cmd\nmy hand-edit here\n".to_string(),
                    origin: None,
                    harmonized_at: None,
                }],
            };
            let code = encode_manifest(&manifest);
            import_bundle_share(&code).unwrap();

            let path = library_dir().join("commands").join("special-cmd.md");
            let written = std::fs::read_to_string(&path).unwrap();
            assert!(written.contains("my hand-edit here"), "hand-edited content must survive");
        });
    }

    #[test]
    fn import_skips_assets_that_already_exist() {
        with_temp_home(|| {
            let cmd_path = library_dir().join("commands").join("existing.md");
            std::fs::write(&cmd_path, "original content\n").unwrap();

            let manifest = ShareManifest {
                v: 1,
                name: "skip-test".to_string(),
                description: None,
                assets: vec![SharedAsset {
                    kind: AssetKind::Commands,
                    name: "existing".to_string(),
                    content: "new content\n".to_string(),
                    origin: None,
                    harmonized_at: None,
                }],
            };
            let code = encode_manifest(&manifest);
            let result = import_bundle_share(&code).unwrap();

            assert!(result.skipped.contains(&"commands:existing".to_string()));
            let content = std::fs::read_to_string(&cmd_path).unwrap();
            assert_eq!(content, "original content\n", "local file must not be overwritten");
        });
    }

    // ── unique_bundle_name (tested via import) ────────────────────────

    #[test]
    fn import_deduplicates_bundle_name() {
        with_temp_home(|| {
            // Pre-occupy "my-bundle" so the import must pick "my-bundle-2"
            create_bundle("my-bundle", None).unwrap();

            let manifest = ShareManifest {
                v: 1,
                name: "my-bundle".to_string(),
                description: None,
                assets: vec![make_cmd_asset("fresh-cmd")],
            };
            let code = encode_manifest(&manifest);
            let result = import_bundle_share(&code).unwrap();
            assert_eq!(result.bundle_name, "my-bundle-2");
        });
    }

    #[test]
    fn import_deduplicates_further_when_slot_taken() {
        with_temp_home(|| {
            create_bundle("b", None).unwrap();
            create_bundle("b-2", None).unwrap();

            let manifest = ShareManifest {
                v: 1,
                name: "b".to_string(),
                description: None,
                assets: vec![make_cmd_asset("some-cmd")],
            };
            let code = encode_manifest(&manifest);
            let result = import_bundle_share(&code).unwrap();
            assert_eq!(result.bundle_name, "b-3");
        });
    }

    #[test]
    fn import_creates_bundle_with_correct_assets() {
        with_temp_home(|| {
            let manifest = ShareManifest {
                v: 1,
                name: "full-bundle".to_string(),
                description: Some("desc".to_string()),
                assets: vec![make_cmd_asset("cmd-a"), make_cmd_asset("cmd-b")],
            };
            let code = encode_manifest(&manifest);
            import_bundle_share(&code).unwrap();

            let bundle = read_bundle("full-bundle").expect("bundle must exist");
            assert_eq!(bundle.assets.len(), 2);
            assert_eq!(bundle.description.as_deref(), Some("desc"));
        });
    }
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
