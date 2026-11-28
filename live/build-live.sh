#!/usr/bin/bash
# Transforma a imagem do Tamagi no ambiente live da mídia de instalação.
# Roda dentro do build (live/Containerfile). O ambiente live é o próprio
# sistema, com: o instalador (tamagi-welcome --install, a mesma janela
# libcosmic do primeiro login), um initramfs que sabe iniciar de uma ISO, e um
# usuário "live" sem senha que entra direto no COSMIC.
set -euo pipefail

CTX=/ctx/live

# ── Pacotes só da mídia: dracut-live (boot pela ISO), xorriso e mtools (o gerador
#    da ISO roda dentro desta mesma imagem) ──
dnf5 -y install --setopt=install_weak_deps=False dracut-live xorriso mtools
command -v tamagi-welcome >/dev/null || { echo "a janela do instalador (tamagi-welcome) não está na imagem" >&2; exit 1; }

# ── Arquivos da sessão live ─────────────────────────────────────────────
cp -a "$CTX/files/." /
chmod 755 /usr/libexec/tamagi-install /usr/libexec/tamagi-autoinstall /usr/libexec/tamagi-installer-launch /usr/libexec/tamagi-verify-media /usr/libexec/tamagi-detect-home
desktop-file-validate /usr/share/applications/tamagi-installer.desktop /etc/xdg/autostart/tamagi-installer.desktop
cat > /etc/tamagi-live.conf <<CONF
# De onde o instalador tira o sistema e para onde o sistema instalado olha.
PAYLOAD_DIR=/run/initramfs/live/tamagi-oci
TARGET_IMGREF=${TARGET_IMGREF:-}
SKL_TARGET_IMGREF=${SKL_TARGET_IMGREF:-}
SKL_FORK_TARGET_IMGREF=${SKL_FORK_TARGET_IMGREF:-}
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
# A sessão live (painel, janela do instalador) em inglês e com teclado US, o idioma
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
# Na mídia live (pendrive USB + squashfs), reconstruir caches em tempo de boot
# (ldconfig, hwdb, catálogo do journal, appstream, mandb) ou subir sockets de
# host (VMs, Cockpit, impressão, Snapper) disputa leitura aleatória no pendrive
# e trava o sysinit.target. Os caches são pré-gerados aqui no build.
systemctl disable \
    tamagi-flatpak-setup.service tamagi-firstboot.service tamagi-combustion.service \
    tamagi-layer-rebuild.service tamagi-layer-units.service \
    cockpit.socket virtqemud.socket virtnetworkd.socket virtstoraged.socket \
    virtnodedevd.socket virtsecretd.socket virtinterfaced.socket virtnwfilterd.socket \
    virtlogd.socket virtlockd.socket virtproxyd.socket \
    cups.socket cups.path avahi-daemon.socket avahi-daemon.service \
    iscsid.socket iscsiuio.socket \
    snapper-timeline.timer snapper-cleanup.timer \
    btrfs-scrub.timer btrfs-balance.timer fstrim.timer 2>/dev/null || true

systemctl mask \
    ldconfig.service systemd-hwdb-update.service systemd-journal-catalog-update.service \
    systemd-update-done.service \
    fedora-atomic-desktop-appstream-cache-refresh.service \
    fedora-atomic-desktop-mandb-update.service \
    plymouth-start.service plymouth-read-write.service \
    plymouth-quit.service plymouth-quit-wait.service \
    rpm-ostreed-automatic.timer rpm-ostreed-automatic.service \
    bootc-fetch-apply-updates.timer bootloader-update.service \
    ostree-remount.service systemd-remount-fs.service \
    tamagi-boot-check.service tamagi-tpm-seal.service \
    tamagi-home-snapshots.service tamagi-lease-expire.service \
    tamagi-flatpak-cleanup.service tamagi-loader-conf.service \
    tamagi-system-status.service tamagi-system-update.timer tamagi-system-update.service \
    systemd-sysext.service \
    lvm2-monitor.service lvm-devices-import.service lvm-devices-import.path \
    raid-check.timer 2>/dev/null || true
# A tela não apaga nem o computador suspende no meio da instalação.
idle=/usr/share/cosmic/com.system76.CosmicIdle/v1
install -d "$idle"
echo 'None' > "$idle/screen_off_time"
echo 'None' > "$idle/suspend_on_ac_time"
echo 'None' > "$idle/suspend_on_battery_time"

# ── initramfs live: enxuto (sem módulos do sistema instalado que inflam para
#    270+ MB e atrasam o dracut-initqueue em pendrives USB 2.0) e com drivers
#    USB/live explícitos para achar a ISO e montar o squashfs com overlay ────
kver=$(ls /usr/lib/modules | head -1)
dracut --force --no-hostonly --reproducible --zstd \
    --add "dmsquash-live pollcdrom" \
    --add-drivers "usb-storage uas sd_mod sr_mod isofs squashfs overlay loop" \
    --omit "ostree bootc plymouth drm network network-manager nfs cifs nvmf iscsi fcoe fcoe-uefi \
            crypt systemd-cryptsetup fido2 pcsc pkcs11 tpm2-tss systemd-pcrextend \
            clevis clevis-pin-file clevis-pin-null clevis-pin-sss clevis-pin-tang clevis-pin-tpm2 \
            fips fips-crypto-policies nss-softokn lvm mdraid resume lunmask nvdimm" \
    --kver "$kver" "/usr/lib/modules/$kver/initramfs-live.img"
ls -lh "/usr/lib/modules/$kver/initramfs-live.img"

dnf5 clean all
ldconfig -X
[[ -s /etc/ld.so.cache ]] || { echo "live: /etc/ld.so.cache não foi gerado" >&2; exit 1; }
systemd-hwdb update 2>/dev/null || true
journalctl --update-catalog 2>/dev/null || true
find /var -mindepth 1 -maxdepth 1 ! -name tmp ! -name cache ! -name home -exec rm -rf {} +
rm -rf /var/tmp/* /run/dnf 2>/dev/null || true
echo "Ambiente live pronto (kernel $kver)."
