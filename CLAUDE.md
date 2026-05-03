# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working
with code in this repository.

## What this app does

claude-kit is a Tauri 2 + Vue 3 desktop app for managing Claude Code
**skills, commands, and agents** as reusable bundles, with import from
the official `claude-plugins-official` marketplace. The user keeps a
central library at `~/.claude-assets/` and "applies" bundles to a
project, which materializes as **relative symlinks** inside that
project's `.claude/` directory.

Persistence layout:

- `~/.claude-assets/library/{skills,commands,agents}/` — source of truth
  for asset content. Edits go here.
- `~/.claude-assets/library/.origins.json` — provenance map keyed
  `<kind>:<name>` → `Origin { marketplace, plugin, imported_at, version?, git_ref? }`
  for assets that came from a marketplace import. Used to group the UI
  by plugin, surface "in library" / "update available" badges, and
  trace where an asset came from.
- `~/.claude-assets/bundles/<name>.json` — bundle definitions (just
  lists of `{kind, name}` refs).
- `<project>/.claude/{skills,commands,agents}/` — symlinks pointing back
  into the library.

Override the library root with `CLAUDE_KIT_HOME` (handy for tests/dev).

## Architecture

**Two processes, one IPC contract.**

- Frontend: Vue 3 SFCs in `src/`. State lives in a single Pinia store
  at `src/stores/app.ts`. Routing is hash-based via
  [`src/router/index.ts`](src/router/index.ts) with code-split lazy
  imports per view.
- Backend: Rust in `src-tauri/src/` exposes `#[tauri::command]`
  functions. The frontend calls them through a thin wrapper at
  [`src/lib/api.ts`](src/lib/api.ts).

**Adding a backend command requires four edits in lockstep:**

1. Implement it in the relevant module (`library.rs`, `bundles.rs`,
   `marketplace.rs`, or `project.rs`).
2. Re-export / wrap it as a `#[tauri::command]` in `src-tauri/src/main.rs`.
3. Register it in the `tauri::generate_handler![...]` list in `main()`
   — **commands not in this list will fail at runtime with no compile
   error.**
4. Add the typed wrapper to `src/lib/api.ts` and any shared types to
   `src/lib/types.ts`. The TS shapes must mirror the Rust serde output
   (e.g. `AssetKind` is `#[serde(rename_all = "lowercase")]` on the
   Rust side, so TS sees `"skills" | "commands" | "agents"`).

### Backend modules

- **`library.rs`** — scans the library directory, parses YAML
  frontmatter via `gray_matter` to surface `description` / `tags`.
  Skills are **directories** containing `SKILL.md`; commands and agents
  are **flat `.md` files**. Asset names are stored without `.md`
  extension. Owns:
  - `Origin` struct + `.origins.json` read/write (with auto-migration
    for older entries that had `.md` suffixes in keys, and `#[serde(default)]`
    on optional `version`/`git_ref` fields).
  - `import_from_plugin` — copies plugin-style folders into the library,
    skipping anything that already exists. Only walks `skills/`,
    `commands/`, `agents/`; everything else (`hooks/`, `mcp.json`, …)
    is silently ignored. **This is intentional but flagged for future
    expansion** — see `docs/cli-roadmap.md` and the project memory.
  - `create_asset` — refuses to overwrite, validates name as
    `[A-Za-z0-9_-]+` ≤ 64 chars, scaffolds frontmatter + heading.
- **`bundles.rs`** — bundles are JSON files at
  `~/.claude-assets/bundles/<name>.json`, each holding a list of
  `BundleRef { kind, name }`.
- **`marketplace.rs`** — fetches `claude-plugins-official`'s
  `marketplace.json`, downloads plugin tarballs from
  `codeload.github.com`, extracts to a tempdir, hands the plugin root
  to `library::import_from_plugin`, then tags every freshly-imported
  asset with an `Origin` (capturing `version` and `git_ref` for update
  detection). Source resolution handles inline paths, `url`,
  `git-subdir`, and `github` polymorphic shapes. **`parse_kind_name`
  strips `.md` for commands/agents** so origin keys match the bare
  names used elsewhere.
- **`project.rs`** — handles symlink creation/removal in
  `<project>/.claude/`. Key invariant: removal only ever touches
  symlinks that point back to *our* library (see `is_our_symlink`) — so
  it will never delete a user's hand-written `.claude/` files. Symlinks
  are created **relative** via the bundled `pathdiff` helper for
  portability.

### Frontend conventions

- **Pinia store as single source of truth.** All persistent state
  (`library`, `bundles`, `installed`, `projectPath`, marketplace
  cache) lives in `src/stores/app.ts`. Components consume it via
  `storeToRefs` and call store actions; **don't fetch directly from
  components**.
- **All IPC goes through `src/lib/api.ts`.** No `invoke()` calls
  outside that file.
- **Routing is hash-based** (`createWebHashHistory`) for Tauri
  compatibility. Top-level routes:
  - `/bundles`, `/bundles/:name`
  - `/browse` → redirect `/browse/library`
    - `/browse/library` (plugin grid)
    - `/browse/library/:plugin` (per-plugin asset list, `__local__` for
      the local bucket)
    - `/browse/marketplace`
  - `/library/:kind/:name` (asset editor)
  - `/plugins/:name` (marketplace plugin detail)
  - `/project`
- **Asset identity** across the UI is `${kind}:${name}` (see
  `assetKey` in `src/lib/types.ts`) — use it for set membership and
  v-for keys.
- **Plugin import status** logic lives in `src/lib/origins.ts`
  (`pluginImportStatus(library, marketplace, plugin)` returns
  `not-imported | current | update | unknown`). Reuse it; don't
  re-derive in views.
- **Selected project path** is persisted in `localStorage` under
  `claude-kit:last-project`.
- **Toolkit:** shadcn-vue components live in `src/components/ui/`
  (style `new-york`). Tailwind v4 with `@tailwindcss/vite`. Icons via
  `lucide-vue-next`. Toasts via `vue-sonner`. Markdown editor uses
  CodeMirror 6 (`@codemirror/lang-markdown` + `oneDark` theme).

## Commands

```bash
npm install                       # install JS deps; Cargo deps fetch on first build
npm run dev                       # Vite only (port 1420, strictPort) — rarely useful alone
npm run tauri dev                 # full app: spawns Vite then launches the native window
npm run tauri build               # production bundle → src-tauri/target/release/bundle/
npm run build                     # vue-tsc typecheck + vite build (frontend only)
npx vue-tsc --noEmit              # typecheck without building
```

For the Rust side specifically:

```bash
cd src-tauri && cargo check       # fast type-check
cd src-tauri && cargo clippy      # lint
cd src-tauri && cargo test        # network-tagged tests are #[ignore]'d; run with --ignored
```

There is no test runner configured for the frontend.

For producing real installable builds (icon prep, per-OS gotchas,
distribution caveats), see [`docs/build.md`](docs/build.md).

## Gotchas

- **Skills vs commands/agents asymmetry** lives everywhere:
  `scan_kind`, `asset_file_path`, `source_path`, `target_path`, and
  `import_from_plugin` all branch on `AssetKind::Skills` to treat it
  as a directory. When adding logic that handles assets generically,
  mirror this pattern.
- **`apply_one` refuses to overwrite a non-symlink target unless
  `force: true`.** This is intentional safety against clobbering user
  files; don't switch to `force: true` to make errors go away.
- **Origin keys must match bare asset names.** For commands/agents,
  strip `.md` before writing. The library auto-migrates old keys with
  `.md` on read, but new code must produce clean keys.
- **The marketplace import is scope-limited** to skills/commands/agents
  — `hooks/`, `mcp.json`, `plugin.json`, `LICENSE`, etc. are silently
  ignored. This is intentional in the current scope but a known future
  evolution (see `docs/cli-roadmap.md` and project memory).
- **Update detection only kicks in for assets imported with version
  tracking**. Older imports show as `unknown` (in-library, can't tell
  if updates exist) until they're re-imported.
- **Window chrome is OS-adaptive.** macOS keeps native traffic lights
  via `titleBarStyle: "Overlay"`; Windows/Linux strip decorations in
  `main.rs::setup` and rely on the topbar in `App.vue` for drag +
  custom controls. The `core:window:allow-*` capabilities in
  `src-tauri/capabilities/default.json` are required — adding window
  ops needs the corresponding permission entry there.
- **Tauri is a desktop app** — `npm run tauri dev` opens a native
  window. Inside a headless container it needs DISPLAY / X11
  forwarding (WSLg works). If WSLg drops, GTK errors with
  `Connection reset by peer` and the window dies; relaunch the dev
  server.
- **Tauri does not cross-compile.** To produce a Windows binary you
  must build from a Windows host (PowerShell, not WSL). Same applies
  to macOS / Linux outputs.
