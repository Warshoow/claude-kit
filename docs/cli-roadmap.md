# CLI tool + Claude Code plugin — future roadmap

> Status: **deferred**. Captured here so we can pick it up later. The
> desktop app does not depend on this work; everything below is additive.

## Goal

Let users apply a bundle from a project's terminal without opening the
desktop app:

```bash
ck apply python-backend
ck apply python-backend --replace
ck installed
ck clean
```

Same target audience: someone who already created bundles via the
desktop UI, then wants a fast CLI to ship them into a project.

## Why it's not a big lift

The Rust backend is already factored as if it were a library. The Tauri
app's `main.rs` only contains thin `#[tauri::command]` wrappers around
real logic that lives in three modules:

| Module          | Already exposes                                              |
| --------------- | ------------------------------------------------------------ |
| `library.rs`    | `scan_all`, `ensure_layout`, `read_asset_content`, …         |
| `bundles.rs`    | `list_bundles`, `read_bundle`, `set_bundle_assets`, …        |
| `project.rs`    | `apply_one`, `remove_one`, `list_installed`, `is_our_symlink`|

None of those depend on Tauri. Building a CLI = wrapping the same
functions behind clap arg parsing instead of behind `invoke()`.

## Target architecture (Cargo workspace)

```
claude-kit/
├── Cargo.toml                ← workspace root, lists members
├── crates/
│   ├── claude-kit-core/      ← library, bundles, project, marketplace
│   ├── claude-kit-app/       ← current Tauri binary
│   └── claude-kit-cli/       ← new CLI binary
└── src/                      ← Vue frontend, unchanged
```

Both binaries depend on `claude-kit-core` → zero duplication, zero
chance of the CLI and the desktop app drifting apart.

The current `src-tauri/` becomes `crates/claude-kit-app/`. The Vue
frontend's `tauri.conf.json` paths get updated, but the frontend code
itself doesn't move.

## MVP command surface

```bash
ck apply <bundle> [--replace]    # apply to .claude/ of cwd
ck list                          # list available bundles
ck installed                     # show what's installed in cwd
ck clean                         # remove all our symlinks from cwd
ck --version
```

Implementation is ~150 lines of Rust with clap. Most of it is
formatting tables for stdout; the actual logic is one-liners against
`claude-kit-core`.

The CLI shares `~/.claude-assets/` with the desktop app — same library,
same bundles, same `.origins.json`. That's the whole point.

## Claude Code plugin layer (optional, on top)

A Claude Code plugin can wrap the CLI to add slash commands like
`/kit:apply`. Each command is a small markdown file in the plugin's
`commands/` directory:

```markdown
---
description: Apply a claude-kit bundle to the current project
---
!ck apply $1 ${2:+--replace}
```

That plugin lives in its own repo (or just a folder), gets imported
into the user's library like any other plugin, and shells out to `ck`.
Total weight: ~10 lines per command. **The CLI is the load-bearing
piece** — the plugin is sugar on top.

## Effort estimate

| Step                                          | Approx. effort |
| --------------------------------------------- | -------------- |
| Refactor into workspace + extract `core`      | ~half day      |
| CLI binary with clap (apply / list / clean)   | ~2-3 hours     |
| Distribution & PATH integration (installer)   | ~half day      |
| **Total to ship a clean version**             | **1-2 days**   |
| MVP "works on my machine"                     | ~3-4 hours     |

The slowest part is the workspace refactor, and even that is mostly
mechanical: move files, fix paths, update `Cargo.toml` references.

## Decisions to make when we revisit this

1. **Binary name.** `ck` is short and ergonomic but high collision risk
   with other tools. `claude-kit` is clearer but verbose. Decision can
   wait until the CLI exists.
2. **Distribution.** Bundle the CLI inside the desktop installer (so
   `ck` lands on PATH automatically when the app is installed)? Or
   ship a separate `cargo install claude-kit-cli` for terminal-first
   users? Probably both, eventually.
3. **Init flow.** What happens if a user runs `ck apply` on a machine
   that has never opened the desktop app? Should the CLI auto-create
   `~/.claude-assets/` (like the app does on startup) or refuse?
   Reasonable default: auto-create the layout but require an existing
   bundle (`ck apply` errors out with a clear "no bundles found, create
   one with the desktop app first").
4. **Output format.** Plain text by default, optional `--json` for
   scripting (`ck list --json | jq`). Easy to add later.

## Why this is the easiest of the deferred chantiers

Compared to the other roadmap items we've discussed:

- **Full plugin import** (capture hooks/MCP/etc., not just the three
  asset kinds) — touches data model, library scanner, UI. Bigger.
- **Update force-overwrite** (real "Update" action that replaces
  existing files with diff/conflict handling) — touches import logic,
  needs UI for conflicts. Medium-to-big.
- **CLI tool (this)** — pure additive work, no UI surface, business
  logic already exists. Smallest of the three.

So if we want quick wins for breadth-of-features, this is the cheapest
to ship.
