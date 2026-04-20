mod bundles;
mod library;
mod project;

use bundles::{Bundle, BundleRef};
use library::{ensure_layout, scan_all, Asset, AssetKind, ImportResult};
use project::InstalledAsset;
use std::path::PathBuf;

#[tauri::command]
fn init_library() -> Result<String, String> {
    ensure_layout().map_err(|e| e.to_string())?;
    Ok(library::kit_home().to_string_lossy().to_string())
}

#[tauri::command]
fn list_library() -> Vec<Asset> {
    scan_all()
}

#[tauri::command]
fn list_bundles_cmd() -> Vec<Bundle> {
    bundles::list_bundles()
}

#[tauri::command]
fn create_bundle(name: String, description: Option<String>) -> Result<Bundle, String> {
    bundles::create_bundle(&name, description).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_bundle(name: String) -> Result<(), String> {
    bundles::delete_bundle(&name).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_bundle_assets(name: String, assets: Vec<BundleRef>) -> Result<Bundle, String> {
    bundles::set_bundle_assets(&name, assets).map_err(|e| e.to_string())
}

#[tauri::command]
fn apply_bundle(
    project_path: String,
    bundle_name: String,
    replace: bool,
) -> Result<ApplyResult, String> {
    let project = PathBuf::from(project_path);
    let bundle = bundles::read_bundle(&bundle_name)
        .ok_or_else(|| format!("bundle not found: {bundle_name}"))?;

    let mut ok = vec![];
    let mut errors = vec![];

    if replace {
        for i in project::list_installed(&project) {
            let _ = project::remove_one(&project, i.kind, &i.name);
        }
    }

    for a in &bundle.assets {
        match project::apply_one(&project, a.kind, &a.name, false) {
            Ok(()) => ok.push(format!("{}/{}", a.kind.as_str(), a.name)),
            Err(e) => errors.push(format!("{}/{}: {e}", a.kind.as_str(), a.name)),
        }
    }
    Ok(ApplyResult { ok, errors })
}

#[tauri::command]
fn apply_single(
    project_path: String,
    kind: AssetKind,
    name: String,
) -> Result<(), String> {
    project::apply_one(&PathBuf::from(project_path), kind, &name, false)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_single(
    project_path: String,
    kind: AssetKind,
    name: String,
) -> Result<bool, String> {
    project::remove_one(&PathBuf::from(project_path), kind, &name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_installed_cmd(project_path: String) -> Vec<InstalledAsset> {
    project::list_installed(&PathBuf::from(project_path))
}

#[tauri::command]
fn import_plugin(source_path: String) -> Result<ImportResult, String> {
    library::import_from_plugin(&PathBuf::from(source_path)).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_asset(kind: AssetKind, name: String) -> Result<String, String> {
    library::read_asset_content(kind, &name).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_asset(kind: AssetKind, name: String, content: String) -> Result<(), String> {
    library::write_asset_content(kind, &name, &content).map_err(|e| e.to_string())
}

#[tauri::command]
fn clean_project(project_path: String) -> Result<usize, String> {
    let project = PathBuf::from(project_path);
    let installed = project::list_installed(&project);
    let mut count = 0;
    for i in &installed {
        if project::remove_one(&project, i.kind, &i.name)
            .map_err(|e| e.to_string())?
        {
            count += 1;
        }
    }
    Ok(count)
}

#[derive(serde::Serialize)]
struct ApplyResult {
    ok: Vec<String>,
    errors: Vec<String>,
}

fn main() {
    // create library on startup so the UI has something to show
    let _ = ensure_layout();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            init_library,
            list_library,
            list_bundles_cmd,
            create_bundle,
            delete_bundle,
            set_bundle_assets,
            apply_bundle,
            apply_single,
            remove_single,
            list_installed_cmd,
            clean_project,
            import_plugin,
            read_asset,
            write_asset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running claude-kit");
}
