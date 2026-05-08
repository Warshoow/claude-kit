# claude-kit — Roadmap

## Fait ✅

- Fix écran blanc (circular import stores Pinia)
- Fix fenêtre invisible Linux (`transparent: false`)
- Icône intégrée + tous les formats générés
- Palette de couleurs cohérente avec l'icône (orange #F97316)
- Theme toggler dark/light (persisté localStorage, sans flash)
- Contraste amélioré + scrollbar visible + suppression backdrop-blur
- Build Linux `.deb` fonctionnel sur Ubuntu WSL
- Bind mount devcontainer → `~/claude-kit-dist/`
- Onboarding first-run (WelcomeScreen avec dismiss persisté)
- Import complet plugins : `hooks/` + `.mcp.json` copiés dans la library
- CI/CD GitHub Actions : `ci.yml` + `release.yml` (multi-OS sur tag `v*`)
- Flow complet testé et validé sur WSL
- UI Hooks & MCP : list, apply (symlink), viewer contenu, merge project
- Import status marketplace aware hooks/mcp (plugins mcp-only → "current")
- Viewer contenu hooks (collapsible), bouton Docs → README, Remove plugin

---

## Court terme — avant publication

- Transparence macOS (cosmétique, dernier moment)
- README public + pousser sur GitHub + tag `v0.2.0`

---

## Intégration IA

Deux usages distincts, même infra sous-jacente.

### Features

**1. Générateur d'assets**
Bouton "Generate with AI" dans l'éditeur d'assets. L'user décrit ce qu'il veut en langage naturel, l'IA produit le contenu markdown du skill, command ou agent.

**2. Recommandeur de bundle**
L'user décrit son besoin ("setup TypeScript backend avec tests et git hooks"), l'IA choisit quels plugins importer depuis la marketplace et quels assets regrouper en bundle. Valeur principale : découverte dans un catalogue qui grossit vite.

**3. Harmonisateur de bundle**
Bouton "Harmonize" sur un bundle. Les assets viennent de plugins différents, écrits par différents auteurs avec différents tons, conventions, structures de fichiers, vocabulaire. L'IA réécrit pour rendre l'ensemble cohérent (ton, terminologie, format de frontmatter, références croisées entre assets) tout en préservant le sens de chaque asset.

UX critique : **diff review obligatoire**, style git côte à côte ou unifié — lignes ajoutées en vert, retirées en rouge — avec accept/reject par hunk ou par asset entier. Sinon, risque de casser sémantiquement un asset sans s'en rendre compte. Implique aussi un système de revert/historique par asset (pas en place aujourd'hui).

Bonus : pourrait surfacer les conflits / doublons entre assets ("ces deux skills font la même chose, garder lequel ?").

### Backend IA — deux modes, détection automatique

**Mode Claude Code CLI (prioritaire, zéro config)**

Appel subprocess depuis Rust :
```bash
claude -p "..." --output-format json
```
Tous les users de claude-kit ont `claude` installé par définition. Détection du binaire au lancement (`PATH`, `~/.claude/local/claude`, etc.).

**Mode API key (fallback si `claude` non trouvé)**

Champ dans les settings de l'app. Compatible avec n'importe quel provider via base URL + clé optionnelle :
- Modèles propriétaires : Anthropic, OpenAI, Google, etc.
- Modèles locaux / self-hosted / open-source : Ollama (`http://localhost:11434`), LM Studio, vLLM, ou tout serveur compatible OpenAI API

Pas de liste de providers figée — l'user entre la base URL et la clé, ça couvre tout.

### UX settings IA

- Détection auto au lancement : si `claude` trouvé → *"Using Claude Code (your subscription)"*
- Sinon → formulaire : base URL + API key
- Ordre d'implémentation recommandé : (1) générateur d'assets — scope réduit, teste le plumbing IA. (2) Harmonisateur — réutilise la même infra, ajoute la review diff. (3) Recommandeur — le plus complexe car nécessite de raisonner sur le catalogue marketplace.

---

## Marketplaces multiples

Aujourd'hui l'app est câblée sur `claude-plugins-official` (Anthropic). L'idée : permettre d'ajouter d'autres sources.

- Gestion d'une liste de marketplaces dans les settings (URL du `marketplace.json` + nom d'affichage)
- La marketplace officielle reste présente par défaut et non supprimable
- Browse Marketplace affiche les plugins de toutes les sources actives, avec un indicateur d'origine par plugin
- Origins/import tracking reste cohérent (`marketplace` field dans `.origins.json` identifie déjà la source)
- Cas d'usage : marketplace communautaire, marketplace privée d'équipe, fork local pour tests

## Long terme

- CLI `ck` — interface ligne de commande pour les mêmes opérations (apply bundle, import plugin, etc.)
- Site web statique de présentation, hébergé via GitHub Pages (mono-repo, dossier `website/`). Stack pressentie : **VitePress** (Vue-based, matches la stack du projet, réutilise shadcn-vue + Tailwind). Alternative : Astro pour un design plus custom. Custom domain optionnel (config DNS CNAME). Sert de landing + showcase + lien vers les releases.
