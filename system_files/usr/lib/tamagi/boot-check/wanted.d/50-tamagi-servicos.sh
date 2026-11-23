#!/usr/bin/bash
# Verificação DESEJÁVEL: não provoca rollback, só fica registrada no journal.
failed=$(systemctl --failed --no-legend --plain | awk '{print $1}' | tr '\n' ' ')
[[ -z $failed ]] && { echo "ok: nenhum serviço falhou"; exit 0; }
echo "serviços que falharam neste boot: $failed"
exit 1
