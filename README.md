# claude-kit

Desktop app (Tauri + Vue 3) pour gérer tes skills, commands et agents Claude Code sous forme de **bundles réutilisables**.

## Concept

- Ta **library** centrale vit dans `~/.claude-assets/library/{skills,commands,agents}`
- Tu crées des **bundles** (`python-backend`, `web-frontend`, etc.) qui listent des assets
- Tu sélectionnes un projet, tu cliques "Apply" → des **symlinks relatifs** sont créés dans `.claude/` du projet
- Tu peux aussi toggle un asset individuellement (extras hors bundle)

## Fonctionnalités

- **Library** : parcours, recherche (name / description / tags / kind), édition inline du contenu (`✎`)
- **Bundles** : création, édition par **drag & drop** depuis la library, ou via le picker "Add from library…"
- **Import** : bouton "Import…" dans la library pour aspirer skills/commands/agents depuis un dossier plugin existant
- **Apply** : un bundle en un clic (mode *additif* ou *replace*) ou asset par asset

## Prérequis

- Node 20+
- Rust stable (via [rustup](https://rustup.rs))
- Sur Linux : dépendances système Tauri (`libwebkit2gtk-4.1`, etc.) — voir [docs Tauri](https://v2.tauri.app/start/prerequisites/)

## Démarrer

```bash
npm install
npm run tauri dev
```

## Build pour distribution

```bash
npm run tauri build
# binaire: src-tauri/target/release/bundle/
```

## Créer sa library

L'app crée automatiquement `~/.claude-assets/` au premier lancement. Pour remplir :

**Option 1 : via l'UI (recommandé)**

Clique sur **Import…** dans la colonne Library, choisis un dossier plugin contenant `skills/`, `commands/` et/ou `agents/`. Les entrées déjà présentes sont skippées.

**Option 2 : copier à la main depuis un plugin existant**

```bash
cp -r ~/.claude/plugins/everything-claude-code/skills/* ~/.claude-assets/library/skills/
cp ~/.claude/plugins/everything-claude-code/commands/*.md ~/.claude-assets/library/commands/
```

**Option 3 : versionner la library dans un repo Git à toi**

```bash
cd ~/.claude-assets
git init
git remote add origin git@github.com:toi/claude-assets.git
```

## Structure d'un skill

```
~/.claude-assets/library/skills/python-tdd/
└── SKILL.md     # frontmatter YAML + contenu markdown
```

Exemple `SKILL.md` :

```markdown
---
name: python-tdd
description: Guide Claude through strict TDD cycles in Python with pytest.
tags: [python, testing]
---

# Python TDD

Red, green, refactor. Use pytest.
...
```

## Raccourcis

- Dans l'éditeur de contenu : `Ctrl+S` pour sauver, `Esc` pour fermer (prompt si non sauvegardé).

## Roadmap

- [x] Commande d'import depuis un plugin
- [x] Recherche/filtre dans la library
- [x] Drag & drop pour éditer les bundles
- [x] Éditeur de contenu intégré
- [ ] Icône de l'app
