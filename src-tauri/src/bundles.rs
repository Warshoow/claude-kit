use crate::library::{bundles_dir, library_dir, mark_harmonized, read_harmonized, read_origins,
                     set_origin, AssetKind, Origin};
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

/// Kinds of entries a bundle can carry. Wider than `AssetKind` —
/// covers hooks and MCP server configs in addition to the three
/// "main" markdown asset kinds. Skills/commands/agents are
/// uniquely identified by `(kind, name)`; hooks/mcp also need a
/// `plugin` (origin folder name) since they can be plugin-scoped.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum BundleEntryKind {
    Skills,
    Commands,
    Agents,
    Hooks,
    Mcp,
}

impl BundleEntryKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            BundleEntryKind::Skills => "skills",
            BundleEntryKind::Commands => "commands",
            BundleEntryKind::Agents => "agents",
            BundleEntryKind::Hooks => "hooks",
            BundleEntryKind::Mcp => "mcp",
        }
    }

    /// Map back to the narrower AssetKind. Returns `None` for hooks/mcp,
    /// which aren't representable as `AssetKind`.
    pub fn as_asset_kind(&self) -> Option<AssetKind> {
        match self {
            BundleEntryKind::Skills => Some(AssetKind::Skills),
            BundleEntryKind::Commands => Some(AssetKind::Commands),
            BundleEntryKind::Agents => Some(AssetKind::Agents),
            _ => None,
        }
    }
}

impl From<AssetKind> for BundleEntryKind {
    fn from(k: AssetKind) -> Self {
        match k {
            AssetKind::Skills => BundleEntryKind::Skills,
            AssetKind::Commands => BundleEntryKind::Commands,
            AssetKind::Agents => BundleEntryKind::Agents,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleRef {
    pub kind: BundleEntryKind,
    pub name: String,
    /// Set for hooks/mcp entries that live under a plugin folder
    /// (`library/hooks/<plugin>/<name>` or `library/mcp/<plugin>.json`).
    /// `None` is reserved for the future flat layout used by entries
    /// the user creates locally (AI-generated hooks, hand-built MCP
    /// servers). For skills/commands/agents this field is always `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
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
    pub kind: BundleEntryKind,
    pub name: String,
    pub content: String,
    /// Plugin folder name for hooks/mcp entries; absent for skills/
    /// commands/agents and for future flat hooks/mcp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
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

/// Path the share encoder reads from / the importer writes to for an
/// entry. For skills/commands/agents we go through `asset_content_path`.
/// For hooks/mcp the `plugin` field is required in step 1 — the flat
/// layout will be wired in alongside the manual MCP / AI hook UI in
/// later steps.
fn entry_disk_path(entry_kind: BundleEntryKind, name: &str, plugin: Option<&str>) -> Result<PathBuf> {
    if let Some(asset_kind) = entry_kind.as_asset_kind() {
        return Ok(asset_content_path(asset_kind, name));
    }
    let plugin = plugin.ok_or_else(|| {
        anyhow!(
            "{} entry '{name}' missing plugin field — flat layout not supported yet",
            entry_kind.as_str()
        )
    })?;
    match entry_kind {
        BundleEntryKind::Hooks => Ok(library_dir().join("hooks").join(plugin).join(name)),
        BundleEntryKind::Mcp => Ok(library_dir().join("mcp").join(format!("{plugin}.json"))),
        _ => unreachable!("as_asset_kind handled the markdown kinds"),
    }
}

pub fn encode_bundle_share(bundle_name: &str) -> Result<String> {
    let bundle =
        read_bundle(bundle_name).ok_or_else(|| anyhow!("bundle not found: {bundle_name}"))?;

    let origins = read_origins();
    let harmonized = read_harmonized();

    let mut assets = Vec::new();
    for r in &bundle.assets {
        let content_path = entry_disk_path(r.kind, &r.name, r.plugin.as_deref())?;
        let content = fs::read_to_string(&content_path)
            .with_context(|| format!("reading {}", content_path.display()))?;
        // Origins + harmonized maps key by AssetKind only (the three
        // markdown kinds). Hooks/mcp don't carry harmonized state and
        // their provenance lives via their plugin folder, not here.
        let (origin, harmonized_at) = match r.kind.as_asset_kind() {
            Some(ak) => {
                let key = format!("{}:{}", ak.as_str(), &r.name);
                (origins.get(&key).cloned(), harmonized.get(&key).cloned())
            }
            None => (None, None),
        };
        assets.push(SharedAsset {
            kind: r.kind,
            name: r.name.clone(),
            content,
            plugin: r.plugin.clone(),
            origin,
            harmonized_at,
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
        let content_path =
            entry_disk_path(asset.kind, &asset.name, asset.plugin.as_deref())?;
        let label = match &asset.plugin {
            Some(p) => format!("{}:{}/{}", asset.kind.as_str(), p, &asset.name),
            None => format!("{}:{}", asset.kind.as_str(), &asset.name),
        };
        if content_path.exists() {
            skipped.push(label);
            continue;
        }
        fs::create_dir_all(content_path.parent().unwrap())?;
        fs::write(&content_path, &asset.content)?;
        // Origin + harmonized state only applies to skills/commands/agents.
        if let Some(asset_kind) = asset.kind.as_asset_kind() {
            if let Some(origin) = &asset.origin {
                set_origin(asset_kind, &asset.name, origin.clone())?;
            }
            if let Some(harmonized_at) = &asset.harmonized_at {
                mark_harmonized(asset_kind, &asset.name, harmonized_at)?;
            }
        }
        imported.push(label);
    }

    // Deduplicate bundle name if one already exists locally.
    let bundle_name = unique_bundle_name(&manifest.name);
    let bundle = Bundle {
        name: bundle_name.clone(),
        description: manifest.description,
        assets: manifest
            .assets
            .iter()
            .map(|a| BundleRef {
                kind: a.kind,
                name: a.name.clone(),
                plugin: a.plugin.clone(),
            })
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
            kind: BundleEntryKind::Commands,
            name: name.to_string(),
            content: format!("---\ndescription: test\n---\n# {name}\n"),
            plugin: None,
            origin: None,
            harmonized_at: None,
        }
    }

    fn make_cmd_ref(name: &str) -> BundleRef {
        BundleRef {
            kind: BundleEntryKind::Commands,
            name: name.to_string(),
            plugin: None,
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
            kind: BundleEntryKind::Commands,
            name: "cmd".to_string(),
            content: "body".to_string(),
            plugin: None,
            origin: None,
            harmonized_at: None,
        };
        let json = serde_json::to_string(&asset).unwrap();
        assert!(!json.contains("origin"), "None origin must be omitted");
        assert!(!json.contains("harmonized_at"), "None harmonized_at must be omitted");
        assert!(!json.contains("plugin"), "None plugin must be omitted");
    }

    // ── encode_bundle_share ───────────────────────────────────────────

    #[test]
    fn encode_produces_ck1_prefix() {
        with_temp_home(|| {
            create_bundle("my-bundle", None).unwrap();
            let cmd_path = library_dir().join("commands").join("hello.md");
            std::fs::write(&cmd_path, "# hello\n").unwrap();
            set_bundle_assets("my-bundle", vec![make_cmd_ref("hello")]).unwrap();

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
            set_bundle_assets("source-bundle", vec![make_cmd_ref("shared-cmd")]).unwrap();

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
                    kind: BundleEntryKind::Commands,
                    name: "special-cmd".to_string(),
                    content: "---\ndescription: test\n---\n# special-cmd\nmy hand-edit here\n".to_string(),
                    plugin: None,
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
                    kind: BundleEntryKind::Commands,
                    name: "existing".to_string(),
                    content: "new content\n".to_string(),
                    plugin: None,
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

    // ── BundleEntryKind serde + conversion ─────────────────────────

    #[test]
    fn entry_kind_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&BundleEntryKind::Hooks).unwrap(), "\"hooks\"");
        assert_eq!(serde_json::to_string(&BundleEntryKind::Mcp).unwrap(), "\"mcp\"");
        assert_eq!(serde_json::to_string(&BundleEntryKind::Skills).unwrap(), "\"skills\"");
    }

    #[test]
    fn entry_kind_as_asset_kind_narrows_correctly() {
        assert_eq!(BundleEntryKind::Skills.as_asset_kind(), Some(AssetKind::Skills));
        assert_eq!(BundleEntryKind::Commands.as_asset_kind(), Some(AssetKind::Commands));
        assert_eq!(BundleEntryKind::Agents.as_asset_kind(), Some(AssetKind::Agents));
        assert!(BundleEntryKind::Hooks.as_asset_kind().is_none());
        assert!(BundleEntryKind::Mcp.as_asset_kind().is_none());
    }

    #[test]
    fn legacy_bundle_ref_without_plugin_deserializes() {
        // Older bundle files predate the `plugin` field. They must
        // still parse cleanly — the field defaults to None.
        let json = r#"{"kind":"commands","name":"foo"}"#;
        let r: BundleRef = serde_json::from_str(json).unwrap();
        assert_eq!(r.kind, BundleEntryKind::Commands);
        assert_eq!(r.name, "foo");
        assert!(r.plugin.is_none());
    }

    // ── Hooks/MCP share roundtrip ──────────────────────────────────

    #[test]
    fn encode_share_picks_up_hook_content_from_plugin_folder() {
        with_temp_home(|| {
            // Hook lives at library/hooks/<plugin>/<file>
            let plugin_dir = library_dir().join("hooks").join("my-plugin");
            std::fs::create_dir_all(&plugin_dir).unwrap();
            std::fs::write(plugin_dir.join("precommit.sh"), "#!/bin/sh\necho hi\n").unwrap();

            create_bundle("with-hook", None).unwrap();
            set_bundle_assets(
                "with-hook",
                vec![BundleRef {
                    kind: BundleEntryKind::Hooks,
                    name: "precommit.sh".to_string(),
                    plugin: Some("my-plugin".to_string()),
                }],
            )
            .unwrap();

            let code = encode_bundle_share("with-hook").unwrap();
            assert!(code.starts_with("ck1:"));
        });
    }

    #[test]
    fn encode_errors_when_hook_ref_missing_plugin() {
        with_temp_home(|| {
            create_bundle("broken", None).unwrap();
            set_bundle_assets(
                "broken",
                vec![BundleRef {
                    kind: BundleEntryKind::Hooks,
                    name: "orphan.sh".to_string(),
                    plugin: None,
                }],
            )
            .unwrap();

            let err = encode_bundle_share("broken").unwrap_err();
            assert!(
                err.to_string().contains("flat layout not supported yet"),
                "got: {err}"
            );
        });
    }

    #[test]
    fn import_roundtrips_hook_under_plugin_folder() {
        with_temp_home(|| {
            // Plant a plugin-scoped hook + bundle, encode, then nuke
            // local state to simulate a fresh recipient.
            let plugin_dir = library_dir().join("hooks").join("source-plugin");
            std::fs::create_dir_all(&plugin_dir).unwrap();
            let hook_path = plugin_dir.join("on-save.sh");
            std::fs::write(&hook_path, "#!/bin/sh\necho saved\n").unwrap();

            create_bundle("hook-share", None).unwrap();
            set_bundle_assets(
                "hook-share",
                vec![BundleRef {
                    kind: BundleEntryKind::Hooks,
                    name: "on-save.sh".to_string(),
                    plugin: Some("source-plugin".to_string()),
                }],
            )
            .unwrap();

            let code = encode_bundle_share("hook-share").unwrap();

            std::fs::remove_file(&hook_path).unwrap();
            delete_bundle("hook-share").unwrap();

            let result = import_bundle_share(&code).unwrap();
            assert_eq!(result.bundle_name, "hook-share");
            assert!(
                result
                    .imported
                    .iter()
                    .any(|l| l.contains("hooks:source-plugin/on-save.sh")),
                "imported list: {:?}",
                result.imported
            );
            assert!(hook_path.exists(), "hook file must land back in plugin folder");
            let restored = std::fs::read_to_string(&hook_path).unwrap();
            assert!(restored.contains("echo saved"));

            // Bundle ref preserved with plugin field.
            let bundle = read_bundle("hook-share").unwrap();
            assert_eq!(bundle.assets.len(), 1);
            assert_eq!(bundle.assets[0].kind, BundleEntryKind::Hooks);
            assert_eq!(bundle.assets[0].plugin.as_deref(), Some("source-plugin"));
        });
    }

    #[test]
    fn import_roundtrips_mcp_file() {
        with_temp_home(|| {
            let mcp_path = library_dir().join("mcp").join("playwright.json");
            let content = r#"{"mcpServers":{"playwright":{"command":"npx"}}}"#;
            std::fs::write(&mcp_path, content).unwrap();

            create_bundle("mcp-share", None).unwrap();
            set_bundle_assets(
                "mcp-share",
                vec![BundleRef {
                    kind: BundleEntryKind::Mcp,
                    name: "playwright".to_string(),
                    plugin: Some("playwright".to_string()),
                }],
            )
            .unwrap();

            let code = encode_bundle_share("mcp-share").unwrap();
            std::fs::remove_file(&mcp_path).unwrap();
            delete_bundle("mcp-share").unwrap();

            import_bundle_share(&code).unwrap();
            assert!(mcp_path.exists(), "MCP file must land back");
            let restored = std::fs::read_to_string(&mcp_path).unwrap();
            assert!(restored.contains("playwright"));
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
