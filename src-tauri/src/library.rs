use anyhow::{anyhow, Result};
use gray_matter::{engine::YAML, Matter};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AssetKind {
    Skills,
    Commands,
    Agents,
}

impl AssetKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetKind::Skills => "skills",
            AssetKind::Commands => "commands",
            AssetKind::Agents => "agents",
        }
    }

    pub fn all() -> [AssetKind; 3] {
        [AssetKind::Skills, AssetKind::Commands, AssetKind::Agents]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub kind: AssetKind,
    pub name: String,
    pub path: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub imported: Vec<String>,
    pub skipped: Vec<String>,
}

pub fn kit_home() -> PathBuf {
    if let Ok(p) = std::env::var("CLAUDE_KIT_HOME") {
        return PathBuf::from(p);
    }
    dirs::home_dir()
        .expect("home dir")
        .join(".claude-assets")
}

pub fn library_dir() -> PathBuf {
    kit_home().join("library")
}

pub fn bundles_dir() -> PathBuf {
    kit_home().join("bundles")
}

pub fn ensure_layout() -> Result<()> {
    for kind in AssetKind::all() {
        fs::create_dir_all(library_dir().join(kind.as_str()))?;
    }
    fs::create_dir_all(bundles_dir())?;
    Ok(())
}

fn read_frontmatter(md_path: &Path) -> (Option<String>, Option<Vec<String>>) {
    let Ok(content) = fs::read_to_string(md_path) else {
        return (None, None);
    };
    let matter = Matter::<YAML>::new();
    let Some(result) = matter.parse(&content).data else {
        return (None, None);
    };
    let Ok(map) = result.as_hashmap() else {
        return (None, None);
    };

    let desc = map.get("description").and_then(|v| v.as_string().ok());
    let tags = map.get("tags").and_then(|v| v.as_vec().ok()).map(|items| {
        items
            .iter()
            .filter_map(|x| x.as_string().ok())
            .collect::<Vec<_>>()
    });
    (desc, tags)
}

pub fn scan_kind(kind: AssetKind) -> Vec<Asset> {
    let dir = library_dir().join(kind.as_str());
    let mut out = Vec::new();

    let Ok(entries) = fs::read_dir(&dir) else {
        return out;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        match kind {
            AssetKind::Skills => {
                if !path.is_dir() {
                    continue;
                }
                let skill_md = path.join("SKILL.md");
                if !skill_md.exists() {
                    continue;
                }
                let (description, tags) = read_frontmatter(&skill_md);
                out.push(Asset {
                    kind,
                    name,
                    path: path.to_string_lossy().to_string(),
                    description,
                    tags,
                });
            }
            _ => {
                if !path.is_file() || !name.ends_with(".md") {
                    continue;
                }
                let bare = name.trim_end_matches(".md").to_string();
                let (description, tags) = read_frontmatter(&path);
                out.push(Asset {
                    kind,
                    name: bare,
                    path: path.to_string_lossy().to_string(),
                    description,
                    tags,
                });
            }
        }
    }

    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

pub fn scan_all() -> Vec<Asset> {
    AssetKind::all()
        .iter()
        .flat_map(|k| scan_kind(*k))
        .collect()
}

fn asset_file_path(kind: AssetKind, name: &str) -> PathBuf {
    let base = library_dir().join(kind.as_str());
    match kind {
        AssetKind::Skills => base.join(name).join("SKILL.md"),
        _ => base.join(format!("{name}.md")),
    }
}

pub fn read_asset_content(kind: AssetKind, name: &str) -> Result<String> {
    let p = asset_file_path(kind, name);
    fs::read_to_string(&p).map_err(|e| anyhow!("read {}: {e}", p.display()))
}

pub fn write_asset_content(kind: AssetKind, name: &str, content: &str) -> Result<()> {
    let p = asset_file_path(kind, name);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&p, content).map_err(|e| anyhow!("write {}: {e}", p.display()))?;
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if ty.is_file() {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Import skills/commands/agents from a plugin-style folder.
/// Looks for `<source>/skills/*`, `<source>/commands/*.md`, `<source>/agents/*.md`
/// and copies them into the library. Existing entries are skipped.
pub fn import_from_plugin(source: &Path) -> Result<ImportResult> {
    if !source.is_dir() {
        return Err(anyhow!("not a directory: {}", source.display()));
    }
    ensure_layout()?;

    let mut imported = Vec::new();
    let mut skipped = Vec::new();

    for kind in AssetKind::all() {
        let kind_src = source.join(kind.as_str());
        if !kind_src.is_dir() {
            continue;
        }
        let kind_dst = library_dir().join(kind.as_str());

        for entry in fs::read_dir(&kind_src)?.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let src_path = entry.path();

            match kind {
                AssetKind::Skills => {
                    if !src_path.is_dir() {
                        continue;
                    }
                    if !src_path.join("SKILL.md").exists() {
                        continue;
                    }
                    let dst = kind_dst.join(&name);
                    if dst.exists() {
                        skipped.push(format!("{}/{name}", kind.as_str()));
                        continue;
                    }
                    copy_dir_recursive(&src_path, &dst)?;
                    imported.push(format!("{}/{name}", kind.as_str()));
                }
                _ => {
                    if !src_path.is_file() || !name.ends_with(".md") {
                        continue;
                    }
                    let dst = kind_dst.join(&name);
                    if dst.exists() {
                        skipped.push(format!("{}/{name}", kind.as_str()));
                        continue;
                    }
                    fs::copy(&src_path, &dst)?;
                    imported.push(format!("{}/{name}", kind.as_str()));
                }
            }
        }
    }

    Ok(ImportResult { imported, skipped })
}
