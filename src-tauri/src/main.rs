// Hide the console window on Windows when running the release binary.
// In debug we keep it so cargo / Tauri logs are visible. Without this,
// double-clicking the installed .exe on Windows opens both the GUI and
// a stray cmd window behind it.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai;
mod bundle_chat;
mod bundles;
mod harmonize;
mod library;
mod marketplace;
mod project;
mod recommend;
mod settings;
mod update;

// Single lock shared across all test modules that mutate CLAUDE_KIT_HOME.
// Using crate::HOME_LOCK from each module's #[cfg(test)] block prevents races
// between modules that otherwise run in parallel.
#[cfg(test)]
pub static HOME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

use bundles::{Bundle, BundleRef, ImportShareResult};
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
fn encode_bundle_share(name: String) -> Result<String, String> {
    bundles::encode_bundle_share(&name).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_bundle_share(code: String) -> Result<ImportShareResult, String> {
    bundles::import_bundle_share(&code).map_err(|e| e.to_string())
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
        let label = match &a.plugin {
            Some(p) => format!("{}/{p}/{}", a.kind.as_str(), a.name),
            None => format!("{}/{}", a.kind.as_str(), a.name),
        };
        let result: Result<(), anyhow::Error> = match a.kind.as_asset_kind() {
            // Skills / commands / agents — symlink the markdown asset.
            Some(asset_kind) => project::apply_one(&project, asset_kind, &a.name, false),
            // Hooks: symlink the script. Step 1 requires `plugin` to
            // be set (flat layout comes with the AI-hook UI later).
            None if a.kind == bundles::BundleEntryKind::Hooks => match &a.plugin {
                Some(plugin) => project::apply_hook(&project, plugin, &a.name),
                None => Err(anyhow::anyhow!(
                    "hook ref missing plugin — flat hooks not supported yet"
                )),
            },
            // MCP: merge servers into project's .claude/mcp.json.
            // Plugin-scoped → reads `mcp/<plugin>.json` whole; local →
            // reads `mcp/__local__/<name>.json` (one-server-per-file
            // shape produced by the manual create flow).
            None if a.kind == bundles::BundleEntryKind::Mcp => match &a.plugin {
                Some(plugin) => project::apply_mcp(&project, plugin),
                None => project::apply_mcp_local(&project, &a.name),
            },
            None => unreachable!("BundleEntryKind covered above"),
        };
        match result {
            Ok(()) => ok.push(label),
            Err(e) => errors.push(format!("{label}: {e}")),
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
    library::import_from_plugin(&PathBuf::from(source_path), None).map_err(|e| e.to_string())
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
fn list_marketplace_sources_cmd() -> Vec<settings::MarketplaceSource> {
    marketplace::list_sources()
}

#[tauri::command]
fn add_marketplace_source_cmd(url: String) -> Result<settings::MarketplaceSource, String> {
    marketplace::add_source(&url).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_marketplace_source_cmd(url: String) -> Result<(), String> {
    marketplace::remove_source(&url).map_err(|e| e.to_string())
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
fn fetch_plugin_readme(
    plugin: Plugin,
    marketplace_name: Option<String>,
) -> Result<Option<String>, String> {
    marketplace::fetch_readme(&plugin, marketplace_name.as_deref())
        .map_err(|e| e.to_string())
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
fn apply_mcp_local_cmd(project_path: String, name: String) -> Result<(), String> {
    project::apply_mcp_local(&PathBuf::from(project_path), &name).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_local_mcp_cmd(
    name: String,
    server_config: serde_json::Value,
) -> Result<(), String> {
    library::create_local_mcp(&name, server_config).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_local_mcp_cmd(
    name: String,
    server_config: serde_json::Value,
) -> Result<(), String> {
    library::update_local_mcp(&name, server_config).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_local_mcp_cmd(name: String) -> Result<bool, String> {
    library::delete_local_mcp(&name).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_local_mcp_cmd(name: String) -> Option<serde_json::Value> {
    library::read_local_mcp(&name)
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
fn refine_asset_chat(
    on_event: tauri::ipc::Channel<ai::StreamEvent>,
    kind: AssetKind,
    current_content: String,
    history: Vec<ai::ChatMessage>,
    user_message: String,
) -> Result<(), String> {
    ai::refine_asset_chat(on_event, kind, current_content, history, user_message)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn generate_bundle_chat(
    on_event: tauri::ipc::Channel<bundle_chat::BundleChatEvent>,
    history: Vec<ai::ChatMessage>,
    user_message: String,
) -> Result<(), String> {
    bundle_chat::generate_bundle_chat(on_event, history, user_message)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn materialize_generated_bundle(
    bundle_name: String,
    bundle_description: Option<String>,
    assets: Vec<bundle_chat::GeneratedAsset>,
) -> Result<bundle_chat::MaterializeResult, String> {
    bundle_chat::materialize_generated_bundle(bundle_name, bundle_description, assets)
        .map_err(|e| e.to_string())
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

#[derive(serde::Deserialize)]
struct HarmonizedMark {
    kind: AssetKind,
    name: String,
}

#[tauri::command]
fn mark_assets_harmonized(items: Vec<HarmonizedMark>) -> Result<usize, String> {
    use time::format_description::well_known::Rfc3339;
    use time::OffsetDateTime;

    let now = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default();
    let mut count = 0usize;
    for item in items {
        library::mark_harmonized(item.kind, &item.name, &now)
            .map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(count)
}

#[tauri::command]
fn recommend_bundle_cmd(
    user_need: String,
    marketplace_url: Option<String>,
) -> Result<recommend::RecommendationResult, String> {
    recommend::recommend_bundle(&user_need, marketplace_url.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn preview_plugin_update_cmd(
    plugin: Plugin,
    marketplace_name: Option<String>,
) -> Result<update::UpdatePreview, String> {
    let name = marketplace_name.unwrap_or_else(|| "claude-plugins-official".to_string());
    update::preview_update(&plugin, &name).map_err(|e| e.to_string())
}

#[tauri::command]
fn apply_plugin_update_cmd(
    plugin_name: String,
    marketplace_name: Option<String>,
    new_version: Option<String>,
    new_git_ref: String,
    writes: Vec<update::AssetWrite>,
) -> Result<update::ApplyUpdateResult, String> {
    let market = marketplace_name.unwrap_or_else(|| "claude-plugins-official".to_string());
    update::apply_update(&plugin_name, &market, new_version, new_git_ref, writes)
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
        // Opens URLs and files in the host OS — used for "View on GitHub"
        // buttons and any anchor inside rendered markdown so external
        // links don't navigate the WebView (which has no back button and
        // would dead-end the app).
        .plugin(tauri_plugin_opener::init())
        // In-app auto-update: checks the GitHub Releases manifest, prompts
        // the user, downloads + verifies + installs in place. Fails open
        // (silently) when no signed manifest is available — that's the
        // case before the maintainer has set up the signing keys + GitHub
        // Secrets, see docs/release.md.
        .plugin(tauri_plugin_updater::Builder::new().build())
        // Needed by `relaunch()` after the updater downloads + installs
        // a new version.
        .plugin(tauri_plugin_process::init())
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
            encode_bundle_share,
            import_bundle_share,
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
            apply_mcp_local_cmd,
            create_local_mcp_cmd,
            update_local_mcp_cmd,
            delete_local_mcp_cmd,
            read_local_mcp_cmd,
            remove_plugin_cmd,
            ai_status_cmd,
            ai_generate,
            refine_asset_chat,
            generate_bundle_chat,
            materialize_generated_bundle,
            read_settings_cmd,
            write_settings_cmd,
            harmonize_bundle_cmd,
            mark_assets_harmonized,
            recommend_bundle_cmd,
            preview_plugin_update_cmd,
            apply_plugin_update_cmd,
            list_marketplace_sources_cmd,
            add_marketplace_source_cmd,
            remove_marketplace_source_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running claude-kit");
}
