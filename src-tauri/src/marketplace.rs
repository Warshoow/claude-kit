use crate::library::{self, AssetKind, ImportResult, Origin};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::Duration;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub const DEFAULT_MARKETPLACE_URL: &str =
    "https://raw.githubusercontent.com/anthropics/claude-plugins-official/main/.claude-plugin/marketplace.json";

// Repo that hosts the official marketplace.json. Inline `source` strings
// (e.g. "./plugins/foo") are resolved relative to a tarball of this repo.
const MARKETPLACE_REPO: &str = "anthropics/claude-plugins-official";
const DEFAULT_REF: &str = "main";

#[derive(Debug, Serialize, Deserialize)]
pub struct Marketplace {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub owner: Option<Owner>,
    pub plugins: Vec<Plugin>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Owner {
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Plugin {
    pub name: String,
    pub description: String,
    pub source: PluginSource,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub author: Option<Author>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
}

// `source` is polymorphic: either a string ("./plugins/foo" — relative subdir of
// the marketplace repo itself) or a tagged object describing where to fetch from.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginSource {
    Inline(String),
    Object(PluginSourceObject),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "kebab-case")]
pub enum PluginSourceObject {
    Url {
        url: String,
        #[serde(default)]
        sha: Option<String>,
    },
    GitSubdir {
        url: String,
        path: String,
        #[serde(default, rename = "ref")]
        git_ref: Option<String>,
        #[serde(default)]
        sha: Option<String>,
        #[serde(default)]
        branch: Option<String>,
    },
    Github {
        repo: String,
        #[serde(default)]
        commit: Option<String>,
    },
}

pub fn fetch_marketplace(url: &str) -> Result<Marketplace> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("claude-kit/0.1")
        .build()?;

    let resp = client.get(url).send()?.error_for_status()?;
    let m: Marketplace = resp.json()?;
    Ok(m)
}

struct Resolved {
    tarball_url: String,
    /// Path inside the extracted top-level dir where the plugin lives.
    /// Empty string means the plugin sits at the repo root.
    subdir: String,
}

fn github_tarball_url(repo: &str, git_ref: &str) -> String {
    // codeload.github.com is the direct tar.gz endpoint (no API rate limit, no auth).
    format!("https://codeload.github.com/{repo}/tar.gz/{git_ref}")
}

fn parse_github_url(url: &str) -> Result<String> {
    let trimmed = url.trim_end_matches('/').trim_end_matches(".git");
    let after = trimmed
        .strip_prefix("https://github.com/")
        .or_else(|| trimmed.strip_prefix("http://github.com/"))
        .or_else(|| trimmed.strip_prefix("git@github.com:"))
        .ok_or_else(|| anyhow!("not a github url: {url}"))?;
    if !after.contains('/') {
        return Err(anyhow!("malformed github url: {url}"));
    }
    Ok(after.to_string())
}

fn resolve_source(src: &PluginSource) -> Result<Resolved> {
    match src {
        PluginSource::Inline(path) => {
            // Path is relative to the marketplace's host repo (e.g. "./plugins/foo").
            let subdir = path.trim_start_matches("./").trim_start_matches('/').to_string();
            Ok(Resolved {
                tarball_url: github_tarball_url(MARKETPLACE_REPO, DEFAULT_REF),
                subdir,
            })
        }
        PluginSource::Object(obj) => match obj {
            PluginSourceObject::Url { url, sha } => {
                let repo = parse_github_url(url)?;
                let git_ref = sha.as_deref().unwrap_or(DEFAULT_REF);
                Ok(Resolved {
                    tarball_url: github_tarball_url(&repo, git_ref),
                    subdir: String::new(),
                })
            }
            PluginSourceObject::GitSubdir { url, path, git_ref, sha, branch } => {
                let repo = parse_github_url(url)?;
                let r = sha.as_deref()
                    .or(git_ref.as_deref())
                    .or(branch.as_deref())
                    .unwrap_or(DEFAULT_REF);
                Ok(Resolved {
                    tarball_url: github_tarball_url(&repo, r),
                    subdir: path.clone(),
                })
            }
            PluginSourceObject::Github { repo, commit } => {
                let git_ref = commit.as_deref().unwrap_or(DEFAULT_REF);
                Ok(Resolved {
                    tarball_url: github_tarball_url(repo, git_ref),
                    subdir: String::new(),
                })
            }
        },
    }
}

fn parse_kind_name(entry: &str) -> Option<(AssetKind, &str)> {
    let (kind_str, name) = entry.split_once('/')?;
    let kind = match kind_str {
        "skills" => AssetKind::Skills,
        "commands" => AssetKind::Commands,
        "agents" => AssetKind::Agents,
        _ => return None,
    };
    Some((kind, name))
}

pub fn import_plugin(plugin: &Plugin, marketplace_name: &str) -> Result<ImportResult> {
    let resolved = resolve_source(&plugin.source)?;

    // Download tarball into memory. Plugins are small (a few hundred KB after
    // gzip in the worst case), so we don't bother streaming to disk.
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent("claude-kit/0.1")
        .build()?;
    let bytes = client
        .get(&resolved.tarball_url)
        .send()?
        .error_for_status()?
        .bytes()?;

    // Extract gzipped tar to a temp dir (auto-cleaned when `temp` is dropped).
    let temp = tempfile::tempdir()?;
    let gz = flate2::read::GzDecoder::new(&bytes[..]);
    let mut archive = tar::Archive::new(gz);
    archive.unpack(temp.path())?;

    // codeload always wraps content in a single top-level dir like "{repo}-{ref}/".
    let mut top_dirs: Vec<_> = fs::read_dir(temp.path())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    if top_dirs.len() != 1 {
        return Err(anyhow!(
            "expected exactly 1 top-level dir in tarball, got {}",
            top_dirs.len()
        ));
    }
    let extracted_root = top_dirs.remove(0);

    let plugin_root = if resolved.subdir.is_empty() {
        extracted_root
    } else {
        extracted_root.join(&resolved.subdir)
    };

    if !plugin_root.is_dir() {
        return Err(anyhow!(
            "subdir not found in tarball: {}",
            resolved.subdir
        ));
    }

    let result = library::import_from_plugin(&plugin_root)?;

    // Tag freshly-imported assets with their origin. Failures here don't roll
    // back the import — the assets are on disk; missing the origin trace is a
    // soft degradation we'd rather log than rollback.
    let imported_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| String::new());
    for entry in &result.imported {
        let Some((kind, name)) = parse_kind_name(entry) else { continue };
        let origin = Origin {
            marketplace: marketplace_name.to_string(),
            plugin: plugin.name.clone(),
            imported_at: imported_at.clone(),
        };
        let _ = library::set_origin(kind, name, origin);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Network tests — run with: cargo test --no-default-features -- --ignored --test-threads=1
    // (--test-threads=1 because they share CLAUDE_KIT_HOME via env var)

    #[test]
    #[ignore]
    fn fetches_and_parses_official_marketplace() {
        let m = fetch_marketplace(DEFAULT_MARKETPLACE_URL).expect("fetch");
        assert_eq!(m.name, "claude-plugins-official");
        assert!(m.plugins.len() > 100, "expected many plugins, got {}", m.plugins.len());
        for p in &m.plugins {
            assert!(!p.name.is_empty());
            assert!(!p.description.is_empty());
        }
    }

    #[test]
    #[ignore]
    fn imports_inline_plugin_from_official_marketplace() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::env::set_var("CLAUDE_KIT_HOME", temp.path());

        // `code-review` is a small Anthropic-authored plugin under ./plugins/
        let plugin = Plugin {
            name: "code-review".into(),
            description: "test".into(),
            source: PluginSource::Inline("./plugins/code-review".into()),
            category: None,
            author: None,
            homepage: None,
            version: None,
        };

        let result = import_plugin(&plugin, "claude-plugins-official").expect("import");
        println!("imported: {:?}", result.imported);
        println!("skipped: {:?}", result.skipped);
        assert!(
            !result.imported.is_empty(),
            "expected at least one asset to be imported"
        );

        // Origins manifest should contain an entry for every imported asset.
        let origins = library::read_origins();
        assert_eq!(
            origins.len(),
            result.imported.len(),
            "expected one origin per imported asset"
        );
        for entry in &result.imported {
            let key = entry.replace('/', ":");
            let origin = origins.get(&key).expect("origin entry");
            assert_eq!(origin.marketplace, "claude-plugins-official");
            assert_eq!(origin.plugin, "code-review");
            assert!(!origin.imported_at.is_empty());
        }

        // Re-importing should now skip everything we already wrote.
        let again = import_plugin(&plugin, "claude-plugins-official").expect("re-import");
        assert!(again.imported.is_empty(), "re-import should skip");
        assert_eq!(again.skipped.len(), result.imported.len());
    }
}
