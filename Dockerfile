# ── Base : image officielle Rust de Microsoft ──────────────
FROM mcr.microsoft.com/devcontainers/rust:1-bookworm

# ── Dépendances système pour Tauri v2 ──────────────────────
# https://v2.tauri.app/start/prerequisites/#linux
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get install -y --no-install-recommends \
        # Build essentials
        build-essential \
        pkg-config \
        # Tauri / WebView
        libwebkit2gtk-4.1-dev \
        libgtk-3-dev \
        libayatana-appindicator3-dev \
        librsvg2-dev \
        libssl-dev \
        # Utilitaires
        curl \
        wget \
        file \
    && apt-get autoremove -y && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

# ── Node.js 20 LTS (pour le frontend Tauri) ───────────────
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

# ── Outils Rust supplémentaires ────────────────────────────
# tauri-cli n'est pas installé ici : le projet le fournit via @tauri-apps/cli (npm)
RUN rustup component add clippy rustfmt \
    && cargo install cargo-watch

# ── Pré-création des dossiers de volumes Docker ────────────
# Les volumes nommés (cf .devcontainer/devcontainer.json : claude-kit-node-modules,
# rust-tauri-target, rust-cargo-registry) sont créés en root par défaut quand
# ils s'attachent à un dossier inexistant. En pré-créant ces dossiers en tant
# que vscode dans l'image, le volume hérite des bonnes permissions au premier
# mount et `npm install` / `cargo build` peuvent écrire sans sudo.
ARG WORKSPACE=/workspaces/claude-kit
RUN mkdir -p $WORKSPACE/node_modules $WORKSPACE/src-tauri/target \
             /usr/local/cargo/registry \
    && chown -R vscode:vscode $WORKSPACE /usr/local/cargo/registry

# ── Vérification ───────────────────────────────────────────
RUN rustc --version && cargo --version && node --version && npm --version
