#!/bin/bash
# .devcontainer/setup-dev.sh
set -euo pipefail

cp ${HOME}/.gitconfig /tmp/gitconfig 2>/dev/null || touch /tmp/gitconfig

# Installer Claude Code
curl -fsSL https://claude.ai/install.sh | bash

# Autoriser Git à lire n'importe quel repo monté (chemin variable selon le devcontainer)
git config --global --add safe.directory '*'

# Prompt Git dans le terminal
cat >> ${HOME}/.bashrc << 'EOF'
parse_git_branch() { git branch 2>/dev/null | grep "^*" | sed "s/* //"; }
export PS1="\[\033[01;34m\]\w\[\033[33m\] (\$(parse_git_branch))\[\033[00m\] \$ "
EOF

# Filet de sécurité : si un volume existait déjà en root (cas d'un volume
# créé avant le fix de permissions du Dockerfile), on le rebascule sur vscode.
# Le Dockerfile pré-crée ces dossiers avec les bonnes permissions, donc en
# théorie ces chown sont des no-op.
fix_owner() {
    local path="$1"
    if [ -e "$path" ] && [ "$(stat -c %u "$path")" != "$(id -u)" ]; then
        echo "→ $path root-owned, fix permissions…"
        sudo chown -R "$(id -u):$(id -g)" "$path"
    fi
}
fix_owner node_modules
fix_owner src-tauri/target
fix_owner /usr/local/cargo/registry

# Installer les deps Node si le volume est vide (premier lancement / après un wipe)
if [ ! -d node_modules/.bin ]; then
    npm install
fi

echo '✅ Devcontainer prêt ! Lance: cargo --version && node --version'