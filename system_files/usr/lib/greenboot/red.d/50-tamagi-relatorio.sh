#!/usr/bin/bash
# Roda quando a verificação de saúde falha, antes de o computador reiniciar:
# guarda o que deu errado em /var/lib/tamagi/boot-failures (o /var é o mesmo
# em todas as imagens), para o sistema explicar depois, já na imagem boa.
DIR=/var/lib/tamagi/boot-failures
mkdir -p "$DIR"
report="$DIR/$(date +%Y%m%d-%H%M%S).txt"
{
    echo "Data: $(date '+%d/%m/%Y %H:%M')"
    echo "Imagem que falhou: $(rpm-ostree status -b 2>/dev/null | grep -m1 -E 'Digest|ostree-|docker|registry' | sed 's/^ *//')"
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
