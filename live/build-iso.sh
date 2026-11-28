#!/usr/bin/bash
# Gera a ISO de instalação multi-edição do Tamagi (Tamagi-DxAI + Tamagi-SKL).
# Tudo rootless: nada aqui pede sudo.
#
#   live/build-iso.sh <imagem-dxai> <imagem-live> <saida.iso> [imagem-skl]
set -euo pipefail

IMAGE=${1:?imagem Tamagi-DxAI}
LIVE=${2:?imagem live}
OUT=${3:?arquivo .iso de saída}
SKL_IMAGE=${4:-}
SKL_FORK_IMAGE=${5:-}
LABEL=TAMAGI_LIVE

mkdir -p "$(dirname "$OUT")"
OUT_DIR=$(cd "$(dirname "$OUT")" && pwd)
OUT_NAME=$(basename "$OUT")
WORK="$OUT_DIR/iso-work"
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)

step() { echo; echo -e "\033[1;34m── $*\033[0m"; }

podman image exists "$IMAGE" || { echo "imagem $IMAGE não existe (rode: just build)" >&2; exit 1; }
podman image exists "$LIVE" || { echo "imagem $LIVE não existe (rode: just build-live)" >&2; exit 1; }

mkdir -p "$WORK"
rm -f -- "$OUT" 2>/dev/null || { echo "não consegui apagar a ISO antiga $OUT (dono: $(stat -c %U "$OUT")); apague-a e rode de novo" >&2; exit 1; }
podman run --rm -v "$OUT_DIR":/out:Z "$LIVE" bash -c 'rm -rf /out/iso-work && mkdir -p /out/iso-work/tree'

if [[ ${TAMAGI_ISO_NO_PAYLOAD:-0} != 1 ]]; then
    step "Imagem Tamagi-DxAI em formato OCI (tag :dxai)"
    podman push --quiet --compression-format zstd --compression-level "${TAMAGI_ISO_ZSTD_LEVEL:-19}" \
        "$IMAGE" "oci:$WORK/tree/tamagi-oci:dxai"
    if [[ -n $SKL_IMAGE ]] && podman image exists "$SKL_IMAGE"; then
        step "Imagem Tamagi-SKL em formato OCI (tag :skl, camadas compartilhadas deduplicadas)"
        podman push --quiet --compression-format zstd --compression-level "${TAMAGI_ISO_ZSTD_LEVEL:-19}" \
            "$SKL_IMAGE" "oci:$WORK/tree/tamagi-oci:skl"
    else
        echo "aviso: imagem Tamagi-SKL ($SKL_IMAGE) não encontrada; pulando tag :skl" >&2
    fi
    if [[ -n $SKL_FORK_IMAGE ]] && podman image exists "$SKL_FORK_IMAGE"; then
        step "Imagem SKL (fork) em formato OCI (tag :skl-os, camadas compartilhadas deduplicadas)"
        podman push --quiet --compression-format zstd --compression-level "${TAMAGI_ISO_ZSTD_LEVEL:-19}" \
            "$SKL_FORK_IMAGE" "oci:$WORK/tree/tamagi-oci:skl-os"
    else
        echo "aviso: imagem SKL fork ($SKL_FORK_IMAGE) não encontrada; pulando tag :skl-os" >&2
    fi
    du -sh "$WORK/tree/tamagi-oci"
fi

step "Ambiente live em squashfs, kernel, initramfs e GRUB (Tamagi-DxAI / Tamagi-SKL)"
podman run --rm \
    --mount "type=image,source=$LIVE,destination=/live" \
    -v "$OUT_DIR":/out:Z -v "$SCRIPT_DIR":/ctx:ro,Z \
    -e LABEL="$LABEL" -e OUT_NAME="$OUT_NAME" -e SQUASH_COMP="${TAMAGI_ISO_SQUASH:-zstd}" \
    "$LIVE" bash /ctx/mkiso-inner.sh

step "Pronto"
( cd "$OUT_DIR" && sha256sum "$OUT_NAME" > "$OUT_NAME.sha256" )
ls -lh "$OUT_DIR/$OUT_NAME" "$OUT_DIR/$OUT_NAME.sha256"
cat "$OUT_DIR/$OUT_NAME.sha256"
echo "Gravar num pendrive:  sudo bash live/write-usb.sh /dev/disk/by-id/usb-<seu-pendrive> --yes"
echo "Testar numa VM:       just test-iso  (DxAI)  ou  just test-iso-skl  (SKL)"
