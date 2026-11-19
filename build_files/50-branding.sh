#!/usr/bin/bash
# Identidade da imagem. ID=fedora e VERSION_ID (versão do Fedora) são mantidos de propósito: instaladores
# (bootc-image-builder), distrobox e repositórios decidem pelo ID.

OS_RELEASE=/usr/lib/os-release
BUILD_DATE=$(date -u +%Y%m%d)

sed -i \
    -e 's|^NAME=.*|NAME="Tamagi"|' \
    -e "s|^PRETTY_NAME=.*|PRETTY_NAME=\"Tamagi ${TAMAGI_VERSION} (${TAMAGI_CODENAME})\"|" \
    -e "s|^VERSION=.*|VERSION=\"${TAMAGI_VERSION} (${TAMAGI_CODENAME})\"|" \
    -e "s|^VERSION_CODENAME=.*|VERSION_CODENAME=\"${TAMAGI_CODENAME}\"|" \
    -e '/^VARIANT=/d' -e '/^VARIANT_ID=/d' \
    -e 's|^DEFAULT_HOSTNAME=.*|DEFAULT_HOSTNAME="tamagi"|' \
    -e '/^BUG_REPORT_URL=/d' \
    -e '/^IMAGE_ID=/d' -e '/^IMAGE_VERSION=/d' \
    "$OS_RELEASE"
{
    echo 'VARIANT="Tamagi"'
    echo "VARIANT_ID=tamagi"
    echo "IMAGE_ID=\"${IMAGE_NAME}\""
    echo "IMAGE_VERSION=\"${TAMAGI_VERSION}.${BUILD_DATE}\""
} >> "$OS_RELEASE"

mkdir -p /usr/share/ublue-os
cat > /usr/share/ublue-os/image-info.json <<JSON
{
  "image-name": "${IMAGE_NAME}",
  "image-vendor": "${IMAGE_VENDOR}",
  "image-ref": "${IMAGE_REF}",
  "image-tag": "latest",
  "base-image-name": "base-main",
  "desktop": "cosmic",
  "version": "${TAMAGI_VERSION}",
  "codename": "${TAMAGI_CODENAME}",
  "fedora-version": "${FEDORA_VERSION}",
  "build-date": "${BUILD_DATE}"
}
JSON

install -Dm644 /ctx/README.md /usr/share/doc/tamagi/README.md
