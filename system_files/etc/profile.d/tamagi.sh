# shellcheck shell=bash
# Tamagi — ambiente do shell interativo (bash).
# Para desligar tudo isto para o seu usuário: touch ~/.config/tamagi/no-shell-setup

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

# Cache dos scripts de inicialização do shell: evita 4 subprocessos em todo novo terminal.
_tamagi_shell_cache="${XDG_CACHE_HOME:-$HOME/.cache}/tamagi/shell-init"
_tamagi_eval_cached() {
    local _name=$1 _bin=$2 _cache; shift 2
    _bin=$(command -v "$_bin" 2>/dev/null) || return 0
    _cache="$_tamagi_shell_cache/$_name.bash"
    if [ ! -r "$_cache" ] || [ "$_bin" -nt "$_cache" ]; then
        mkdir -p "$_tamagi_shell_cache" 2>/dev/null || true
        "$@" > "$_cache.tmp.$$" 2>/dev/null && mv -f "$_cache.tmp.$$" "$_cache" 2>/dev/null || { rm -f "$_cache.tmp.$$"; return 0; }
    fi
    # shellcheck source=/dev/null
    . "$_cache"
}
if command -v starship >/dev/null 2>&1; then
    [ -e "${XDG_CONFIG_HOME:-$HOME/.config}/starship.toml" ] \
        || export STARSHIP_CONFIG="${STARSHIP_CONFIG:-/usr/share/tamagi/starship.toml}"
fi
# mise grava o PATH atual na ativação: nunca vai para o cache (senão ignora mudanças de PATH / no-rust-defaults).
if command -v mise >/dev/null 2>&1; then eval "$(mise activate bash)"; fi
_tamagi_eval_cached zoxide zoxide zoxide init bash
_tamagi_eval_cached fzf fzf fzf --bash
_tamagi_eval_cached starship starship starship init bash
unset -f _tamagi_eval_cached
unset _tamagi_shell_cache

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
