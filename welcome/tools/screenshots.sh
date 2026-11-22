#!/usr/bin/bash
# Renders tamagi-welcome headless and saves a PNG of each page. Runs inside a
# Fedora container (see headless-env.sh for the packages), under a D-Bus
# session:
#   OUT=/shots SHARE=/sysfiles/usr/share/tamagi BIN=/target/release/tamagi-welcome \
#       dbus-run-session tools/screenshots.sh [scenario…]
# The back ends are the stubs in tools/stubs: nothing is installed.
set -u
. "$(dirname "$0")/headless-env.sh"
build_tools
W=900 H=680

# Item by item, with the encrypted-disk question: every page, in English.
pages() {
    echo "pages (item by item, English)"
    TAMAGI_WELCOME_TPM=1 STUB_SLEEP=2 session_start $W $H
    TAMAGI_WELCOME_TPM=1 STUB_SLEEP=2 app
    shot 1-welcome
    key Return
    shot 2-profile
    click 237 527            # "Choose item by item from the catalog"
    next
    click 200 275            # Google Chrome
    sleep 0.5
    click 650 313            # Native
    shot 3-browser
    next
    click 400 423            # Steam
    click 400 322            # Windows 11 in a container
    scroll 400 400 60
    shot 4-catalog
    click 300 186            # the search field
    type_text "agent"
    shot 4b-catalog-search
    wtype -M ctrl -k a -m ctrl; key BackSpace
    type_text "claude"
    sleep 0.5
    click 400 272            # the only row left: Claude Code
    wtype -M ctrl -k a -m ctrl; key BackSpace
    next
    click 450 181            # unlock with the TPM
    shot 5-disk
    next
    shot 6-review
    key Return
    shot 7-installing 2.5
    click 802 350            # Show details
    shot 7b-installing-details 1.5
    sleep 9
    shot 8-done
    cat "$STUB_LOG"
    ls "$HOME/.config/tamagi"
    key Return; sleep 1
    kill -0 "$APP_PID" 2>/dev/null && echo "  !! still running after Enter on the last page" || echo "  window closed with Enter"
    session_end
}

# The default path: Enter on every page (profile "AI and agents", Firefox).
profile() {
    echo "profile (Enter all the way)"
    STUB_SLEEP=1 session_start $W $H
    STUB_SLEEP=1 app
    key Return; key Return; key Return
    shot p6-review-profile
    key Return
    shot p7-installing-profile 3
    sleep 8
    shot p8-done-profile
    cat "$STUB_LOG"
    session_end
}

# Portuguese: translated interface, categories and catalog descriptions.
pt() {
    echo "pt (TAMAGI_LANG=pt)"
    TAMAGI_LANG=pt session_start $W $H
    TAMAGI_LANG=pt app
    shot pt-1-welcome
    key Return
    shot pt-2-profile
    click 237 527
    next; next
    shot pt-4-catalog
    cat "$HOME/.config/cosmic/com.system76.CosmicAppletTime/v1/military_time"
    session_end
}

# The light theme.
light() {
    echo "light theme"
    session_start $W $H light
    app
    key Return
    shot light-2-profile
    key Return; key Return
    shot light-6-review
    session_end
}

# An installer fails: what the last page says.
failure() {
    echo "failure"
    STUB_FAIL="steam" STUB_SLEEP=0.3 session_start $W $H
    STUB_FAIL="steam" STUB_SLEEP=0.3 app
    key Return; key Return; key Return; key Return
    sleep 5
    shot fail-8-done
    session_end
}

# Skip on the first page: straight to the end, stamp written.
skip() {
    echo "skip"
    session_start $W $H
    app
    click 90 638             # Skip for now
    shot skip-8-done
    cat "$STUB_LOG"; ls "$HOME/.config/tamagi"
    session_end
}

# The password dialog alone: prints what is typed, exit 1 on Esc.
askpass() {
    echo "askpass"
    session_start 520 300
    "$BIN" --askpass "[sudo] password for ana: " >"$HOME/answer" 2>"$HOME/app.log" &
    APP_PID=$!
    sleep 2
    type_text "s3cret"
    shot askpass
    key Return; sleep 1
    echo "  stdout: $(cat "$HOME/answer")"
    "$BIN" --askpass "Password:" >"$HOME/answer" 2>>"$HOME/app.log" &
    APP_PID=$!
    sleep 2
    key Escape; sleep 1
    wait "$APP_PID"; echo "  Esc: exit $? stdout: '$(cat "$HOME/answer")'"
    session_end
}

# Real sudo (needs a user with a password and sudo rights; see the report):
# the installer stub calls sudo, sudo opens the dialog through SUDO_ASKPASS.
sudo_dialog() {
    echo "sudo through SUDO_ASKPASS (password: ${SUDO_TEST_PASSWORD:?})"
    TAMAGI_WELCOME_TPM=1 STUB_SUDO=1 STUB_SLEEP=1 session_start $W $H
    TAMAGI_WELCOME_TPM=1 STUB_SUDO=1 STUB_SLEEP=1 app
    key Return
    click 660 527            # Not now (no profile)
    next
    click 200 423            # Not now (no browser)
    next
    click 450 181            # unlock with the TPM
    next
    shot sudo-6-review
    key Return
    shot sudo-7-askpass 2.5
    type_text "$SUDO_TEST_PASSWORD"
    key Return
    sleep 3
    click 802 446 2>/dev/null
    shot sudo-8-done
    cat "$STUB_LOG"
    session_end
}

[[ $# -gt 0 ]] || set -- pages profile pt light failure skip askpass
for scenario in "$@"; do "$scenario"; done
rm -rf "$WORK"
