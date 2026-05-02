# 🦀 Devcontainer Rust + Tauri

## Mise en place

1. **Copie** le dossier `.devcontainer/` à la racine de ton projet
2. **Ouvre** le projet dans VS Code
3. **Ctrl+Shift+P** → `Dev Containers: Reopen in Container`
4. Attends que le build se termine (la première fois prend ~5 min)

## Créer un nouveau projet Tauri

Une fois dans le conteneur :

```bash
# Crée un projet Tauri v2 avec le frontend de ton choix
npm create tauri-app@latest mon-app

# Ou si tu veux un template spécifique :
npm create tauri-app@latest mon-app -- --template vanilla  # HTML/JS simple
npm create tauri-app@latest mon-app -- --template react    # React
npm create tauri-app@latest mon-app -- --template svelte   # Svelte
```

## Commandes utiles

```bash
# Lancer en mode dev (compile Rust + lance le frontend)
npm run tauri dev

# Builder l'app
npm run tauri build

# Recompilation auto quand tu modifies le Rust
cargo watch -x check

# Lancer les tests
cargo test
```

## Ce qui est inclus

| Outil               | Usage                                    |
|----------------------|------------------------------------------|
| Rust (stable)        | Langage principal                        |
| rust-analyzer        | Autocomplétion, erreurs en temps réel    |
| clippy               | Linter Rust                              |
| cargo-watch          | Recompile automatiquement                |
| Node.js 20 + npm     | Frontend Tauri (CLI fournie via @tauri-apps/cli) |
| CodeLLDB             | Debugger Rust dans VS Code               |

## Structure type d'un projet Tauri

```
mon-app/
├── src-tauri/         # Code Rust (backend)
│   ├── src/
│   │   └── main.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/               # Code frontend (HTML/JS/React...)
├── package.json
└── .devcontainer/     # ← Ce que tu viens d'installer
```

## Volumes Docker

Les dossiers `target/` et le registre Cargo sont dans des volumes Docker
nommés pour éviter les problèmes de performance I/O.
Si tu veux repartir de zéro :

```bash
docker volume rm rust-tauri-target rust-cargo-registry
```

## Pour ton projet Ollama

Quand tu seras prêt à connecter Ollama, tu pourras :
- Ajouter le port `11434` dans `forwardPorts` du devcontainer
- Utiliser la crate `reqwest` pour appeler l'API Ollama depuis Rust
- Ollama tourne sur ta machine hôte (WSL), le conteneur y accède via le réseau Docker
