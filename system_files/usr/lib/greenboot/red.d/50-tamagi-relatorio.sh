#!/usr/bin/bash
# Roda quando a verificação de saúde falha, antes de o computador reiniciar:
# 1. guarda o relatório em /var/lib/tamagi/boot-failures (compartilhado entre imagens);
# 2. registra o digest da imagem que falhou em /var/lib/tamagi/quarantine.list,
#    impedindo que a atualização automática reinstale a mesma imagem defeituosa.
DIR=${TAMAGI_BOOT_FAILURES:-/var/lib/tamagi/boot-failures}
QUARANTINE=${TAMAGI_QUARANTINE:-/var/lib/tamagi/quarantine.list}
mkdir -p "$DIR" "$(dirname "$QUARANTINE")"

failed_digest() {
    local d=""
    if command -v bootc >/dev/null 2>&1; then
        d=$(bootc status --format json 2>/dev/null \
            | jq -r '.status.booted.image.imageDigest // empty' 2>/dev/null || true)
    fi
    if [[ -z $d ]] && command -v rpm-ostree >/dev/null 2>&1; then
        d=$(rpm-ostree status --json 2>/dev/null \
            | jq -r '.deployments[] | select(.booted == true) | ."base-commit-meta"."ostree.manifest-digest" // .checksum // empty' 2>/dev/null | head -1 || true)
    fi
    echo "$d"
}

digest=$(failed_digest)
if [[ -n $digest ]]; then
    touch "$QUARANTINE"
    grep -qxF "$digest" "$QUARANTINE" 2>/dev/null || echo "$digest" >> "$QUARANTINE"
    chmod 644 "$QUARANTINE" 2>/dev/null || true
fi

report="$DIR/$(date +%Y%m%d-%H%M%S).txt"
{
    echo "Data: $(date '+%d/%m/%Y %H:%M')"
    echo "Imagem que falhou: $(rpm-ostree status -b 2>/dev/null | grep -m1 -E 'Digest|ostree-|docker|registry' | sed 's/^ *//')"
    [[ -n $digest ]] && echo "Digest em quarentena: $digest"
    echo "Versão: $(rpm-ostree status -b 2>/dev/null | grep -m1 'Version' | sed 's/^ *//')"
    echo
    echo "== Verificações de saúde =="
    journalctl -b -u greenboot-healthcheck.service --no-pager -o cat 2>/dev/null | grep -E 'FALHOU|ERROR|failed|ok:' | cut -c1-300 | tail -n 20
    echo
    echo "== Serviços que falharam =="
    systemctl --failed --no-legend --plain 2>/dev/null | awk '{print $1}'
    echo
    echo "== Erros do boot (journal, prioridade err) =="
    journalctl -b -p err --no-pager -o short 2>/dev/null | grep -v "Failed to resolve group" | tail -n 30 | cut -c1-300
} > "$report" 2>&1
chgrp wheel "$report" 2>/dev/null; chmod 640 "$report"
# Guarda só os dez mais recentes.
ls -1t "$DIR"/*.txt 2>/dev/null | tail -n +11 | xargs -r rm -f --
exit 0
