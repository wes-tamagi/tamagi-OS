#!/usr/bin/bash
# Pacotes sempre na versão mais nova (decisão de 2026-11-24).
#
# 1. Tudo o que veio na base sobe para a última versão do Fedora estável
#    (repositório updates): a imagem base pode ter alguns dias, o build não.
#    O kernel fica de fora: ele é trocado logo depois pelo do sabor escolhido
#    (12-kernel.sh), e uma imagem bootc tem um kernel só.
# 2. O bootc vem do updates-testing do Fedora, que é onde as versões novas
#    chegam primeiro (correções do backend composefs e do systemd-boot, que o
#    Tamagi usa). Só ele: o resto continua no estável. O build confere que a
#    versão instalada é a mais nova que o Fedora oferece.
set -euo pipefail

dnf5 -y upgrade --refresh --exclude='kernel*'

dnf5 -y upgrade --enablerepo=updates-testing bootc
have=$(rpm -q --qf '%{VERSION}-%{RELEASE}' bootc)
newest=$(dnf5 -q repoquery --enablerepo=updates-testing --latest-limit 1 --qf '%{VERSION}-%{RELEASE}' bootc | tail -1)
[[ -n $newest && $have == "$newest" ]] || { echo "bootc $have instalado, mas o Fedora oferece $newest" >&2; exit 1; }
echo "bootc $have (o mais novo do Fedora)"
