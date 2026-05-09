---
layout: home

hero:
  name: "claude-kit"
  text: "Bundle your Claude Code assets. Apply them anywhere."
  tagline: "A desktop app for managing skills, commands, agents, hooks and MCP configs as reusable bundles — imported from the official marketplace, polished by AI, applied to any project via symlinks in `.claude/`."
  image:
    src: /icon.png
    alt: claude-kit
  actions:
    - theme: brand
      text: Download for your OS
      link: https://github.com/Warshoow/claude-kit/releases
    - theme: alt
      text: View on GitHub
      link: https://github.com/Warshoow/claude-kit

features:
  - icon: 📦
    title: Plugin library
    details: |
      Import plugins from the official marketplace or any local folder. Skills,
      commands, agents, hooks and MCP configs all land in a single library you
      curate over time.
  - icon: 🎯
    title: Bundles
    details: |
      Group the assets that go together (e.g. python-backend, web-frontend) and
      apply them to any project in one click — relative symlinks under
      <code>.claude/</code>, no duplication on disk.
  - icon: ✨
    title: AI generator
    details: |
      Describe an asset in plain language, the model produces a Claude-Code-shaped
      file ready to edit. Works via the Claude CLI subprocess or any
      OpenAI-compatible API.
  - icon: 🪄
    title: Bundle harmonizer
    details: |
      Rewrite every asset in a bundle so it reads like one author wrote it.
      Per-hunk diff review (green / red, accept or reject) before anything hits
      disk.
  - icon: 🧠
    title: Bundle recommender
    details: |
      Tell the AI what you're trying to do and get a ready-made bundle:
      curated plugins to import, specific assets to include, with per-line
      checkboxes to refine.
  - icon: 🔄
    title: Force-overwrite updates
    details: |
      When a marketplace plugin gains a new version, review the diff hunk by
      hunk before overwriting your local copy. Local edits are never silently
      replaced.
  - icon: 🛍️
    title: Multiple marketplaces
    details: |
      Add any community or private marketplace alongside the official one — just
      paste the <code>marketplace.json</code> URL. Plugins from all sources show
      up in one searchable grid with a source badge.
  - icon: 🔧
    title: Built-in editor
    details: |
      CodeMirror 6 with Markdown highlighting, edit/preview toggle and
      <code>Ctrl+S</code> to save. Edits live in your library, not in the
      project — apply when ready.
---

<div class="vp-doc kit-section">

## How it works

```
~/.claude-assets/
├── library/
│   ├── skills/        # each is a directory with SKILL.md
│   ├── commands/      # flat .md files
│   ├── agents/        # flat .md files
│   ├── hooks/         # <plugin>/<script>
│   ├── mcp/           # <plugin>.json
│   └── .origins.json  # provenance tracking
└── bundles/
    └── python-backend.json
```

1. **Import** plugins from the marketplace or any local folder
2. **Bundle** the assets you actually use, name and describe them
3. **Apply** a bundle to a project — symlinks land in `.claude/`
4. Switch project, apply another bundle. Each project keeps its own set.

Symlinks are **relative**, so the layout survives moving the project or the library.

</div>

<div class="kit-section">

## Screenshots

<div class="kit-screenshots">

![Browse Library — plugin grid with skill, hook and MCP badges](/screenshots/library.png)
*Browse Library — imported plugins with asset counts*

![Browse Marketplace — official plugin catalog](/screenshots/marketplace.png)
*Browse Marketplace — 100+ official plugins*

![Bundle editor — pick assets from the library](/screenshots/bundle-editor.png)
*Bundle editor — pick assets from your library*

![Asset editor — CodeMirror with markdown highlighting](/screenshots/asset-editor.png)
*Asset editor — CodeMirror 6 with Markdown highlighting*

</div>

</div>

<div class="kit-section vp-doc">

## What's next

The roadmap lives in [docs/roadmap.md](https://github.com/Warshoow/claude-kit/blob/master/docs/roadmap.md). Highlights:

- **CLI tool `ck`** — `ck apply <bundle>` from a project terminal
- **Refinement of the AI flows** based on real-user feedback
- **Multiple marketplace sources** — already shipped

</div>

<div class="kit-section vp-doc">

## Stack

| Layer    | Tech                                                              |
| -------- | ----------------------------------------------------------------- |
| Shell    | [Tauri 2](https://tauri.app) (Rust)                               |
| Frontend | [Vue 3](https://vuejs.org) + [Pinia](https://pinia.vuejs.org)     |
| Styling  | [Tailwind v4](https://tailwindcss.com) + [shadcn-vue](https://www.shadcn-vue.com) |
| Editor   | [CodeMirror 6](https://codemirror.net)                            |
| Icons    | [Lucide](https://lucide.dev)                                      |

</div>
