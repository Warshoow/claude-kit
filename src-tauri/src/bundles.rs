use crate::library::{bundles_dir, AssetKind};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;

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

fn bundle_path(name: &str) -> std::path::PathBuf {
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
