# Sourced by screenshots.sh (inside a Fedora container): headless sway
# sessions with a keyboard and a pointer, the stub back ends on PATH and the
# repository's data files.
#   SHARE   directory with catalog.tsv, flatpaks.list, i18n/, profiles/
#   BIN     the tamagi-welcome binary
#   OUT     where the PNGs go
# Packages: sway grim wtype wlr-protocols-devel wayland-devel gcc dbus-daemon
#           cosmic-icon-theme google-noto-sans-fonts google-noto-sans-mono-fonts
# For the light theme, COSMIC's default theme files must exist (libcosmic
# falls back to dark without them): /usr/share/cosmic/com.system76.CosmicTheme.*
# from the cosmic-settings package (extracting just those files is enough).
HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
: "${SHARE:=$HERE/../../system_files/usr/share/tamagi}"
: "${BIN:=${CARGO_TARGET_DIR:-$HERE/../target}/release/tamagi-welcome}"
: "${OUT:=$PWD}"
WORK=$(mktemp -d)
ORIGINAL_PATH=$PATH
export WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=pixman
export XDG_CURRENT_DESKTOP=COSMIC LANG=C.UTF-8
export TAMAGI_CATALOG="$SHARE/catalog.tsv" TAMAGI_FLATPAKS="$SHARE/flatpaks.list"
export TAMAGI_I18N_DIR="$SHARE/i18n" TAMAGI_PROFILES="$SHARE/profiles"
unset LC_ALL LC_MESSAGES TAMAGI_LANG XDG_CONFIG_HOME DISPLAY

# Tools built once: the virtual pointer, and a sway without the file
# capability (cap_sys_nice) Fedora gives it, which a container may refuse.
build_tools() {
    local xml=/usr/share/wlr-protocols/unstable/wlr-virtual-pointer-unstable-v1.xml
    wayland-scanner client-header "$xml" "$WORK/wlr-virtual-pointer.h"
    wayland-scanner private-code "$xml" "$WORK/wlr-virtual-pointer.c"
    cc -o "$WORK/vpointer" -I"$WORK" "$HERE/vpointer.c" "$WORK/wlr-virtual-pointer.c" -lwayland-client
    install -m755 "$(command -v sway)" "$WORK/sway"
}

# session_start <width> <height> [dark|light]: a fresh home and compositor.
session_start() {
    export HOME=$(mktemp -d -p "$WORK") XDG_RUNTIME_DIR=$(mktemp -d -p "$WORK")
    chmod 700 "$XDG_RUNTIME_DIR"
    export PATH="$HERE/stubs:$ORIGINAL_PATH" STUB_LOG="$HOME/stub.log"
    unset WAYLAND_DISPLAY SWAYSOCK
    if [[ ${3:-dark} == light ]]; then
        mkdir -p "$HOME/.config/cosmic/com.system76.CosmicTheme.Mode/v1"
        echo false > "$HOME/.config/cosmic/com.system76.CosmicTheme.Mode/v1/is_dark"
    fi
    cat > "$HOME/sway.conf" <<CONF
output HEADLESS-1 mode ${1}x${2}
output HEADLESS-1 bg #404040 solid_color
default_border none
focus_follows_mouse no
for_window [title="^Password needed$"] floating enable
CONF
    "$WORK/sway" -c "$HOME/sway.conf" >"$HOME/sway.log" 2>&1 &
    SWAY_PID=$!
    for _ in $(seq 50); do
        WAYLAND_DISPLAY=$(cd "$XDG_RUNTIME_DIR" && ls wayland-* 2>/dev/null | grep -v lock | head -1)
        [[ -n $WAYLAND_DISPLAY ]] && break
        sleep 0.1
    done
    export WAYLAND_DISPLAY SWAYSOCK=$(ls "$XDG_RUNTIME_DIR"/sway-ipc.* | head -1)
    # wtype's virtual keyboard lives only while wtype runs; one that stays
    # keeps the keyboard capability on the seat, so the window keeps its focus.
    wtype -s 3600000 -k a &
    KEYBOARD_PID=$!
    # The same for the pointer (tools/vpointer.c), fed through a FIFO.
    mkfifo "$HOME/pointer.fifo"
    "$WORK/vpointer" "$1" "$2" <"$HOME/pointer.fifo" &
    POINTER_PID=$!
    exec 9>"$HOME/pointer.fifo"
    sleep 0.3
}

session_end() {
    exec 9>&-
    kill "${APP_PID:-}" "$KEYBOARD_PID" "$POINTER_PID" "$SWAY_PID" 2>/dev/null
    wait 2>/dev/null
}

# app [args…]: starts tamagi-welcome and waits for its window.
app() {
    "$BIN" "$@" >"$HOME/app.log" 2>&1 &
    APP_PID=$!
    sleep "${APP_WAIT:-2}"
}

shot() { sleep "${2:-0.6}"; grim "$OUT/$1.png" && echo "  $1.png"; }
key() { wtype -k "$1"; sleep 0.3; }
type_text() { wtype "$1"; sleep 0.3; }
next() { wtype -M ctrl -k Return -m ctrl; sleep 0.4; }   # Ctrl+Enter: next, wherever the focus is
click() { # click <x> <y>
    echo "move $1 $2" >&9; sleep 0.3
    echo click >&9; sleep 0.5
}
scroll() { # scroll <x> <y> <amount>   (positive = down)
    echo "move $1 $2" >&9; sleep 0.15
    echo "scroll $3" >&9; sleep 0.4
}
