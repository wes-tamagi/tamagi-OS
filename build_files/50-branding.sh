#!/usr/bin/bash
# Identidade da imagem (Tamagi-DxAI ou Tamagi-SKL). ID=fedora e VERSION_ID são
# mantidos de propósito para bootc, distrobox e repositórios.

OS_RELEASE=/usr/lib/os-release
BUILD_DATE=$(date -u +%Y%m%d)
TAMAGI_URL=https://github.com/wes-tamagi/tamagi-OS
EDITION=${TAMAGI_EDITION:-dxai}

case "$EDITION" in
    dxai)
        PRODUCT_NAME="Tamagi-DxAI"
        PRODUCT_VARIANT="Tamagi-DxAI"
        PRODUCT_VARIANT_ID="tamagi-dxai"
        PRODUCT_HOST="tamagi-dxai"
        LOGO_NAME="tamagi"
        ;;
    skl)
        PRODUCT_NAME="Tamagi-SKL"
        PRODUCT_VARIANT="Tamagi-SKL"
        PRODUCT_VARIANT_ID="tamagi-skl"
        PRODUCT_HOST="tamagi-skl"
        LOGO_NAME="tamagi"
        ;;
    skl-os)
        PRODUCT_NAME="SKL"
        PRODUCT_VARIANT="SKL"
        PRODUCT_VARIANT_ID="skl"
        PRODUCT_HOST="skl"
        LOGO_NAME="skl"
        ;;
    *)
        echo "TAMAGI_EDITION inválido: $EDITION" >&2
        exit 1
        ;;
esac

mkdir -p /usr/share/tamagi
echo "$EDITION" > /usr/share/tamagi/edition

dnf5 -y swap fedora-logos generic-logos 2>/dev/null || true
dnf5 -y remove fedora-workstation-backgrounds 2>/dev/null || true
rpm -q fedora-logos >/dev/null 2>&1 && { echo "fedora-logos continua na imagem" >&2; exit 1; }

sed -i \
    -e "s|^NAME=.*|NAME=\"${PRODUCT_NAME}\"|" \
    -e "s|^PRETTY_NAME=.*|PRETTY_NAME=\"${PRODUCT_NAME} ${TAMAGI_VERSION} (${TAMAGI_CODENAME})\"|" \
    -e "s|^VERSION=.*|VERSION=\"${TAMAGI_VERSION} (${TAMAGI_CODENAME})\"|" \
    -e "s|^VERSION_CODENAME=.*|VERSION_CODENAME=\"${TAMAGI_CODENAME}\"|" \
    -e '/^VARIANT=/d' -e '/^VARIANT_ID=/d' \
    -e "s|^DEFAULT_HOSTNAME=.*|DEFAULT_HOSTNAME=\"${PRODUCT_HOST}\"|" \
    -e '/^BUG_REPORT_URL=/d' \
    -e "s|^LOGO=.*|LOGO=${LOGO_NAME}|" \
    -e "s|^HOME_URL=.*|HOME_URL=\"${TAMAGI_URL}\"|" \
    -e "s|^DOCUMENTATION_URL=.*|DOCUMENTATION_URL=\"${TAMAGI_URL}#readme\"|" \
    -e "s|^SUPPORT_URL=.*|SUPPORT_URL=\"${TAMAGI_URL}/issues\"|" \
    -e '/^REDHAT_BUGZILLA_PRODUCT/d' -e '/^REDHAT_SUPPORT_PRODUCT/d' \
    -e 's|^ANSI_COLOR=.*|ANSI_COLOR="0;38;2;242;193;78"|' \
    -e '/^IMAGE_ID=/d' -e '/^IMAGE_VERSION=/d' \
    "$OS_RELEASE"
{
    echo "VARIANT=\"${PRODUCT_VARIANT}\""
    echo "VARIANT_ID=${PRODUCT_VARIANT_ID}"
    echo "IMAGE_ID=\"${IMAGE_NAME}\""
    echo "IMAGE_VERSION=\"${TAMAGI_VERSION}.${BUILD_DATE}\""
} >> "$OS_RELEASE"

mkdir -p /usr/share/ublue-os
cat > /usr/share/ublue-os/image-info.json <<JSON
{
  "image-name": "${IMAGE_NAME}",
  "edition": "${EDITION}",
  "product-name": "${PRODUCT_NAME}",
  "image-vendor": "${IMAGE_VENDOR}",
  "image-ref": "${IMAGE_REF}",
  "image-tag": "latest",
  "base-image-name": "fedora-bootc",
  "desktop": "cosmic",
  "version": "${TAMAGI_VERSION}",
  "codename": "${TAMAGI_CODENAME}",
  "fedora-version": "${FEDORA_VERSION}",
  "build-date": "${BUILD_DATE}"
}
JSON

install -Dm644 /ctx/README.md /usr/share/doc/tamagi/README.md
install -Dm644 /ctx/NOTICE /usr/share/doc/tamagi/NOTICE
grep -q "^LOGO=${LOGO_NAME}\$" "$OS_RELEASE" && [[ -f /usr/share/icons/hicolor/scalable/apps/${LOGO_NAME}.svg ]] \
    || { echo "os-release sem o logo (${LOGO_NAME})" >&2; exit 1; }
grep -qi 'fedoraproject' <(grep -E '^(HOME|DOCUMENTATION|SUPPORT)_URL=' "$OS_RELEASE") \
    && { echo "os-release ainda aponta para o Fedora" >&2; exit 1; } || true

# ── Tela de boot (Plymouth): o brasão do Tamagi ──
T=/usr/share/plymouth/themes/tamagi
python3 /ctx/build_files/make-plymouth.py "$T"
for f in entry bullet lock capslock keyboard keymap-render; do
    install -m644 "/usr/share/plymouth/themes/spinner/$f.png" "$T/$f.png"
done
cat > "$T/tamagi.plymouth" <<'THEME'
[Plymouth Theme]
Name=Tamagi
Description=Tamagi crest: the ring (tamaki) with three jewels (tama)
ModuleName=two-step

[two-step]
Font=Noto Sans 12
TitleFont=Noto Sans Light 26
ImageDir=/usr/share/plymouth/themes/tamagi
DialogHorizontalAlignment=.5
DialogVerticalAlignment=.70
TitleHorizontalAlignment=.5
TitleVerticalAlignment=.30
HorizontalAlignment=.5
VerticalAlignment=.45
WatermarkHorizontalAlignment=.5
WatermarkVerticalAlignment=.66
Transition=none
TransitionDuration=0.0
BackgroundStartColor=0x101a15
BackgroundEndColor=0x101a15
ProgressBarBackgroundColor=0x3f5247
ProgressBarForegroundColor=0xf2c14e
DialogClearsFirmwareBackground=true
MessageBelowAnimation=true

[boot-up]
UseEndAnimation=false
UseFirmwareBackground=false

[shutdown]
UseEndAnimation=false
UseFirmwareBackground=false

[reboot]
UseEndAnimation=false
UseFirmwareBackground=false

[updates]
SuppressMessages=true
ProgressBarShowPercentComplete=true
UseProgressBar=true
Title=Installing updates...
SubTitle=Do not turn off your computer

[system-upgrade]
SuppressMessages=true
ProgressBarShowPercentComplete=true
UseProgressBar=true
Title=Upgrading the system...
SubTitle=Do not turn off your computer

[firmware-upgrade]
SuppressMessages=true
ProgressBarShowPercentComplete=true
UseProgressBar=true
Title=Upgrading firmware...
SubTitle=Do not turn off your computer

[system-reset]
SuppressMessages=true
ProgressBarShowPercentComplete=true
UseProgressBar=true
Title=Resetting the system...
SubTitle=Do not turn off your computer
THEME
plymouth-set-default-theme tamagi
[[ $(plymouth-set-default-theme) == tamagi ]] || { echo "Plymouth: o tema padrão não virou tamagi" >&2; exit 1; }

kver=$(ls /usr/lib/modules)
img="/usr/lib/modules/$kver/initramfs.img"
before=$(lsinitrd -m "$img" 2>/dev/null | sed -n '/^dracut modules:/,/^=/p' | grep -vE '^(dracut modules:|=)' | sort)
[[ $(wc -l <<<"$before") -gt 20 ]] || { echo "não consegui ler os módulos do initramfs da base" >&2; exit 1; }
dracut --force --no-hostonly --reproducible --zstd --add ostree --kver "$kver" "$img"
chmod 0600 "$img"
after=$(lsinitrd -m "$img" | sed -n '/^dracut modules:/,/^=/p' | grep -vE '^(dracut modules:|=)' | sort)
missing=$(comm -23 <(echo "$before") <(echo "$after"))
[[ -z $missing ]] || { echo "initramfs novo perdeu módulos: $missing" >&2; exit 1; }
listing=$(lsinitrd "$img")
grep -q 'usr/share/plymouth/themes/tamagi/throbber-0001.png' <<<"$listing" \
    || { echo "o tema do Plymouth não entrou no initramfs" >&2; exit 1; }
echo "Plymouth: tema tamagi no initramfs ($(wc -l <<<"$after") módulos do dracut)"

# ── Só imagens do Tamagi assinadas ──
POLICY=/etc/containers/policy.json
if [[ ! -f $POLICY ]]; then
    install -d /etc/containers
    if [[ -f /usr/share/containers/policy.json ]]; then
        cp /usr/share/containers/policy.json "$POLICY"
    else
        printf '{"default":[{"type":"insecureAcceptAnything"}],"transports":{"docker":{}}}\n' > "$POLICY"
    fi
fi
jq '.transports //= {} | .transports.docker //= {} | .transports.docker["ghcr.io/wes-tamagi"] = [{"type": "sigstoreSigned", "keyPaths": ["/etc/pki/containers/tamagi.pub", "/etc/pki/containers/tamagi-2.pub"], "signedIdentity": {"type": "matchRepository"}}]' \
    "$POLICY" > "$POLICY.new"
mv "$POLICY.new" "$POLICY"
jq -e '.transports.docker["ghcr.io/wes-tamagi"][0].type == "sigstoreSigned"' "$POLICY" >/dev/null
grep -q "BEGIN PUBLIC KEY" /etc/pki/containers/tamagi.pub
grep -q "BEGIN PUBLIC KEY" /etc/pki/containers/tamagi-2.pub
