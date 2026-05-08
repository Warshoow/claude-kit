use crate::ai::generate_text;
use crate::bundles::read_bundle;
use crate::library::{read_asset_content, AssetKind};
use anyhow::{anyhow, Result};
use serde::Serialize;

const BEGIN: &str = "<<<HARMONIZE-BEGIN:";
const END: &str = "<<<HARMONIZE-END>>>";
const CLOSE_HEADER: &str = ">>>";

/// One asset's before/after pair, ready for the frontend to diff and
/// display. `proposed` may equal `original` when the model decided no
/// changes were needed — we still surface it so the UI can render an
/// empty-diff card rather than silently hiding the asset.
#[derive(Debug, Clone, Serialize)]
pub struct HarmonizationResult {
    pub kind: AssetKind,
    pub name: String,
    pub original: String,
    pub proposed: String,
}

pub fn harmonize_bundle(
    bundle_name: &str,
    instruction: Option<&str>,
) -> Result<Vec<HarmonizationResult>> {
    let bundle = read_bundle(bundle_name)
        .ok_or_else(|| anyhow!("bundle not found: {bundle_name}"))?;

    if bundle.assets.is_empty() {
        return Err(anyhow!("bundle '{bundle_name}' is empty — nothing to harmonize"));
    }

    // Snapshot the originals up front so we can pair the AI's output
    // back to the right assets even if the model reorders them.
    let mut originals: Vec<HarmonizationResult> = Vec::with_capacity(bundle.assets.len());
    for r in &bundle.assets {
        let original = read_asset_content(r.kind, &r.name)
            .map_err(|e| anyhow!("read {}/{}: {e}", r.kind.as_str(), r.name))?;
        originals.push(HarmonizationResult {
            kind: r.kind,
            name: r.name.clone(),
            original,
            proposed: String::new(),
        });
    }

    let system = build_system_prompt();
    let user = build_user_prompt(&bundle.name, instruction, &originals);
    let raw = generate_text(&system, &user)?;
    let parsed = parse_response(&raw);

    if parsed.is_empty() {
        return Err(anyhow!(
            "model didn't return any rewrites in the expected format"
        ));
    }

    // Match parsed entries back to originals. We require exact kind/name
    // matches; anything the model invented is dropped silently. If an
    // asset is missing from the response, we keep its original as the
    // proposed content so the UI shows a clean no-op diff instead of an
    // error.
    for (header, content) in parsed {
        let Some((kind_str, name)) = header.split_once('/') else { continue };
        let Some(kind) = parse_kind(kind_str) else { continue };
        if let Some(target) = originals
            .iter_mut()
            .find(|r| r.kind == kind && r.name == name)
        {
            target.proposed = content;
        }
    }

    // Any asset the model skipped: pass through unchanged.
    for r in &mut originals {
        if r.proposed.is_empty() {
            r.proposed = r.original.clone();
        }
    }

    Ok(originals)
}

fn parse_kind(s: &str) -> Option<AssetKind> {
    match s {
        "skills" => Some(AssetKind::Skills),
        "commands" => Some(AssetKind::Commands),
        "agents" => Some(AssetKind::Agents),
        _ => None,
    }
}

fn build_system_prompt() -> String {
    r#"You are harmonizing a Claude Code bundle: a curated set of skills, commands, and agents the user assembled to be applied to a project together.

Your goal: rewrite each asset so the bundle reads like it was authored by one person. PRESERVE each asset's purpose and core behaviour — never turn a skill into something it isn't.

Apply consistency across:
- Tone of voice (active, terse, professional)
- Terminology (use the same words for the same concepts everywhere)
- YAML frontmatter format (description as a single sentence; tags optional and lowercase-hyphenated)
- Document structure (heading levels, section ordering, list style)
- Cross-references between assets (if asset A mentions asset B, name it correctly)

If an asset is already consistent with the rest, return it unchanged — repeat its content verbatim.

OUTPUT FORMAT (strict, machine-parsed):

For every asset in the input, emit exactly:

<<<HARMONIZE-BEGIN: <kind>/<name>>>>
<the rewritten file content, including frontmatter>
<<<HARMONIZE-END>>>

Use the same `<kind>/<name>` header you received in the input. Output nothing outside these blocks — no commentary, no markdown code fences wrapping the content."#.to_string()
}

fn build_user_prompt(
    bundle_name: &str,
    instruction: Option<&str>,
    assets: &[HarmonizationResult],
) -> String {
    let mut s = String::new();
    s.push_str(&format!("Bundle: {bundle_name}\n"));
    if let Some(extra) = instruction.map(str::trim).filter(|x| !x.is_empty()) {
        s.push_str(&format!("Additional instruction: {extra}\n"));
    }
    s.push_str("\nAssets to harmonize:\n\n");
    for a in assets {
        s.push_str(&format!(
            "<<<HARMONIZE-BEGIN: {}/{}>>>\n{}\n<<<HARMONIZE-END>>>\n",
            a.kind.as_str(),
            a.name,
            a.original
        ));
    }
    s.push_str(
        "\nReturn one HARMONIZE block per asset above, with the same kind/name header.",
    );
    s
}

/// Walk the model's output and pull out (header, content) pairs. We don't
/// pull in a regex crate for one delimiter format — the linear scan is
/// fine and plays nicer with malformed output (we just skip whatever we
/// can't parse instead of aborting).
fn parse_response(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut cursor = 0;
    while let Some(begin) = text[cursor..].find(BEGIN) {
        let abs_begin = cursor + begin;
        let header_start = abs_begin + BEGIN.len();
        let Some(header_end_off) = text[header_start..].find(CLOSE_HEADER) else { break };
        let header_end = header_start + header_end_off;
        let header = text[header_start..header_end].trim().to_string();
        let content_start = header_end + CLOSE_HEADER.len();
        let Some(end_off) = text[content_start..].find(END) else { break };
        let content_end = content_start + end_off;
        let content = text[content_start..content_end]
            .trim_matches(|c: char| c == '\n' || c == '\r')
            .to_string();
        out.push((header, content));
        cursor = content_end + END.len();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_well_formed_output() {
        let txt = "<<<HARMONIZE-BEGIN: skills/foo>>>\nhello\n<<<HARMONIZE-END>>>\n<<<HARMONIZE-BEGIN: commands/bar>>>\nworld\n<<<HARMONIZE-END>>>";
        let r = parse_response(txt);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].0, "skills/foo");
        assert_eq!(r[0].1, "hello");
        assert_eq!(r[1].0, "commands/bar");
        assert_eq!(r[1].1, "world");
    }

    #[test]
    fn skips_truncated_block_at_end() {
        let txt = "<<<HARMONIZE-BEGIN: skills/foo>>>\nhello\n<<<HARMONIZE-END>>>\n<<<HARMONIZE-BEGIN: commands/bar>>>\nincomplete";
        let r = parse_response(txt);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].0, "skills/foo");
    }

    #[test]
    fn ignores_text_outside_blocks() {
        let txt = "Sure! Here are the rewrites:\n\n<<<HARMONIZE-BEGIN: skills/foo>>>\nbody\n<<<HARMONIZE-END>>>\n\nLet me know!";
        let r = parse_response(txt);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].1, "body");
    }
}
