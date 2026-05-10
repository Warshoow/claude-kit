<div align="center">
  <img src="src/assets/icon.png" alt="claude-kit" width="96" />
  <h1>claude-kit</h1>
  <p><strong>Manage your Claude Code assets as reusable bundles — and apply them to any project in one click.</strong></p>
  <p>
    <img alt="Platform" src="https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-orange" />
    <img alt="Tauri" src="https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white" />
    <img alt="Vue" src="https://img.shields.io/badge/Vue-3-4FC08D?logo=vue.js&logoColor=white" />
  </p>
</div>

---

claude-kit is a desktop app for developers who use [Claude Code](https://claude.ai/code). It gives you a central library of skills, commands, agents, hooks and MCP server configs — organized as plugins, grouped into bundles, and applied to projects via symlinks in `.claude/`.

Import from the official [claude-plugins-official](https://github.com/anthropics/claude-plugins-official) marketplace, from any local plugin folder, or create your own assets from scratch.

## Screenshots

<table>
  <tr>
    <td><img src="images/library.png" alt="Browse Library — plugin grid with skill, hook and MCP badges" /></td>
    <td><img src="images/marketplace.png" alt="Browse Marketplace — official plugin catalog" /></td>
  </tr>
  <tr>
    <td align="center"><sub>Browse Library — imported plugins with asset counts</sub></td>
    <td align="center"><sub>Browse Marketplace — 100+ official plugins</sub></td>
  </tr>
  <tr>
    <td><img src="images/adding assets into bundle.png" alt="Bundle editor — pick assets from the library" /></td>
    <td><img src="images/Create or edit assets.png" alt="Asset editor — CodeMirror with markdown highlighting" /></td>
  </tr>
  <tr>
    <td align="center"><sub>Bundle editor — pick assets from your library</sub></td>
    <td align="center"><sub>Asset editor — CodeMirror 6 with Markdown highlighting</sub></td>
  </tr>
</table>

## How it works

1. **Import** plugins from the marketplace or a local folder — skills, commands, agents, hooks and MCP configs land in `~/.claude-assets/library/`
2. **Bundle** the assets you want together (`python-backend`, `web-frontend`, …)
3. **Apply** a bundle to a project — claude-kit creates relative symlinks in `<project>/.claude/`
4. Switch project, apply another bundle — each project keeps its own set of assets

```
~/.claude-assets/
├── library/
│   ├── skills/          # each is a directory with SKILL.md
│   ├── commands/        # flat .md files
│   ├── agents/          # flat .md files
│   ├── hooks/           # <plugin>/<script> — hook files
│   ├── mcp/             # <plugin>.json — MCP server configs
│   └── .origins.json    # provenance (plugin, version, imported_at)
└── bundles/
    └── python-backend.json
```

Symlinks are **relative** — they survive if you move the project or the library.

## Features

**Library & plugins**
- Plugin grid with asset counts (skills, commands, agents, hooks, MCP)
- Search by plugin name or asset name/description/tag
- Import from any marketplace or local folder, or create assets from scratch with the *+ New* button
- Automatic update detection: badge shows when a newer version is available
- View hook file contents inline, browse MCP server configs as JSON
- Docs button: opens the plugin README fetched from GitHub
- Remove any plugin in one click (with symlink safety warning)

**Bundles**
- Create named bundles and pick assets from your library
- Apply *additively* (add on top) or with *replace* (clean then apply)
- Each bundle tracks which assets it owns in the project view

**Project view**
- Dashboard showing active bundles, standalone assets, and installed hooks
- Clean all: removes every symlink claude-kit created, leaves hand-written files untouched

**Marketplace**
- Bundles plugins from the official `claude-plugins-official` catalog (100+ plugins) by default
- Add any number of additional marketplaces from Settings — paste a `marketplace.json` URL, the app validates and merges it into the browse grid with a per-plugin source badge
- Per-plugin status: In Library / Update available / Not imported
- Works for plugins that only ship MCP configs (like Playwright)

**Plugin updates with hunk-level review**
- When a marketplace plugin gains a new version, click *Update…* on any *Update available* badge
- The app downloads the upstream tarball, computes a per-asset diff, and shows a `git`-style review screen — accept or reject each hunk before anything hits disk
- Local files are never silently overwritten; rejected hunks keep your edits
- Origins refresh automatically on apply so the badge clears

**AI integration** *(opt-in, configured in Settings)*

Two backends, auto-detected: the local Claude Code CLI (zero-config — uses your existing subscription) or any OpenAI-compatible API endpoint (Anthropic, OpenAI, Ollama, LM Studio, vLLM…).

- **Generate an asset** — describe what you want, the model produces a Claude-Code-shaped file. Available both when creating a new asset and when editing an existing one (with a *Refine* mode that sends current content as context).
- **Harmonize a bundle** — rewrite every asset of a bundle so it reads like one author wrote it. Per-hunk diff review with accept/reject before any write.

**MCP support**
- Import `.mcp.json` from any plugin
- Merge MCP server configs into `<project>/.claude/mcp.json` without overwriting existing keys

**Asset editor**
- CodeMirror 6 with Markdown syntax highlighting
- Edit / Preview toggle, `Ctrl+S` to save

**UI**
- Dark / light theme (persisted, no flash on load)
- Custom slim window chrome, native traffic lights on macOS
- Runs on Linux (including WSLg), macOS, and Windows

## Prerequisites

- **Node 20+** and **Rust stable** ([rustup](https://rustup.rs/))
- **Linux:** system dependencies required by Tauri:
  ```bash
  sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
  ```
  On Ubuntu 24.04, use `libasound2t64` instead of `libasound2`.
- **macOS / Windows:** no extra dependencies

## Getting started

```bash
git clone https://github.com/Warshoow/claude-kit
cd claude-kit
npm install
npm run tauri dev
```

The app creates `~/.claude-assets/` on first launch. Override with `CLAUDE_KIT_HOME` (useful for testing):

```bash
CLAUDE_KIT_HOME=/tmp/kit-test npm run tauri dev
```

## Building for distribution

```bash
npm run tauri build   # → src-tauri/target/release/bundle/
```

> **Note:** Tauri does not cross-compile. Build on the target OS (Linux for `.deb`/`.AppImage`, macOS for `.dmg`, Windows for `.msi`/`.exe`).

Pre-built binaries are available on the [Releases](../../releases) page.

See [`docs/build.md`](docs/build.md) for the full procedure including icon generation and WSL-specific notes.

## Roadmap

See [`docs/roadmap.md`](docs/roadmap.md) for the full picture. The big features are now in: AI flows (generator, harmonizer), force-overwrite plugin updates, multiple marketplaces, and a VitePress landing site at `website/`.

What's still to do:

- **CLI tool `ck`** — `ck apply <bundle>` from a project terminal (plan in [`docs/cli-roadmap.md`](docs/cli-roadmap.md))
- **Streaming** for the AI generator (today the dialog blocks until the full response arrives)
- **macOS window transparency** — last cosmetic tweak before the first public tag

## Tech stack

| Layer | Tech |
|-------|------|
| Shell | [Tauri 2](https://tauri.app) (Rust) |
| Frontend | [Vue 3](https://vuejs.org) + [Pinia](https://pinia.vuejs.org) |
| Styling | [Tailwind v4](https://tailwindcss.com) + [shadcn-vue](https://www.shadcn-vue.com) |
| Editor | [CodeMirror 6](https://codemirror.net) |
| Icons | [Lucide](https://lucide.dev) |

## License

MIT
