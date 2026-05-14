use crate::library::AssetKind;
use crate::settings::{read_settings, AiMode};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

// ── Windows-specific subprocess helpers ────────────────────────────
//
// Tauri release binaries don't have a console parent on Windows, so
// any subprocess we spawn would otherwise pop a fresh `cmd.exe` window
// — visible as a flicker every time we shell out. CREATE_NO_WINDOW is
// the documented way to suppress it.

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(target_os = "windows")]
fn no_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(target_os = "windows"))]
fn no_window(_cmd: &mut Command) {}

/// Build a `Command` to invoke the resolved `claude` binary. On Windows
/// the npm-installed CLI lands as a `.cmd` shim (a batch wrapper around
/// node), and `Command::new("foo.cmd")` can't execute it directly —
/// CreateProcess returns ERROR_FILENAME_EXCED_RANGE (206). The fix is to
/// route through `cmd /C` for those extensions; native `.exe` paths and
/// Linux/macOS binaries work as-is.
fn build_cli_command(cli: &Path) -> Command {
    #[cfg(target_os = "windows")]
    {
        let needs_cmd_wrapper = cli
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("cmd") || e.eq_ignore_ascii_case("bat"))
            .unwrap_or(false);
        if needs_cmd_wrapper {
            let mut cmd = Command::new("cmd");
            cmd.arg("/C").arg(cli);
            no_window(&mut cmd);
            return cmd;
        }
    }
    let mut cmd = Command::new(cli);
    no_window(&mut cmd);
    cmd
}

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
    //    `where` returns the *first* match on its own line, which is
    //    usually the npm shim — exactly what we want.
    let lookup = if cfg!(target_os = "windows") { "where" } else { "which" };
    let mut lookup_cmd = Command::new(lookup);
    lookup_cmd.arg("claude");
    no_window(&mut lookup_cmd);
    if let Ok(out) = lookup_cmd.output() {
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

// ── Streaming primitives ──────────────────────────────────────────
//
// Used by the refine-chat flow. Token events are forwarded to the
// frontend live (via a Tauri Channel) so the user sees the response
// materialize while it's being generated. The final `done` event
// carries the cleaned full content; auto-apply happens client-side.

/// One message in the refine-chat history. `role` is "user" or
/// "assistant"; assistant turns hold the *full asset content* the
/// model produced last time (we don't currently store an explainer).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Sent over the Tauri Channel to the frontend. Tagged enum so the TS
/// side gets a clean discriminated union.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", content = "data", rename_all = "lowercase")]
pub enum StreamEvent {
    Token { delta: String },
    Done { content: String },
    Error { message: String },
}

fn build_refine_system_prompt(kind: AssetKind) -> String {
    let kind_label = match kind {
        AssetKind::Skills => "Claude Code skill (SKILL.md)",
        AssetKind::Commands => "Claude Code slash-command",
        AssetKind::Agents => "Claude Code sub-agent definition",
    };
    format!(
        "You are refining a {kind_label} via conversation with the user. \
         Each user message asks for changes to the asset. You MUST output \
         ONLY the new full file contents — frontmatter (YAML between `---` \
         lines) plus markdown body. No commentary, no explanation, no \
         surrounding code fences. The file must keep a valid `description:` \
         field in its frontmatter. Preserve any content the user didn't \
         ask to change."
    )
}

fn build_refine_user_prompt(
    current_content: &str,
    history: &[ChatMessage],
    user_message: &str,
) -> String {
    let mut out = String::new();
    out.push_str("Current asset content:\n```\n");
    out.push_str(current_content.trim());
    out.push_str("\n```\n\n");

    if !history.is_empty() {
        out.push_str("Conversation so far (each assistant turn was a \
                      previous version of the asset):\n\n");
        for msg in history {
            let role = if msg.role == "assistant" { "ASSISTANT" } else { "USER" };
            out.push_str(role);
            out.push_str(":\n");
            out.push_str(msg.content.trim());
            out.push_str("\n\n");
        }
    }

    out.push_str("USER (latest request):\n");
    out.push_str(user_message.trim());
    out.push_str("\n\nOutput the new full asset content now.");
    out
}

/// Drive a streaming refine turn. Spawns a worker thread so the
/// caller (a Tauri command) returns immediately; the channel carries
/// `token` events as the model generates, then `done` (or `error`).
pub fn refine_asset_chat(
    on_event: tauri::ipc::Channel<StreamEvent>,
    kind: AssetKind,
    current_content: String,
    history: Vec<ChatMessage>,
    user_message: String,
) -> Result<()> {
    if user_message.trim().is_empty() {
        return Err(anyhow!("message is empty"));
    }
    let system = build_refine_system_prompt(kind);
    let user = build_refine_user_prompt(&current_content, &history, &user_message);

    let chan_token = on_event.clone();
    std::thread::spawn(move || {
        let result = stream_text(&system, &user, |delta| {
            let _ = chan_token.send(StreamEvent::Token {
                delta: delta.to_string(),
            });
        });

        match result {
            Ok(full) => {
                let cleaned = strip_code_fences(&full);
                let _ = on_event.send(StreamEvent::Done { content: cleaned });
            }
            Err(e) => {
                let _ = on_event.send(StreamEvent::Error {
                    message: e.to_string(),
                });
            }
        }
    });

    Ok(())
}

/// Dispatch a streaming text generation. Calls `emit` once per delta
/// chunk and returns the full accumulated string at the end.
pub(crate) fn stream_text(
    system: &str,
    user: &str,
    mut emit: impl FnMut(&str),
) -> Result<String> {
    let status = ai_status();
    match status.mode.as_str() {
        "claude-cli" => stream_via_cli(system, user, &mut emit),
        "api" => stream_via_api(system, user, &mut emit),
        _ => Err(anyhow!(
            "No AI backend available. Configure one in Settings."
        )),
    }
}

fn stream_via_cli(
    system: &str,
    user: &str,
    emit: &mut dyn FnMut(&str),
) -> Result<String> {
    let cli = detect_claude_cli().ok_or_else(|| anyhow!("claude CLI not found on PATH"))?;

    // `--output-format text` streams the response straight to stdout —
    // no JSON envelope to parse, no buffering until the end. Stdin still
    // carries the user prompt (Windows command-line length limits make
    // the arg form risky for prompts that pack the bundle/history).
    let mut cmd = build_cli_command(&cli);
    cmd.arg("-p")
        .arg("--append-system-prompt")
        .arg(system)
        .arg("--output-format")
        .arg("text")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| anyhow!("spawn claude CLI: {e}"))?;

    let user_owned = user.to_string();
    if let Some(mut stdin) = child.stdin.take() {
        std::thread::spawn(move || {
            let _ = stdin.write_all(user_owned.as_bytes());
        });
    }

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("CLI stdout not available"))?;
    let mut reader = BufReader::new(stdout);
    let mut accumulated = String::new();
    let mut buf = [0u8; 256];

    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let chunk = String::from_utf8_lossy(&buf[..n]).to_string();
                accumulated.push_str(&chunk);
                emit(&chunk);
            }
            Err(e) => return Err(anyhow!("read CLI stdout: {e}")),
        }
    }

    let status = child
        .wait()
        .map_err(|e| anyhow!("wait on claude CLI: {e}"))?;
    if !status.success() {
        let mut stderr = String::new();
        if let Some(mut e) = child.stderr.take() {
            e.read_to_string(&mut stderr).ok();
        }
        return Err(anyhow!(
            "claude CLI failed (exit {status}): {}",
            stderr.trim()
        ));
    }

    Ok(accumulated)
}

fn stream_via_api(
    system: &str,
    user: &str,
    emit: &mut dyn FnMut(&str),
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

    // Streaming requests can take a while; bump the read timeout well
    // above the blocking call's 120s. The connect timeout stays short.
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(300))
        .user_agent("claude-kit/0.1")
        .build()?;

    let url = format!("{base}/chat/completions");
    let body = serde_json::json!({
        "model": model,
        "stream": true,
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

    // SSE framing: each event is `data: <payload>\n\n`. The OpenAI-style
    // protocol terminates with `data: [DONE]`. We read line-by-line and
    // ignore everything that isn't a `data:` line (keep-alives, comments).
    let mut reader = BufReader::new(resp);
    let mut accumulated = String::new();
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .map_err(|e| anyhow!("read SSE stream: {e}"))?;
        if n == 0 {
            break;
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        let payload = match trimmed.strip_prefix("data: ") {
            Some(p) => p,
            None => continue,
        };
        if payload == "[DONE]" {
            break;
        }
        let v: serde_json::Value = match serde_json::from_str(payload) {
            Ok(v) => v,
            Err(_) => continue,
        };
        // Standard OpenAI/Anthropic-compat shape: choices[0].delta.content
        if let Some(delta) = v["choices"][0]["delta"]["content"].as_str() {
            if !delta.is_empty() {
                accumulated.push_str(delta);
                emit(delta);
            }
        }
    }

    Ok(accumulated)
}

pub fn generate_asset(
    kind: AssetKind,
    prompt: &str,
    context: Option<&str>,
) -> Result<String> {
    if prompt.trim().is_empty() {
        return Err(anyhow!("prompt is empty"));
    }
    let system = build_system_prompt(kind);
    let user = build_user_prompt(prompt, context);
    let raw = generate_text(&system, &user)?;
    Ok(strip_code_fences(&raw))
}

/// Lower-level entry point used by the asset generator AND the bundle
/// harmonizer. Picks the configured backend and returns the raw model
/// output. Callers post-process (strip code fences, parse delimiters…)
/// according to what they expect.
pub fn generate_text(system: &str, user: &str) -> Result<String> {
    let status = ai_status();
    match status.mode.as_str() {
        "claude-cli" => generate_via_cli_raw(system, user),
        "api" => generate_via_api_raw(system, user),
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
pub(crate) fn strip_code_fences(s: &str) -> String {
    let trimmed = s.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        let body = rest.split_once('\n').map(|(_, b)| b).unwrap_or(rest);
        if let Some(stripped) = body.trim_end().strip_suffix("```") {
            return stripped.trim_end().to_string();
        }
    }
    trimmed.to_string()
}

fn generate_via_cli_raw(system: &str, user: &str) -> Result<String> {
    let cli = detect_claude_cli().ok_or_else(|| anyhow!("claude CLI not found on PATH"))?;

    // `claude -p --append-system-prompt "<system>" --output-format json` with
    // the user prompt piped in on stdin. The harmonizer and recommender flows
    // both produce user prompts in the 5-50 KB range (full bundle contents,
    // entire marketplace catalog) — putting that on the command line blows
    // past Windows' CreateProcess limit (32 767 chars, 8 191 when going
    // through cmd.exe for .cmd shims) and surfaces as the misleading
    // "filename or extension too long" error 206. Stdin keeps the command
    // line tiny regardless of how big the prompt grows.
    //
    // `build_cli_command` routes through `cmd /C` on Windows when the
    // resolved path is a `.cmd` shim (npm-installed CLIs), and suppresses
    // the otherwise-flashing console window on every spawn. We also use
    // --append-system-prompt rather than --system-prompt so we don't wipe
    // Claude Code's default behaviour entirely.
    let mut cmd = build_cli_command(&cli);
    cmd.arg("-p")
        .arg("--append-system-prompt")
        .arg(system)
        .arg("--output-format")
        .arg("json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| anyhow!("spawn claude CLI: {e}"))?;

    // Feed the user prompt via stdin on a worker thread so we can read
    // stdout in parallel — claude streams its response as it computes,
    // and a serial write-then-read could deadlock on a large prompt that
    // doesn't fit in the OS pipe buffer (typically 64 KB).
    let user_owned = user.to_string();
    if let Some(mut stdin) = child.stdin.take() {
        std::thread::spawn(move || {
            let _ = stdin.write_all(user_owned.as_bytes());
            // Dropping stdin closes the pipe so claude knows EOF.
        });
    }

    let output = child
        .wait_with_output()
        .map_err(|e| anyhow!("wait on claude CLI: {e}"))?;

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

    Ok(content)
}

fn generate_via_api_raw(system: &str, user: &str) -> Result<String> {
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

    Ok(content)
}
