//! Bundle-chat: generate a coherent group of assets from a chat
//! conversation. Each user turn produces a complete bundle (every
//! asset, including unchanged ones from prior turns) so the parser
//! always has a self-contained answer to materialize on disk.
//!
//! Wire format the model has to emit:
//!
//! ```text
//! BUNDLE-META:
//! name: <kebab-case-slug>
//! description: <one-line description>
//! END-BUNDLE-META
//!
//! <<<ASSET-BEGIN: <kind>/<name>>>>
//! ---
//! description: <one-line>
//! ---
//! <markdown body>
//! <<<ASSET-END>>>
//! ```
//!
//! Tokens stream live to the frontend; the parsed result is emitted as
//! a single `done` event so the UI can update the assets list in one go.

use crate::ai::{self, ChatMessage};
use crate::bundles::{self, BundleRef};
use crate::library::{self, AssetKind};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedAsset {
    pub kind: AssetKind,
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", content = "data", rename_all = "lowercase")]
pub enum BundleChatEvent {
    /// One delta chunk from the model — raw text. Frontend just
    /// accumulates these in a live preview during generation.
    Token { delta: String },
    /// Stream completed cleanly. Carries the parsed result.
    Done {
        assets: Vec<GeneratedAsset>,
        bundle_name: Option<String>,
        bundle_description: Option<String>,
    },
    /// Stream failed (backend error or parser couldn't find any asset
    /// blocks). Frontend rolls back the optimistic user message.
    Error { message: String },
}

const SYSTEM_PROMPT: &str = "\
You design coherent groups of Claude Code assets — a \"bundle\". A bundle is a small set of related assets that work together for a specific developer workflow (e.g. \"Python backend with pytest\", \"React + Tailwind frontend\", \"Rust embedded firmware\").\n\
\n\
Asset kinds you may produce:\n\
- skills: auto-activate based on file/context. Use when the user is working on something specific. The skill's description field is what triggers it.\n\
- commands: invoked manually by the user with /<name>. Use for explicit short tasks.\n\
- agents: sub-agents specialized for one kind of task. Use sparingly.\n\
\n\
EXACT output format — emit ONLY this, no commentary, no preamble:\n\
\n\
BUNDLE-META:\n\
name: <kebab-case-slug>\n\
description: <one-sentence bundle description>\n\
END-BUNDLE-META\n\
\n\
<<<ASSET-BEGIN: <kind>/<asset-name>>>>\n\
---\n\
description: <one-sentence description that explains when this asset activates / what it does>\n\
---\n\
<markdown body>\n\
<<<ASSET-END>>>\n\
\n\
Repeat the asset block for every asset in the bundle. <kind> MUST be one of: skills, commands, agents. <asset-name> MUST be a kebab-case slug, 1-64 chars, [a-z0-9-_] only.\n\
\n\
Rules:\n\
- Output the COMPLETE bundle every turn, including unchanged assets from previous turns — the parser only reads the latest message.\n\
- Keep bundles small and coherent: 3 to 8 assets is typical. Don't pad.\n\
- Each asset's frontmatter MUST have a valid one-sentence description.\n\
- Asset names must be unique within a single bundle. Pick distinct slugs.\n\
- No fences around the whole output. No `\\`\\`\\`` blocks.\n\
- If the user asks something that isn't a bundle request, still output a minimal bundle answering as best you can.\n";

fn build_user_prompt(history: &[ChatMessage], user_message: &str) -> String {
    let mut out = String::new();
    if history.is_empty() {
        out.push_str("USER (initial bundle request):\n");
        out.push_str(user_message.trim());
        out.push_str("\n\nOutput the bundle now.");
    } else {
        out.push_str(
            "Conversation so far. Each ASSISTANT turn was the full bundle output \
             produced at that step. Treat the latest USER request as a refinement \
             to be merged into a NEW complete bundle.\n\n",
        );
        for msg in history {
            let role = if msg.role == "assistant" {
                "ASSISTANT"
            } else {
                "USER"
            };
            out.push_str(role);
            out.push_str(":\n");
            out.push_str(msg.content.trim());
            out.push_str("\n\n");
        }
        out.push_str("USER (latest refinement):\n");
        out.push_str(user_message.trim());
        out.push_str("\n\nOutput the full new bundle now.");
    }
    out
}

pub fn generate_bundle_chat(
    on_event: tauri::ipc::Channel<BundleChatEvent>,
    history: Vec<ChatMessage>,
    user_message: String,
) -> Result<()> {
    if user_message.trim().is_empty() {
        return Err(anyhow!("message is empty"));
    }
    let user_prompt = build_user_prompt(&history, &user_message);

    let chan_token = on_event.clone();
    std::thread::spawn(move || {
        let result = ai::stream_text(SYSTEM_PROMPT, &user_prompt, |delta| {
            let _ = chan_token.send(BundleChatEvent::Token {
                delta: delta.to_string(),
            });
        });

        match result {
            Ok(full) => {
                let cleaned = ai::strip_code_fences(&full);
                let (assets, bundle_name, bundle_description) =
                    parse_bundle_response(&cleaned);
                if assets.is_empty() {
                    let _ = on_event.send(BundleChatEvent::Error {
                        message: "Model returned no parseable asset blocks. \
                                  Try rephrasing your request."
                            .to_string(),
                    });
                } else {
                    let _ = on_event.send(BundleChatEvent::Done {
                        assets,
                        bundle_name,
                        bundle_description,
                    });
                }
            }
            Err(e) => {
                let _ = on_event.send(BundleChatEvent::Error {
                    message: e.to_string(),
                });
            }
        }
    });

    Ok(())
}

/// Public for tests and main.rs to drive parsing directly if ever
/// needed. Best-effort: silently drops asset blocks with malformed
/// headers (unknown kind / invalid name) — the model is allowed to
/// be a bit sloppy without aborting the whole turn.
pub fn parse_bundle_response(raw: &str) -> (Vec<GeneratedAsset>, Option<String>, Option<String>) {
    let mut assets = Vec::new();
    let mut seen_names: std::collections::HashSet<(AssetKind, String)> =
        std::collections::HashSet::new();

    let mut bundle_name = None;
    let mut bundle_description = None;

    if let Some(meta_start) = raw.find("BUNDLE-META:") {
        let after = &raw[meta_start + "BUNDLE-META:".len()..];
        let meta_block_end = after.find("END-BUNDLE-META").unwrap_or(after.len());
        let meta_block = &after[..meta_block_end];
        for line in meta_block.lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("name:") {
                let v = v.trim();
                if !v.is_empty() {
                    bundle_name = Some(v.to_string());
                }
            } else if let Some(v) = line.strip_prefix("description:") {
                let v = v.trim();
                if !v.is_empty() {
                    bundle_description = Some(v.to_string());
                }
            }
        }
    }

    let begin_marker = "<<<ASSET-BEGIN:";
    let end_marker = "<<<ASSET-END>>>";

    let mut cursor = 0usize;
    while let Some(rel_begin) = raw[cursor..].find(begin_marker) {
        let abs_begin = cursor + rel_begin;
        let header_start = abs_begin + begin_marker.len();
        let header_close = match raw[header_start..].find(">>>") {
            Some(p) => header_start + p,
            None => break,
        };
        let header = raw[header_start..header_close].trim();
        let body_start = header_close + ">>>".len();

        let body_end_rel = match raw[body_start..].find(end_marker) {
            Some(p) => p,
            None => break,
        };
        let body_end = body_start + body_end_rel;
        let content = raw[body_start..body_end].trim().to_string();
        cursor = body_end + end_marker.len();

        let mut parts = header.splitn(2, '/');
        let kind_str = match parts.next() {
            Some(s) => s.trim(),
            None => continue,
        };
        let name = match parts.next() {
            Some(s) => s.trim(),
            None => continue,
        };

        let kind = match kind_str {
            "skills" => AssetKind::Skills,
            "commands" => AssetKind::Commands,
            "agents" => AssetKind::Agents,
            _ => continue,
        };
        if library::validate_asset_name(name).is_err() {
            continue;
        }
        if !seen_names.insert((kind, name.to_string())) {
            // Dedup: keep the first occurrence within this single
            // response. The model is supposed to emit unique slugs.
            continue;
        }
        if content.is_empty() {
            continue;
        }

        assets.push(GeneratedAsset {
            kind,
            name: name.to_string(),
            content,
        });
    }

    (assets, bundle_name, bundle_description)
}

#[derive(Debug, Clone, Serialize)]
pub struct MaterializeResult {
    pub bundle_name: String,
    pub asset_count: usize,
    pub created: Vec<BundleRef>,
}

/// Write every generated asset to the library, then create a bundle
/// pointing at them. Refuses any collision (existing asset file OR
/// existing bundle name) — the frontend is expected to surface this
/// and let the user rename or re-prompt. We deliberately don't try
/// to auto-rename here: the user's intent is much clearer at the UI
/// layer.
pub fn materialize_generated_bundle(
    bundle_name: String,
    bundle_description: Option<String>,
    assets: Vec<GeneratedAsset>,
) -> Result<MaterializeResult> {
    if assets.is_empty() {
        return Err(anyhow!("nothing to materialize: assets list is empty"));
    }

    // Validate the bundle name slug — same rules as asset names since
    // bundles also live as filenames on disk.
    let bundle_name = bundle_name.trim().to_string();
    library::validate_asset_name(&bundle_name)
        .map_err(|e| anyhow!("invalid bundle name: {e}"))?;
    if bundles::read_bundle(&bundle_name).is_some() {
        return Err(anyhow!(
            "bundle '{bundle_name}' already exists — pick another name"
        ));
    }

    // Check asset collisions before writing any file. We want all-or-
    // nothing semantics: either every asset lands or nothing does.
    let mut collisions = Vec::new();
    for asset in &assets {
        library::validate_asset_name(&asset.name)
            .map_err(|e| anyhow!("invalid asset name '{}': {e}", asset.name))?;
        if library::asset_file_path(asset.kind, &asset.name).exists() {
            collisions.push(format!("{}/{}", asset.kind.as_str(), asset.name));
        }
    }
    if !collisions.is_empty() {
        return Err(anyhow!(
            "asset(s) already exist: {} — refine your prompt to use different names",
            collisions.join(", ")
        ));
    }

    // All-clear: write files, then create the bundle.
    let mut created: Vec<BundleRef> = Vec::with_capacity(assets.len());
    for asset in &assets {
        library::write_asset_content(asset.kind, &asset.name, &asset.content)?;
        created.push(BundleRef {
            kind: asset.kind,
            name: asset.name.clone(),
        });
    }

    bundles::create_bundle(&bundle_name, bundle_description)?;
    bundles::set_bundle_assets(&bundle_name, created.clone())?;

    Ok(MaterializeResult {
        bundle_name,
        asset_count: assets.len(),
        created,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_extracts_bundle_meta_and_assets() {
        let raw = "\
BUNDLE-META:
name: python-backend
description: Python backend dev kit
END-BUNDLE-META

<<<ASSET-BEGIN: skills/python-setup>>>
---
description: Setup Python project
---
# Python setup
do things
<<<ASSET-END>>>

<<<ASSET-BEGIN: commands/run-tests>>>
---
description: Run pytest with coverage
---
# /run-tests
Run pytest now.
<<<ASSET-END>>>
";
        let (assets, name, desc) = parse_bundle_response(raw);
        assert_eq!(name.as_deref(), Some("python-backend"));
        assert_eq!(desc.as_deref(), Some("Python backend dev kit"));
        assert_eq!(assets.len(), 2);
        assert_eq!(assets[0].kind, AssetKind::Skills);
        assert_eq!(assets[0].name, "python-setup");
        assert!(assets[0].content.contains("Python setup"));
        assert_eq!(assets[1].kind, AssetKind::Commands);
        assert_eq!(assets[1].name, "run-tests");
    }

    #[test]
    fn parse_skips_unknown_kind() {
        let raw = "\
<<<ASSET-BEGIN: hooks/precommit>>>
some content
<<<ASSET-END>>>
<<<ASSET-BEGIN: skills/valid>>>
---
description: ok
---
body
<<<ASSET-END>>>
";
        let (assets, _, _) = parse_bundle_response(raw);
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].name, "valid");
    }

    #[test]
    fn parse_skips_invalid_name() {
        let raw = "\
<<<ASSET-BEGIN: skills/bad name with spaces>>>
content
<<<ASSET-END>>>
<<<ASSET-BEGIN: agents/ok-name>>>
content2
<<<ASSET-END>>>
";
        let (assets, _, _) = parse_bundle_response(raw);
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].kind, AssetKind::Agents);
        assert_eq!(assets[0].name, "ok-name");
    }

    #[test]
    fn parse_dedups_repeated_asset() {
        let raw = "\
<<<ASSET-BEGIN: skills/foo>>>
first
<<<ASSET-END>>>
<<<ASSET-BEGIN: skills/foo>>>
second
<<<ASSET-END>>>
";
        let (assets, _, _) = parse_bundle_response(raw);
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].content, "first");
    }

    #[test]
    fn parse_tolerates_preamble_and_postscript() {
        let raw = "\
Here you go!

BUNDLE-META:
name: kit
END-BUNDLE-META

Some chatter from the model that shouldn't end up anywhere.

<<<ASSET-BEGIN: commands/cmd>>>
---
description: x
---
body
<<<ASSET-END>>>

And that's it.
";
        let (assets, name, desc) = parse_bundle_response(raw);
        assert_eq!(name.as_deref(), Some("kit"));
        assert!(desc.is_none());
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].name, "cmd");
    }

    #[test]
    fn parse_returns_empty_on_no_blocks() {
        let raw = "I don't have any assets to generate, sorry.";
        let (assets, name, _) = parse_bundle_response(raw);
        assert!(assets.is_empty());
        assert!(name.is_none());
    }

    #[test]
    fn parse_handles_unterminated_block() {
        // No <<<ASSET-END>>> marker — the parser must not panic or
        // infinite-loop, just stop at that point.
        let raw = "\
<<<ASSET-BEGIN: skills/ok>>>
---
description: x
---
body
<<<ASSET-END>>>
<<<ASSET-BEGIN: skills/unterminated>>>
never closed
";
        let (assets, _, _) = parse_bundle_response(raw);
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].name, "ok");
    }

    // ── materialize_generated_bundle ──────────────────────────────

    fn with_temp_home<F: FnOnce()>(f: F) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let _guard = crate::HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("CLAUDE_KIT_HOME", tmp.path());
        library::ensure_layout().expect("ensure_layout");
        f();
        std::env::remove_var("CLAUDE_KIT_HOME");
    }

    fn sample_assets() -> Vec<GeneratedAsset> {
        vec![
            GeneratedAsset {
                kind: AssetKind::Skills,
                name: "alpha".into(),
                content: "---\ndescription: a\n---\nbody-a".into(),
            },
            GeneratedAsset {
                kind: AssetKind::Commands,
                name: "beta".into(),
                content: "---\ndescription: b\n---\nbody-b".into(),
            },
        ]
    }

    #[test]
    fn materialize_writes_assets_and_bundle() {
        with_temp_home(|| {
            let res = materialize_generated_bundle(
                "kit-x".into(),
                Some("a kit".into()),
                sample_assets(),
            )
            .expect("materialize");

            assert_eq!(res.bundle_name, "kit-x");
            assert_eq!(res.asset_count, 2);
            assert_eq!(res.created.len(), 2);

            // Files actually written?
            assert!(library::asset_file_path(AssetKind::Skills, "alpha").exists());
            assert!(library::asset_file_path(AssetKind::Commands, "beta").exists());
            let bundle = bundles::read_bundle("kit-x").expect("bundle present");
            assert_eq!(bundle.description.as_deref(), Some("a kit"));
            assert_eq!(bundle.assets.len(), 2);
        });
    }

    #[test]
    fn materialize_refuses_existing_bundle() {
        with_temp_home(|| {
            bundles::create_bundle("kit-x", None).unwrap();
            let err = materialize_generated_bundle(
                "kit-x".into(),
                None,
                sample_assets(),
            )
            .expect_err("should fail");
            assert!(
                err.to_string().contains("already exists"),
                "got: {err}"
            );
        });
    }

    #[test]
    fn materialize_refuses_asset_collision_atomically() {
        with_temp_home(|| {
            // Pre-create one of the assets to force a collision.
            library::write_asset_content(
                AssetKind::Skills,
                "alpha",
                "---\ndescription: pre-existing\n---\n",
            )
            .unwrap();

            let err = materialize_generated_bundle(
                "kit-x".into(),
                None,
                sample_assets(),
            )
            .expect_err("should fail");
            assert!(err.to_string().contains("skills/alpha"), "got: {err}");

            // The OTHER asset must NOT have been written — all-or-nothing.
            assert!(!library::asset_file_path(AssetKind::Commands, "beta").exists());
            // And no bundle created either.
            assert!(bundles::read_bundle("kit-x").is_none());
        });
    }

    #[test]
    fn materialize_refuses_invalid_bundle_name() {
        with_temp_home(|| {
            let err = materialize_generated_bundle(
                "has spaces".into(),
                None,
                sample_assets(),
            )
            .expect_err("should fail");
            assert!(err.to_string().contains("invalid bundle name"), "got: {err}");
        });
    }

    #[test]
    fn materialize_refuses_empty_assets() {
        with_temp_home(|| {
            let err = materialize_generated_bundle("kit".into(), None, vec![])
                .expect_err("should fail");
            assert!(err.to_string().contains("empty"), "got: {err}");
        });
    }
}
