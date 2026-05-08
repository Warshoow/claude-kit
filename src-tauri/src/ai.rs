use crate::library::AssetKind;
use crate::settings::{read_settings, AiMode};
use anyhow::{anyhow, Result};
use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

/// Snapshot of what AI backend is currently usable. Surfaced to the
/// frontend so we can show the right UI (e.g. disable Generate buttons
/// when nothing is configured, or hint at fixing the API key).
#[derive(Debug, Clone, Serialize)]
pub struct AiStatus {
    /// Effective mode given current settings + detection — one of
    /// "claude-cli", "api", or "none".
    pub mode: String,
    /// Resolved path to the `claude` binary if we found one.
    pub claude_cli_path: Option<String>,
    /// True iff `api_base_url` and `api_key` are both set in settings.
    pub api_configured: bool,
    /// User-facing one-liner explaining the state.
    pub message: String,
}

/// Locate the `claude` CLI. We try the binary directly via `which`,
/// then fall back to `~/.claude/local/claude` which is where the
/// official installer puts it on Linux/macOS. Returning the absolute
/// path matters because we hand it to `Command::new` later.
pub fn detect_claude_cli() -> Option<PathBuf> {
    // 1. Standard PATH lookup (works on Linux + macOS via `which`,
    //    Windows ships `where`). Fall through silently on failure.
    let lookup = if cfg!(target_os = "windows") { "where" } else { "which" };
    if let Ok(out) = Command::new(lookup).arg("claude").output() {
        if out.status.success() {
            let path = String::from_utf8_lossy(&out.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }

    // 2. Fall back to the per-user installer location.
    if let Some(home) = dirs::home_dir() {
        for candidate in [
            home.join(".claude/local/claude"),
            home.join(".claude/local/claude.exe"),
        ] {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

pub fn ai_status() -> AiStatus {
    let s = read_settings().ai;
    let cli = detect_claude_cli();
    let api_configured =
        s.api_base_url.as_deref().map(|x| !x.is_empty()).unwrap_or(false)
        && s.api_key.as_deref().map(|x| !x.is_empty()).unwrap_or(false);

    let (mode, message) = match s.mode {
        AiMode::ClaudeCli => match &cli {
            Some(p) => (
                "claude-cli",
                format!("Forced to Claude CLI ({}).", p.display()),
            ),
            None => (
                "none",
                "Mode set to Claude CLI but the `claude` binary wasn't found.".to_string(),
            ),
        },
        AiMode::Api => {
            if api_configured {
                ("api", format!(
                    "Forced to API mode ({}).",
                    s.api_base_url.clone().unwrap_or_default()
                ))
            } else {
                ("none", "Mode set to API but no base URL or key configured.".to_string())
            }
        }
        AiMode::Auto => match (&cli, api_configured) {
            (Some(p), _) => (
                "claude-cli",
                format!("Using Claude Code CLI ({}).", p.display()),
            ),
            (None, true) => (
                "api",
                format!(
                    "Using configured API ({}).",
                    s.api_base_url.clone().unwrap_or_default()
                ),
            ),
            (None, false) => (
                "none",
                "No AI backend available. Install Claude Code or configure an API in Settings.".to_string(),
            ),
        },
    };

    AiStatus {
        mode: mode.to_string(),
        claude_cli_path: cli.map(|p| p.to_string_lossy().to_string()),
        api_configured,
        message,
    }
}

pub fn generate_asset(
    kind: AssetKind,
    prompt: &str,
    context: Option<&str>,
) -> Result<String> {
    if prompt.trim().is_empty() {
        return Err(anyhow!("prompt is empty"));
    }
    let status = ai_status();
    match status.mode.as_str() {
        "claude-cli" => generate_via_cli(kind, prompt, context),
        "api" => generate_via_api(kind, prompt, context),
        _ => Err(anyhow!(
            "No AI backend available. Configure one in Settings."
        )),
    }
}

fn build_system_prompt(kind: AssetKind) -> String {
    let common = "You are writing a Claude Code asset file. Output ONLY the raw file contents — no commentary, no explanation, no surrounding code fences. The file MUST start with YAML frontmatter delimited by `---` lines, containing at minimum a single-sentence `description:` field. Tags are optional. After the closing `---`, write the markdown body that defines the asset.";
    match kind {
        AssetKind::Skills => format!(
            "{common}\n\nThis is a SKILL.md. A skill activates automatically when its description matches the user's current context, so be specific about WHEN it should trigger. Document the steps Claude should follow clearly, in markdown."
        ),
        AssetKind::Commands => format!(
            "{common}\n\nThis is a slash-command file. The user invokes it explicitly as /<name>. Describe what the command does and any arguments it accepts. Use `$1`, `$2`… for positional args."
        ),
        AssetKind::Agents => format!(
            "{common}\n\nThis is a sub-agent definition. Describe the agent's specialty, the kinds of tasks it should handle, and any tools or constraints it should respect."
        ),
    }
}

fn build_user_prompt(prompt: &str, context: Option<&str>) -> String {
    match context {
        Some(c) if !c.trim().is_empty() => format!(
            "Existing content:\n```\n{}\n```\n\nTask: {prompt}",
            c.trim()
        ),
        _ => prompt.to_string(),
    }
}

/// Some models still wrap their output in a markdown code fence even when
/// asked not to. Strip a single leading/trailing fence if present.
fn strip_code_fences(s: &str) -> String {
    let trimmed = s.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        let body = rest.split_once('\n').map(|(_, b)| b).unwrap_or(rest);
        if let Some(stripped) = body.trim_end().strip_suffix("```") {
            return stripped.trim_end().to_string();
        }
    }
    trimmed.to_string()
}

fn generate_via_cli(
    kind: AssetKind,
    prompt: &str,
    context: Option<&str>,
) -> Result<String> {
    let cli = detect_claude_cli().ok_or_else(|| anyhow!("claude CLI not found on PATH"))?;
    let system = build_system_prompt(kind);
    let user = build_user_prompt(prompt, context);

    // `claude -p "<user>" --append-system-prompt "<system>" --output-format json`
    // We use --append-system-prompt rather than --system-prompt so we don't
    // wipe Claude Code's default behaviour entirely.
    let output = Command::new(&cli)
        .arg("-p")
        .arg(&user)
        .arg("--append-system-prompt")
        .arg(&system)
        .arg("--output-format")
        .arg("json")
        .output()
        .map_err(|e| anyhow!("spawn claude CLI: {e}"))?;

    if !output.status.success() {
        return Err(anyhow!(
            "claude CLI failed (exit {}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow!("claude CLI output isn't JSON: {e}\n{stdout}"))?;
    let content = parsed
        .get("result")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("claude CLI JSON response missing `result` field"))?
        .to_string();

    Ok(strip_code_fences(&content))
}

fn generate_via_api(
    kind: AssetKind,
    prompt: &str,
    context: Option<&str>,
) -> Result<String> {
    let s = read_settings().ai;
    let base = s
        .api_base_url
        .as_ref()
        .filter(|x| !x.is_empty())
        .ok_or_else(|| anyhow!("API base URL not configured"))?
        .trim_end_matches('/')
        .to_string();
    let key = s
        .api_key
        .as_ref()
        .filter(|x| !x.is_empty())
        .ok_or_else(|| anyhow!("API key not configured"))?
        .clone();
    let model = s
        .api_model
        .as_deref()
        .filter(|x| !x.is_empty())
        .unwrap_or("claude-sonnet-4-5")
        .to_string();

    let system = build_system_prompt(kind);
    let user = build_user_prompt(prompt, context);

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .user_agent("claude-kit/0.1")
        .build()?;

    let url = format!("{base}/chat/completions");
    let body = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user",   "content": user   },
        ],
    });

    let resp = client
        .post(&url)
        .bearer_auth(&key)
        .json(&body)
        .send()?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().unwrap_or_default();
        return Err(anyhow!("API call failed ({status}): {text}"));
    }

    let parsed: serde_json::Value = resp.json()?;
    let content = parsed["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow!("API response missing choices[0].message.content"))?
        .to_string();

    Ok(strip_code_fences(&content))
}
