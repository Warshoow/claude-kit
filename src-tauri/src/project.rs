use crate::library::{library_dir, AssetKind};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledHook {
    pub plugin: String,
    pub filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledAsset {
    pub kind: AssetKind,
    pub name: String,
}

fn source_path(kind: AssetKind, name: &str) -> PathBuf {
    let base = library_dir().join(kind.as_str());
    match kind {
        AssetKind::Skills => base.join(name),
        _ => base.join(format!("{name}.md")),
    }
}

fn target_path(project: &Path, kind: AssetKind, name: &str) -> PathBuf {
    let base = project.join(".claude").join(kind.as_str());
    match kind {
        AssetKind::Skills => base.join(name),
        _ => base.join(format!("{name}.md")),
    }
}

#[cfg(unix)]
fn make_symlink(source: &Path, target: &Path) -> Result<()> {
    use std::os::unix::fs::symlink;
    // relative link for portability
    let rel = pathdiff(target.parent().unwrap(), source);
    symlink(rel, target)?;
    Ok(())
}

#[cfg(windows)]
fn make_symlink(source: &Path, target: &Path) -> Result<()> {
    use std::io::ErrorKind;
    use std::os::windows::fs::{symlink_dir, symlink_file};

    let result = if source.is_dir() {
        symlink_dir(source, target)
    } else {
        symlink_file(source, target)
    };

    // Windows blocks symlink creation for non-admin users by default.
    // Error 1314 = ERROR_PRIVILEGE_NOT_HELD; PermissionDenied catches the
    // generic case. Translate this into a message that tells the user how
    // to actually fix it instead of dumping the OS error code.
    match result {
        Ok(()) => Ok(()),
        Err(e)
            if e.raw_os_error() == Some(1314)
                || e.kind() == ErrorKind::PermissionDenied =>
        {
            Err(anyhow!(
                "Windows refused to create the symlink (permission denied). \
                 Enable Developer Mode in Windows Settings → Privacy & Security → For developers, \
                 then retry. Alternatively, run claude-kit as administrator."
            ))
        }
        Err(e) => Err(e.into()),
    }
}

/// minimal pathdiff (no external crate)
fn pathdiff(from: &Path, to: &Path) -> PathBuf {
    let from_comps: Vec<_> = from.components().collect();
    let to_comps: Vec<_> = to.components().collect();
    let mut common = 0;
    while common < from_comps.len()
        && common < to_comps.len()
        && from_comps[common] == to_comps[common]
    {
        common += 1;
    }
    let mut result = PathBuf::new();
    for _ in common..from_comps.len() {
        result.push("..");
    }
    for c in &to_comps[common..] {
        result.push(c.as_os_str());
    }
    result
}

fn is_our_symlink(target: &Path, expected_source: &Path) -> bool {
    let Ok(meta) = fs::symlink_metadata(target) else {
        return false;
    };
    if !meta.file_type().is_symlink() {
        return false;
    }
    let Ok(linked) = fs::read_link(target) else {
        return false;
    };
    let resolved = if linked.is_absolute() {
        linked
    } else {
        target.parent().unwrap().join(linked)
    };
    // compare canonicalized if possible, else lexically
    match (fs::canonicalize(&resolved), fs::canonicalize(expected_source)) {
        (Ok(a), Ok(b)) => a == b,
        _ => resolved == expected_source,
    }
}

pub fn apply_one(project: &Path, kind: AssetKind, name: &str, force: bool) -> Result<()> {
    let source = source_path(kind, name);
    let target = target_path(project, kind, name);

    if !source.exists() {
        return Err(anyhow!("source missing: {}", source.display()));
    }
    fs::create_dir_all(target.parent().unwrap())?;

    if fs::symlink_metadata(&target).is_ok() {
        if is_our_symlink(&target, &source) {
            return Ok(());
        }
        if !force {
            return Err(anyhow!(
                "target exists and is not our symlink: {}",
                target.display()
            ));
        }
        remove_path(&target)?;
    }
    make_symlink(&source, &target)?;
    Ok(())
}

fn remove_path(p: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(p)?;
    if meta.file_type().is_dir() && !meta.file_type().is_symlink() {
        fs::remove_dir_all(p)?;
    } else {
        fs::remove_file(p)?;
    }
    Ok(())
}

pub fn remove_one(project: &Path, kind: AssetKind, name: &str) -> Result<bool> {
    let target = target_path(project, kind, name);
    let source = source_path(kind, name);
    if fs::symlink_metadata(&target).is_ok() && is_our_symlink(&target, &source) {
        remove_path(&target)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn list_installed(project: &Path) -> Vec<InstalledAsset> {
    let mut out = Vec::new();
    for kind in AssetKind::all() {
        let dir = project.join(".claude").join(kind.as_str());
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let file_name = e.file_name().to_string_lossy().to_string();
            let name = match kind {
                AssetKind::Skills => file_name.clone(),
                _ => file_name.trim_end_matches(".md").to_string(),
            };
            let source = source_path(kind, &name);
            if is_our_symlink(&e.path(), &source) {
                out.push(InstalledAsset { kind, name });
            }
        }
    }
    out
}

/// Returns the canonical source path if the symlink at `target` points into
/// our hooks library directory; returns None otherwise.
fn our_hook_source(target: &Path) -> Option<PathBuf> {
    let meta = fs::symlink_metadata(target).ok()?;
    if !meta.file_type().is_symlink() {
        return None;
    }
    let linked = fs::read_link(target).ok()?;
    let resolved = if linked.is_absolute() {
        linked
    } else {
        target.parent()?.join(linked)
    };
    let canonical = fs::canonicalize(&resolved).unwrap_or(resolved);
    let hooks_lib = fs::canonicalize(library_dir().join("hooks")).ok()?;
    if canonical.starts_with(&hooks_lib) {
        Some(canonical)
    } else {
        None
    }
}

pub fn apply_hook(project: &Path, plugin: &str, filename: &str) -> Result<()> {
    let source = library_dir().join("hooks").join(plugin).join(filename);
    let target = project.join(".claude").join("hooks").join(filename);

    if !source.exists() {
        return Err(anyhow!("hook source missing: {}", source.display()));
    }
    fs::create_dir_all(target.parent().unwrap())?;

    if fs::symlink_metadata(&target).is_ok() {
        if our_hook_source(&target).is_some() {
            // Already one of ours — nothing to do.
            return Ok(());
        }
        return Err(anyhow!(
            "target exists and is not our symlink: {}",
            target.display()
        ));
    }
    make_symlink(&source, &target)?;
    Ok(())
}

pub fn remove_hook(project: &Path, filename: &str) -> Result<bool> {
    let target = project.join(".claude").join("hooks").join(filename);
    if our_hook_source(&target).is_some() {
        fs::remove_file(&target)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn list_installed_hooks(project: &Path) -> Vec<InstalledHook> {
    let hooks_dir = project.join(".claude").join("hooks");
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(&hooks_dir) else {
        return out;
    };
    let hooks_lib = match fs::canonicalize(library_dir().join("hooks")) {
        Ok(p) => p,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let filename = entry.file_name().to_string_lossy().to_string();
        if let Some(source) = our_hook_source(&entry.path()) {
            // source is a canonical path: <hooks_lib>/<plugin>/<filename>
            if let Some(plugin_dir) = source.parent() {
                if plugin_dir.parent().map(|p| p == hooks_lib).unwrap_or(false) {
                    if let Some(plugin) = plugin_dir.file_name() {
                        out.push(InstalledHook {
                            plugin: plugin.to_string_lossy().to_string(),
                            filename,
                        });
                    }
                }
            }
        }
    }
    out
}

pub fn apply_mcp(project: &Path, plugin: &str) -> Result<()> {
    let mcp_src = library_dir().join("mcp").join(format!("{plugin}.json"));
    if !mcp_src.exists() {
        return Err(anyhow!("MCP source missing: {}", mcp_src.display()));
    }

    let src_content = fs::read_to_string(&mcp_src)?;
    let src_json: serde_json::Value = serde_json::from_str(&src_content)?;
    let src_servers = src_json
        .as_object()
        .ok_or_else(|| anyhow!("MCP file must be a JSON object"))?;

    let project_mcp = project.join(".claude").join("mcp.json");
    fs::create_dir_all(project_mcp.parent().unwrap())?;

    let mut existing: serde_json::Map<String, serde_json::Value> = if project_mcp.exists() {
        let content = fs::read_to_string(&project_mcp)?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        serde_json::Map::new()
    };

    for (key, value) in src_servers {
        // Skip keys already present — never overwrite user config.
        existing.entry(key.clone()).or_insert_with(|| value.clone());
    }

    let output = serde_json::to_string_pretty(&serde_json::Value::Object(existing))?;
    fs::write(&project_mcp, output)?;
    Ok(())
}
