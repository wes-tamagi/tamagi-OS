#!/usr/bin/bash
# Grava a ISO do Tamagi num pendrive e confere a gravação. APAGA o pendrive.
#
#   sudo bash live/write-usb.sh <dispositivo> [iso] --yes
#   ex.: sudo bash live/write-usb.sh /dev/disk/by-id/usb-Marca_Modelo-0:0 --yes
#
# Só aceita disco USB removível inteiro (nunca partição nem disco interno) e
# confere, lendo de volta, que o que ficou no pendrive é igual à ISO.
set -euo pipefail

DEV="" ISO="$(cd "$(dirname "$0")/.." && pwd)/output/tamagi.iso" YES=0
for arg in "$@"; do
    case "$arg" in
        --yes) YES=1 ;;
        *.iso) ISO=$arg ;;
        *)     DEV=$arg ;;
    esac
done
die() { echo "erro: $*" >&2; exit 1; }

[[ $(id -u) == 0 ]] || die "rode com sudo"
[[ -n $DEV && -b $DEV ]] || die "informe o dispositivo do pendrive (veja: lsblk -o NAME,SIZE,TRAN,MODEL)"
[[ -f $ISO ]] || die "ISO não encontrada: $ISO"
DEV=$(realpath "$DEV")
name=$(basename "$DEV")
[[ $(lsblk -dno TYPE "$DEV") == disk ]] || die "$DEV não é um disco inteiro"
[[ $(lsblk -dno TRAN "$DEV") == usb ]] || die "$DEV não é USB"
[[ $(cat "/sys/block/$name/removable") == 1 ]] || die "$DEV não é removível (parece um disco externo, não um pendrive)"
iso_size=$(stat -c %s "$ISO")
(( $(lsblk -bdno SIZE "$DEV") >= iso_size )) || die "a ISO não cabe em $DEV"

echo "Pendrive: $DEV — $(lsblk -dno SIZE,MODEL "$DEV" | sed -E 's/ +/ /g')"
lsblk -no NAME,SIZE,FSTYPE,LABEL "$DEV" | sed 's/^/    /'
echo "ISO     : $ISO ($(( iso_size / 1024 / 1024 )) MiB)"
(( YES )) || die "isto APAGA o pendrive acima. Para confirmar, repita o comando com --yes"

for part in $(lsblk -nrpo NAME "$DEV" | tail -n +2); do umount "$part" 2>/dev/null || true; done
echo "Limpando assinaturas antigas do pendrive…"
wipefs -af "$DEV" >/dev/null 2>&1 || true
echo "Gravando…"
dd if="$ISO" of="$DEV" bs=4M oflag=direct status=progress conv=fsync
sync
blockdev --flushbufs "$DEV" 2>/dev/null || true
partprobe "$DEV" 2>/dev/null || true
udevadm settle 2>/dev/null || true
echo "Conferindo a gravação (lendo do hardware, fora do cache)…"
if cmp -n "$iso_size" "$ISO" <(dd if="$DEV" iflag=direct bs=4M status=none); then
    echo "Pronto: a ISO foi gravada e conferida bit a bit. Pode retirar o pendrive."
else
    die "o conteúdo do pendrive NÃO confere com a ISO (pendrive ou porta USB com defeito?)"
fi
