#!/usr/bin/bash
# Verificação de saúde OBRIGATÓRIA do Tamagi (boot-check). Roda a cada boot; se
# falhar numa imagem nova, o computador reinicia e, na segunda falha seguida,
# volta sozinho para a imagem anterior (bootc rollback). Por isso só entra aqui o que, quebrado,
# deixa o sistema inutilizável: nada que dependa de rede ou de hardware opcional.
set -u

fail() { echo "FALHOU: $*"; exit 1; }

# Para testar o mecanismo: tamagi.failboot na linha do kernel simula uma imagem ruim.
grep -qw tamagi.failboot /proc/cmdline && fail "falha de boot simulada (tamagi.failboot na linha do kernel)"

wait_active() { # wait_active <unit> <segundos>: espera a unit subir; falha se ela falhar ou não subir
    local unit=$1 limit=$2 state i
    for (( i = 0; i < limit; i++ )); do
        state=$(systemctl is-active "$unit" 2>/dev/null)
        [[ $state == active ]] && return 0
        [[ $state == failed ]] && return 1
        sleep 1
    done
    return 1
}

touch /var/lib/.tamagi-healthcheck 2>/dev/null && rm -f /var/lib/.tamagi-healthcheck || fail "/var não aceita gravação"
wait_active dbus.service 30 || fail "o barramento do sistema (dbus) não subiu"
wait_active systemd-logind.service 30 || fail "o gerenciador de sessões (logind) não subiu"
wait_active NetworkManager.service 60 || fail "o NetworkManager não subiu"
if [[ $(systemctl get-default) == graphical.target ]]; then
    wait_active display-manager.service 120 || fail "a tela de login (cosmic-greeter) não subiu"
fi
echo "ok: essenciais do Tamagi no ar"
