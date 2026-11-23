#!/usr/bin/bash
# Roda quando a verificação de saúde falha, antes de o computador reiniciar:
# 1. guarda o relatório em /var/lib/tamagi/boot-failures (compartilhado entre imagens);
# 2. registra o digest da imagem que falhou em /var/lib/tamagi/quarantine.list,
#    impedindo que a atualização automática reinstale a mesma imagem defeituosa.
DIR=${TAMAGI_BOOT_FAILURES:-/var/lib/tamagi/boot-failures}
QUARANTINE=${TAMAGI_QUARANTINE:-/var/lib/tamagi/quarantine.list}
mkdir -p "$DIR" "$(dirname "$QUARANTINE")"

STATUS=${TAMAGI_BOOTC_STATUS:-/run/tamagi/bootc-status.json}
digest=$(jq -r '.status.booted.image.imageDigest // empty' "$STATUS" 2>/dev/null || true)
# Só a imagem nova que falhou vai para a quarentena (boot-check decide).
if [[ -n $digest && -z ${TAMAGI_BOOT_NO_QUARANTINE:-} ]]; then
    touch "$QUARANTINE"
    grep -qxF "$digest" "$QUARANTINE" 2>/dev/null || echo "$digest" >> "$QUARANTINE"
    chmod 644 "$QUARANTINE" 2>/dev/null || true
fi

report="$DIR/$(date +%Y%m%d-%H%M%S).txt"
{
    echo "Data: $(date '+%d/%m/%Y %H:%M')"
    echo "Imagem que falhou: $(jq -r '.status.booted.image | "\(.image.image) \(.imageDigest)"' "$STATUS" 2>/dev/null)"
    [[ -n $digest && -z ${TAMAGI_BOOT_NO_QUARANTINE:-} ]] && echo "Digest em quarentena: $digest"
    echo "Versão: $(jq -r '.status.booted.image | "\(.version) (\(.timestamp))"' "$STATUS" 2>/dev/null)"
    echo
    echo "== Verificações de saúde =="
    journalctl -b -u tamagi-boot-check.service --no-pager -o cat 2>/dev/null | grep -E 'FALHOU|ERROR|failed|ok:' | cut -c1-300 | tail -n 20
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
