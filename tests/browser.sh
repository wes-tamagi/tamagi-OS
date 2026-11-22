#!/usr/bin/bash
# Testa a caixa de navegadores DE VERDADE neste computador (precisa de podman
# e distrobox): cria uma caixa Fedora, instala o Chrome nela (tamagi-browser
# install chrome --box)
# e confere a aceleração de vídeo. Roda fora da imagem porque depende da placa
# de vídeo e do podman do host.
#
#   tests/browser.sh             cria/reaproveita a caixa de teste e confere
#   tests/browser.sh --clean     apaga a caixa de teste
#
# Não toca na sua pasta pessoal: a caixa usa um home separado em
# ~/.cache/tamagi-browser-test, e nada é exportado para o menu.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
P=$ROOT/system_files
# Fica na pasta pessoal, e não ao lado do repositório: o root de dentro da
# caixa precisa alcançar o home de teste e os auxiliares, e não alcança um
# disco externo montado só para o seu usuário (/run/media/<você>).
T=${TAMAGI_BROWSER_TEST_DIR:-$(realpath "$HOME")/.cache/tamagi-browser-test}
export TAMAGI_BROWSER_BOX=tamagi-test-nav
export TAMAGI_BROWSER_HOME=$T/home
mkdir -p "$T/libexec"
cp -f "$P"/usr/libexec/tamagi/browser-provision "$T/libexec/"
export TAMAGI_BROWSER_LIBEXEC=/run/host$T/libexec
export XDG_CONFIG_HOME=$T/config XDG_CACHE_HOME=$T/cache
browser() { bash "$P/usr/bin/tamagi-browser" "$@"; }

if [[ ${1:-} == --clean ]]; then
    podman container exists "$TAMAGI_BROWSER_BOX" && distrobox rm --force "$TAMAGI_BROWSER_BOX"
    echo "Caixa de teste apagada. Os arquivos dela ficaram em $T (pode apagar a pasta)."
    exit 0
fi

command -v distrobox >/dev/null && command -v podman >/dev/null \
    || { echo "precisa de podman e distrobox neste computador" >&2; exit 1; }
mkdir -p "$TAMAGI_BROWSER_HOME" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME"

pass=0; fail=0
ok()  { echo "  ok    $*"; pass=$((pass + 1)); }
bad() { echo "  FALHA $*"; fail=$((fail + 1)); }
check() { local desc=$1; shift; if "$@" >/dev/null 2>&1; then ok "$desc"; else bad "$desc"; fi; }

echo "== Caixa de navegadores (Fedora)"
browser install chrome --box </dev/null 2>&1 | tail -3
check "caixa criada"                         podman container exists "$TAMAGI_BROWSER_BOX"
status=$(browser status 2>/dev/null)
echo "$status" | sed 's/^/    /'
check "é Fedora"                             grep -q 'OS .*Fedora' <<<"$status"
check "Chrome instalado"                     grep -q 'Browser .*Google Chrome' <<<"$status"
check "Chrome abre com as opções de aceleração" bash -c "bash '$P/usr/bin/tamagi-browser' chrome --version | grep -q 'Google Chrome'"
check "navegador desconhecido é recusado"    bash -c "! bash '$P/usr/bin/tamagi-browser' install opera"

echo "== Vídeo"
if grep -q 'decode  *: .*H264' <<<"$status"; then
    ok "VA-API dentro da caixa: $(sed -n 's/.*decode  *: //p' <<<"$status")"
else
    echo "  aviso sem decodificação de vídeo por hardware nesta máquina (sem driver VA-API)"
fi
if grep -q 'Vulkan  *: -$' <<<"$status"; then
    echo "  aviso Vulkan indisponível nesta máquina"
else
    ok "Vulkan dentro da caixa: $(sed -n 's/^Vulkan  *: //p' <<<"$status")"
fi


echo
echo "Resultado: $pass ok, $fail falha(s)   (apagar a caixa de teste: tests/browser.sh --clean)"
exit $(( fail > 0 ))
