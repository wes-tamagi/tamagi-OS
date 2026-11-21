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

# ── Tela de boot (Plymouth): o brasão do Tamagi ──
# O anel (tamaki) com as três joias (tama), nas cores do tema Ipê; durante o
# boot as joias giram e a pedra do centro pulsa (make-plymouth.py desenha).
# A senha do disco (LUKS) aparece no mesmo fundo.
T=/usr/share/plymouth/themes/tamagi
python3 /ctx/build_files/make-plymouth.py "$T"
# Campo de senha, cadeado e indicadores de teclado: os do tema spinner do Fedora.
for f in entry bullet lock capslock keyboard keymap-render; do
    install -m644 "/usr/share/plymouth/themes/spinner/$f.png" "$T/$f.png"
done
cat > "$T/tamagi.plymouth" <<'THEME'
[Plymouth Theme]
Name=Tamagi
Description=O brasão do Tamagi: o anel (tamaki) com as três joias (tama)
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

# O tema vive dentro do initramfs (é de lá que a animação e a senha do disco
# aparecem): gera de novo, com as mesmas opções do 12-kernel.sh, e confere que
# nenhum módulo do initramfs anterior (TPM, LUKS, ostree…) ficou de fora.
kver=$(ls /usr/lib/modules)
img="/usr/lib/modules/$kver/initramfs.img"
before=$(lsinitrd -m "$img" 2>/dev/null | sed -n '/^dracut modules:/,/^=/p' | grep -vE '^(dracut modules:|=)' | sort)
[[ $(wc -l <<<"$before") -gt 20 ]] || { echo "não consegui ler os módulos do initramfs da base" >&2; exit 1; }
dracut --force --no-hostonly --reproducible --zstd --add ostree --kver "$kver" "$img"
chmod 0600 "$img"
after=$(lsinitrd -m "$img" | sed -n '/^dracut modules:/,/^=/p' | grep -vE '^(dracut modules:|=)' | sort)
missing=$(comm -23 <(echo "$before") <(echo "$after"))
[[ -z $missing ]] || { echo "initramfs novo perdeu módulos: $missing" >&2; exit 1; }
# (a listagem vai para uma variável: com pipefail, `lsinitrd | grep -q` falha
# quando o grep fecha o cano antes do fim)
listing=$(lsinitrd "$img")
grep -q 'usr/share/plymouth/themes/tamagi/throbber-0001.png' <<<"$listing" \
    || { echo "o tema do Plymouth não entrou no initramfs" >&2; exit 1; }
echo "Plymouth: tema tamagi no initramfs ($(wc -l <<<"$after") módulos do dracut)"
