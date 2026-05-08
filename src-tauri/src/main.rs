mod ai;
mod bundles;
mod harmonize;
mod library;
mod marketplace;
mod project;
mod settings;

use bundles::{Bundle, BundleRef};
use library::{ensure_layout, scan_all, Asset, AssetKind, HookEntry, ImportResult, McpEntry};
use marketplace::{Marketplace, Plugin};
use project::{InstalledAsset, InstalledHook};
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
fn create_asset(
    kind: AssetKind,
    name: String,
    description: Option<String>,
) -> Result<(), String> {
    library::create_asset(kind, &name, description.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_marketplace_plugins(url: Option<String>) -> Result<Marketplace, String> {
    let url = url.unwrap_or_else(|| marketplace::DEFAULT_MARKETPLACE_URL.to_string());
    marketplace::fetch_marketplace(&url).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_marketplace_plugin(
    plugin: Plugin,
    marketplace_name: Option<String>,
) -> Result<ImportResult, String> {
    let name = marketplace_name.unwrap_or_else(|| "claude-plugins-official".to_string());
    marketplace::import_plugin(&plugin, &name).map_err(|e| e.to_string())
}

#[tauri::command]
fn fetch_plugin_readme(plugin: Plugin) -> Result<Option<String>, String> {
    marketplace::fetch_readme(&plugin).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_hooks_cmd() -> Vec<HookEntry> {
    library::list_hooks()
}

#[tauri::command]
fn list_mcp_cmd() -> Vec<McpEntry> {
    library::list_mcp()
}

#[tauri::command]
fn apply_hook_cmd(project_path: String, plugin: String, filename: String) -> Result<(), String> {
    project::apply_hook(&PathBuf::from(project_path), &plugin, &filename)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_hook_cmd(project_path: String, filename: String) -> Result<bool, String> {
    project::remove_hook(&PathBuf::from(project_path), &filename).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_installed_hooks_cmd(project_path: String) -> Vec<InstalledHook> {
    project::list_installed_hooks(&PathBuf::from(project_path))
}

#[tauri::command]
fn apply_mcp_cmd(project_path: String, plugin: String) -> Result<(), String> {
    project::apply_mcp(&PathBuf::from(project_path), &plugin).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_plugin_cmd(plugin_name: String) -> Result<usize, String> {
    library::remove_plugin(&plugin_name).map_err(|e| e.to_string())
}

#[tauri::command]
fn ai_status_cmd() -> ai::AiStatus {
    ai::ai_status()
}

#[tauri::command]
fn ai_generate(
    kind: AssetKind,
    prompt: String,
    context: Option<String>,
) -> Result<String, String> {
    ai::generate_asset(kind, &prompt, context.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_settings_cmd() -> settings::Settings {
    settings::read_settings()
}

#[tauri::command]
fn write_settings_cmd(settings: settings::Settings) -> Result<(), String> {
    settings::write_settings(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
fn harmonize_bundle_cmd(
    bundle_name: String,
    instruction: Option<String>,
) -> Result<Vec<harmonize::HarmonizationResult>, String> {
    harmonize::harmonize_bundle(&bundle_name, instruction.as_deref())
        .map_err(|e| e.to_string())
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
        .setup(|app| {
            // macOS keeps the native chrome (traffic lights overlaying our
            // topbar via titleBarStyle = Overlay). On Windows and Linux we
            // strip the native title bar so our topbar can serve as the
            // window's drag region with custom min/max/close buttons.
            #[cfg(not(target_os = "macos"))]
            {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_decorations(false);
                }
            }
            Ok(())
        })
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
            create_asset,
            list_marketplace_plugins,
            import_marketplace_plugin,
            fetch_plugin_readme,
            list_hooks_cmd,
            list_mcp_cmd,
            apply_hook_cmd,
            remove_hook_cmd,
            list_installed_hooks_cmd,
            apply_mcp_cmd,
            remove_plugin_cmd,
            ai_status_cmd,
            ai_generate,
            read_settings_cmd,
            write_settings_cmd,
            harmonize_bundle_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running claude-kit");
}
