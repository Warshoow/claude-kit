use crate::ai::generate_text;
use crate::library::{scan_all, AssetKind};
use crate::marketplace::{fetch_marketplace, Plugin, DEFAULT_MARKETPLACE_URL};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Final shape returned to the frontend. The model's raw output is parsed
/// then enriched with `already_imported` / `already_in_library` flags so
/// the UI can show "already in library" badges without re-deriving.
#[derive(Debug, Clone, Serialize)]
pub struct RecommendationResult {
    pub bundle_name: String,
    pub bundle_description: Option<String>,
    pub rationale: String,
    pub plugins_to_import: Vec<RecommendedPlugin>,
    pub assets: Vec<RecommendedAsset>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecommendedPlugin {
    pub name: String,
    pub already_imported: bool,
    pub reason: String,
    /// Re-attached so the frontend can call importMarketplacePlugin
    /// without round-tripping to the marketplace store again.
    pub plugin: Option<Plugin>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecommendedAsset {
    pub kind: AssetKind,
    pub name: String,
    pub plugin: String,
    pub already_in_library: bool,
    pub reason: String,
}

// ── Raw model output (intermediate, not exposed) ───────────────────

#[derive(Debug, Deserialize)]
struct RawRecommendation {
    bundle: RawBundle,
    #[serde(default)]
    rationale: String,
    #[serde(default)]
    plugins: Vec<RawPlugin>,
    #[serde(default)]
    assets: Vec<RawAsset>,
}

#[derive(Debug, Deserialize)]
struct RawBundle {
    name: String,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawPlugin {
    name: String,
    #[serde(default)]
    reason: String,
}

#[derive(Debug, Deserialize)]
struct RawAsset {
    kind: AssetKind,
    name: String,
    plugin: String,
    #[serde(default)]
    reason: String,
}

pub fn recommend_bundle(user_need: &str, marketplace_url: Option<&str>) -> Result<RecommendationResult> {
    if user_need.trim().is_empty() {
        return Err(anyhow!("describe what you want the bundle to do"));
    }

    let url = marketplace_url.unwrap_or(DEFAULT_MARKETPLACE_URL);
    let marketplace = fetch_marketplace(url)?;
    let library = scan_all();

    // Build a compact catalog string. Trying to keep prompt size sane —
    // 100+ plugins with full description = ~30KB, still well under any
    // model's context window.
    let catalog = marketplace
        .plugins
        .iter()
        .map(|p| {
            let cat = p.category.as_deref().unwrap_or("");
            let cat_part = if cat.is_empty() { String::new() } else { format!(" [{cat}]") };
            format!("- {}{cat_part}: {}", p.name, p.description)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let imported_plugins: HashSet<String> = library
        .iter()
        .filter_map(|a| a.origin.as_ref().map(|o| o.plugin.clone()))
        .collect();

    let library_listing = if library.is_empty() {
        "(empty)".to_string()
    } else {
        library
            .iter()
            .map(|a| {
                let from = a
                    .origin
                    .as_ref()
                    .map(|o| format!(" (from {})", o.plugin))
                    .unwrap_or_default();
                format!("- {}/{}{}", a.kind.as_str(), a.name, from)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let system = build_system_prompt();
    let user = format!(
        "User's need:\n{user_need}\n\nMarketplace catalog ({} plugins):\n{catalog}\n\nUser's existing library:\n{library_listing}",
        marketplace.plugins.len(),
    );

    let raw = generate_text(&system, &user)?;
    let parsed = parse_json_recommendation(&raw)?;

    // Resolve plugin entries against the marketplace so the frontend can
    // import them without an extra lookup. Drop anything the model invented.
    let mut plugins_to_import: Vec<RecommendedPlugin> = parsed
        .plugins
        .into_iter()
        .filter_map(|rp| {
            let plugin = marketplace
                .plugins
                .iter()
                .find(|p| p.name == rp.name)
                .cloned();
            // If the model invents a name, drop it — UX-wise we'd rather
            // omit a hallucinated entry than show a broken row.
            plugin.as_ref()?;
            let already_imported = imported_plugins.contains(&rp.name);
            Some(RecommendedPlugin {
                name: rp.name,
                already_imported,
                reason: rp.reason,
                plugin,
            })
        })
        .collect();

    // Stable order: not-yet-imported first, then alphabetical.
    plugins_to_import.sort_by(|a, b| match (a.already_imported, b.already_imported) {
        (false, true) => std::cmp::Ordering::Less,
        (true, false) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    let assets = parsed
        .assets
        .into_iter()
        .map(|ra| {
            let already_in_library = library
                .iter()
                .any(|a| a.kind == ra.kind && a.name == ra.name);
            RecommendedAsset {
                kind: ra.kind,
                name: ra.name,
                plugin: ra.plugin,
                already_in_library,
                reason: ra.reason,
            }
        })
        .collect();

    Ok(RecommendationResult {
        bundle_name: parsed.bundle.name,
        bundle_description: parsed.bundle.description,
        rationale: parsed.rationale,
        plugins_to_import,
        assets,
    })
}

fn build_system_prompt() -> String {
    r#"You are recommending a Claude Code bundle for a developer.

Inputs you receive:
- The user's need (free-form natural language).
- A catalog of marketplace plugins (`- name [category]: description`).
- The user's existing library of skills/commands/agents (already-imported assets).

Your job:
1. Pick the marketplace plugins most relevant to the user's need. Prefer reusing assets the user already has over re-importing.
2. Compose a bundle by selecting specific assets from those plugins (and the existing library).
3. Suggest a short bundle name (lowercase-hyphenated, ≤32 chars) and a single-sentence description.
4. Keep the bundle focused — 5-15 assets is a good range. Don't dump every plugin's contents.

OUTPUT FORMAT — strict JSON, nothing else. No markdown code fences, no commentary, no preamble.

```json schema (don't echo this — match the shape):
{
  "bundle": {
    "name": "kebab-case-name",
    "description": "One sentence."
  },
  "rationale": "2-3 sentences explaining why this bundle fits the user's need.",
  "plugins": [
    { "name": "marketplace-plugin-name", "reason": "Why this plugin is relevant." }
  ],
  "assets": [
    {
      "kind": "skills" | "commands" | "agents",
      "name": "asset-bare-name",
      "plugin": "owning-plugin-name OR an empty string for already-local assets",
      "reason": "Why this specific asset belongs in the bundle."
    }
  ]
}
```

Rules:
- Use ONLY plugin names that appear in the catalog. Don't invent.
- Use ONLY asset names that exist in the user's library OR will be imported via the plugins you list. Don't make up names.
- Match `kind` exactly to one of: skills, commands, agents.
- Output strictly valid JSON. The first character of your response is `{`."#.to_string()
}

/// Robust JSON extraction. Some models still wrap output in markdown code
/// fences ("```json … ```") despite instructions; some preface with text.
/// We strip leading whitespace + an optional fence, then balance braces to
/// find the first complete JSON object.
fn parse_json_recommendation(raw: &str) -> Result<RawRecommendation> {
    let cleaned = strip_optional_fence(raw.trim());
    let json_str = extract_first_json_object(cleaned).ok_or_else(|| {
        anyhow!("model didn't return a JSON object: {}", truncate_for_error(raw))
    })?;
    serde_json::from_str::<RawRecommendation>(&json_str)
        .map_err(|e| anyhow!("recommendation JSON parse failed: {e}\n{json_str}"))
}

fn strip_optional_fence(s: &str) -> &str {
    let mut s = s;
    if let Some(rest) = s.strip_prefix("```json") {
        s = rest.trim_start_matches('\n');
    } else if let Some(rest) = s.strip_prefix("```") {
        s = rest.trim_start_matches('\n');
    }
    if let Some(stripped) = s.trim_end().strip_suffix("```") {
        return stripped.trim_end();
    }
    s
}

fn extract_first_json_object(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let start = bytes.iter().position(|b| *b == b'{')?;
    let mut depth: i32 = 0;
    let mut in_str = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if in_str {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(s[start..=i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn truncate_for_error(s: &str) -> String {
    if s.len() <= 200 {
        s.to_string()
    } else {
        format!("{}…", &s[..200])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_plain_json() {
        let s = r#"{"bundle":{"name":"x","description":"y"},"rationale":"z","plugins":[],"assets":[]}"#;
        let r = parse_json_recommendation(s).expect("parse");
        assert_eq!(r.bundle.name, "x");
    }

    #[test]
    fn strips_markdown_fence() {
        let s = "```json\n{\"bundle\":{\"name\":\"x\"},\"rationale\":\"z\",\"plugins\":[],\"assets\":[]}\n```";
        let r = parse_json_recommendation(s).expect("parse");
        assert_eq!(r.bundle.name, "x");
    }

    #[test]
    fn extracts_object_after_preamble() {
        let s = "Sure! Here you go:\n\n{\"bundle\":{\"name\":\"x\"},\"rationale\":\"\",\"plugins\":[],\"assets\":[]}";
        let r = parse_json_recommendation(s).expect("parse");
        assert_eq!(r.bundle.name, "x");
    }

    #[test]
    fn ignores_braces_inside_strings() {
        let s = r#"{"bundle":{"name":"x","description":"contains } and { in text"},"rationale":"","plugins":[],"assets":[]}"#;
        let r = parse_json_recommendation(s).expect("parse");
        assert_eq!(
            r.bundle.description.as_deref(),
            Some("contains } and { in text")
        );
    }

    #[test]
    fn strips_generic_fence_without_json_tag() {
        let s = "```\n{\"bundle\":{\"name\":\"x\"},\"rationale\":\"\",\"plugins\":[],\"assets\":[]}\n```";
        let r = parse_json_recommendation(s).expect("parse");
        assert_eq!(r.bundle.name, "x");
    }

    #[test]
    fn handles_escaped_backslash_in_string() {
        let s = r#"{"bundle":{"name":"x","description":"path: C:\\Users\\foo"},"rationale":"","plugins":[],"assets":[]}"#;
        let r = parse_json_recommendation(s).expect("parse");
        assert_eq!(r.bundle.description.as_deref(), Some(r"path: C:\Users\foo"));
    }

    #[test]
    fn returns_error_when_no_json_object() {
        let err = parse_json_recommendation("no json here at all").unwrap_err();
        assert!(err.to_string().contains("JSON object") || err.to_string().contains("json"));
    }

    #[test]
    fn returns_error_on_empty_input() {
        assert!(parse_json_recommendation("").is_err());
    }

    #[test]
    fn parses_assets_array_with_kinds() {
        let s = r#"{
            "bundle":{"name":"b","description":"d"},
            "rationale":"r",
            "plugins":[{"name":"plug","reason":"because"}],
            "assets":[
                {"kind":"skills","name":"my-skill","plugin":"plug","reason":"r1"},
                {"kind":"commands","name":"my-cmd","plugin":"plug","reason":"r2"}
            ]
        }"#;
        let r = parse_json_recommendation(s).expect("parse");
        assert_eq!(r.assets.len(), 2);
        assert_eq!(r.assets[0].kind, crate::library::AssetKind::Skills);
        assert_eq!(r.assets[1].name, "my-cmd");
        assert_eq!(r.plugins[0].name, "plug");
    }

    #[test]
    fn truncate_for_error_caps_at_200_chars() {
        let long = "x".repeat(300);
        let truncated = truncate_for_error(&long);
        assert!(truncated.len() <= 204, "should be ~200 chars + ellipsis");
        assert!(truncated.ends_with('…'));
    }

    #[test]
    fn truncate_for_error_leaves_short_strings_intact() {
        let short = "hello world";
        assert_eq!(truncate_for_error(short), short);
    }
}
