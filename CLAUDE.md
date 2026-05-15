# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working
with code in this repository.

## What this app does

claude-kit is a Tauri 2 + Vue 3 desktop app for managing Claude Code
**skills, commands, agents, hooks and MCP server configs** as reusable
bundles, with import from any number of `marketplace.json` sources (the
official `claude-plugins-official` catalog ships built-in). The user
keeps a central library at `~/.claude-assets/` and "applies" bundles to
a project, which materializes as **relative symlinks** inside that
project's `.claude/` directory.

Four AI flows are exposed in the UI:
- **Asset generator** — describe what you want, get a Claude-Code-shaped file.
- **Asset refine chat** — iterate on an existing asset through a streaming back-and-forth conversation; current content is sent as context.
- **Bundle generator via chat** — generate a complete, coherent bundle from a conversation; each turn produces the full asset set so the UI can always display a clean diff.
- **Bundle harmonizer** — rewrites every markdown asset of a bundle to bridge workflow gaps and unify terminology, per-hunk diff review before writing. Hooks and MCP entries are skipped (not freeform markdown).

A fifth — the **bundle recommender** — is fully implemented backend +
frontend but its entry-point button is intentionally hidden right now;
the route `/recommend` still works for direct testing. It's pending a
content-aware rebuild (the model currently sees only plugin
names/descriptions and hallucinates asset names — see [`docs/roadmap.md`](docs/roadmap.md)).

Bundles can also be **shared between machines** via a single compact
code (gzipped + base64-encoded manifest of the bundle's content +
metadata). No server in the loop — the user copies a code, pastes it
on another install, the bundle is recreated locally.

Persistence layout:

- `~/.claude-assets/library/{skills,commands,agents}/` — source of truth
  for asset content. Edits go here.
- `~/.claude-assets/library/hooks/<plugin>/<file>` — hook scripts copied
  verbatim from imported plugins.
- `~/.claude-assets/library/hooks/__local__/<file>` — AI-generated or
  manually created local hooks. Each script has a sidecar
  `<file>.meta.json` (`LocalHookMeta { event, matcher }`) so the app
  can reconstruct the `settings.json` registration on any machine.
- `~/.claude-assets/library/mcp/<plugin>.json` — MCP server configs
  copied from imported plugins, merged into the project on apply.
- `~/.claude-assets/library/mcp/__local__/<name>.json` — MCP server
  configs created manually in the app (not from a plugin import).
- `~/.claude-assets/library/.origins.json` — provenance map keyed
  `<kind>:<name>` → `Origin { marketplace, plugin, imported_at, version?, git_ref? }`
  for assets that came from a marketplace import. Used to group the UI
  by plugin, surface "in library" / "update available" badges, drive
  the per-hunk update flow, and trace where an asset came from.
- `~/.claude-assets/library/.harmonized.json` — sibling of `.origins.json`,
  keyed the same way → ISO timestamp of the last time the harmonizer
  touched the asset. Powers the `✨ Harmonized` badge in the library
  and asset editor. Stored separately from `Origin` because it has to
  work for locally-created assets too (which have no `Origin`).
- `~/.claude-assets/bundles/<name>.json` — bundle definitions (just
  lists of `{kind, name}` refs, plus an optional `description`).
- `~/.claude-assets/settings.json` — AI backend config + the user's
  list of registered marketplaces.
- `<project>/.claude/{skills,commands,agents,hooks}/` — symlinks
  pointing back into the library. `<project>/.claude/mcp.json` is a
  merged JSON file (not a symlink, because users typically already have
  their own MCP entries we shouldn't clobber).

Override the library root with `CLAUDE_KIT_HOME` (handy for tests/dev).

## Architecture

**Two processes, one IPC contract.**

- Frontend: Vue 3 SFCs in `src/`. State lives in domain-decomposed
  Pinia stores at [`src/stores/`](src/stores/) (one per concern, listed
  below). Routing is hash-based via
  [`src/router/index.ts`](src/router/index.ts) with code-split lazy
  imports per view.
- Backend: Rust in `src-tauri/src/` exposes `#[tauri::command]`
  functions. The frontend calls them through a thin wrapper at
  [`src/lib/api.ts`](src/lib/api.ts).
- Static landing site lives in [`website/`](website/) (VitePress 1.6
  with a **custom `Home.vue` that fully replaces the default home
  layout** — the YAML-hero-and-features layout was too constrained for
  the screenshots / smart download CTA combo we wanted). Independent
  `package.json`, deployed to GitHub Pages by
  `.github/workflows/pages.yml` on every push to `master` that touches
  `website/**`.

**Adding a backend command requires four edits in lockstep:**

1. Implement it in the relevant module (see "Backend modules" below).
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
  - `import_from_plugin(source, canonical_name: Option<&str>)` — copies
    plugin-style folders into the library: walks `skills/`, `commands/`,
    `agents/`, `hooks/`, and the plugin's `.mcp.json` if present.
    Existing entries are skipped. When `canonical_name` is passed (the
    marketplace flow always does), hooks/ and mcp/ entries are keyed
    by that name instead of the extracted tarball folder name — fixes
    grouping when GitHub renames the tarball root (`repo-main/`) and
    the marketplace's declared plugin name differ.
  - `create_asset` — refuses to overwrite, validates name as
    `[A-Za-z0-9_-]+` ≤ 64 chars, scaffolds frontmatter + heading.
  - `mark_harmonized(kind, name, iso)` + `.harmonized.json` read/write
    — stamped by the harmonize flow on every asset that actually got
    rewritten. The library scan attaches `harmonized_at` to each
    `Asset` so the UI can render the badge.
  - Hook CRUD for local hooks: `create_local_hook`, `update_local_hook`,
    `delete_local_hook`, `read_local_hook` — operate on the
    `hooks/__local__/` folder and maintain the `.meta.json` sidecar.
  - MCP CRUD for local configs: `create_local_mcp`, `update_local_mcp`,
    `delete_local_mcp`, `read_local_mcp` — operate on `mcp/__local__/`.
  - Listing helpers (`list_hooks`, `list_mcp`) surface both plugin-
    imported and locally-created entries in one flat list.
  - `remove_plugin`.
- **`bundles.rs`** — bundles are JSON files at
  `~/.claude-assets/bundles/<name>.json`, each holding a list of
  `BundleRef { kind: BundleEntryKind, name, plugin? }` plus an optional
  `description`. `BundleEntryKind` is wider than `AssetKind` — it adds
  `Hooks` and `Mcp` variants so hooks and MCP configs can be bundle
  members. `BundleEntryKind::as_asset_kind()` maps back to `AssetKind`
  and returns `None` for hooks/mcp (used by harmonize to skip them).
  Also owns the **share-by-code** flow:
  - `encode_bundle_share(name)` — packs the bundle's manifest +
    every asset's full content + each asset's `Origin` and
    `harmonized_at` into a `ShareManifest { v: 1, name, description?,
    assets: [...] }`, serialises to JSON, gzip-compresses, base64-
    encodes (`STANDARD_NO_PAD`), and prefixes with `ck1:` to mark
    the schema version.
  - `import_bundle_share(code)` — reverses the above. Skips assets
    whose files already exist locally (no overwrite), restores
    origin + harmonized metadata for new ones, and deduplicates the
    bundle name (`my-bundle` → `my-bundle (2)`) if a bundle of the
    same name already exists. Returns `ImportShareResult { bundle_name,
    imported: [...], skipped: [...] }`.
- **`marketplace.rs`** — fetches a `marketplace.json` from any URL,
  downloads plugin tarballs from `codeload.github.com`, extracts to a
  tempdir via `download_and_extract` (a public helper that both the
  import path and the update-preview path share), hands the plugin
  root to `library::import_from_plugin`, then tags every freshly-
  imported asset with an `Origin` (capturing `version` and `git_ref`
  for update detection). Source resolution handles inline paths, `url`,
  `git-subdir`, and `github` polymorphic shapes.
  Also owns the **marketplace registry**: `list_sources`, `add_source`
  (validates by fetching), `remove_source` (refuses to drop the
  built-in entry).
- **`update.rs`** — drives the force-overwrite update flow.
  `preview_update(plugin, marketplace)` re-downloads the upstream
  tarball, walks its skills/commands/agents, pairs each one against
  the user's library entries, and returns `{ modified: AssetPair[],
  added: NewAssetEntry[] }`. Identical files are dropped to avoid
  noise; new files include their content inline so apply doesn't need
  to re-download. `apply_update` writes whatever the frontend sends
  back (the merged result of accepted hunks) and **refreshes the
  origin records for every asset of that plugin** — including unchanged
  ones — so the "update available" badge clears even when the user
  rejected hunks.
- **`project.rs`** — handles symlink creation/removal in
  `<project>/.claude/`. Key invariant: removal only ever touches
  symlinks that point back to *our* library (see `is_our_symlink`) — so
  it will never delete a user's hand-written `.claude/` files. Symlinks
  are created **relative** via the bundled `pathdiff` helper for
  portability. Hooks get their own symlink path
  (`.claude/hooks/<plugin>/<file>`) and MCP entries are merged into
  `.claude/mcp.json` by deep-extending only keys we don't already
  recognize. **Local hooks** are also auto-registered into the project's
  `settings.json` (`hooks[event][matcher]` array) via `register_hook` /
  `unregister_hook` — these functions are idempotent and merge safely
  into an existing settings file.
- **`settings.rs`** — `~/.claude-assets/settings.json` read/write.
  Holds `ai: AiSettings { mode, api_base_url, api_key, api_model }`
  and `marketplaces: Vec<MarketplaceSource>`. The official marketplace
  is auto-injected on read so a manually-edited config can never lose
  access to the canonical catalog.
- **`ai.rs`** — backend dispatch for AI calls. `detect_claude_cli()`
  hunts `which claude` then `~/.claude/local/claude`. `ai_status()`
  reports the effective backend ("claude-cli" | "api" | "none") with
  a one-line user-facing message. Two call paths:
  - `generate_text(system, user)` — blocking single-turn call used by
    the asset generator, harmonizer, and recommender.
  - `generate_text_streaming(system, messages, on_token)` — streaming
    multi-turn call used by the asset refine chat and bundle generator;
    takes a `Vec<ChatMessage>` history and calls `on_token` for each
    delta. API mode emits SSE-style chunks via the Tauri event channel.
  API mode talks to any OpenAI-compatible `{base_url}/chat/completions`
  endpoint. Output is post-processed by `strip_code_fences`.
- **`bundle_chat.rs`** — chat-based bundle generator. Wire format uses
  `BUNDLE-META: … END-BUNDLE-META` for name/description and
  `<<<ASSET-BEGIN: kind/name>>>…<<<ASSET-END>>>` for each asset (mirrors
  the harmonizer's delimiter style). Each model turn must return the
  **complete** asset set (including unchanged assets from prior turns)
  so the parser always has a self-contained snapshot to materialize.
  `generate_bundle_chat` streams tokens to the frontend via
  `BundleChatEvent` (Token / Done / Error); `parse_bundle_response`
  extracts assets from the raw text; `materialize_generated_bundle`
  writes them to disk and creates the bundle JSON.
- **`harmonize.rs`** — `harmonize_bundle(name, instruction?)` packs
  every **markdown** asset of the bundle (skills/commands/agents only —
  hooks and MCP entries are skipped via `BundleEntryKind::as_asset_kind()`)
  into a delimited prompt (`<<<HARMONIZE-BEGIN: kind/name>>>…<<<HARMONIZE-END>>>`),
  calls `ai::generate_text`, and parses the response back into asset
  before/after pairs. The parser tolerates truncated blocks and preamble
  text. Unit-tested. **System prompt focus is workflow gap-filling** —
  the model first reads all assets to infer the bundle's purpose, then
  bridges steps between assets that don't reference each other and
  unifies terminology. The bundle's `description` (if any) is passed as
  an optional hint; the assets are the source of truth.
- **`recommend.rs`** — `recommend_bundle(user_need)` builds a compact
  marketplace catalog string + a listing of the user's existing
  library, sends them to the model, expects strict JSON back.
  `parse_json_recommendation` strips optional markdown fences and
  finds the first balanced JSON object even when the model prefaces
  with text. Plugins the model invents (no match in the catalog) are
  silently dropped. Unit-tested.

### Frontend conventions

- **Pinia stores are decomposed by concern** under
  [`src/stores/`](src/stores/), all re-exported from
  [`src/stores/index.ts`](src/stores/index.ts):
  - `app.ts` — library, bundles, installed assets, hooks, MCP, project
    path, the cross-cutting refresh actions
  - `bundle.ts` — bundle CRUD plus the share flow (`encodeShare`,
    `importShare`); the latter dedupes the imported bundle's name
    locally and surfaces the resulting name back to the caller
  - `library.ts` — `createAsset` action
  - `plugin.ts` — `removePlugin`
  - `marketplace.ts` — sources + per-source catalogs (one fetch per
    source, errors per source, aggregated `allPlugins` view), import
    action that propagates the source name into the origin
  - `ai.ts` — AI status, settings, generation actions
- **All IPC goes through `src/lib/api.ts`.** No `invoke()` calls
  outside that file.
- **Routing is hash-based** (`createWebHashHistory`) for Tauri
  compatibility. Top-level routes:
  - `/bundles`, `/bundles/:name`
  - `/bundles/:name/harmonize` (per-hunk AI rewrite review)
  - `/bundles/generate` (chat-based bundle generation — placed before
    `/:name` in the router so it isn't captured as a bundle name)
  - `/recommend` (top-level — placed outside `/bundles/` to avoid
    shadowing `/bundles/:name`)
  - `/browse` → redirect `/browse/library`
    - `/browse/library` (plugin grid)
    - `/browse/library/:plugin` (per-plugin asset list, `__local__` for
      the local bucket)
    - `/browse/marketplace`
  - `/library/:kind/:name` (asset editor)
  - `/plugins/:name` (marketplace plugin detail)
  - `/plugins/:name/update` (per-hunk update review)
  - `/project`
  - `/settings` (AI backend + marketplaces)
- **Asset identity** across the UI is `${kind}:${name}` (see
  `assetKey` in `src/lib/types.ts`) — use it for set membership and
  v-for keys.
- **Plugin import status** logic lives in [`src/lib/origins.ts`](src/lib/origins.ts)
  (`pluginImportStatus(library, marketplace, plugin, hooks?, mcp?)`
  returns `not-imported | current | update | unknown`). Reuse it; don't
  re-derive in views. The hooks/mcp arguments cover plugins that ship
  only a `.mcp.json` and would otherwise look "not imported" despite
  having an MCP entry on disk.
- **Per-hunk diff review** is shared between
  [`BundleHarmonizeView`](src/views/BundleHarmonizeView.vue) and
  [`PluginUpdateView`](src/views/PluginUpdateView.vue). Both use
  `diffLines` from the `diff` npm package, group consecutive
  added/removed parts into hunks, and reconstruct the final content
  by walking the diff parts and respecting each hunk's `accepted`
  flag. If a third diff-review screen lands, extract this into a
  shared component.
- **Selected project path** is persisted in `localStorage` under
  `claude-kit:last-project`.
- **Auto-update is surfaced via a topbar button**, not a startup toast.
  The first `App.vue::onMounted` defers a `check()` from
  `tauri-plugin-updater` by a few seconds; when a newer release is
  found the button becomes clickable and shows the target version.
  Clicking calls `downloadAndInstall()` then `relaunch()` (the
  `tauri-plugin-process` dep). Fails open silently in dev / before
  signing keys are set — `console.warn` only.
- **Toolkit:** shadcn-vue components live in `src/components/ui/`
  (style `new-york`). Tailwind v4 with `@tailwindcss/vite`. Icons via
  `lucide-vue-next`. Toasts via `vue-sonner`. Markdown editor uses
  CodeMirror 6 (`@codemirror/lang-markdown` + `oneDark` theme).
  Diff-review screens use `diff` (npm) for line-level diffing.

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
cd src-tauri && cargo check --no-default-features    # fast type-check
cd src-tauri && cargo clippy --no-default-features   # lint
cd src-tauri && cargo test  --no-default-features    # network-tagged tests are #[ignore]'d; run with --ignored
```

**Why `--no-default-features`?** The default `custom-protocol` feature
makes Tauri's `generate_context!()` macro validate at compile time that
`frontendDist: "../dist"` (per `tauri.conf.json`) actually exists. Plain
`cargo check` therefore fails with `The "frontendDist" configuration is
set to "../dist" but this path doesn't exist` whenever the frontend
hasn't been built. The flag skips that check while still type-checking
all the regular Rust code. The release workflow doesn't need it because
`tauri-action` runs `npm run tauri build`, which triggers
`beforeBuildCommand: "npm run build"` and generates `dist/` first.

For the website:

```bash
cd website && npm install
cd website && npm run dev         # http://localhost:5173 with HMR
cd website && npm run build       # → website/.vitepress/dist/
```

There is no test runner configured for the frontend itself. Backend
modules carry **~145 unit tests** across `library`, `bundles`,
`harmonize`, `recommend`, `marketplace`, `settings`, `project`, and
`bundle_chat`. They mutate `CLAUDE_KIT_HOME` via env var, so they all
serialize on a crate-level `HOME_LOCK: Mutex<()>` declared in `main.rs`
— when adding a new test module, use `crate::HOME_LOCK` (not a
per-module static) so tests from different modules don't race on the env
var when the test runner parallelises across modules.

For producing real installable builds (icon prep, per-OS gotchas,
distribution caveats), see [`docs/build.md`](docs/build.md).

## Gotchas

- **`BundleEntryKind` vs `AssetKind`** — `BundleEntryKind` is the wider
  enum used in bundle JSON (`skills | commands | agents | hooks | mcp`).
  `AssetKind` is the narrower enum for the markdown-asset flows
  (library scan, harmonize, update). Use `BundleEntryKind::as_asset_kind()`
  to convert; it returns `None` for hooks/mcp. Never store a `BundleRef`
  with an `AssetKind` directly — the bundle JSON will reject it at parse
  time.
- **Local hooks require a `.meta.json` sidecar.** Every script under
  `hooks/__local__/` needs a matching `<file>.meta.json` carrying
  `LocalHookMeta { event, matcher }`. Without it the hook can't be
  re-registered into `settings.json` on another machine (or after a
  clean install). Always write them together via `create_local_hook` /
  `update_local_hook` — never write the script file directly.
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
- **Update detection only kicks in for assets imported with version
  tracking**. Older imports show as `unknown` (in-library, can't tell
  if updates exist) until they're re-imported. The `apply_update`
  flow refreshes origins for *every* asset of the plugin including
  ones the user kept unchanged, so the badge clears even on partial
  acceptance.
- **AI generation may fail open.** If neither the Claude CLI nor a
  configured API are detected, every Generate / Harmonize / Recommend
  button shows an inline link to Settings. Don't spawn the dialogs
  without checking `aiStatus.mode !== "none"` first.
- **Marketplace name is the stable identifier.** Origins record the
  marketplace by `name`, not URL — the user can edit a source's URL
  later (or several sources can be reachable at multiple URLs) without
  breaking origin matching. The official `claude-plugins-official`
  entry preserves that name forever.
- **For skills (multi-file directories), the update flow only diffs
  `SKILL.md`.** Other files inside the skill folder aren't compared
  or written. Acceptable v1 limitation; revisit if multi-file skills
  become common upstream.
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
