# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this app does

claude-kit is a Tauri 2 + Vue 3 desktop app for managing Claude Code **skills, commands, and agents** as reusable bundles. The user keeps a central library at `~/.claude-assets/` and "applies" bundles to a project, which materializes as **relative symlinks** inside that project's `.claude/` directory.

Three persistence locations matter:

- `~/.claude-assets/library/{skills,commands,agents}/` — source of truth for asset content. Edits go here.
- `~/.claude-assets/bundles/<name>.json` — bundle definitions (just lists of `{kind, name}` refs).
- `<project>/.claude/{skills,commands,agents}/` — symlinks pointing back into the library.

Override the library root with the `CLAUDE_KIT_HOME` env var (handy for tests / dev).

## Architecture

**Two processes, one IPC contract.**

- Frontend: Vue 3 SFCs in `src/`. `App.vue` owns all state and orchestrates three columns (`LibraryColumn`, `BundlesColumn`, `ProjectColumn`) plus a modal `AssetEditor`.
- Backend: Rust in `src-tauri/src/` exposes `#[tauri::command]` functions. The frontend calls them through a thin wrapper in `src/lib/api.ts`.

**Adding a backend command requires four edits in lockstep:**

1. Implement it in the relevant module (`library.rs`, `bundles.rs`, or `project.rs`).
2. Re-export / wrap it as a `#[tauri::command]` in `src-tauri/src/main.rs`.
3. Register it in the `tauri::generate_handler![...]` list in `main()` — **commands not in this list will fail at runtime with no compile error.**
4. Add the typed wrapper to `src/lib/api.ts` and any shared types to `src/lib/types.ts`.

`AssetKind` is serialized as lowercase (`"skills" | "commands" | "agents"`) — the Rust enum uses `#[serde(rename_all = "lowercase")]` and the TS type must stay in sync.

### Backend modules

- `library.rs` — scans the library directory, parses YAML frontmatter via `gray_matter` to surface `description` / `tags`. Skills are **directories** containing `SKILL.md`; commands and agents are **flat `.md` files**. `import_from_plugin` copies plugin-style folders into the library, skipping anything that already exists.
- `bundles.rs` — bundles are JSON files at `~/.claude-assets/bundles/<name>.json`, each holding a list of `BundleRef { kind, name }`.
- `project.rs` — handles symlink creation/removal in `<project>/.claude/`. Key invariant: removal only ever touches symlinks that point back to *our* library (see `is_our_symlink`) — so it will never delete a user's hand-written `.claude/` files. Symlinks are created **relative** via the bundled `pathdiff` helper for portability.

### Frontend conventions

- All IPC goes through `src/lib/api.ts`. Components never call `invoke` directly.
- `App.vue` is the single source of truth for `library`, `bundles`, `installed`, `projectPath`. Children emit events; the parent calls the API and refreshes. Don't add component-local fetches.
- The selected project path is persisted in `localStorage` under `claude-kit:last-project`.
- Asset identity across UI is `${kind}:${name}` (see `assetKey` in `types.ts`) — use it for set membership and keys.

## Commands

```bash
npm install                # install JS deps; Cargo deps fetch on first build
npm run dev                # Vite only (port 1420, strictPort) — rarely useful alone
npm run tauri dev          # full app: spawns Vite then launches the native window
npm run tauri build        # production bundle → src-tauri/target/release/bundle/
npm run build              # vue-tsc typecheck + vite build (frontend only)
```

For the Rust side specifically:

```bash
cd src-tauri && cargo check       # fast type-check
cd src-tauri && cargo clippy      # lint
cd src-tauri && cargo test        # no tests yet, but this is the entry point
```

There is no test runner configured for the frontend.

## Gotchas

- **Skills vs commands/agents asymmetry** lives everywhere: `scan_kind`, `asset_file_path`, `source_path`, `target_path`, and `import_from_plugin` all branch on `AssetKind::Skills` to treat it as a directory. When adding logic that handles assets generically, mirror this pattern.
- **`apply_one` refuses to overwrite a non-symlink target unless `force: true`.** This is intentional safety against clobbering user files; don't switch to `force: true` to make errors go away.
- **Tauri is a desktop app** — `npm run tauri dev` opens a native window. Inside a headless container it needs DISPLAY / X11 forwarding, otherwise it'll fail to open. Run on the host for normal development.
