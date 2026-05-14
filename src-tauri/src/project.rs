use crate::library::{library_dir, AssetKind, LOCAL_PLUGIN};
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

/// Apply a locally-created hook: symlink the script AND auto-register
/// it in the project's `.claude/settings.json` using the metadata the
/// user stored when creating the hook. We do the registration here
/// (and not for plugin hooks) because plugin hooks carry their
/// matcher/event in the plugin's own config and would need a more
/// involved import-time pipeline to surface that to us.
pub fn apply_hook_local(project: &Path, filename: &str) -> Result<()> {
    // 1. Symlink the script the same way `apply_hook` does — same
    //    safety check against clobbering user files.
    apply_hook(project, crate::library::LOCAL_PLUGIN, filename)?;

    // 2. Register the hook in settings.json under the right event /
    //    matcher bucket. The target path on disk is the symlink we
    //    just created — claude-code resolves it transparently.
    let meta = crate::library::read_local_hook(filename)
        .map(|(_, m)| m)
        .ok_or_else(|| anyhow!("missing meta for local hook '{filename}'"))?;
    let symlink_path = project.join(".claude").join("hooks").join(filename);
    let command_path = symlink_path.to_string_lossy().to_string();
    register_hook_in_settings(project, &meta.event, &meta.matcher, &command_path)?;
    Ok(())
}

/// Remove a local hook from the project: drop the symlink AND
/// unregister from settings.json. Returns whether anything actually
/// changed (matches the existing `remove_hook` semantics).
pub fn remove_hook_local(project: &Path, filename: &str) -> Result<bool> {
    let removed_link = remove_hook(project, filename)?;

    // Even if the symlink was missing, the registration in
    // settings.json might still need cleaning up — try regardless.
    let meta = crate::library::read_local_hook(filename).map(|(_, m)| m);
    if let Some(meta) = meta {
        let symlink_path = project.join(".claude").join("hooks").join(filename);
        let command_path = symlink_path.to_string_lossy().to_string();
        let _ = unregister_hook_in_settings(project, &meta.event, &meta.matcher, &command_path);
    }
    Ok(removed_link)
}

/// Merge a hook registration into `.claude/settings.json`. Idempotent:
/// if our exact `(event, matcher, command)` entry is already present,
/// we leave the file unchanged.
fn register_hook_in_settings(
    project: &Path,
    event: &str,
    matcher: &str,
    command: &str,
) -> Result<()> {
    let settings_path = project.join(".claude").join("settings.json");
    fs::create_dir_all(settings_path.parent().unwrap())?;

    let mut root: serde_json::Value = if settings_path.exists() {
        let raw = fs::read_to_string(&settings_path)?;
        serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Ensure root is an object so the in-place edits below are valid.
    if !root.is_object() {
        root = serde_json::json!({});
    }
    let root_obj = root.as_object_mut().unwrap();

    let hooks_section = root_obj
        .entry("hooks".to_string())
        .or_insert_with(|| serde_json::json!({}));
    if !hooks_section.is_object() {
        *hooks_section = serde_json::json!({});
    }
    let hooks_obj = hooks_section.as_object_mut().unwrap();

    let event_arr = hooks_obj
        .entry(event.to_string())
        .or_insert_with(|| serde_json::json!([]));
    if !event_arr.is_array() {
        *event_arr = serde_json::json!([]);
    }
    let event_arr = event_arr.as_array_mut().unwrap();

    let normalized_matcher = if matcher.is_empty() { "*" } else { matcher };
    let our_hook = serde_json::json!({
        "type": "command",
        "command": command,
    });

    // 1) Find an existing matcher entry for this event — merge into it.
    for entry in event_arr.iter_mut() {
        let entry_matcher = entry
            .get("matcher")
            .and_then(|v| v.as_str())
            .unwrap_or("*");
        if entry_matcher == normalized_matcher {
            let hooks_array = entry
                .get_mut("hooks")
                .and_then(|v| v.as_array_mut());
            if let Some(arr) = hooks_array {
                // Skip duplicate registrations on re-apply.
                let already = arr.iter().any(|h| {
                    h.get("command").and_then(|v| v.as_str()) == Some(command)
                });
                if !already {
                    arr.push(our_hook);
                }
                fs::write(&settings_path, serde_json::to_string_pretty(&root)?)?;
                return Ok(());
            }
        }
    }

    // 2) No existing matcher entry — append a fresh one.
    event_arr.push(serde_json::json!({
        "matcher": normalized_matcher,
        "hooks": [our_hook],
    }));
    fs::write(&settings_path, serde_json::to_string_pretty(&root)?)?;
    Ok(())
}

/// Reverse of `register_hook_in_settings`. Removes our exact
/// `command` from the matcher's hooks array; prunes empty matcher /
/// event / hooks scaffolding so we don't leave dangling structure.
fn unregister_hook_in_settings(
    project: &Path,
    event: &str,
    matcher: &str,
    command: &str,
) -> Result<()> {
    let settings_path = project.join(".claude").join("settings.json");
    if !settings_path.exists() {
        return Ok(());
    }
    let raw = fs::read_to_string(&settings_path)?;
    let mut root: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };
    let Some(root_obj) = root.as_object_mut() else {
        return Ok(());
    };
    let Some(hooks_section) = root_obj.get_mut("hooks").and_then(|v| v.as_object_mut())
    else {
        return Ok(());
    };

    let normalized_matcher = if matcher.is_empty() { "*" } else { matcher };

    let mut prune_event = false;
    if let Some(event_arr) = hooks_section.get_mut(event).and_then(|v| v.as_array_mut()) {
        let mut prune_matchers: Vec<usize> = Vec::new();
        for (idx, entry) in event_arr.iter_mut().enumerate() {
            let entry_matcher = entry
                .get("matcher")
                .and_then(|v| v.as_str())
                .unwrap_or("*");
            if entry_matcher != normalized_matcher {
                continue;
            }
            if let Some(arr) = entry.get_mut("hooks").and_then(|v| v.as_array_mut()) {
                arr.retain(|h| h.get("command").and_then(|v| v.as_str()) != Some(command));
                if arr.is_empty() {
                    prune_matchers.push(idx);
                }
            }
        }
        // Remove emptied matcher entries from highest index down.
        for idx in prune_matchers.into_iter().rev() {
            event_arr.remove(idx);
        }
        if event_arr.is_empty() {
            prune_event = true;
        }
    }
    if prune_event {
        hooks_section.remove(event);
    }
    // Don't bother pruning empty `hooks` map — keeping the key is
    // harmless and avoids churning the file on every apply/remove.

    fs::write(&settings_path, serde_json::to_string_pretty(&root)?)?;
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
    apply_mcp_from_source(project, &mcp_src)
}

/// Apply a local MCP entry stored under `mcp/__local__/<name>.json`.
/// Same merge semantics as `apply_mcp` — only the source path differs.
pub fn apply_mcp_local(project: &Path, name: &str) -> Result<()> {
    let mcp_src = library_dir()
        .join("mcp")
        .join(LOCAL_PLUGIN)
        .join(format!("{name}.json"));
    apply_mcp_from_source(project, &mcp_src)
}

/// Shared merge logic. Reads the source MCP file, peels off the
/// `mcpServers` envelope if present (newer claude-code format), and
/// merges each server key into the project's `.claude/mcp.json`
/// without ever overwriting one that's already there.
fn apply_mcp_from_source(project: &Path, mcp_src: &Path) -> Result<()> {
    if !mcp_src.exists() {
        return Err(anyhow!("MCP source missing: {}", mcp_src.display()));
    }
    let src_content = fs::read_to_string(mcp_src)?;
    let src_json: serde_json::Value = serde_json::from_str(&src_content)?;

    // Accept both shapes:
    //   1. `{"mcpServers": {<name>: ...}}` — standard claude-code wrapper
    //   2. `{<name>: ...}` — older / wrapper-less form some plugins ship
    let src_servers = match src_json.get("mcpServers") {
        Some(inner) => inner.as_object().ok_or_else(|| {
            anyhow!("`mcpServers` must be an object in {}", mcp_src.display())
        })?,
        None => src_json.as_object().ok_or_else(|| {
            anyhow!("MCP file must be a JSON object in {}", mcp_src.display())
        })?,
    };

    let project_mcp = project.join(".claude").join("mcp.json");
    fs::create_dir_all(project_mcp.parent().unwrap())?;

    let mut existing: serde_json::Map<String, serde_json::Value> = if project_mcp.exists() {
        let content = fs::read_to_string(&project_mcp)?;
        // Unwrap the same envelope on the project side if present so
        // we round-trip the user's existing structure.
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
        match parsed.get("mcpServers").and_then(|v| v.as_object()) {
            Some(inner) => inner.clone(),
            None => parsed.as_object().cloned().unwrap_or_default(),
        }
    } else {
        serde_json::Map::new()
    };

    for (key, value) in src_servers {
        // Skip keys already present — never overwrite user config.
        existing.entry(key.clone()).or_insert_with(|| value.clone());
    }

    let wrapped = serde_json::json!({ "mcpServers": existing });
    let output = serde_json::to_string_pretty(&wrapped)?;
    fs::write(&project_mcp, output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::{create_local_hook, ensure_layout};

    fn with_temp_home<F: FnOnce(&Path)>(f: F) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let _guard = crate::HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("CLAUDE_KIT_HOME", tmp.path());
        ensure_layout().expect("ensure_layout");

        let project_dir = tempfile::tempdir().expect("project tempdir");
        f(project_dir.path());

        std::env::remove_var("CLAUDE_KIT_HOME");
    }

    fn read_settings_json(project: &Path) -> serde_json::Value {
        let p = project.join(".claude").join("settings.json");
        let raw = fs::read_to_string(&p).unwrap();
        serde_json::from_str(&raw).unwrap()
    }

    #[test]
    fn register_hook_creates_settings_when_absent() {
        with_temp_home(|project| {
            register_hook_in_settings(project, "PreToolUse", "Bash", "/path/to/x")
                .unwrap();
            let s = read_settings_json(project);
            let entry = &s["hooks"]["PreToolUse"][0];
            assert_eq!(entry["matcher"].as_str(), Some("Bash"));
            assert_eq!(entry["hooks"][0]["type"].as_str(), Some("command"));
            assert_eq!(entry["hooks"][0]["command"].as_str(), Some("/path/to/x"));
        });
    }

    #[test]
    fn register_hook_is_idempotent() {
        with_temp_home(|project| {
            register_hook_in_settings(project, "PreToolUse", "Bash", "/x").unwrap();
            register_hook_in_settings(project, "PreToolUse", "Bash", "/x").unwrap();
            let s = read_settings_json(project);
            // Still only one hook entry, even after a re-apply.
            assert_eq!(s["hooks"]["PreToolUse"][0]["hooks"].as_array().unwrap().len(), 1);
        });
    }

    #[test]
    fn register_hook_merges_into_existing_matcher() {
        with_temp_home(|project| {
            register_hook_in_settings(project, "PreToolUse", "Bash", "/a").unwrap();
            register_hook_in_settings(project, "PreToolUse", "Bash", "/b").unwrap();
            let s = read_settings_json(project);
            let arr = s["hooks"]["PreToolUse"].as_array().unwrap();
            assert_eq!(arr.len(), 1, "should reuse the Bash-matcher entry");
            let hooks = arr[0]["hooks"].as_array().unwrap();
            assert_eq!(hooks.len(), 2);
        });
    }

    #[test]
    fn register_hook_appends_new_matcher_block() {
        with_temp_home(|project| {
            register_hook_in_settings(project, "PreToolUse", "Bash", "/a").unwrap();
            register_hook_in_settings(project, "PreToolUse", "Edit", "/b").unwrap();
            let s = read_settings_json(project);
            assert_eq!(s["hooks"]["PreToolUse"].as_array().unwrap().len(), 2);
        });
    }

    #[test]
    fn register_hook_preserves_unrelated_settings() {
        with_temp_home(|project| {
            // Existing settings the user has hand-written — must survive.
            let claude_dir = project.join(".claude");
            fs::create_dir_all(&claude_dir).unwrap();
            fs::write(
                claude_dir.join("settings.json"),
                r#"{"permissions":{"allow":["Bash"]},"theme":"dark"}"#,
            )
            .unwrap();

            register_hook_in_settings(project, "Stop", "*", "/x").unwrap();

            let s = read_settings_json(project);
            assert_eq!(s["theme"].as_str(), Some("dark"), "untouched");
            assert!(s["permissions"]["allow"].is_array(), "untouched");
            assert_eq!(s["hooks"]["Stop"][0]["matcher"].as_str(), Some("*"));
        });
    }

    #[test]
    fn register_hook_normalizes_empty_matcher_to_star() {
        with_temp_home(|project| {
            register_hook_in_settings(project, "Stop", "", "/x").unwrap();
            let s = read_settings_json(project);
            assert_eq!(s["hooks"]["Stop"][0]["matcher"].as_str(), Some("*"));
        });
    }

    #[test]
    fn unregister_hook_removes_only_our_entry() {
        with_temp_home(|project| {
            register_hook_in_settings(project, "PreToolUse", "Bash", "/ours").unwrap();
            register_hook_in_settings(project, "PreToolUse", "Bash", "/theirs").unwrap();

            unregister_hook_in_settings(project, "PreToolUse", "Bash", "/ours")
                .unwrap();

            let s = read_settings_json(project);
            let hooks = s["hooks"]["PreToolUse"][0]["hooks"].as_array().unwrap();
            assert_eq!(hooks.len(), 1);
            assert_eq!(hooks[0]["command"].as_str(), Some("/theirs"));
        });
    }

    #[test]
    fn unregister_hook_prunes_empty_matcher_and_event() {
        with_temp_home(|project| {
            register_hook_in_settings(project, "Stop", "*", "/x").unwrap();
            unregister_hook_in_settings(project, "Stop", "*", "/x").unwrap();
            let s = read_settings_json(project);
            assert!(
                s.get("hooks").and_then(|h| h.get("Stop")).is_none(),
                "emptied event must be pruned"
            );
        });
    }

    #[test]
    fn apply_hook_local_creates_symlink_and_registers() {
        with_temp_home(|project| {
            create_local_hook("h.sh", "PreToolUse", "Bash", None, "#!/bin/bash\n")
                .unwrap();

            apply_hook_local(project, "h.sh").unwrap();

            // Symlink in place.
            let link = project.join(".claude").join("hooks").join("h.sh");
            assert!(fs::symlink_metadata(&link).is_ok());

            // Registration landed.
            let s = read_settings_json(project);
            let cmd = s["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
                .as_str()
                .unwrap();
            assert!(cmd.ends_with("h.sh"));
        });
    }

    #[test]
    fn remove_hook_local_unlinks_and_unregisters() {
        with_temp_home(|project| {
            create_local_hook("h.sh", "PreToolUse", "Bash", None, "#!/bin/bash\n")
                .unwrap();
            apply_hook_local(project, "h.sh").unwrap();
            remove_hook_local(project, "h.sh").unwrap();

            assert!(fs::symlink_metadata(
                project.join(".claude").join("hooks").join("h.sh")
            )
            .is_err());

            let s = read_settings_json(project);
            assert!(
                s.get("hooks")
                    .and_then(|h| h.get("PreToolUse"))
                    .is_none(),
                "registration should be gone"
            );
        });
    }
}
