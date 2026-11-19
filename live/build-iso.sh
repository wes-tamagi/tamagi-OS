#!/usr/bin/bash
# Gera a ISO de instalação do Tamagi. Tudo rootless: nada aqui pede sudo.
#
#   live/build-iso.sh <imagem-do-sistema> <imagem-live> <saida.iso>
#   (normalmente: just build-iso)
#
# A ISO tem três partes:
#   LiveOS/squashfs.img   o ambiente live (o sistema + tik), em squashfs
#   tamagi-oci/           a imagem do sistema em formato OCI: é dela que o
#                         `bootc install` copia, sem precisar de rede
#   boot/ + partição EFI  kernel, initramfs live e GRUB (só UEFI)
#
# Compressão (padrão: zstd nas duas partes — descompacta várias vezes mais
# rápido que o xz, então a mídia inicia, o instalador responde e a cópia do
# sistema anda no ritmo do pendrive, não do processador):
#   TAMAGI_ISO_SQUASH=zstd|xz     squashfs do ambiente live (xz: ISO ~10% menor, bem mais lenta)
#   TAMAGI_ISO_ZSTD_LEVEL=1..22   camadas da imagem embutida
#
# TAMAGI_ISO_NO_PAYLOAD=1 gera uma ISO menor, sem a imagem embutida: a
# instalação baixa do registry (precisa de TARGET_IMGREF no build-live).
set -euo pipefail

IMAGE=${1:?imagem do sistema}
LIVE=${2:?imagem live}
OUT=${3:?arquivo .iso de saída}
LABEL=TAMAGI_LIVE

OUT_DIR=$(cd "$(dirname "$OUT")" && pwd)
OUT_NAME=$(basename "$OUT")
WORK="$OUT_DIR/iso-work"
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)

step() { echo; echo -e "\033[1;34m── $*\033[0m"; }

podman image exists "$IMAGE" || { echo "imagem $IMAGE não existe (rode: just build)" >&2; exit 1; }
podman image exists "$LIVE" || { echo "imagem $LIVE não existe (rode: just build-live)" >&2; exit 1; }

# Pasta de trabalho recriada do zero; quem apaga é o contêiner (os arquivos
# do squashfs saem com donos do espaço de usuário do podman).
mkdir -p "$WORK"
# Uma ISO antiga pode ter mudado de dono (o libvirt faz isso ao abri-la numa VM)
# e o xorriso não consegue gravar por cima: sai antes de começar.
rm -f -- "$OUT" 2>/dev/null || { echo "não consegui apagar a ISO antiga $OUT (dono: $(stat -c %U "$OUT")); apague-a e rode de novo" >&2; exit 1; }
podman run --rm -v "$OUT_DIR":/out:Z "$LIVE" bash -c 'rm -rf /out/iso-work && mkdir -p /out/iso-work/tree'

if [[ ${TAMAGI_ISO_NO_PAYLOAD:-0} != 1 ]]; then
    step "Imagem do sistema em formato OCI (é o que o bootc instala)"
    podman push --quiet --compression-format zstd --compression-level "${TAMAGI_ISO_ZSTD_LEVEL:-19}" \
        "$IMAGE" "oci:$WORK/tree/tamagi-oci:latest"
    du -sh "$WORK/tree/tamagi-oci"
fi

step "Ambiente live em squashfs, kernel, initramfs e GRUB"
# A imagem live é montada somente-leitura em /live; as ferramentas (mksquashfs,
# xorriso, mtools) também vêm dela.
podman run --rm \
    --mount "type=image,source=$LIVE,destination=/live" \
    -v "$OUT_DIR":/out:Z -v "$SCRIPT_DIR":/ctx:ro,Z \
    -e LABEL="$LABEL" -e OUT_NAME="$OUT_NAME" -e SQUASH_COMP="${TAMAGI_ISO_SQUASH:-zstd}" \
    "$LIVE" bash /ctx/mkiso-inner.sh

step "Pronto"
ls -lh "$OUT_DIR/$OUT_NAME"
echo "Gravar num pendrive:  sudo bash live/write-usb.sh /dev/disk/by-id/usb-<seu-pendrive> --yes"
echo "Testar numa VM:       just test-iso   (instala sozinho)   ou   just run-iso"
