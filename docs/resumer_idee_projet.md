# claude-kit — Récapitulatif de session

> Synthèse de la réflexion et des décisions prises pour concevoir un gestionnaire de skills/commands/agents pour Claude Code, sous forme d'app desktop.

---

## 1. Le problème de départ

### Contexte

- Utilisateur de `everything-claude-code` (plugin de workflow IA pour Claude Code), à laquelle l'utilisateur a contribué (guide d'optimisation des tokens référencé dans le README).
- **Frustrations** :
  - Le plugin "fait tout, et trop de trucs"
  - Installation dans `~/.claude` peu accessible et offrant peu de contrôle
  - Pas de granularité projet par projet

### Question initiale

> Comment installer des skills/plugins **dans le répertoire courant du projet**, plutôt que globalement ?

---

## 2. Première réponse : c'est natif dans Claude Code

Claude Code supporte officiellement les assets en scope projet :

| Niveau | Emplacement | Portée |
|---|---|---|
| Global | `~/.claude/skills/` | Tous projets |
| **Projet** | `.claude/skills/` (à la racine du repo) | Ce projet uniquement |

Idem pour `.claude/commands/` et `.claude/agents/`. **Les assets de projet ont priorité** sur les globaux de même nom.

Trois mécanismes natifs :
1. **`.claude/skills/` du projet** — placement direct, scanné automatiquement
2. **`claude --plugin-dir ./mon-plugin`** — pour tester un plugin local sans installer globalement
3. **Marketplace local** (un dossier `.claude-plugin/marketplace.json` dans le repo)

### Conclusion de cette étape

Pas besoin d'outil custom **juste** pour mettre des skills dans le projet. Le besoin réel est ailleurs.

---

## 3. Le vrai besoin émerge : un "skills manager"

### Reformulation du besoin

> Un **gestionnaire** pour skills/agents/commands. Pouvoir faire son "petit marché", packager ça en bundles réutilisables, et appliquer sur un projet en un geste.

### L'analogie fondatrice : CurseForge / modpacks

| CurseForge (Minecraft) | claude-kit |
|---|---|
| Dossier mods installés | Library centrale `~/.claude-assets/library/` |
| Modpack (Better MC, etc.) | Bundle (`python-backend`, `web-frontend`...) |
| `.minecraft/mods/` du profil | `.claude/skills/` du projet |
| Installer un modpack | `claude-kit apply <bundle>` |

**Insight clé** : le vrai problème n'est pas technique, c'est **cognitif**. Ce qui coûte du temps, c'est :
- Se souvenir de quoi on a et ce qui existe
- Décider ce qui va bien ensemble
- Au démarrage projet, avoir à *réfléchir* au lieu d'avoir un geste unique

CurseForge marche parce que tu ne *choisis pas 200 mods* à chaque fois, tu cliques "Better Minecraft". Quelqu'un (toi, une fois) a fait le travail de pensée. Après, c'est un clic.

---

## 4. Solutions existantes étudiées

| Outil | Forme | Couvre quoi | Verdict |
|---|---|---|---|
| **`sk` (caude-skill-manager)** | CLI Go | install/list/search/uninstall, registry GitHub | Le plus proche, mais asset par asset uniquement, pas de bundles |
| **`asm`** | CLI npm | Plus orienté création/publication | OK mais pas le bon use-case |
| **`Skild`** | "npm pour agents" | Registry centralisé multi-IDE | Multi-agent (Claude/Cursor/Windsurf) |
| **`ccpi`** | CLI + marketplace | 423 plugins / 2849 skills | Marketplace dédié |
| **`@leeovery/claude-manager`** | npm postinstall hooks | Skills as npm deps | Approche élégante mais différente |
| **`CCHub`** | App desktop Tauri | MCP + skills + CLAUDE.md + hooks | Le plus visuel mais pas axé bundles |

### Le constat

**Aucun de ces outils n'a la notion de bundles/packs réutilisables.** Tous gèrent l'install/uninstall asset par asset. La couche "modpack" n'existe pas.

### La question légitime : extension sans fork ?

- `sk` : binaire Go, pas de système de plugins. Seule extension propre = pointer vers son propre registry JSON. Ne permet pas d'ajouter des commandes ou la notion de bundles.
- `CCHub` : app desktop, encore moins extensible.

**Trois options sans fork** :
1. **Custom registry** — léger mais limité (pas de bundles)
2. **Wrapper CLI par-dessus** — ajouter uniquement la couche bundles, déléguer le reste à `sk`. Unix philosophy.
3. **Contribuer upstream** — proposer les bundles dans `sk` directement.

---

## 5. La vraie question : pourquoi tant de complexité ?

### Observation lucide de l'utilisateur

> "C'est globalement de la gestion de markdown, et de la copy ou link vers des dossiers spécifiques. J'ai du mal comprendre pourquoi il y a eu tant de complexité de développer."

**Il a raison.** Le cœur fonctionnel :

```bash
# "installer" un skill
ln -s ~/source/skill-name .claude/skills/

# "désinstaller"
rm -rf .claude/skills/skill-name
```

C'est tout. 90% du job. Le reste (registry, TUI, frontmatter parsing...) est de la sophistication :
- Vouloir faire "propre" (CLI framework, tests)
- Vouloir être générique (multi-sources, multi-agents)
- Vouloir être joli (TUI, couleurs)
- Vouloir exister comme produit (registry branded, ecosystem)

**Aucun de ces points n'est un vrai besoin pour l'utilisateur.**

---

## 6. Décision finale : app desktop Tauri + Vue 3

### Choix techniques

| Choix | Raison |
|---|---|
| **App desktop** (vs CLI) | Geste unique, click-click, expérience CurseForge |
| **Tauri 2** (vs Electron) | Binaire ~5 Mo vs 100+ Mo, Rust solide pour FS/symlinks |
| **Vue 3** | Préférence utilisateur, équilibre verbosité/structure |
| **Bundles JSON** simples | Lisibles, versionnables, hackable à la main si besoin |
| **Symlinks relatifs** | Édition library = MAJ automatique tous projets, portable |

### Échelle visée

5-15 bundles par stack précise (ex: `python-backend`, `web-frontend`, `data-science`, `devops`, `quick-prototyping`...).

---

## 7. Architecture livrée

### Structure du projet

```
claude-kit-app/
├── package.json              # Vue 3 + Tauri 2
├── vite.config.ts
├── tsconfig.json
├── index.html
├── README.md
├── src/                      # Frontend Vue
│   ├── main.ts
│   ├── App.vue               # Layout 3 colonnes + état global
│   ├── styles.css            # Thème dark sobre (Linear/Raycast-like)
│   ├── lib/
│   │   ├── types.ts          # Types partagés (miroir Rust)
│   │   └── api.ts            # Wrapper typé des commandes Tauri
│   └── components/
│       ├── LibraryColumn.vue # Étagère
│       ├── BundlesColumn.vue # Création/édition bundles
│       └── ProjectColumn.vue # État du projet courant
└── src-tauri/                # Backend Rust
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── build.rs
    └── src/
        ├── main.rs           # Commandes Tauri exposées
        ├── library.rs        # Scan ~/.claude-assets + frontmatter
        ├── bundles.rs        # CRUD bundles JSON
        └── project.rs        # Symlinks relatifs (Unix + Windows)
```

### Layout UI : 3 colonnes

```
┌────────────────────────────────────────────────────────────┐
│  claude-kit                       [Current project ▼]      │
├──────────────────┬───────────────────┬─────────────────────┤
│  LIBRARY         │  BUNDLES          │  THIS PROJECT       │
│                  │                   │                     │
│  skills/         │  ▸ python-backend │  Applied bundles:   │
│   ☐ python-tdd   │  ▸ web-frontend   │   python-backend    │
│   ☑ react-patt.. │  ▾ devops         │                     │
│  commands/       │    [tags ✓]       │  Extras (one-off):  │
│   ☐ review       │  [Apply][Replace] │   + security-aud.   │
│  agents/         │  [+ New bundle]   │  [Clean all]        │
└──────────────────┴───────────────────┴─────────────────────┘
```

### Layout filesystem

```
~/.claude-assets/                # Library centrale (peut être un repo Git perso)
├── library/
│   ├── skills/<name>/SKILL.md
│   ├── commands/<name>.md
│   └── agents/<name>.md
└── bundles/
    └── <bundle-name>.json       # Liste de refs vers les assets

<project>/
├── .claude/                     # Symlinks relatifs créés par apply
│   ├── skills/<name>    -> ../../../../.claude-assets/library/skills/<name>
│   ├── commands/<name>.md
│   └── agents/<name>.md
└── claude-kit.json              # Manifeste : quels bundles appliqués
```

### Fonctionnalités déjà fonctionnelles

- Sélection projet via dialog natif (mémorisé localStorage)
- Scan auto de `~/.claude-assets/library/`
- Toggle direct asset (cocher = apply, décocher = remove)
- CRUD bundles complet
- Apply bundle (normal + "replace" qui clean d'abord)
- Détection auto des bundles "fully applied" dans le projet
- Clean total
- **Symlinks relatifs avec safety check** : on ne supprime QUE ce qui est un symlink vers la library (pas de risque pour les fichiers manuels)

---

## 8. Roadmap

### Court terme (à venir)

- [ ] **Import depuis dossier local** (depuis un plugin existant comme `everything-claude-code`)
- [ ] Recherche/filtre dans la library
- [ ] Drag & drop pour éditer les bundles
- [ ] Icône de l'app

### Moyen terme : intégration marketplace

**Découverte importante** : le marketplace Claude Code n'est **pas une API REST propriétaire**. C'est juste un fichier `.claude-plugin/marketplace.json` hébergé sur GitHub. Le marketplace officiel = repo `anthropics/claude-plugins-official`.

Cela ouvre la voie à un **onglet "Discover"** dans l'app :

- Fetch le JSON via HTTPS GitHub raw (pas d'auth, pas de rate limit)
- Affichage en grille des plugins disponibles avec descriptions/keywords/catégories
- Multi-sources (officiel Anthropic + tiers comme `buildwithclaude` + repo perso)
- Clic install → téléchargement du sous-dossier du plugin → extraction dans la library
- Choix : installer le plugin entier OU le décomposer en skills/commands/agents pour bundling fin

Format type :
```json
{
  "name": "claude-plugins-official",
  "plugins": [
    {
      "name": "github-integration",
      "source": { "source": "github", "repo": "...", "path": "plugins/github" },
      "version": "1.0.0",
      "description": "...",
      "category": "...",
      "keywords": [...]
    }
  ]
}
```

Ajouts techniques nécessaires (~50 lignes Rust) :
- `reqwest` + `tokio` pour fetch HTTPS
- Lib d'extraction tar pour récupérer un subdir d'un repo (via `https://api.github.com/repos/{repo}/tarball`)
- Module `marketplace.rs` côté Rust
- Onglet "Discover" côté Vue

Cette feature transforme `claude-kit` en **alternative visuelle complète au `/plugin` de Claude Code**, avec en plus la couche bundles que personne d'autre n'a.

### Long terme (idées)

- Sync de la library via Git (pull/push depuis un repo perso)
- Édition in-place du contenu SKILL.md (Monaco editor ou ouverture dans `$EDITOR`)
- Stats d'usage par skill (fréquence d'application)
- Petit plugin Claude Code optionnel `/kit:apply` pour piloter depuis une session

---

## 9. Démarrage du projet

```bash
# Décompresser le zip livré
cd claude-kit-app
npm install
npm run tauri dev          # mode dev avec hot reload
# Quand satisfait :
npm run tauri build        # binaire dans src-tauri/target/release/bundle/
```

**Prérequis** : Node 20+, Rust stable, dépendances système Tauri sur Linux (`libwebkit2gtk-4.1` etc.).

**Premier remplissage de la library** :

```bash
# L'app crée automatiquement ~/.claude-assets/ au premier lancement.
# Pour la peupler :
cp -r ~/.claude/plugins/everything-claude-code/skills/* \
      ~/.claude-assets/library/skills/

cp ~/.claude/plugins/everything-claude-code/commands/*.md \
   ~/.claude-assets/library/commands/

# Optionnel : versionner la library
cd ~/.claude-assets && git init
git remote add origin git@github.com:toi/claude-assets.git
```

---

## 10. Principes de design retenus

À garder en tête pour les futures itérations :

1. **Le filesystem est l'API** — pas de DB, pas de format propriétaire. Markdown + JSON, lisibles à la main, hackables, versionnables.
2. **Symlinks > copies** — édition library = MAJ tous projets, pas de drift.
3. **Safety first sur les FS ops** — ne jamais toucher ce qu'on n'a pas créé. Vérifier que c'est notre symlink avant de supprimer.
4. **L'ergonomie est la valeur** — la plomberie est triviale, ce qui compte c'est l'UX qui supprime la charge cognitive.
5. **Petit et focalisé** — résister à la tentation de faire un "produit". Ce n'est pas SaaS, c'est un outil perso d'abord.
6. **Composabilité** — `claude-kit.json` versionnable, library clonable depuis Git. Tu peux partager ton setup, mais ce n'est pas l'objectif premier.

---

## 11. Liens utiles

- Repo officiel marketplace Anthropic : `anthropics/claude-plugins-official`
- Doc plugins Claude Code : https://code.claude.com/docs/en/discover-plugins
- Marketplace tiers populaire : `davepoon/buildwithclaude`
- Outils similaires inspirants : `sk` (caude-skill-manager), `CCHub`, `ccpi`
- Doc Tauri 2 : https://v2.tauri.app/

---

*Session synthétisée le 02/05/2026.*