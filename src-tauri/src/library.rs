use anyhow::{anyhow, Result};
use gray_matter::{engine::YAML, Matter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    #[serde(default)]
    pub origin: Option<Origin>,
    /// ISO 8601 timestamp of the most recent harmonize-apply that touched
    /// this asset. `None` means it has never been harmonized; the UI uses
    /// this to surface a small badge so the user can spot AI-rewritten
    /// content at a glance. Tracked in `.harmonized.json` (sibling of
    /// `.origins.json`) rather than mixed into Origin so it works for
    /// local assets too.
    #[serde(default)]
    pub harmonized_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Origin {
    /// Marketplace name (e.g. "claude-plugins-official").
    pub marketplace: String,
    /// Plugin name as listed in the marketplace.
    pub plugin: String,
    /// ISO 8601 timestamp.
    pub imported_at: String,
    /// Plugin version reported by the marketplace at import time, if any.
    /// Used to detect when a newer version becomes available.
    #[serde(default)]
    pub version: Option<String>,
    /// Resolved git ref (branch / tag / sha) we actually pulled from. Useful
    /// for traceability — version-less plugins still get an audit trail.
    #[serde(default)]
    pub git_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub imported: Vec<String>,
    pub skipped: Vec<String>,
}

pub type OriginsMap = HashMap<String, Origin>;

fn origins_path() -> PathBuf {
    library_dir().join(".origins.json")
}

pub fn read_origins() -> OriginsMap {
    let p = origins_path();
    let Ok(s) = fs::read_to_string(&p) else { return HashMap::new() };
    let raw: OriginsMap = serde_json::from_str(&s).unwrap_or_default();

    // Migrate keys written by older versions: command/agent entries used to
    // include the `.md` suffix (e.g. `commands:foo.md`), which never matched
    // the bare names produced by scan_kind. Strip the suffix on read and
    // persist the cleaned map back so subsequent calls hit the fast path.
    let mut needs_rewrite = false;
    let mut fixed: OriginsMap = HashMap::with_capacity(raw.len());
    for (k, v) in raw {
        let new_k = match k.split_once(':') {
            Some((kind, name))
                if (kind == "commands" || kind == "agents") && name.ends_with(".md") =>
            {
                needs_rewrite = true;
                format!("{kind}:{}", name.trim_end_matches(".md"))
            }
            _ => k,
        };
        fixed.insert(new_k, v);
    }
    if needs_rewrite {
        let _ = write_origins(&fixed);
    }
    fixed
}

fn write_origins(origins: &OriginsMap) -> Result<()> {
    fs::create_dir_all(library_dir())?;
    let s = serde_json::to_string_pretty(origins)?;
    fs::write(origins_path(), s)?;
    Ok(())
}

pub fn set_origin(kind: AssetKind, name: &str, origin: Origin) -> Result<()> {
    let mut origins = read_origins();
    origins.insert(format!("{}:{}", kind.as_str(), name), origin);
    write_origins(&origins)
}

// ── Harmonized tracking ─────────────────────────────────────────────
//
// Maps `<kind>:<name>` → ISO 8601 timestamp of the last successful
// harmonize-apply that touched the asset. Stored as a separate JSON
// file from `.origins.json` because: (1) it's an orthogonal concern
// (provenance vs editing history), and (2) it has to work for assets
// that have no Origin (locally-created ones).

pub type HarmonizedMap = HashMap<String, String>;

fn harmonized_path() -> PathBuf {
    library_dir().join(".harmonized.json")
}

pub fn read_harmonized() -> HarmonizedMap {
    let p = harmonized_path();
    let Ok(s) = fs::read_to_string(&p) else { return HashMap::new() };
    serde_json::from_str(&s).unwrap_or_default()
}

fn write_harmonized(map: &HarmonizedMap) -> Result<()> {
    fs::create_dir_all(library_dir())?;
    let s = serde_json::to_string_pretty(map)?;
    fs::write(harmonized_path(), s)?;
    Ok(())
}

pub fn mark_harmonized(kind: AssetKind, name: &str, when: &str) -> Result<()> {
    let mut map = read_harmonized();
    map.insert(format!("{}:{}", kind.as_str(), name), when.to_string());
    write_harmonized(&map)
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEntry {
    pub plugin: String,
    pub filename: String,
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpEntry {
    pub plugin: String,
    pub path: String,
    pub servers: serde_json::Value,
}

pub fn list_hooks() -> Vec<HookEntry> {
    let hooks_dir = library_dir().join("hooks");
    let mut out = Vec::new();
    let Ok(plugins) = fs::read_dir(&hooks_dir) else {
        return out;
    };
    for plugin_entry in plugins.flatten() {
        if !plugin_entry.path().is_dir() {
            continue;
        }
        let plugin = plugin_entry.file_name().to_string_lossy().to_string();
        let Ok(files) = fs::read_dir(plugin_entry.path()) else {
            continue;
        };
        for file_entry in files.flatten() {
            if !file_entry.path().is_file() {
                continue;
            }
            let filename = file_entry.file_name().to_string_lossy().to_string();
            let content = fs::read_to_string(file_entry.path()).unwrap_or_default();
            out.push(HookEntry {
                plugin: plugin.clone(),
                filename,
                path: file_entry.path().to_string_lossy().to_string(),
                content,
            });
        }
    }
    out.sort_by(|a, b| a.plugin.cmp(&b.plugin).then(a.filename.cmp(&b.filename)));
    out
}

pub fn list_mcp() -> Vec<McpEntry> {
    let mcp_dir = library_dir().join("mcp");
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(&mcp_dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if !path.is_file() || !name.ends_with(".json") {
            continue;
        }
        let plugin = name.trim_end_matches(".json").to_string();
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let servers: serde_json::Value = serde_json::from_str(&content).unwrap_or(serde_json::Value::Object(Default::default()));
        out.push(McpEntry {
            plugin,
            path: path.to_string_lossy().to_string(),
            servers,
        });
    }
    out.sort_by(|a, b| a.plugin.cmp(&b.plugin));
    out
}

pub fn ensure_layout() -> Result<()> {
    for kind in AssetKind::all() {
        fs::create_dir_all(library_dir().join(kind.as_str()))?;
    }
    fs::create_dir_all(library_dir().join("hooks"))?;
    fs::create_dir_all(library_dir().join("mcp"))?;
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
                    origin: None,
                    harmonized_at: None,
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
                    origin: None,
                    harmonized_at: None,
                });
            }
        }
    }

    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

pub fn scan_all() -> Vec<Asset> {
    let origins = read_origins();
    let harmonized = read_harmonized();
    let mut all: Vec<Asset> = AssetKind::all()
        .iter()
        .flat_map(|k| scan_kind(*k))
        .collect();
    for a in &mut all {
        let key = format!("{}:{}", a.kind.as_str(), a.name);
        a.origin = origins.get(&key).cloned();
        a.harmonized_at = harmonized.get(&key).cloned();
    }
    all
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

/// Validate a slug used as an asset name. We refuse anything that would either
/// confuse the filesystem (slashes / dots) or look weird in URLs.
fn validate_asset_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("name cannot be empty"));
    }
    if name.len() > 64 {
        return Err(anyhow!("name too long (max 64 chars)"));
    }
    let ok = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !ok {
        return Err(anyhow!(
            "name must contain only letters, digits, '-' or '_'"
        ));
    }
    Ok(())
}

/// Create a brand-new asset with a minimal frontmatter template. Errors if an
/// asset of the same kind+name already exists — callers should not have to
/// guard against accidental overwrites.
pub fn create_asset(kind: AssetKind, name: &str, description: Option<&str>) -> Result<()> {
    validate_asset_name(name)?;
    ensure_layout()?;

    let target = asset_file_path(kind, name);
    if target.exists() {
        return Err(anyhow!("{} '{name}' already exists", kind.as_str()));
    }
    // For skills we also need to refuse a pre-existing dir (without SKILL.md).
    if matches!(kind, AssetKind::Skills) {
        let dir = library_dir().join(kind.as_str()).join(name);
        if dir.exists() {
            return Err(anyhow!("skills/{name} directory already exists"));
        }
    }

    let desc = description.unwrap_or("").trim();
    let frontmatter = if desc.is_empty() {
        String::from("---\ndescription:\n---\n\n")
    } else {
        format!("---\ndescription: {desc}\n---\n\n")
    };
    let body = format!("# {name}\n");
    let content = format!("{frontmatter}{body}");

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&target, content).map_err(|e| anyhow!("write {}: {e}", target.display()))?;
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

    // --- hooks/ ---
    let hooks_src = source.join("hooks");
    if hooks_src.is_dir() {
        let plugin_folder = source
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let hooks_dst = library_dir().join("hooks").join(&plugin_folder);
        if hooks_dst.exists() {
            skipped.push(format!("hooks/{plugin_folder}"));
        } else {
            fs::create_dir_all(&hooks_dst)?;
            for entry in fs::read_dir(&hooks_src)?.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                let src_path = entry.path();
                if !src_path.is_file() {
                    continue;
                }
                let dst_path = hooks_dst.join(&file_name);
                fs::copy(&src_path, &dst_path)?;
                imported.push(format!("hooks/{file_name}"));
            }
        }
    }

    // --- mcp.json / .mcp.json ---
    // The official marketplace uses ".mcp.json" (dot-prefixed); local plugins
    // may use the plain "mcp.json" name. We check both.
    let mcp_src_candidates = [source.join(".mcp.json"), source.join("mcp.json")];
    if let Some(mcp_src) = mcp_src_candidates.iter().find(|p| p.is_file()) {
        let plugin_folder = source
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let mcp_dst = library_dir().join("mcp").join(format!("{plugin_folder}.json"));
        if mcp_dst.exists() {
            skipped.push(format!("mcp/{plugin_folder}.json"));
        } else {
            fs::copy(mcp_src, &mcp_dst)?;
            imported.push(format!("mcp/{plugin_folder}.json"));
        }
    }

    Ok(ImportResult { imported, skipped })
}

/// Remove all library entries that originated from `plugin_name`:
/// assets tracked in `.origins.json`, the hooks subfolder, and the mcp JSON.
/// Returns the count of removed items.
pub fn remove_plugin(plugin_name: &str) -> Result<usize> {
    let mut count = 0usize;
    let origins = read_origins();
    let mut new_origins = origins.clone();

    for kind in AssetKind::all() {
        let dir = library_dir().join(kind.as_str());
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();
            let bare = match kind {
                AssetKind::Skills => file_name.clone(),
                _ => file_name.trim_end_matches(".md").to_string(),
            };
            let key = format!("{}:{}", kind.as_str(), bare);
            if origins.get(&key).map(|o| o.plugin == plugin_name).unwrap_or(false) {
                if path.is_dir() {
                    fs::remove_dir_all(&path)?;
                } else {
                    fs::remove_file(&path)?;
                }
                new_origins.remove(&key);
                count += 1;
            }
        }
    }
    write_origins(&new_origins)?;

    let hooks_dir = library_dir().join("hooks").join(plugin_name);
    if hooks_dir.exists() {
        fs::remove_dir_all(&hooks_dir)?;
        count += 1;
    }

    let mcp_file = library_dir().join("mcp").join(format!("{plugin_name}.json"));
    if mcp_file.exists() {
        fs::remove_file(&mcp_file)?;
        count += 1;
    }

    Ok(count)
}
