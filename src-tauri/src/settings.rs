use crate::library::kit_home;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Display name + URL of the official marketplace. Always present in
/// the settings list, even on a fresh install.
pub const OFFICIAL_MARKETPLACE_NAME: &str = "claude-plugins-official";
pub const OFFICIAL_MARKETPLACE_URL: &str =
    "https://raw.githubusercontent.com/anthropics/claude-plugins-official/main/.claude-plugin/marketplace.json";

/// On-disk app settings, persisted at `~/.claude-assets/settings.json`.
/// Adding a new field requires `#[serde(default)]` so existing files
/// keep parsing after upgrades.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub ai: AiSettings,
    #[serde(default)]
    pub marketplaces: Vec<MarketplaceSource>,
}

/// A marketplace the user has registered. The `name` is whatever the
/// `marketplace.json`'s top-level `name` field said at registration
/// time — used as the stable identifier (also written into asset
/// origins).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketplaceSource {
    pub name: String,
    pub url: String,
    /// True for the official entry — refuses removal so the user
    /// can't accidentally lose access to the canonical catalog.
    #[serde(default)]
    pub builtin: bool,
}

impl MarketplaceSource {
    pub fn official() -> Self {
        Self {
            name: OFFICIAL_MARKETPLACE_NAME.to_string(),
            url: OFFICIAL_MARKETPLACE_URL.to_string(),
            builtin: true,
        }
    }
}

/// AI integration config. `mode = "auto"` is the default — the backend
/// picks `claude-cli` if the binary is detected on PATH, otherwise falls
/// back to `api` if an api_key is set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSettings {
    /// "auto" | "claude-cli" | "api"
    pub mode: AiMode,

    /// Optional override for the Claude CLI binary path. When set and the
    /// file exists, it takes precedence over auto-detection. Useful when
    /// `claude` isn't on PATH (e.g. installed to a non-standard location).
    #[serde(default)]
    pub claude_cli_path: Option<String>,

    /// OpenAI-compatible base URL — e.g. `https://api.openai.com/v1`,
    /// `https://api.anthropic.com/v1` (Anthropic's OpenAI-compat endpoint),
    /// `http://localhost:11434/v1` for Ollama, etc.
    pub api_base_url: Option<String>,

    /// Stored as plain text. The settings file lives in the user's home
    /// directory alongside other secrets they manage themselves; we don't
    /// pretend to encrypt it.
    pub api_key: Option<String>,

    /// Model id to send in the chat-completions payload. If `None`, falls
    /// back to a sensible default per provider.
    pub api_model: Option<String>,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            mode: AiMode::Auto,
            claude_cli_path: None,
            api_base_url: None,
            api_key: None,
            api_model: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiMode {
    Auto,
    ClaudeCli,
    Api,
}

pub fn settings_path() -> PathBuf {
    kit_home().join("settings.json")
}

pub fn read_settings() -> Settings {
    let p = settings_path();
    let mut s: Settings = match fs::read_to_string(&p) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => Settings::default(),
    };

    // Migration / invariant: the official marketplace must always be
    // present, marked as builtin. Old configs predating this field get
    // it injected on first read; manual edits that drop or downgrade
    // it get healed transparently.
    let official = MarketplaceSource::official();
    match s.marketplaces.iter_mut().find(|m| m.url == official.url) {
        Some(existing) => {
            existing.builtin = true;
            existing.name = official.name.clone();
        }
        None => s.marketplaces.insert(0, official),
    }
    s
}

pub fn write_settings(settings: &Settings) -> Result<()> {
    let p = settings_path();
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    let s = serde_json::to_string_pretty(settings)
        .map_err(|e| anyhow!("serialize settings: {e}"))?;
    fs::write(&p, s).map_err(|e| anyhow!("write {}: {e}", p.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_temp_home<F: FnOnce()>(f: F) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let _guard = crate::HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("CLAUDE_KIT_HOME", tmp.path());
        f();
        std::env::remove_var("CLAUDE_KIT_HOME");
    }

    // ── MarketplaceSource::official ───────────────────────────────

    #[test]
    fn official_source_fields() {
        let o = MarketplaceSource::official();
        assert_eq!(o.name, OFFICIAL_MARKETPLACE_NAME);
        assert_eq!(o.url, OFFICIAL_MARKETPLACE_URL);
        assert!(o.builtin);
    }

    // ── AiSettings default ────────────────────────────────────────

    #[test]
    fn ai_settings_default_is_auto() {
        let a = AiSettings::default();
        assert_eq!(a.mode, AiMode::Auto);
        assert!(a.api_key.is_none());
        assert!(a.api_base_url.is_none());
        assert!(a.api_model.is_none());
    }

    // ── AiMode serialization ──────────────────────────────────────

    #[test]
    fn ai_mode_serializes_kebab_case() {
        assert_eq!(serde_json::to_string(&AiMode::Auto).unwrap(), "\"auto\"");
        assert_eq!(serde_json::to_string(&AiMode::ClaudeCli).unwrap(), "\"claude-cli\"");
        assert_eq!(serde_json::to_string(&AiMode::Api).unwrap(), "\"api\"");
    }

    #[test]
    fn ai_mode_deserializes_kebab_case() {
        assert_eq!(serde_json::from_str::<AiMode>("\"auto\"").unwrap(), AiMode::Auto);
        assert_eq!(serde_json::from_str::<AiMode>("\"claude-cli\"").unwrap(), AiMode::ClaudeCli);
        assert_eq!(serde_json::from_str::<AiMode>("\"api\"").unwrap(), AiMode::Api);
    }

    // ── read_settings — fresh install ─────────────────────────────

    #[test]
    fn read_settings_no_file_returns_default_with_official() {
        with_temp_home(|| {
            let s = read_settings();
            assert_eq!(s.marketplaces.len(), 1);
            assert_eq!(s.marketplaces[0].name, OFFICIAL_MARKETPLACE_NAME);
            assert!(s.marketplaces[0].builtin);
            assert_eq!(s.ai.mode, AiMode::Auto);
        });
    }

    // ── read_settings — malformed JSON falls back to default ──────

    #[test]
    fn read_settings_malformed_json_falls_back_to_default() {
        with_temp_home(|| {
            let p = settings_path();
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, "{ this is not valid json").unwrap();
            let s = read_settings();
            assert_eq!(s.marketplaces.len(), 1);
            assert!(s.marketplaces[0].builtin);
        });
    }

    // ── read_settings — migration: inject official if missing ─────

    #[test]
    fn read_settings_injects_official_when_missing() {
        with_temp_home(|| {
            let mut base = Settings::default();
            base.marketplaces.clear(); // remove the official entry
            base.marketplaces.push(MarketplaceSource {
                name: "other".to_string(),
                url: "https://example.com/mp.json".to_string(),
                builtin: false,
            });
            write_settings(&base).unwrap();

            let s = read_settings();
            // Official should be inserted at position 0.
            assert_eq!(s.marketplaces[0].name, OFFICIAL_MARKETPLACE_NAME);
            assert!(s.marketplaces[0].builtin);
            assert_eq!(s.marketplaces.len(), 2);
        });
    }

    // ── read_settings — migration: heal builtin=false ─────────────

    #[test]
    fn read_settings_heals_builtin_false_on_official_entry() {
        with_temp_home(|| {
            // Write a config where the official URL is present but builtin=false.
            let raw = format!(
                r#"{{"ai":{{"mode":"auto"}},"marketplaces":[{{"name":"{name}","url":"{url}","builtin":false}}]}}"#,
                name = OFFICIAL_MARKETPLACE_NAME,
                url = OFFICIAL_MARKETPLACE_URL,
            );
            let p = settings_path();
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, raw).unwrap();

            let s = read_settings();
            assert!(s.marketplaces[0].builtin, "builtin should be healed to true");
        });
    }

    // ── write_settings + read_settings roundtrip ──────────────────

    #[test]
    fn write_then_read_preserves_ai_settings() {
        with_temp_home(|| {
            let mut s = Settings::default();
            s.ai.mode = AiMode::Api;
            s.ai.api_key = Some("test-key".to_string());
            s.ai.api_model = Some("gpt-4o".to_string());
            s.ai.api_base_url = Some("https://api.openai.com/v1".to_string());
            write_settings(&s).unwrap();

            let loaded = read_settings();
            assert_eq!(loaded.ai.mode, AiMode::Api);
            assert_eq!(loaded.ai.api_key.as_deref(), Some("test-key"));
            assert_eq!(loaded.ai.api_model.as_deref(), Some("gpt-4o"));
            assert_eq!(
                loaded.ai.api_base_url.as_deref(),
                Some("https://api.openai.com/v1")
            );
        });
    }

    #[test]
    fn write_then_read_preserves_extra_marketplaces() {
        with_temp_home(|| {
            let mut s = read_settings();
            s.marketplaces.push(MarketplaceSource {
                name: "community".to_string(),
                url: "https://community.example.com/mp.json".to_string(),
                builtin: false,
            });
            write_settings(&s).unwrap();

            let loaded = read_settings();
            assert_eq!(loaded.marketplaces.len(), 2);
            assert!(loaded.marketplaces.iter().any(|m| m.name == "community"));
        });
    }

    #[test]
    fn settings_file_is_pretty_printed_json() {
        with_temp_home(|| {
            let s = read_settings();
            write_settings(&s).unwrap();
            let raw = fs::read_to_string(settings_path()).unwrap();
            assert!(raw.contains('\n'), "should be pretty-printed");
        });
    }
}
