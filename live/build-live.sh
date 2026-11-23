#!/usr/bin/bash
# Transforma a imagem do Tamagi no ambiente live da mídia de instalação.
# Roda dentro do build (live/Containerfile). O ambiente live é o próprio
# sistema, com: o tik e o módulo do Tamagi, um initramfs que sabe iniciar de
# uma ISO, e um usuário "live" sem senha que entra direto no COSMIC.
set -euo pipefail

CTX=/ctx/live
TIK_REPO=https://github.com/sysrich/tik
: "${TIK_REF:?}" "${TIK_COMMIT:?}"

# ── Pacotes só da mídia: zenity (janelas do tik), dracut-live (boot pela
#    ISO) e xorriso (o gerador da ISO roda dentro desta mesma imagem) ──
dnf5 -y install --setopt=install_weak_deps=False zenity dracut-live xorriso git-core

# ── tik, na versão fixada ───────────────────────────────────────────────
src=$(mktemp -d)
git clone --quiet --depth 1 --branch "$TIK_REF" "$TIK_REPO" "$src"
got=$(git -C "$src" rev-parse HEAD)
[[ $got == "$TIK_COMMIT" ]] || { echo "tik $TIK_REF aponta para $got, esperava $TIK_COMMIT" >&2; exit 1; }
install -Dm755 "$src/usr/bin/tik" /usr/bin/tik
install -d /usr/lib/tik/lib /usr/lib/tik/img /usr/lib/tik/modules/pre /usr/lib/tik/modules/post
install -m644 "$src"/usr/lib/tik/lib/* /usr/lib/tik/lib/
install -m644 "$src/usr/lib/tik/config" /usr/lib/tik/config
install -Dm644 "$src/LICENSE" /usr/share/licenses/tik/LICENSE
# Os módulos do Aeon (GNOME, sdbootutil, migração) não se aplicam: o Tamagi
# traz o seu, que instala com o bootc.
install -Dm644 "$CTX/tik/config" /etc/tik/config
install -Dm644 "$CTX/tik/modules/pre/10-tamagi" /etc/tik/modules/pre/10-tamagi
install -d /etc/tik/modules/post
rm -rf "$src"

# O núcleo do tik fala inglês e procura a mídia pelo rótulo do Aeon. Cada
# troca abaixo é conferida: se uma versão nova do tik mudar o texto, o build
# falha aqui em vez de a tradução sumir em silêncio.
patch_tik() { # patch_tik <arquivo> <texto original> <texto novo>
    local file=/usr/lib/tik/lib/$1
    grep -qF -- "$2" "$file" || { echo "tik mudou: não achei '$2' em $1" >&2; exit 1; }
    python3 - "$file" "$2" "$3" <<'PY'
import sys
path, old, new = sys.argv[1:4]
text = open(path, encoding="utf-8").read()
open(path, "w", encoding="utf-8").write(text.replace(old, new))
PY
}
# Os textos viram variáveis que o módulo do Tamagi preenche no idioma escolhido.
patch_tik tik-functions 'Do you really want to quit?' '${tamagi_t_quit}'
patch_tik tik-core 'tik_volid="TIKINSTALL"' 'tik_volid="TAMAGI_LIVE"'
patch_tik tik-core '--column=Disk --column=Size --column=Partitions --column=Filesystems' \
                   '--column="${tamagi_t_col_disk}" --column="${tamagi_t_col_size}" --column="${tamagi_t_col_parts}" --column="${tamagi_t_col_fs}"'
patch_tik tik-core '--title="Select A Disk"' '--title="${tamagi_t_disk_title}"'
patch_tik tik-core 'Select the disk to install the operating system to. <b>Make sure any important documents and files have been backed up.</b>' \
                   '${tamagi_t_disk_text}'
patch_tik tik-core 'No device(s) for installation found.' '${tamagi_t_no_disk}'
# Disco sem nome em /dev/disk/by-id (comum em VMs): o tik abre um aviso em
# inglês que não pede nenhuma ação. Vira só uma linha no registro.
patch_tik tik-core-helper 'warn "Could not find <tt>${schema}</tt> representation' 'log "Could not find <tt>${schema}</tt> representation'
bash -n /usr/bin/tik
for f in /usr/lib/tik/lib/* /etc/tik/modules/pre/10-tamagi; do bash -n "$f"; done

# ── Arquivos da sessão live ─────────────────────────────────────────────
cp -a "$CTX/files/." /
chmod 755 /usr/libexec/tamagi-install /usr/libexec/tamagi-installer-launch /usr/libexec/tamagi-installer-splash /usr/libexec/tamagi-verify-media
desktop-file-validate /usr/share/applications/tamagi-installer.desktop /etc/xdg/autostart/tamagi-installer.desktop
cat > /etc/tamagi-live.conf <<CONF
# De onde o instalador tira o sistema e para onde o sistema instalado olha.
PAYLOAD_DIR=/run/initramfs/live/tamagi-oci
TARGET_IMGREF=${TARGET_IMGREF:-}
CONF
# Marca de "estamos na mídia": o assistente de primeiro login não aparece aqui.
echo 'f /run/tamagi-live 0644 root root -' > /usr/lib/tmpfiles.d/tamagi-live.conf
systemctl enable tamagi-autoinstall.service

# ── Usuário live: sem senha, entra direto no COSMIC ─────────────────────
useradd --create-home --comment "Tamagi Live" --groups wheel live
passwd -d live >/dev/null
echo 'live ALL=(ALL) NOPASSWD: ALL' > /etc/sudoers.d/tamagi-live
chmod 440 /etc/sudoers.d/tamagi-live
cat >> /etc/greetd/cosmic-greeter.toml <<'TOML'

# Mídia de instalação: entra direto na sessão do usuário live.
[initial_session]
command = "start-cosmic"
user = "live"
TOML
# A sessão live (painel, janelas do tik) em inglês e com teclado US, o idioma
# padrão do Tamagi; o instalador pergunta o idioma logo no começo e o sistema
# instalado recebe o que for escolhido.
if [ "$(cat /usr/share/tamagi/default-lang 2>/dev/null)" = pt ]; then
    echo 'LANG=pt_BR.UTF-8' > /etc/locale.conf
    echo 'KEYMAP=br-abnt2' > /etc/vconsole.conf
else
    echo 'LANG=en_US.UTF-8' > /etc/locale.conf
    echo 'KEYMAP=us' > /etc/vconsole.conf
fi
# Na mídia o assistente de primeiro login do Tamagi não aparece.
install -d -o live -g live /var/home/live/.config /var/home/live/.config/tamagi
touch /var/home/live/.config/tamagi/firstrun-done
chown -R live:live /var/home/live

# ── Serviços que só fazem sentido no sistema instalado ──────────────────
systemctl disable tamagi-flatpak-setup.service tamagi-firstboot.service tamagi-combustion.service 2>/dev/null || true
systemctl mask rpm-ostreed-automatic.timer rpm-ostreed-automatic.service \
    bootc-fetch-apply-updates.timer bootloader-update.service \
    ostree-remount.service systemd-remount-fs.service 2>/dev/null || true
# A tela não apaga nem o computador suspende no meio da instalação.
idle=/usr/share/cosmic/com.system76.CosmicIdle/v1
install -d "$idle"
echo 'None' > "$idle/screen_off_time"
echo 'None' > "$idle/suspend_on_ac_time"
echo 'None' > "$idle/suspend_on_battery_time"

# ── initramfs live: sabe achar a ISO e montar o squashfs com overlay ────
kver=$(ls /usr/lib/modules | head -1)
dracut --force --no-hostonly --reproducible --zstd \
    --add "dmsquash-live pollcdrom" \
    --kver "$kver" "/usr/lib/modules/$kver/initramfs-live.img"
ls -la "/usr/lib/modules/$kver/initramfs-live.img"

dnf5 clean all
find /var -mindepth 1 -maxdepth 1 ! -name tmp ! -name cache ! -name home -exec rm -rf {} +
rm -rf /var/tmp/* /run/dnf 2>/dev/null || true
echo "Ambiente live pronto (tik $TIK_REF, kernel $kver)."
