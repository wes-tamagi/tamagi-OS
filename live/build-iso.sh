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
# Compressão (padrão: a mais forte, para a ISO caber num pendrive de 8 GB):
#   TAMAGI_ISO_SQUASH=xz|zstd     squashfs do ambiente live (zstd: build mais rápido, ISO maior)
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
    -e LABEL="$LABEL" -e OUT_NAME="$OUT_NAME" -e SQUASH_COMP="${TAMAGI_ISO_SQUASH:-xz}" \
    "$LIVE" bash /ctx/mkiso-inner.sh

step "Pronto"
ls -lh "$OUT_DIR/$OUT_NAME"
echo "Gravar num pendrive:  sudo bash live/write-usb.sh /dev/disk/by-id/usb-<seu-pendrive> --yes"
echo "Testar numa VM:       just test-iso   (instala sozinho)   ou   just run-iso"
