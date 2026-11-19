#!/usr/bin/bash
# Teste do desktop, DENTRO do contêiner da imagem: sobe a sessão COSMIC
# sobre um servidor X virtual, confere os componentes e salva /out/desktop.png.
#   just test-desktop
# Sem logind nem GPU o COSMIC mostra a tela de bloqueio; o teste prova que o
# compositor e os componentes da sessão sobem com os padrões do Tamagi.
set -uo pipefail

dnf5 -y -q install xorg-x11-server-Xvfb dbus-daemon ImageMagick libxkbcommon-x11 >/dev/null 2>&1 \
    || { echo "não consegui instalar as dependências do teste"; exit 1; }
useradd -m tester 2>/dev/null || true
mkdir -p /out /run/dbus && chmod 777 /out
[[ -e /var/run ]] || ln -s ../run /var/run
dbus-daemon --system --fork

cat > /tmp/session.sh <<'SESSION'
export XDG_RUNTIME_DIR=/tmp/xdg-tester DISPLAY=:9 XDG_CURRENT_DESKTOP=COSMIC \
       XDG_SESSION_TYPE=wayland LIBGL_ALWAYS_SOFTWARE=1
mkdir -p "$XDG_RUNTIME_DIR" && chmod 700 "$XDG_RUNTIME_DIR"
Xvfb :9 -screen 0 1920x1080x24 >/dev/null 2>&1 &
sleep 2
cosmic-session >/tmp/cosmic.log 2>&1 &
sleep 30
fail=0
for proc in cosmic-comp cosmic-panel cosmic-bg cosmic-launcher cosmic-settings-daemon cosmic-notifications; do
    # -f e não -x: o kernel corta o nome do processo em 15 letras
    if pgrep -f "(^|/)$proc( |$)" >/dev/null; then echo "  $proc: rodando"; else echo "  $proc: NÃO subiu"; fail=1; fi
done
import -window root /out/desktop.png 2>/dev/null && echo "  captura: /out/desktop.png"
sed 's/\x1b\[[0-9;]*m//g' /tmp/cosmic.log | grep -iE 'panic|shortcuts.*(error|invalid)' | head -5
exit $fail
SESSION
runuser -u tester -- dbus-run-session -- bash /tmp/session.sh 2>/dev/null
rc=$?
echo
[[ $rc -eq 0 ]] && echo "RESULTADO: ok — a sessão COSMIC sobe com os padrões do Tamagi" || echo "RESULTADO: FALHA"
exit $rc
