# shellcheck shell=bash
# Tamagi — ambiente do shell interativo (bash).
# Para desligar tudo isto para o seu usuário: touch ~/.config/tamagi/no-shell-setup

# O Claude Code da imagem fica em /usr (somente-leitura) e é atualizado com ela.
[ -x /usr/bin/claude ] && [ ! -x "$HOME/.local/bin/claude" ] && export DISABLE_AUTOUPDATER=1

# Botões do gum (Sim/Não) legíveis: vale para ujust e para as ferramentas tamagi-*.
# shellcheck source=/dev/null
[ -r /usr/share/tamagi/gum.env ] && . /usr/share/tamagi/gum.env

[ -n "${BASH_VERSION:-}" ] || return 0
case $- in *i*) ;; *) return 0 ;; esac
[ -e "${XDG_CONFIG_HOME:-$HOME/.config}/tamagi/no-shell-setup" ] && return 0

# Rust no shell interativo: sudo-rs e os utilitários do uutils na frente do
# PATH (os originais continuam em /usr/bin, para serviços e scripts).
# Para desligar só isto: touch ~/.config/tamagi/no-rust-defaults
if [ -d /usr/libexec/tamagi/rust-bin ] && [ ! -e "${XDG_CONFIG_HOME:-$HOME/.config}/tamagi/no-rust-defaults" ]; then
    case ":$PATH:" in *:/usr/libexec/tamagi/rust-bin:*) ;; *) PATH="/usr/libexec/tamagi/rust-bin:$PATH" ;; esac
fi

export EDITOR="${EDITOR:-nvim}"
export BAT_THEME="${BAT_THEME:-ansi}"

if command -v mise >/dev/null 2>&1; then eval "$(mise activate bash)"; fi
if command -v zoxide >/dev/null 2>&1; then eval "$(zoxide init bash)"; fi
if command -v fzf >/dev/null 2>&1; then eval "$(fzf --bash 2>/dev/null)"; fi
if command -v starship >/dev/null 2>&1; then
    [ -e "${XDG_CONFIG_HOME:-$HOME/.config}/starship.toml" ] \
        || export STARSHIP_CONFIG="${STARSHIP_CONFIG:-/usr/share/tamagi/starship.toml}"
    eval "$(starship init bash)"
fi

if command -v eza >/dev/null 2>&1; then
    alias ls='eza --group-directories-first --icons=auto'
    alias ll='eza -l --group-directories-first --icons=auto --git'
    alias la='eza -la --group-directories-first --icons=auto --git'
    alias lt='eza --tree --level=2 --icons=auto'
fi
if command -v dust >/dev/null 2>&1; then alias du='dust'; fi
if command -v delta >/dev/null 2>&1; then export GIT_PAGER="${GIT_PAGER:-delta}"; fi
alias n='nvim'
alias g='git'
alias lg='lazygit'
alias ff='fastfetch'
alias db='distrobox'
alias dev='distrobox enter dev'
alias ia='tamagi-agent'
