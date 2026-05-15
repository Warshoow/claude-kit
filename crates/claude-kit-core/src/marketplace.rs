use crate::library::{self, AssetKind, ImportResult, Origin};
use crate::settings::{read_settings, write_settings, MarketplaceSource, OFFICIAL_MARKETPLACE_URL};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub const DEFAULT_MARKETPLACE_URL: &str = OFFICIAL_MARKETPLACE_URL;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Owner {
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
}

// `source` is polymorphic: either a string ("./plugins/foo" — relative subdir of
// the marketplace repo itself) or a tagged object describing where to fetch from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginSource {
    Inline(String),
    Object(PluginSourceObject),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

// ── Marketplace source registry ──────────────────────────────────
//
// The user's settings carry a list of marketplaces. The official one
// is built-in and can't be removed. Adding a marketplace fetches its
// JSON to confirm it's valid and to read its self-declared name.

pub fn list_sources() -> Vec<MarketplaceSource> {
    read_settings().marketplaces
}

pub fn add_source(url: &str) -> Result<MarketplaceSource> {
    let url = url.trim();
    if url.is_empty() {
        return Err(anyhow!("URL is empty"));
    }
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(anyhow!("URL must start with http:// or https://"));
    }

    let mut s = read_settings();
    if s.marketplaces.iter().any(|m| m.url == url) {
        return Err(anyhow!("this marketplace is already registered"));
    }

    // Validate by fetching — refuses non-marketplace URLs early instead
    // of letting them sit in settings until the user notices the empty
    // catalog later.
    let fetched = fetch_marketplace(url)
        .map_err(|e| anyhow!("couldn't fetch marketplace at that URL: {e}"))?;

    if s.marketplaces.iter().any(|m| m.name == fetched.name) {
        return Err(anyhow!(
            "a marketplace named '{}' is already registered",
            fetched.name
        ));
    }

    let entry = MarketplaceSource {
        name: fetched.name,
        url: url.to_string(),
        builtin: false,
    };
    s.marketplaces.push(entry.clone());
    write_settings(&s)?;
    Ok(entry)
}

pub fn remove_source(url: &str) -> Result<()> {
    let mut s = read_settings();
    let Some(pos) = s.marketplaces.iter().position(|m| m.url == url) else {
        return Err(anyhow!("marketplace not found"));
    };
    if s.marketplaces[pos].builtin {
        return Err(anyhow!(
            "the official marketplace can't be removed"
        ));
    }
    s.marketplaces.remove(pos);
    write_settings(&s)?;
    Ok(())
}

/// For inline plugin sources (paths like `./plugins/foo`), the
/// "host repo" is the repository that hosts the marketplace itself —
/// not necessarily Anthropic's official one. We recover it from the
/// marketplace's `marketplace.json` URL stored in settings, which
/// always lives at `raw.githubusercontent.com/<owner>/<repo>/<ref>/...`.
///
/// Returns `None` if the marketplace isn't registered (shouldn't
/// happen in normal flow) or if the URL doesn't follow the GitHub raw
/// pattern (e.g. someone hosts their marketplace on Cloudflare). The
/// caller falls back to the legacy hardcoded constant in those cases,
/// preserving the official marketplace's behaviour.
fn host_repo_for_marketplace(marketplace_name: &str) -> Option<String> {
    let s = read_settings();
    let url = s
        .marketplaces
        .iter()
        .find(|m| m.name == marketplace_name)?
        .url
        .clone();
    extract_host_repo(&url)
}

fn extract_host_repo(raw_url: &str) -> Option<String> {
    let after = raw_url.strip_prefix("https://raw.githubusercontent.com/")?;
    let mut iter = after.splitn(3, '/');
    let owner = iter.next()?;
    let repo = iter.next()?;
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some(format!("{owner}/{repo}"))
}

struct ResolvedRef {
    /// "owner/name"
    repo: String,
    /// Branch, tag or commit sha — anything GitHub accepts as a ref.
    git_ref: String,
    /// Path inside the repo where the plugin lives. Empty means repo root.
    subdir: String,
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

fn resolve_source(src: &PluginSource, host_repo: Option<&str>) -> Result<ResolvedRef> {
    match src {
        PluginSource::Inline(path) => {
            // Path is relative to the marketplace's host repo (e.g. "./plugins/foo").
            // For third-party marketplaces the caller passes the resolved host;
            // when missing (legacy callers, or unrecognised marketplace URL
            // shapes) we fall back to the official Anthropic repo so the
            // pre-multi-marketplace code path keeps working.
            let subdir = path.trim_start_matches("./").trim_start_matches('/').to_string();
            Ok(ResolvedRef {
                repo: host_repo.unwrap_or(MARKETPLACE_REPO).to_string(),
                git_ref: DEFAULT_REF.to_string(),
                subdir,
            })
        }
        PluginSource::Object(obj) => match obj {
            PluginSourceObject::Url { url, sha } => {
                let repo = parse_github_url(url)?;
                let git_ref = sha.as_deref().unwrap_or(DEFAULT_REF).to_string();
                Ok(ResolvedRef {
                    repo,
                    git_ref,
                    subdir: String::new(),
                })
            }
            PluginSourceObject::GitSubdir { url, path, git_ref, sha, branch } => {
                let repo = parse_github_url(url)?;
                let r = sha.as_deref()
                    .or(git_ref.as_deref())
                    .or(branch.as_deref())
                    .unwrap_or(DEFAULT_REF)
                    .to_string();
                Ok(ResolvedRef {
                    repo,
                    git_ref: r,
                    subdir: path.clone(),
                })
            }
            PluginSourceObject::Github { repo, commit } => {
                let git_ref = commit.as_deref().unwrap_or(DEFAULT_REF).to_string();
                Ok(ResolvedRef {
                    repo: repo.clone(),
                    git_ref,
                    subdir: String::new(),
                })
            }
        },
    }
}

fn tarball_url(r: &ResolvedRef) -> String {
    // codeload.github.com is the direct tar.gz endpoint (no API rate limit, no auth).
    format!("https://codeload.github.com/{}/tar.gz/{}", r.repo, r.git_ref)
}

fn readme_urls(r: &ResolvedRef) -> Vec<String> {
    let base = format!("https://raw.githubusercontent.com/{}/{}", r.repo, r.git_ref);
    let mut urls = Vec::with_capacity(4);
    // Try the plugin's own subdir first (may have a tailored README), then
    // fall back to the repo root (single-plugin repos host README at root).
    if !r.subdir.is_empty() {
        urls.push(format!("{base}/{}/README.md", r.subdir));
        urls.push(format!("{base}/{}/readme.md", r.subdir));
    }
    urls.push(format!("{base}/README.md"));
    urls.push(format!("{base}/readme.md"));
    urls
}

pub fn fetch_readme(plugin: &Plugin, marketplace_name: Option<&str>) -> Result<Option<String>> {
    let host = marketplace_name.and_then(host_repo_for_marketplace);
    let resolved = resolve_source(&plugin.source, host.as_deref())?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("claude-kit/0.1")
        .build()?;

    for url in readme_urls(&resolved) {
        let resp = client.get(&url).send()?;
        if resp.status().is_success() {
            return Ok(Some(resp.text()?));
        }
    }
    Ok(None)
}

fn parse_kind_name(entry: &str) -> Option<(AssetKind, String)> {
    let (kind_str, name) = entry.split_once('/')?;
    let kind = match kind_str {
        "skills" => AssetKind::Skills,
        "commands" => AssetKind::Commands,
        "agents" => AssetKind::Agents,
        _ => return None,
    };
    // Skills are directories — name has no extension. Commands and agents are
    // `.md` files; the rest of the codebase keys them by their bare name (see
    // scan_kind), so strip the suffix here to match that convention.
    let bare = match kind {
        AssetKind::Skills => name.to_string(),
        _ => name.trim_end_matches(".md").to_string(),
    };
    Some((kind, bare))
}

/// Hold the temp dir alongside the path so the directory stays alive as
/// long as the caller wants to read from it. Dropping `Extraction`
/// auto-cleans the underlying tempdir.
pub struct Extraction {
    pub plugin_root: PathBuf,
    pub git_ref: String,
    // Kept for the Drop side-effect.
    _temp: tempfile::TempDir,
}

/// Fetch and extract a plugin's tarball without touching the library.
/// Used by both `import_plugin` (which then copies into the library) and
/// the update preview flow (which only reads upstream contents).
pub fn download_and_extract(
    plugin: &Plugin,
    marketplace_name: Option<&str>,
) -> Result<Extraction> {
    let host = marketplace_name.and_then(host_repo_for_marketplace);
    let resolved = resolve_source(&plugin.source, host.as_deref())?;

    // Download tarball into memory. Plugins are small (a few hundred KB after
    // gzip in the worst case), so we don't bother streaming to disk.
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent("claude-kit/0.1")
        .build()?;
    let bytes = client
        .get(tarball_url(&resolved))
        .send()?
        .error_for_status()?
        .bytes()?;

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

    Ok(Extraction {
        plugin_root,
        git_ref: resolved.git_ref,
        _temp: temp,
    })
}

pub fn import_plugin(plugin: &Plugin, marketplace_name: &str) -> Result<ImportResult> {
    let extraction = download_and_extract(plugin, Some(marketplace_name))?;
    let result = library::import_from_plugin(&extraction.plugin_root, Some(&plugin.name))?;

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
            version: plugin.version.clone(),
            git_ref: Some(extraction.git_ref.clone()),
        };
        let _ = library::set_origin(kind, &name, origin);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── extract_host_repo ──────────────────────────────────────────

    #[test]
    fn extract_host_repo_valid_raw_url() {
        let url = "https://raw.githubusercontent.com/owner/repo/main/marketplace.json";
        assert_eq!(extract_host_repo(url), Some("owner/repo".to_string()));
    }

    #[test]
    fn extract_host_repo_deeper_path() {
        let url = "https://raw.githubusercontent.com/anthropics/claude-plugins-official/main/plugins/foo/bar.json";
        assert_eq!(
            extract_host_repo(url),
            Some("anthropics/claude-plugins-official".to_string())
        );
    }

    #[test]
    fn extract_host_repo_non_github_returns_none() {
        assert_eq!(extract_host_repo("https://example.com/file.json"), None);
        assert_eq!(extract_host_repo("https://cdn.jsdelivr.net/gh/owner/repo/file.json"), None);
    }

    #[test]
    fn extract_host_repo_empty_owner_returns_none() {
        assert_eq!(extract_host_repo("https://raw.githubusercontent.com//repo/main/f.json"), None);
    }

    // ── parse_github_url ──────────────────────────────────────────

    #[test]
    fn parse_github_url_https() {
        let r = parse_github_url("https://github.com/owner/repo").unwrap();
        assert_eq!(r, "owner/repo");
    }

    #[test]
    fn parse_github_url_strips_dot_git() {
        let r = parse_github_url("https://github.com/owner/repo.git").unwrap();
        assert_eq!(r, "owner/repo");
    }

    #[test]
    fn parse_github_url_strips_trailing_slash() {
        let r = parse_github_url("https://github.com/owner/repo/").unwrap();
        assert_eq!(r, "owner/repo");
    }

    #[test]
    fn parse_github_url_http_accepted() {
        let r = parse_github_url("http://github.com/owner/repo").unwrap();
        assert_eq!(r, "owner/repo");
    }

    #[test]
    fn parse_github_url_ssh_syntax() {
        let r = parse_github_url("git@github.com:owner/repo").unwrap();
        assert_eq!(r, "owner/repo");
    }

    #[test]
    fn parse_github_url_not_github_errors() {
        assert!(parse_github_url("https://gitlab.com/owner/repo").is_err());
    }

    #[test]
    fn parse_github_url_no_slash_in_path_errors() {
        assert!(parse_github_url("https://github.com/justowner").is_err());
    }

    // ── parse_kind_name ───────────────────────────────────────────

    #[test]
    fn parse_kind_name_command_strips_md() {
        let (kind, name) = parse_kind_name("commands/my-cmd.md").unwrap();
        assert_eq!(kind, AssetKind::Commands);
        assert_eq!(name, "my-cmd");
    }

    #[test]
    fn parse_kind_name_agent_strips_md() {
        let (kind, name) = parse_kind_name("agents/my-agent.md").unwrap();
        assert_eq!(kind, AssetKind::Agents);
        assert_eq!(name, "my-agent");
    }

    #[test]
    fn parse_kind_name_skill_keeps_name() {
        let (kind, name) = parse_kind_name("skills/my-skill").unwrap();
        assert_eq!(kind, AssetKind::Skills);
        assert_eq!(name, "my-skill");
    }

    #[test]
    fn parse_kind_name_skill_without_md_extension() {
        // Skills are directories — names never have .md even if present by accident.
        let (kind, name) = parse_kind_name("skills/foo.md").unwrap();
        assert_eq!(kind, AssetKind::Skills);
        assert_eq!(name, "foo.md"); // NOT stripped for skills
    }

    #[test]
    fn parse_kind_name_unknown_kind_returns_none() {
        assert!(parse_kind_name("hooks/something").is_none());
        assert!(parse_kind_name("mcp/config.json").is_none());
    }

    #[test]
    fn parse_kind_name_no_slash_returns_none() {
        assert!(parse_kind_name("commands").is_none());
    }

    // ── resolve_source ────────────────────────────────────────────

    #[test]
    fn resolve_inline_uses_provided_host_repo() {
        let src = PluginSource::Inline("./plugins/foo".to_string());
        let r = resolve_source(&src, Some("myorg/myrepo")).unwrap();
        assert_eq!(r.repo, "myorg/myrepo");
        assert_eq!(r.subdir, "plugins/foo");
        assert_eq!(r.git_ref, DEFAULT_REF);
    }

    #[test]
    fn resolve_inline_falls_back_to_marketplace_repo_when_no_host() {
        let src = PluginSource::Inline("./plugins/bar".to_string());
        let r = resolve_source(&src, None).unwrap();
        assert_eq!(r.repo, MARKETPLACE_REPO);
    }

    #[test]
    fn resolve_inline_strips_leading_slash() {
        let src = PluginSource::Inline("/plugins/baz".to_string());
        let r = resolve_source(&src, None).unwrap();
        assert_eq!(r.subdir, "plugins/baz");
    }

    #[test]
    fn resolve_github_object_no_commit() {
        let src = PluginSource::Object(PluginSourceObject::Github {
            repo: "owner/plugin".to_string(),
            commit: None,
        });
        let r = resolve_source(&src, None).unwrap();
        assert_eq!(r.repo, "owner/plugin");
        assert_eq!(r.git_ref, DEFAULT_REF);
        assert!(r.subdir.is_empty());
    }

    #[test]
    fn resolve_github_object_with_commit() {
        let src = PluginSource::Object(PluginSourceObject::Github {
            repo: "owner/plugin".to_string(),
            commit: Some("abc123".to_string()),
        });
        let r = resolve_source(&src, None).unwrap();
        assert_eq!(r.git_ref, "abc123");
    }

    #[test]
    fn resolve_url_object_with_sha() {
        let src = PluginSource::Object(PluginSourceObject::Url {
            url: "https://github.com/owner/repo".to_string(),
            sha: Some("deadbeef".to_string()),
        });
        let r = resolve_source(&src, None).unwrap();
        assert_eq!(r.repo, "owner/repo");
        assert_eq!(r.git_ref, "deadbeef");
    }

    #[test]
    fn resolve_git_subdir_prefers_sha_over_git_ref() {
        let src = PluginSource::Object(PluginSourceObject::GitSubdir {
            url: "https://github.com/owner/mono".to_string(),
            path: "packages/plugin-a".to_string(),
            git_ref: Some("develop".to_string()),
            sha: Some("sha999".to_string()),
            branch: None,
        });
        let r = resolve_source(&src, None).unwrap();
        assert_eq!(r.git_ref, "sha999");
        assert_eq!(r.subdir, "packages/plugin-a");
    }

    #[test]
    fn resolve_git_subdir_falls_back_to_git_ref() {
        let src = PluginSource::Object(PluginSourceObject::GitSubdir {
            url: "https://github.com/owner/mono".to_string(),
            path: "packages/plugin-b".to_string(),
            git_ref: Some("stable".to_string()),
            sha: None,
            branch: None,
        });
        let r = resolve_source(&src, None).unwrap();
        assert_eq!(r.git_ref, "stable");
    }

    // ── tarball_url ───────────────────────────────────────────────

    #[test]
    fn tarball_url_format() {
        let r = ResolvedRef {
            repo: "owner/repo".to_string(),
            git_ref: "main".to_string(),
            subdir: String::new(),
        };
        assert_eq!(
            tarball_url(&r),
            "https://codeload.github.com/owner/repo/tar.gz/main"
        );
    }

    #[test]
    fn tarball_url_with_sha() {
        let r = ResolvedRef {
            repo: "owner/plugin".to_string(),
            git_ref: "abc1234".to_string(),
            subdir: String::new(),
        };
        assert_eq!(
            tarball_url(&r),
            "https://codeload.github.com/owner/plugin/tar.gz/abc1234"
        );
    }

    // ── readme_urls ───────────────────────────────────────────────

    #[test]
    fn readme_urls_with_subdir_returns_four() {
        let r = ResolvedRef {
            repo: "owner/repo".to_string(),
            git_ref: "main".to_string(),
            subdir: "plugins/foo".to_string(),
        };
        let urls = readme_urls(&r);
        assert_eq!(urls.len(), 4);
        assert!(urls[0].contains("plugins/foo/README.md"));
        assert!(urls[1].contains("plugins/foo/readme.md"));
        assert!(urls[2].ends_with("/README.md"));
        assert!(urls[3].ends_with("/readme.md"));
    }

    #[test]
    fn readme_urls_without_subdir_returns_two() {
        let r = ResolvedRef {
            repo: "owner/repo".to_string(),
            git_ref: "main".to_string(),
            subdir: String::new(),
        };
        let urls = readme_urls(&r);
        assert_eq!(urls.len(), 2);
        assert!(urls[0].ends_with("/README.md"));
        assert!(urls[1].ends_with("/readme.md"));
    }

    #[test]
    fn readme_urls_base_contains_repo_and_ref() {
        let r = ResolvedRef {
            repo: "myorg/myplugin".to_string(),
            git_ref: "v2.0.0".to_string(),
            subdir: String::new(),
        };
        let urls = readme_urls(&r);
        assert!(urls[0].contains("myorg/myplugin"));
        assert!(urls[0].contains("v2.0.0"));
    }

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
            // Origin keys mirror the bare asset name used by scan_kind, so
            // command/agent entries strip their `.md` suffix here too.
            let (kind, name) = parse_kind_name(entry).expect("parse entry");
            let key = format!("{}:{}", kind.as_str(), name);
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
