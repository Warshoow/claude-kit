# claude-kit — Roadmap

## Fait ✅

### Foundations

- Fix écran blanc (circular import stores Pinia)
- Fix fenêtre invisible Linux (`transparent: false`)
- Icône intégrée + tous les formats générés
- Palette de couleurs cohérente avec l'icône (orange #F97316)
- Theme toggler dark/light (persisté localStorage, sans flash)
- Contraste amélioré + scrollbar visible + suppression backdrop-blur
- Build Linux `.deb` fonctionnel sur Ubuntu WSL
- Bind mount devcontainer → `~/claude-kit-dist/`
- Onboarding first-run (WelcomeScreen avec dismiss persisté)
- CI/CD GitHub Actions : `ci.yml` + `release.yml` (multi-OS sur tag `v*`)

### Library & plugins

- Import complet plugins : `hooks/` + `.mcp.json` copiés dans la library
- UI Hooks & MCP : list, apply (symlink), viewer contenu, merge project
- Import status marketplace aware hooks/mcp (plugins mcp-only → "current")
- Viewer contenu hooks (collapsible), bouton Docs → README, Remove plugin
- Décomposition plugin-centric de Browse > Library : grid de cartes par plugin + bucket Local pour les assets créés à la main
- Bouton "+ New" pour créer un skill/command/agent vide directement depuis l'app

### Marketplace

- **Marketplaces multiples** : settings pour ajouter d'autres `marketplace.json` URLs ; UI badge par source dans Browse > Marketplace ; backend valide l'URL via fetch + lit le `name` ; officielle non-supprimable
- **Détection des updates** : `version` + `git_ref` stockés au moment de l'import ; badge "Update available v1.0 → v1.1" sur les cartes quand divergence
- **Force-overwrite update flow** : bouton "Update…" → page `/plugins/:name/update` avec preview du diff par asset/hunk + checkboxes accept/reject ; refresh auto des origins après apply pour clear le badge

### Intégration IA

Trois features shipped avec backend dual mode (Claude CLI subprocess + API OpenAI-compatible avec settings UI pour configurer base URL/key/model) :

- **Générateur d'assets** : bouton "Generate with AI" dans l'éditeur + mode "Generate from prompt" dans le NewAssetDialog
- **Harmoniseur de bundle** : bouton "Harmonize" sur un bundle → page `/bundles/:name/harmonize` avec review per-hunk (accept/reject par bloc de lignes) avant apply
- **Recommandeur de bundle** : page `/recommend` avec checkboxes per-plugin et per-asset, bundle name/description éditables, validation contre collisions. **Bouton d'accès retiré de l'UI** — la feature reste codée et accessible en URL directe pour test, mais le prompt qu'on envoie au modèle ne lui montre que les noms+descriptions des plugins (pas leur contenu réel), donc l'IA hallucine régulièrement les noms d'assets. CurseForge ne fait pas non plus de recommandation auto — la curation est l'expression de l'expertise de l'user. À ressortir si on (a) la rebuilde project-aware via claude CLI agentic, ou (b) pré-cache le contenu des plugins pour fournir un catalogue d'assets garantis-existants au modèle.

### Site web

- Site statique landing à `website/` via VitePress 1.6
- Brand custom (palette orange #F97316, hero gradient, screenshots grid)
- GitHub Action `pages.yml` qui déploie automatiquement à chaque push touchant `website/**`

---

## Court terme — avant publication

- Transparence macOS (cosmétique, dernier moment)
- Tag `v0.3.0` + activation GitHub Pages dans les Settings du repo

---

## Marketplaces (extension future)

Aujourd'hui : ajout/suppression d'URLs custom, agrégation dans le browse, badge par source. Pistes d'évolution :

- Refresh auto périodique des catalogs (TTL côté frontend)
- Cache disque du dernier marketplace.json pour éviter le refetch au démarrage
- Marketplaces privées avec auth (header API key)

---

## Long terme

### CLI `ck`

Interface ligne de commande pour les mêmes opérations (apply bundle, import plugin, etc.). Plan complet dans [`cli-roadmap.md`](cli-roadmap.md).

### Affinements IA basés sur les retours

- Streaming de tokens dans le générateur (aujourd'hui blocking)
- "Discard changes" rapide sur l'éditeur d'asset après une génération qui ne plaît pas (Cmd-Z marche mais c'est pas évident)
- Surfacer les doublons / conflits entre assets dans l'harmoniseur (option qu'on avait notée à l'époque)

### Customisation hooks d'install

- Aujourd'hui : symlink, point. Pas de hook d'application avant/après apply (ex: lancer `npm install` dans le projet, ajouter une ligne au `.gitignore`, etc.)
- Faisable via un champ `post_apply: string[]` dans le bundle JSON ou un mécanisme d'extension côté Rust
