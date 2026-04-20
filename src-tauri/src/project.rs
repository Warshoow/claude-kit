use crate::library::{library_dir, AssetKind};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

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
    use std::os::windows::fs::{symlink_dir, symlink_file};
    if source.is_dir() {
        symlink_dir(source, target)?;
    } else {
        symlink_file(source, target)?;
    }
    Ok(())
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
