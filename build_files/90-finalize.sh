#!/usr/bin/bash
# Habilita os serviços do Tamagi e limpa o que o build deixou para trás.

systemctl enable tamagi-flatpak-setup.service tamagi-firstboot.service tamagi-tpm-seal.service tamagi-combustion.service

# systemd-boot no lugar do GRUB: a partição EFI é montada em /efi (a pasta tem
# de existir na imagem, que é somente-leitura), e o serviço que atualiza o
# GRUB a cada boot (bootupd) não tem o que fazer.
mkdir -p /efi
systemctl mask bootloader-update.service
test -f /usr/lib/systemd/boot/efi/systemd-bootx64.efi || { echo "systemd-boot não está na imagem" >&2; exit 1; }

# Verificação de saúde a cada boot e volta automática para a imagem anterior
# (boot-check): o bootc com systemd-boot não conta tentativas de boot.
systemctl enable tamagi-boot-check.service

# Snapshots da pasta pessoal (home-snapshots / tamagi-snap): os timers do
# snapper só agem nas configurações criadas; o serviço faz as conversões
# agendadas antes do login.
systemctl enable snapper-timeline.timer snapper-cleanup.timer tamagi-home-snapshots.service
# Manutenção do btrfs (btrfsmaintenance): scrub mensal e balance semanal.
systemctl enable btrfs-scrub.timer btrfs-balance.timer
# TRIM: o btrfs já devolve os blocos livres ao SSD aos poucos (discard=async, o
# padrão do kernel em SSD, com limite de taxa) — é o que dá desempenho sem
# rajadas. O fstrim periódico fica como rede de segurança e para /boot e EFI;
# semanal ele reenviaria o TRIM de todo o espaço livre do btrfs a cada vez,
# então passa a mensal e não roda na bateria.
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

# O kernel padrão (Open Gaming Collective) é assinado só pelo Universal Blue: este guarda
# cancela uma atualização que não iniciaria com o Secure Boot (chave ausente).
# (chamado pelo system-update logo depois de preparar a imagem nova).

# A imagem do sistema: procurada a cada seis horas e preparada para o próximo
# boot pelo bootc (system-update). O rpm-ostree não fala com este formato de
# instalação (composefs): o timer dele fica desligado.
systemctl enable tamagi-system-update.timer tamagi-system-status.service
systemctl disable rpm-ostreed-automatic.timer 2>/dev/null || true
systemctl mask bootc-fetch-apply-updates.timer 2>/dev/null || true

# Camadas do tamagi-layer (/var/lib/extensions) entram no /usr a cada boot.
systemctl enable systemd-sysext.service

# Camadas que trazem serviços (ex.: `tamagi vm install`, com o libvirt): depois
# do sysext entrar no /usr, cria usuários/pastas e liga as units pedidas.
systemctl enable tamagi-layer-units.service

# Boot mais rápido e menos ruído em background:
#   - NetworkManager-wait-online atrasa o boot esperando DHCP/Wi-Fi sem necessidade;
#   - dnf-makecache.timer não serve num host imutável (bootc não usa dnf no host).
systemctl disable NetworkManager-wait-online.service dnf-makecache.timer 2>/dev/null || true

# Atualização automática dos subsistemas (caixas), para todo usuário (a dos
# Flatpaks já vem ligada pela base).
systemctl --global enable tamagi-autoupdate.timer

# Os apps de terminal (foot, btop, Neovim, janelas do Tamagi) acompanham o tema
# escolhido nas Configurações do COSMIC.
systemctl --global enable tamagi-theme-sync.path

# Sob demanda: ficam na imagem, desligados.
#   usbguard  -> tamagi-harden usb on
systemctl disable usbguard.service 2>/dev/null || true

# hardened_malloc fica instalado, mas não é carregado em todo processo por
# padrão (quebraria jogos e alguns apps); `tamagi-harden malloc on` liga.
rm -f /etc/ld.so.preload

# Idiomas: o produto tem quatro (en, pt, es, fr). A base traz os locales e as
# traduções de todos os idiomas do mundo (~500 MB); ficam só os quatro.
dnf5 -y install --setopt=install_weak_deps=False \
    glibc-langpack-en glibc-langpack-pt glibc-langpack-es glibc-langpack-fr
dnf5 -y remove glibc-all-langpacks
for l in pt_BR.utf8 en_US.utf8 es_ES.utf8 fr_FR.utf8; do
    locale -a | grep -qix "$l" || { echo "locale $l ausente depois do corte" >&2; exit 1; }
done
find /usr/share/locale -mindepth 1 -maxdepth 1 -type d \
    ! -name 'en*' ! -name 'pt*' ! -name 'es*' ! -name 'fr*' ! -name C -exec rm -rf {} +

# Peso morto herdado da base: o cliente da AWS em Python (nada na imagem o
# usa) e a fonte CJK estática (as variáveis, que ficam, cobrem os mesmos
# idiomas). ~250 MB.
dnf5 -y remove --no-autoremove python3-botocore python3-boto3 python3-s3transfer google-noto-sans-cjk-fonts || true
fc-cache -f >/dev/null 2>&1 || true

dnf5 clean all

# /var não viaja na imagem (veja abaixo), mas os pacotes instalados aqui
# esperam encontrar as pastas deles em /var/lib e /var/log. Como faz o
# rpm-ostree na base, cada pasta vira uma linha de tmpfiles: é criada no boot
# com dono, modo e rótulo SELinux certos.
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
# /var é estado da máquina, não da imagem (a base só traz /var/tmp): o que o
# build criou ali não chegaria às máquinas já instaladas, então não fica.
echo "Removendo de /var: $(find /var -mindepth 1 -maxdepth 1 ! -name tmp ! -name cache -printf '%f ')"
find /var -mindepth 1 -maxdepth 1 ! -name tmp ! -name cache -exec rm -rf {} +
find /var/cache -mindepth 1 -maxdepth 1 ! -name libdnf5 -exec rm -rf {} + 2>/dev/null || true
rm -rf /var/tmp/* /run/dnf 2>/dev/null || true
