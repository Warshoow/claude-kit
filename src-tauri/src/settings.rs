use crate::library::kit_home;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// On-disk app settings, persisted at `~/.claude-assets/settings.json`.
/// Adding a new field requires `#[serde(default)]` so existing files
/// keep parsing after upgrades.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub ai: AiSettings,
}

/// AI integration config. `mode = "auto"` is the default — the backend
/// picks `claude-cli` if the binary is detected on PATH, otherwise falls
/// back to `api` if an api_key is set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSettings {
    /// "auto" | "claude-cli" | "api"
    pub mode: AiMode,

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
    let Ok(s) = fs::read_to_string(&p) else { return Settings::default() };
    serde_json::from_str(&s).unwrap_or_default()
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
