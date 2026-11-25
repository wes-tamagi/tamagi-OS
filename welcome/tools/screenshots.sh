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
W=960 H=740

# Every page, in English, with the encrypted-disk question.
pages() {
    echo "pages (English)"
    TAMAGI_WELCOME_TPM=1 STUB_SLEEP=2 session_start $W $H
    TAMAGI_WELCOME_TPM=1 STUB_SLEEP=2 app
    shot 1-welcome
    key Return
    shot 2-apps
    click 400 "${Y_ONLYOFFICE:-560}"   # a row of the list: the whole row ticks
    shot 2b-apps-ticked
    next
    shot 3-browser
    click 200 "${Y_CHROME:-300}"       # Google Chrome: three ways to install
    shot 3b-browser-chrome
    next
    click 450 "${Y_TPM:-190}"          # unlock with the TPM
    shot 5-disk
    next
    shot 6-review
    key Return
    shot 7-installing 2.5
    sleep 12
    shot 8-done
    cat "$STUB_LOG"
    ls "$HOME/.config/tamagi"
    key Return; sleep 1
    kill -0 "$APP_PID" 2>/dev/null && echo "  !! still running after Enter on the last page" || echo "  window closed with Enter"
    session_end
}

# The default path: Enter on every page (the basics ticked by default, the first browser).
defaults() {
    echo "defaults (Enter all the way)"
    STUB_SLEEP=1 session_start $W $H
    STUB_SLEEP=1 app
    key Return; key Return; key Return
    shot d6-review
    key Return
    sleep 6
    shot d8-done
    cat "$STUB_LOG"
    session_end
}

# A laptop panel: 1366x768, the window maximised by the compositor.
laptop() {
    echo "laptop (1366x768)"
    session_start 1366 768
    app
    key Return
    shot laptop-2-apps
    next
    shot laptop-3-browser
    session_end
}

# Portuguese: translated interface, categories and descriptions.
pt() {
    echo "pt (TAMAGI_LANG=pt)"
    TAMAGI_LANG=pt session_start $W $H
    TAMAGI_LANG=pt app
    shot pt-1-welcome
    key Return
    shot pt-2-apps
    next
    shot pt-3-browser
    cat "$HOME/.config/cosmic/com.system76.CosmicAppletTime/v1/military_time"
    session_end
}

# The light theme.
light() {
    echo "light theme"
    session_start $W $H light
    app
    key Return
    shot light-2-apps
    key Return; key Return
    shot light-6-review
    session_end
}

# An installer fails: what the last page says.
failure() {
    echo "failure"
    STUB_FAIL="Calculator" STUB_SLEEP=0.3 session_start $W $H
    STUB_FAIL="Calculator" STUB_SLEEP=0.3 app
    key Return; key Return; key Return; key Return
    sleep 5
    shot fail-8-done
    session_end
}

# Skip on the first page: straight to the end, stamp written, nothing installed.
skip() {
    echo "skip"
    session_start $W $H
    app
    click 90 "${Y_SKIP:-698}"          # Skip for now
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
    key Return; next; next
    click 450 "${Y_TPM:-190}"          # unlock with the TPM
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

# The installer of the live media (--install), with stub back ends.
# THEME=light renders it in the light theme.
installer() {
    echo "installer (--install, ${THEME:-dark})"
    local stubs; stubs="$(dirname "$0")/stubs/setup"
    export TAMAGI_SETUP_LSBLK="$stubs/lsblk.txt" TAMAGI_SETUP_BACKEND="$stubs/backend" TAMAGI_SETUP_DETECT="$stubs/detect" TAMAGI_SETUP_PRIV= TAMAGI_SETUP_TPM=1
    STUB_SLEEP=0.6 session_start $W $H "${THEME:-dark}"
    STUB_SLEEP=0.6 app --install
    shot i01-welcome
    click 400 "${Y_LANG_PT:-366}"                 # Português
    shot i01b-welcome-pt
    next; shot i02-region
    click 835 "${Y_KEYBOARD:-238}"; sleep 0.5; shot i02c-keyboard-menu
    click 835 "${Y_KEYBOARD:-238}"; sleep 0.5
    click 400 "${Y_ZONE_SEARCH:-361}"; type_text "sao"; shot i02b-region-search
    next; shot i03-account-empty
    next; shot i03b-account-error
    click 400 "${Y_FULLNAME:-291}"; type_text "Ana Lima"
    click 400 "${Y_PASS1:-448}"; type_text "secret1"
    click 400 "${Y_PASS2:-523}"; type_text "secret1"
    shot i03c-account-filled
    next; shot i04-disk
    click 400 "${Y_DISK_VDA:-348}"; sleep 1.5
    shot i04b-disk-keep
    click 400 "${Y_DISK_NVME:-272}"; sleep 1.5
    shot i04c-disk-locked
    click 300 "${Y_OLDPASS:-643}"; type_text "oldpassword"; key Return; sleep 1.5
    shot i04d-disk-unlocked
    next; shot i05-encryption                     # encrypted home kept: only how it opens
    click 250 "${Y_PIN1:-549}"; type_text "246810"
    click 700 "${Y_PIN2:-549}"; type_text "246810"
    shot i05b-encryption-pin
    next; shot i06-review
    key Return
    shot i07-installing 1.5
    sleep 6
    shot i08-done
    echo "--- stub log"; cat "$STUB_LOG"
    echo "--- answers"; sed 's/^PASSWORD_HASH=.*/PASSWORD_HASH=(hash)/' "$STUB_LOG.answers"
    echo "--- secrets left: $(ls "$XDG_RUNTIME_DIR/tamagi-install" 2>/dev/null | tr '\n' ' ')"
    session_end
    # Secure Boot guard on the Welcome screen: warning shown and Next stays on step 1.
    export TAMAGI_SETUP_SECUREBOOT=1
    STUB_SLEEP=0.4 session_start $W $H "${THEME:-dark}"
    STUB_SLEEP=0.4 app --install
    shot i01c-secureboot
    key Return; sleep 0.6
    shot i01d-secureboot-blocked
    session_end
    unset TAMAGI_SETUP_LSBLK TAMAGI_SETUP_BACKEND TAMAGI_SETUP_DETECT TAMAGI_SETUP_PRIV TAMAGI_SETUP_TPM TAMAGI_SETUP_SECUREBOOT
}

[[ $# -gt 0 ]] || set -- pages defaults laptop pt light failure skip askpass
for scenario in "$@"; do "$scenario"; done
rm -rf "$WORK"
