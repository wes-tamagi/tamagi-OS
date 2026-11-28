#!/usr/bin/bash
# Habilita os serviços do Tamagi (por edição: dxai ou skl) e limpa artefatos de build.

EDITION=${TAMAGI_EDITION:-dxai}

systemctl enable tamagi-firstboot.service tamagi-tpm-seal.service tamagi-combustion.service

# Tailscale pré-disponível na imagem, mas desligado por padrão (0 MB RAM em idle).
systemctl disable tailscaled.service 2>/dev/null || true

if [[ $EDITION == skl || $EDITION == skl-os ]]; then
    # Edição Tamagi-SKL (Escola):
    # - Sem loja gráfica nem autoinstalação de Flatpaks (Flatpaks somente via terminal);
    # - SSH + Cockpit web habilitados para monitoramento e gestão do professor/diretor (grupo wheel).
    systemctl --global disable tamagi-flatpak-setup.service 2>/dev/null || true
    systemctl enable sshd.service cockpit.socket
    install -Dm644 /dev/stdin /etc/ssh/sshd_config.d/50-tamagi-skl.conf <<'SSH'
# Tamagi-SKL: acesso remoto apenas para administradores (professor/diretor/TI).
PermitRootLogin no
AllowGroups wheel
SSH
    # Permite SSH na rede local (firewalld já libera ssh por padrão em FedoraWorkstation/public, garantido aqui).
    rm -f /etc/xdg/autostart/tamagi-firstrun.desktop
else
    # Edição Tamagi-DxAI: Flatpak --user automático + virtualização/Cockpit sob demanda por socket.
    systemctl --global enable tamagi-flatpak-setup.service
    systemctl disable virtqemud.service 2>/dev/null || true
    systemctl enable cockpit.socket virtqemud.socket virtnetworkd.socket virtstoraged.socket \
        virtnodedevd.socket virtsecretd.socket virtinterfaced.socket virtnwfilterd.socket \
        virtlogd.socket virtlockd.socket
    if [[ -f /usr/share/libvirt/networks/default.xml && ! -e /etc/libvirt/qemu/networks/default.xml ]]; then
        install -Dm600 /usr/share/libvirt/networks/default.xml /etc/libvirt/qemu/networks/default.xml
        install -d /etc/libvirt/qemu/networks/autostart
        ln -sfn ../default.xml /etc/libvirt/qemu/networks/autostart/default.xml
    fi
    authselect enable-feature with-libvirt >/dev/null 2>&1 || true
fi

# systemd-boot no lugar do GRUB
mkdir -p /efi
systemctl mask bootloader-update.service
test -f /usr/lib/systemd/boot/efi/systemd-bootx64.efi || { echo "systemd-boot não está na imagem" >&2; exit 1; }

# Verificação de saúde a cada boot e rollback automático
systemctl enable tamagi-boot-check.service
systemctl enable tamagi-lease-expire.service

# Snapshots da pasta pessoal + manutenção Btrfs (estilo Aeon) + /etc drift journal
systemctl enable snapper-timeline.timer snapper-cleanup.timer tamagi-home-snapshots.service
systemctl enable tamagi-etc-drift.timer
systemctl enable btrfs-scrub.timer btrfs-balance.timer
install -Dm644 /dev/stdin /usr/lib/systemd/system/fstrim.timer.d/10-tamagi.conf <<'UNIT'
[Timer]
OnCalendar=
OnCalendar=monthly
RandomizedDelaySec=6h
UNIT
install -Dm644 /dev/stdin /usr/lib/systemd/system/fstrim.service.d/10-tamagi.conf <<'UNIT'
[Unit]
ConditionACPower=true
[Service]
IOSchedulingClass=idle
UNIT
systemctl enable fstrim.timer

# Atualizações automáticas atômicas via bootc
systemctl enable tamagi-system-update.timer tamagi-system-status.service
systemctl disable rpm-ostreed-automatic.timer 2>/dev/null || true
systemctl mask bootc-fetch-apply-updates.timer 2>/dev/null || true
systemctl disable flatpak-system-update.timer 2>/dev/null || true
systemctl mask flatpak-add-fedora-repos.service
systemctl enable tamagi-flatpak-cleanup.service
systemctl enable tamagi-loader-conf.service

# Camadas systemd-sysext
systemctl enable systemd-sysext.service
systemctl enable tamagi-layer-units.service tamagi-layer-rebuild.service

# Boot rápido (systemd-homed não é usado; contas ficam em /etc/passwd + subvolume /var/home)
systemctl disable NetworkManager-wait-online.service dnf-makecache.timer systemd-homed.service 2>/dev/null || true

# Timers do usuário
systemctl --global enable tamagi-autoupdate.timer flatpak-user-update.timer
systemctl --global enable tamagi-theme-sync.path

rm -f /etc/ld.so.preload

# Idiomas: en, pt, es, fr (~500 MB economizados)
dnf5 -y install --setopt=install_weak_deps=False \
    glibc-langpack-en glibc-langpack-pt glibc-langpack-es glibc-langpack-fr
dnf5 -y remove glibc-all-langpacks
for l in pt_BR.utf8 en_US.utf8 es_ES.utf8 fr_FR.utf8; do
    locale -a | grep -qix "$l" || { echo "locale $l ausente depois do corte" >&2; exit 1; }
done
find /usr/share/locale -mindepth 1 -maxdepth 1 -type d \
    ! -name 'en*' ! -name 'pt*' ! -name 'es*' ! -name 'fr*' ! -name C -exec rm -rf {} +

# Peso morto herdado da base
dnf5 -y remove --no-autoremove python3-botocore python3-boto3 python3-s3transfer google-noto-sans-cjk-fonts || true
rm -f /usr/libexec/luks-enable-tpm2-autounlock /usr/libexec/luks-disable-tpm2-autounlock
fc-cache -f >/dev/null 2>&1 || true

dnf5 clean all

# Caches pré-gerados na imagem
ldconfig -X
systemd-hwdb update 2>/dev/null || true
journalctl --update-catalog 2>/dev/null || true

# /var -> tmpfiles.d
autovar=/usr/lib/tmpfiles.d/tamagi-autovar.conf
known=$(systemd-tmpfiles --cat-config 2>/dev/null | awk '$1 ~ /^[dDvqQ]/ { print $2 }' | sort -u)
{
    echo "# Gerado no build (90-finalize.sh): pastas de /var dos pacotes do Tamagi."
    for top in /var/lib /var/log /var/spool; do
        [[ -d $top ]] || continue
        find "$top" -mindepth 1 -type d | sort | while read -r dir; do
            [[ $dir =~ ^/var/lib/(dnf|rpm|rpm-state|selinux|containers|alternatives)(/|$) ]] && continue
            if grep -qxF "$dir" <<<"$known"; then continue; fi
            printf 'd %s %s %s %s -\n' "$dir" "$(stat -c '%a' "$dir")" "$(stat -c '%U' "$dir")" "$(stat -c '%G' "$dir")"
        done
    done
} > "$autovar"
echo "tmpfiles gerado: $(grep -c '^d ' "$autovar") pastas de /var"
echo "Removendo de /var: $(find /var -mindepth 1 -maxdepth 1 ! -name tmp ! -name cache -printf '%f ')"
find /var -mindepth 1 -maxdepth 1 ! -name tmp ! -name cache -exec rm -rf {} +
find /var/cache -mindepth 1 -maxdepth 1 ! -name libdnf5 -exec rm -rf {} + 2>/dev/null || true
rm -rf /var/tmp/* /run/dnf 2>/dev/null || true
