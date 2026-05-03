# claude-kit

Desktop app (Tauri 2 + Vue 3) pour gérer tes **skills, commands et agents
Claude Code** sous forme de bundles réutilisables, avec import depuis le
marketplace officiel.

## Concept

- Une **library** centrale vit dans `~/.claude-assets/library/{skills,commands,agents}/`
- Tu crées des **bundles** (`python-backend`, `web-frontend`, etc.) qui
  listent les assets que tu veux appliquer ensemble
- Tu sélectionnes un projet, tu cliques **Apply** → des **symlinks
  relatifs** sont créés dans `<project>/.claude/`
- Tu peux aussi importer des plugins depuis un dossier local ou depuis
  le **marketplace officiel** [`claude-plugins-official`](https://github.com/anthropics/claude-plugins-official),
  avec détection automatique des mises à jour disponibles

## Vue d'ensemble de l'UI

L'app a quatre zones principales (router top-level) :

- **My Bundles** — grille des bundles créés, click pour éditer
- **Browse > Library** — grille des plugins importés (un bucket *Local*
  pour les assets créés à la main), click pour voir/installer/éditer
  les assets
- **Browse > Marketplace** — grille des plugins disponibles depuis
  `claude-plugins-official`, badge `In library` ou `Update available`
  selon l'état
- **Project** — vue dashboard du projet sélectionné, ce qui est installé,
  bouton clean

## Fonctionnalités

- **Library** : grille de plugins, recherche par nom de plugin **ou**
  d'asset, drilldown vers les assets d'un plugin
- **Création** : bouton `+ New` pour créer un skill / command / agent
  vide directement (avec ouverture immédiate dans l'éditeur)
- **Import local** : aspirer skills/commands/agents depuis un dossier
  plugin existant — les fichiers déjà présents sont skippés
- **Marketplace** : liste les plugins officiels, télécharge à la demande,
  trace l'origine (`marketplace`, `plugin`, `version`, `git_ref`,
  `imported_at`) dans `~/.claude-assets/library/.origins.json`
- **Détection des updates** : compare la version stockée à la version
  marketplace courante → badge `Update available v1.0 → v1.1`
- **Bundles** : création, édition par drag & drop ou via picker, apply
  *additif* ou *replace*
- **Éditeur d'asset** : page dédiée avec **CodeMirror 6** (markdown +
  syntax highlighting) et preview rendue
- **Window chrome custom** : top-bar slim + draggable, traffic lights
  natifs sur macOS, controls personnalisés Windows/Linux

## Prérequis

- Node 20+
- Rust stable (via [rustup](https://rustup.rs/))
- Sur Linux : dépendances système Tauri (`libwebkit2gtk-4.1` & co) —
  voir [docs Tauri](https://v2.tauri.app/start/prerequisites/)

## Démarrer

```bash
npm install
npm run tauri dev
```

L'app crée automatiquement `~/.claude-assets/` au premier lancement.

## Build pour distribution

Voir [`docs/build.md`](docs/build.md) pour la procédure complète
(génération des icônes, build par OS, troubleshooting). En résumé :

```bash
npm run tauri icon            # une fois — génère toutes les variantes
# puis update bundle.icon dans tauri.conf.json
npm run tauri build           # outputs dans src-tauri/target/release/bundle/
```

⚠️ **Tauri ne cross-compile pas** : tu dois compiler sur l'OS cible.
Pour produire un installateur Windows, lance `npm run tauri build`
**depuis Windows**, pas depuis WSL.

## Override du chemin de la library

Pour les tests ou un setup non-standard, `CLAUDE_KIT_HOME` redirige
toute la persistance ailleurs :

```bash
CLAUDE_KIT_HOME=/tmp/kit-test npm run tauri dev
```

## Structure d'un asset

**Skill** = un dossier avec un `SKILL.md` :

```
~/.claude-assets/library/skills/python-tdd/
└── SKILL.md     # frontmatter YAML + contenu markdown
```

**Command / agent** = un fichier `.md` plat :

```
~/.claude-assets/library/commands/refactor.md
~/.claude-assets/library/agents/code-reviewer.md
```

Le frontmatter (`description`, `tags`) est lu par l'app pour la
recherche et l'affichage en listing.

## Raccourcis

- Dans l'éditeur d'asset : `Ctrl+S` pour sauver

## Roadmap

Voir aussi les docs dédiées :
- [`docs/cli-roadmap.md`](docs/cli-roadmap.md) — futur outil CLI
  (`ck apply <bundle>` depuis un terminal projet)
- [`docs/build.md`](docs/build.md) — guide de build natif

À faire :
- [ ] **CLI tool** : `ck apply` / `ck list` / `ck clean` depuis le
      terminal d'un projet
- [ ] **Update force-overwrite** : action *Update* qui remplace
      réellement les fichiers modifiés (avec garde-fou pour les éditions
      locales). Aujourd'hui *Update* = re-import qui ajoute mais
      n'écrase pas.
- [ ] **Full plugin import** : capter aussi `hooks/`, `mcp.json` et
      autres artefacts hors skills/commands/agents
- [ ] **Cache disque** du `marketplace.json` (aujourd'hui re-fetché à
      chaque démarrage)
- [ ] Code signing pour distribution publique
