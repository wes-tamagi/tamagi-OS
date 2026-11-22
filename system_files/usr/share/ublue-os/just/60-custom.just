# vim: set ft=make :
# Tamagi recipes (shown in `ujust --list`). Only what has no `tamagi` command:
# everything else lives in one place — run `tamagi` to see the list.

# Install the LazyVim configuration for Neovim
setup-lazyvim:
    #!/usr/bin/bash
    set -euo pipefail
    dir="${XDG_CONFIG_HOME:-$HOME/.config}/nvim"
    if [[ -e $dir ]]; then echo "$dir already exists; nothing was changed."; exit 0; fi
    git clone --depth 1 https://github.com/LazyVim/starter "$dir"
    tamagi-theme apps
    echo "LazyVim installed with the colours of the current theme. Open nvim: plugins download on first start."

# Follow another image (e.g.: ujust switch-image ghcr.io/you/tamagi:latest)
switch-image REF:
    sudo bootc switch {{ REF }}
