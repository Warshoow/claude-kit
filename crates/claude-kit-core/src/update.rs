use crate::library::{
    read_asset_content, read_origins, scan_all, set_origin, write_asset_content, AssetKind,
    Origin,
};
use crate::marketplace::{download_and_extract, Plugin};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

/// Snapshot of the difference between what's in the library for a given
/// plugin and what's in the upstream tarball. The frontend uses this to
/// drive a per-hunk diff review, then sends back the accepted writes via
/// `apply_update`.
#[derive(Debug, Clone, Serialize)]
pub struct UpdatePreview {
    pub plugin_name: String,
    pub current_version: Option<String>,
    pub new_version: Option<String>,
    pub current_git_ref: Option<String>,
    pub new_git_ref: String,
    /// Assets present in BOTH library and upstream with differing content.
    pub modified: Vec<AssetPair>,
    /// Assets present in upstream but missing from the library.
    pub added: Vec<NewAssetEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssetPair {
    pub kind: AssetKind,
    pub name: String,
    pub original: String,
    pub proposed: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NewAssetEntry {
    pub kind: AssetKind,
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetWrite {
    pub kind: AssetKind,
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyUpdateResult {
    pub written: usize,
    pub origins_refreshed: usize,
}

pub fn preview_update(plugin: &Plugin, marketplace_name: &str) -> Result<UpdatePreview> {
    let library = scan_all();

    // The plugin's current footprint in the user's library — anything
    // tagged with this marketplace+plugin pair.
    let lib_assets: Vec<_> = library
        .iter()
        .filter(|a| {
            a.origin
                .as_ref()
                .is_some_and(|o| o.plugin == plugin.name && o.marketplace == marketplace_name)
        })
        .collect();

    if lib_assets.is_empty() {
        return Err(anyhow!(
            "no assets from '{}' (marketplace '{}') found in the library — import the plugin first",
            plugin.name,
            marketplace_name
        ));
    }

    let current_version = lib_assets[0].origin.as_ref().and_then(|o| o.version.clone());
    let current_git_ref = lib_assets[0].origin.as_ref().and_then(|o| o.git_ref.clone());

    let extraction = download_and_extract(plugin, Some(marketplace_name))?;
    let upstream = read_upstream_assets(&extraction.plugin_root)?;

    let mut modified = Vec::new();
    let mut added = Vec::new();

    for ua in upstream {
        let lib_match = lib_assets
            .iter()
            .find(|la| la.kind == ua.kind && la.name == ua.name);
        match lib_match {
            Some(la) => {
                // Read the local content fresh — `Asset.path` is a directory for
                // skills, so we go through the canonical helper.
                let original = read_asset_content(la.kind, &la.name).unwrap_or_default();
                if original != ua.content {
                    modified.push(AssetPair {
                        kind: ua.kind,
                        name: ua.name,
                        original,
                        proposed: ua.content,
                    });
                }
            }
            None => {
                added.push(NewAssetEntry {
                    kind: ua.kind,
                    name: ua.name,
                    content: ua.content,
                });
            }
        }
    }

    Ok(UpdatePreview {
        plugin_name: plugin.name.clone(),
        current_version,
        new_version: plugin.version.clone(),
        current_git_ref,
        new_git_ref: extraction.git_ref,
        modified,
        added,
    })
}

/// Apply user-accepted changes to disk, then refresh the origin entries
/// for every asset of this plugin so the marketplace UI stops nagging
/// about the update.
///
/// The frontend has already merged accepted hunks into the proposed
/// content; this function just writes whatever it receives.
pub fn apply_update(
    plugin_name: &str,
    marketplace_name: &str,
    new_version: Option<String>,
    new_git_ref: String,
    writes: Vec<AssetWrite>,
) -> Result<ApplyUpdateResult> {
    let mut written = 0usize;
    for w in &writes {
        write_asset_content(w.kind, &w.name, &w.content)
            .map_err(|e| anyhow!("write {}/{}: {e}", w.kind.as_str(), w.name))?;
        written += 1;
    }

    // Refresh origins for every asset of this plugin currently in the
    // library — including ones the user kept unchanged. Rationale: the
    // user has reviewed the upstream version and committed to "this is
    // the version I want" (even if it's identical to what they had).
    // Without this the badge keeps saying "Update available v1.0 → v1.1"
    // which is misleading.
    let imported_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| String::new());
    let origins = read_origins();
    let mut origins_refreshed = 0usize;
    for (key, origin) in origins.iter() {
        if origin.plugin == plugin_name && origin.marketplace == marketplace_name {
            let Some((kind_str, name)) = key.split_once(':') else { continue };
            let Some(kind) = parse_kind(kind_str) else { continue };
            let new_origin = Origin {
                marketplace: marketplace_name.to_string(),
                plugin: plugin_name.to_string(),
                imported_at: imported_at.clone(),
                version: new_version.clone(),
                git_ref: Some(new_git_ref.clone()),
            };
            if set_origin(kind, name, new_origin).is_ok() {
                origins_refreshed += 1;
            }
        }
    }

    // Also tag any newly-written assets that didn't have a previous
    // origin (e.g. assets added in upstream).
    for w in &writes {
        let key = format!("{}:{}", w.kind.as_str(), w.name);
        if !origins.contains_key(&key) {
            let new_origin = Origin {
                marketplace: marketplace_name.to_string(),
                plugin: plugin_name.to_string(),
                imported_at: imported_at.clone(),
                version: new_version.clone(),
                git_ref: Some(new_git_ref.clone()),
            };
            let _ = set_origin(w.kind, &w.name, new_origin);
            origins_refreshed += 1;
        }
    }

    Ok(ApplyUpdateResult {
        written,
        origins_refreshed,
    })
}

fn parse_kind(s: &str) -> Option<AssetKind> {
    match s {
        "skills" => Some(AssetKind::Skills),
        "commands" => Some(AssetKind::Commands),
        "agents" => Some(AssetKind::Agents),
        _ => None,
    }
}

struct UpstreamAsset {
    kind: AssetKind,
    name: String,
    content: String,
}

fn read_upstream_assets(plugin_root: &Path) -> Result<Vec<UpstreamAsset>> {
    let mut out = Vec::new();
    for kind in AssetKind::all() {
        let kind_src = plugin_root.join(kind.as_str());
        if !kind_src.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&kind_src)?.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            match kind {
                AssetKind::Skills => {
                    let skill_md = path.join("SKILL.md");
                    if !path.is_dir() || !skill_md.exists() {
                        continue;
                    }
                    let content = fs::read_to_string(&skill_md).unwrap_or_default();
                    out.push(UpstreamAsset {
                        kind,
                        name,
                        content,
                    });
                }
                _ => {
                    if !path.is_file() || !name.ends_with(".md") {
                        continue;
                    }
                    let bare = name.trim_end_matches(".md").to_string();
                    let content = fs::read_to_string(&path).unwrap_or_default();
                    out.push(UpstreamAsset {
                        kind,
                        name: bare,
                        content,
                    });
                }
            }
        }
    }
    Ok(out)
}
