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
- Ordre d'implémentation recommandé : générateur d'assets d'abord (scope réduit, teste le plumbing), recommandeur de bundle ensuite

---

## Long terme

- CLI `ck` — interface ligne de commande pour les mêmes opérations (apply bundle, import plugin, etc.)
