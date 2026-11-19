#!/usr/bin/bash
# Testa a caixa de navegadores DE VERDADE neste computador (precisa de podman
# e distrobox): cria uma caixa AnduinOS, instala o Chrome com o tamagi-browser
# e confere a aceleração de vídeo. Roda fora da imagem porque depende da placa
# de vídeo e do podman do host.
#
#   tests/browser.sh             cria/reaproveita a caixa de teste e confere
#   tests/browser.sh --warsaw    também instala e sobe o Warsaw na caixa
#                                (ao usar esta opção você aceita a licença da Topaz)
#   tests/browser.sh --bank      também testa o modo banco descartável (idem licença)
#   tests/browser.sh --clean     apaga a caixa de teste
#
# Não toca na sua pasta pessoal: a caixa usa um home separado em
# .cache/browser-test, e nada é exportado para o menu.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
P=$ROOT/system_files
T=$ROOT/.cache/browser-test
export TAMAGI_BROWSER_BOX=tamagi-test-nav
export TAMAGI_BROWSER_HOME=$T/home
export TAMAGI_BROWSER_LIBEXEC=/run/host$P/usr/libexec/tamagi
export TAMAGI_WARSAW_CONF=$P/usr/share/tamagi/warsaw/warsaw.conf
export XDG_CONFIG_HOME=$T/config XDG_CACHE_HOME=$T/cache
browser() { bash "$P/usr/bin/tamagi-browser" "$@"; }
warsaw() { bash "$P/usr/bin/tamagi-warsaw" "$@"; }

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

echo "== Caixa de navegadores (AnduinOS)"
browser setup </dev/null 2>&1 | tail -3
check "caixa criada"                         podman container exists "$TAMAGI_BROWSER_BOX"
status=$(browser status 2>/dev/null)
echo "$status" | sed 's/^/    /'
check "é AnduinOS"                           grep -q 'Sistema.*AnduinOS' <<<"$status"
check "Chrome instalado"                     grep -q 'Navegador.*Google Chrome' <<<"$status"
check "Chrome abre com as opções de aceleração" bash -c "bash '$P/usr/bin/tamagi-browser' chrome --version | grep -q 'Google Chrome'"
check "navegador desconhecido é recusado"    bash -c "! bash '$P/usr/bin/tamagi-browser' install opera"

echo "== Vídeo"
if grep -q 'decodifica : .*H264' <<<"$status"; then
    ok "VA-API dentro da caixa: $(sed -n 's/.*decodifica : //p' <<<"$status")"
else
    echo "  aviso sem decodificação de vídeo por hardware nesta máquina (sem driver VA-API)"
fi
if grep -q 'Vulkan  *: indisponível' <<<"$status"; then
    echo "  aviso Vulkan indisponível nesta máquina"
else
    ok "Vulkan dentro da caixa: $(sed -n 's/^Vulkan  *: //p' <<<"$status")"
fi

if [[ ${1:-} == --warsaw ]]; then
    echo "== Warsaw (na caixa)"
    touch "$XDG_CONFIG_HOME/tamagi/warsaw-enabled"   # --warsaw = licença aceita
    warsaw ligar </dev/null 2>&1 | tail -12 | sed 's/^/    /'
    wstatus=$(warsaw status 2>/dev/null)
    check "serviço rodando"                  grep -q 'Serviço (root): rodando' <<<"$wstatus"
    check "porta 30900 responde"             grep -q 'Porta 30900   : respondendo' <<<"$wstatus"
    check "certificado aceito pelos navegadores" grep -q 'Certificado   : aceito' <<<"$wstatus"
    warsaw desligar >/dev/null 2>&1
    wstatus=$(warsaw status 2>/dev/null)
    check "desligar para o serviço e esquece o certificado" bash -c "grep -q 'Serviço (root): parado' <<<'$wstatus' && grep -q 'ainda não registrado' <<<'$wstatus'"
fi

if [[ ${1:-} == --bank ]]; then
    echo "== Modo banco descartável"
    export TAMAGI_BANK_TEMPLATE=localhost/tamagi-banco-teste:latest
    mkdir -p "$XDG_CONFIG_HOME/tamagi"
    printf '#!/usr/bin/bash\nexec bash "%s/usr/bin/tamagi-warsaw" "$@"\n' "$P" > "$T/tamagi-warsaw"; chmod +x "$T/tamagi-warsaw"
    export TAMAGI_WARSAW_BIN=$T/tamagi-warsaw
    touch "$XDG_CONFIG_HOME/tamagi/warsaw-license-accepted"   # --bank = licença aceita
    bank() { bash "$P/usr/bin/tamagi-bank" "$@"; }
    # No lugar do Chrome, um comando que mostra o que a sessão enxerga.
    probe='printf "http=%s\n" "$(curl -s --cacert /usr/local/etc/warsaw/rootca.crt -o /dev/null -w "%{http_code}" https://127.0.0.1:30900/)"; echo "nss=$(certutil -L -d sql:$HOME/.pki/nssdb | grep -c Warsaw)"; echo "home=$HOME"; echo "ca=$(sha256sum /usr/local/etc/warsaw/rootca.crt | cut -c1-16)"; touch "$HOME/rastro"'
    one=$(TAMAGI_BANK_CMD=$probe bank </dev/null 2>&1); echo "$one" | tail -7 | sed 's/^/    /'
    check "modelo montado (Chrome + Warsaw)"   podman image exists "$TAMAGI_BANK_TEMPLATE"
    check "Warsaw responde dentro da sessão"   grep -q '^http=426' <<<"$one"
    check "certificado só no perfil da sessão" grep -q '^nss=1' <<<"$one"
    check "perfil em memória (/run/user)"      grep -qE "^home=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/tamagi-banco\." <<<"$one"
    check "caixa apagada ao fechar"            bash -c "! podman ps -a --format '{{.Names}}' | grep -qE '^banco-'"
    check "perfil apagado ao fechar"           bash -c "! ls -d ${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/tamagi-banco.* 2>/dev/null | grep -q ."
    start=$SECONDS
    two=$(TAMAGI_BANK_CMD=$probe bank </dev/null 2>&1)
    echo "    segunda sessão: $((SECONDS - start)) s para abrir, testar e apagar"
    check "segunda sessão: outra identidade do Warsaw" bash -c "[[ -n '$(sed -n 's/^ca=//p' <<<"$one")' && '$(sed -n 's/^ca=//p' <<<"$one")' != '$(sed -n 's/^ca=//p' <<<"$two")' ]]"
    three=$(TAMAGI_BANK_CMD='ls -A "$HOME"' bank </dev/null 2>&1)
    if grep -qx rastro <<<"$three"; then bad "nada da primeira sessão sobrevive"; else ok "nada da primeira sessão sobrevive"; fi
    bank clean >/dev/null 2>&1
    check "clean apaga o modelo"               bash -c "! podman image exists '$TAMAGI_BANK_TEMPLATE'"
fi

echo
echo "Resultado: $pass ok, $fail falha(s)   (apagar a caixa de teste: tests/browser.sh --clean)"
exit $(( fail > 0 ))
