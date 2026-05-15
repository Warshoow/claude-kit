use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use claude_kit_core::bundles::{self, BundleEntryKind};
use claude_kit_core::library;
use claude_kit_core::project;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "ck",
    about = "claude-kit CLI — manage Claude Code asset bundles from the terminal",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Apply a bundle to the .claude/ directory of the current project
    Apply {
        /// Name of the bundle to apply
        bundle: String,
        /// Remove all currently installed assets before applying
        #[arg(long)]
        replace: bool,
        /// Project directory (defaults to current directory)
        #[arg(long, short)]
        project: Option<PathBuf>,
    },
    /// List all available bundles in your library
    List,
    /// Show assets installed in the current project
    Installed {
        /// Project directory (defaults to current directory)
        #[arg(long, short)]
        project: Option<PathBuf>,
    },
    /// Remove all claude-kit symlinks from the current project
    Clean {
        /// Project directory (defaults to current directory)
        #[arg(long, short)]
        project: Option<PathBuf>,
    },
}

fn resolve_project(arg: Option<PathBuf>) -> Result<PathBuf> {
    let p = match arg {
        Some(p) => p,
        None => std::env::current_dir()?,
    };
    let p = p.canonicalize().map_err(|e| {
        anyhow!("project path does not exist or is inaccessible: {e}")
    })?;
    Ok(p)
}

fn cmd_apply(bundle_name: &str, replace: bool, project: Option<PathBuf>) -> Result<()> {
    library::ensure_layout()?;
    let project = resolve_project(project)?;

    let bundle = bundles::read_bundle(bundle_name)
        .ok_or_else(|| anyhow!("bundle not found: {bundle_name}"))?;

    if bundle.assets.is_empty() {
        println!("Bundle '{}' has no assets.", bundle_name);
        return Ok(());
    }

    if replace {
        for i in project::list_installed(&project) {
            let _ = project::remove_one(&project, i.kind, &i.name);
        }
        let hooks = project::list_installed_hooks(&project);
        for h in hooks {
            let _ = project::remove_hook(&project, &h.filename);
        }
        println!("Removed existing assets from project.");
    }

    let mut ok = 0usize;
    let mut errors: Vec<String> = vec![];

    for a in &bundle.assets {
        let label = match &a.plugin {
            Some(p) => format!("{}/{p}/{}", a.kind.as_str(), a.name),
            None => format!("{}/{}", a.kind.as_str(), a.name),
        };

        let result: Result<(), anyhow::Error> = match a.kind.as_asset_kind() {
            Some(asset_kind) => project::apply_one(&project, asset_kind, &a.name, false),
            None if a.kind == BundleEntryKind::Hooks => match &a.plugin {
                Some(plugin) => project::apply_hook(&project, plugin, &a.name),
                None => project::apply_hook_local(&project, &a.name),
            },
            None if a.kind == BundleEntryKind::Mcp => match &a.plugin {
                Some(plugin) => project::apply_mcp(&project, plugin),
                None => project::apply_mcp_local(&project, &a.name),
            },
            None => unreachable!("BundleEntryKind covered above"),
        };

        match result {
            Ok(()) => {
                println!("  ✓ {label}");
                ok += 1;
            }
            Err(e) => {
                let msg = format!("  ✗ {label}: {e}");
                eprintln!("{msg}");
                errors.push(label);
            }
        }
    }

    println!();
    if errors.is_empty() {
        println!("Applied '{}': {} asset(s) installed.", bundle_name, ok);
    } else {
        println!(
            "Applied '{}': {} ok, {} error(s).",
            bundle_name,
            ok,
            errors.len()
        );
    }
    Ok(())
}

fn cmd_list() -> Result<()> {
    library::ensure_layout()?;
    let bundles = bundles::list_bundles();

    if bundles.is_empty() {
        println!("No bundles found. Create one with the claude-kit desktop app.");
        return Ok(());
    }

    println!("{:<30} {:<6} {}", "NAME", "ASSETS", "DESCRIPTION");
    println!("{}", "-".repeat(70));
    for b in &bundles {
        let desc = b.description.as_deref().unwrap_or("");
        println!("{:<30} {:<6} {}", b.name, b.assets.len(), desc);
    }
    Ok(())
}

fn cmd_installed(project: Option<PathBuf>) -> Result<()> {
    library::ensure_layout()?;
    let project = resolve_project(project)?;

    let assets = project::list_installed(&project);
    let hooks = project::list_installed_hooks(&project);

    if assets.is_empty() && hooks.is_empty() {
        println!("No claude-kit assets installed in this project.");
        return Ok(());
    }

    for a in &assets {
        println!("{}/{}", a.kind.as_str(), a.name);
    }
    for h in &hooks {
        println!("hooks/{}/{}", h.plugin, h.filename);
    }
    println!();
    println!(
        "{} asset(s), {} hook(s).",
        assets.len(),
        hooks.len()
    );
    Ok(())
}

fn cmd_clean(project: Option<PathBuf>) -> Result<()> {
    library::ensure_layout()?;
    let project = resolve_project(project)?;

    let assets = project::list_installed(&project);
    let hooks = project::list_installed_hooks(&project);

    if assets.is_empty() && hooks.is_empty() {
        println!("Nothing to clean — no claude-kit assets found in this project.");
        return Ok(());
    }

    let mut count = 0usize;
    for a in &assets {
        if project::remove_one(&project, a.kind, &a.name).unwrap_or(false) {
            count += 1;
        }
    }
    for h in &hooks {
        if project::remove_hook(&project, &h.filename).unwrap_or(false) {
            count += 1;
        }
    }

    println!("Removed {count} item(s) from project.");
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Cmd::Apply { bundle, replace, project } => cmd_apply(&bundle, replace, project),
        Cmd::List => cmd_list(),
        Cmd::Installed { project } => cmd_installed(project),
        Cmd::Clean { project } => cmd_clean(project),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
