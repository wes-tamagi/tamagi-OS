#!/usr/bin/bash
# Testes da imagem Tamagi. Rodam DENTRO do contêiner da imagem:
#   just test
set -uo pipefail

pass=0; fail=0
ok()   { echo "  ok    $*"; pass=$((pass + 1)); }
bad()  { echo "  FALHA $*"; fail=$((fail + 1)); }
check() { # check "descrição" comando...
    local desc=$1; shift
    if "$@" >/dev/null 2>&1; then ok "$desc"; else bad "$desc"; fi
}
section() { echo; echo "== $*"; }

export HOME=/tmp/test-home XDG_RUNTIME_DIR=/tmp/test-run TAMAGI_LANG=pt
mkdir -p "$HOME" "$XDG_RUNTIME_DIR"

EDITION=$(cat /usr/share/tamagi/edition 2>/dev/null || echo dxai)
EDITION_NAME="Tamagi-DxAI"
[[ $EDITION == skl ]] && EDITION_NAME="Tamagi-SKL"
echo "Edição detectada: $EDITION_NAME ($EDITION)"

section "Identidade"
check "os-release diz $EDITION_NAME"       grep -q "^NAME=\"${EDITION_NAME}\"" /usr/lib/os-release
check "versão 1.6, codinome Kazunobu M"    grep -q "^PRETTY_NAME=\"${EDITION_NAME} 1.6 (Kazunobu M)\"" /usr/lib/os-release
check "VERSION_ID continua o do Fedora"    grep -qE '^VERSION_ID=4[0-9]' /usr/lib/os-release
check "ID continua fedora"                 grep -q '^ID=fedora' /usr/lib/os-release
check "image-info.json é JSON válido"      jq -e '."image-name"' /usr/share/ublue-os/image-info.json
check "root padrão é btrfs"                grep -q 'type = "btrfs"' /usr/lib/bootc/install/10-tamagi.toml

section "Ferramentas na imagem"
COMMON_CMDS=(distrobox podman flatpak bootc ptyxis cosmic-term cosmic-files git fd \
             btm gum starship jq tailscale \
             tamagi-ai tamagi-agent tamagi-theme tamagi-menu tamagi-keys tamagi-panel tamagi-lang \
             tamagi-firstrun tamagi-fboot tamagi-update tamagi-webapp \
             chromium-browser \
             tamagi-browser tamagi-harden tamagi tamagi-layer tamagi-boot-report tamagi-snap snapper tamagi-windows tamagi-db tamagi-capture tamagi-devbox tamagi-font tamagi-antigravity tamagi-isolate bwrap tamagi-rewind tamagi-privacy tamagi-perf tamagi-workspace tamagi-doctor tamagi-gaming tamagi-install tamagi-remove tamagi-mcp tamagi-vm tamagi-try tamagi-why tamagi-skl)
for cmd in "${COMMON_CMDS[@]}"; do
    check "comando $cmd" command -v "$cmd"
done
if [[ $EDITION == skl ]]; then
    check "Tamagi-SKL enxuto: sem cosmic-store, chromedriver, llama-server, mise, libvirt, gamescope, foot ou gnome-disks" bash -c 'for c in cosmic-store chromedriver llama-server llama-cli claude mise virsh qemu-kvm gamescope mangohud foot gnome-disks cosmic-ext-storage tamagi-agents; do ! command -v "$c" >/dev/null 2>&1 || { echo "ainda presente: $c"; exit 1; }; done'
else
    for cmd in rg bat eza zoxide mise scx_rusty scx_rustland llama-server llama-cli claude chromedriver rclone gnome-disks gamescope mangohud gamemoderun virsh qemu-kvm cockpit-bridge; do
        check "comando DxAI $cmd" command -v "$cmd"
    done
    check "imagem DxAI enxuta: Steam/compiladores/guestfs/btop/foot/cosmic-ext-storage fora; base Bazzite + QEMU/libvirt/Cockpit + mise + gnome-disks sim" bash -c 'for c in steam clang go opencode google-chrome-stable firefox tesseract adb iperf3 ncat btop fastfetch dust delta usbguard foot cosmic-ext-storage tamagi-agents tamagi-remind tamagi-recipe tamagi-guest; do ! command -v "$c" >/dev/null 2>&1 || { echo "ainda presente: $c"; exit 1; }; done; ! rpm -q guestfs-tools nm-connection-editor google-roboto-slab-fonts cascadia-code-nf-fonts adw-gtk3-theme hardened_malloc >/dev/null 2>&1 && rpm -q steam-devices gamescope mangohud gamemode mesa-vulkan-drivers.i686 cockpit-machines libvirt-daemon-kvm gnome-disk-utility papirus-icon-theme >/dev/null 2>&1'
fi
check "CLI enxuta (5 comandos, sem aliases legados): tamagi update, tamagi pkg, tamagi ai, tamagi dev, tamagi sys + tamagi-layer" bash -c 'o=$(TAMAGI_LANG=en tamagi); [[ $(grep -c "^  tamagi " <<<"$o") -eq 5 ]] && grep -q "tamagi pkg" <<<"$o" && tamagi sys doctor --help | grep -qi diagn && tamagi sys desktop tiling status | grep -q "^tiling:" && ! tamagi doctor 2>/dev/null && ! tamagi inexistente 2>/dev/null && tamagi sys why /usr/bin/bash | grep -qiE "base|image|rpm" && tamagi ai flights list >/dev/null && tamagi-agent list | grep -q "qwen" && command -v tamagi-layer >/dev/null'
check "comandos e subcomandos em inglês (sem apelidos em português)" bash -c '! ls /usr/bin | grep -qE "^tamagi-(convidado|voltar|privacidade|lembrete|isolar|popos|dev|assist)$" && ! grep -lE "^\s+[a-z|-]*\b(instalar|remover|abrir|ligar|desligar|ajuda|estado|lista)\)" /usr/bin/tamagi-* 2>/dev/null | grep -q .'

section "Base e kernel"
kver=$(ls /usr/lib/modules | head -1); kconf=/usr/lib/modules/$kver/config
echo "  kernel $kver"
check "kernel com suporte a Rust"          grep -qx 'CONFIG_RUST=y' "$kconf"
check "kernel: binder (Android em contêiner), em C ou em Rust" grep -qxE 'CONFIG_ANDROID_BINDER_IPC(_RUST)?=y' "$kconf"
check "kernel é o do sabor escolhido no build (ogc ou main; sem Rawhide ark)" bash -c "f=\$(cat /usr/share/tamagi/kernel-flavor); v=\$(. /usr/lib/os-release; echo \$VERSION_ID); case \$f in ogc) [[ $kver == *ogc* ]] ;; main) [[ $kver != *ogc* && $kver == *.fc\$v.* ]] ;; *) false ;; esac"
check "um kernel só, com initramfs" bash -c "[[ \$(ls /usr/lib/modules | wc -l) -eq 1 && -s /usr/lib/modules/$kver/initramfs.img && -s /usr/lib/modules/$kver/vmlinuz ]]"
check "kernel: aceleradores de IA (DRM_ACCEL)" grep -qx 'CONFIG_DRM_ACCEL=y' "$kconf"
check "kernel: NPU AMD Ryzen AI (amdxdna)" bash -c "find /usr/lib/modules/$kver -name 'amdxdna.ko*' | grep -q ."
check "kernel: NPU Intel (ivpu)"           bash -c "find /usr/lib/modules/$kver -name 'intel_vpu.ko*' | grep -q ."
check "kernel: sched_ext (escalonadores em Rust/BPF)" grep -qx 'CONFIG_SCHED_CLASS_EXT=y' "$kconf"
check "módulo v4l2loopback assinado e sem xone/xpadneo na base" bash -c "find /usr/lib/modules/$kver -name 'v4l2loopback.ko*' | grep -q . && ! rpm -q kmod-xone kmod-xpadneo >/dev/null 2>&1"
check "módulos assinados; chave do Universal Blue na imagem (Secure Boot)" bash -c "test -s /etc/pki/akmods/certs/akmods-ublue.der && f=\$(find /usr/lib/modules/$kver -name 'v4l2loopback.ko*' | head -1) && modinfo -F signer \"\$f\" | grep -q ."

section "Redes e conectividade"
check "VPNs extras (OpenVPN/OpenConnect/L2TP/IPsec) fora da imagem, no catálogo (tamagi install vpn-extra)" bash -c '! rpm -q NetworkManager-openvpn-gnome NetworkManager-openconnect-gnome NetworkManager-l2tp NetworkManager-pptp NetworkManager-sstp NetworkManager-strongswan NetworkManager-libreswan NetworkManager-ssh >/dev/null 2>&1 && grep -q "^vpn-extra	Networks	.*tamagi-layer add vpn-extra .*NetworkManager-openvpn-gnome" /usr/share/tamagi/catalog.tsv'
if [[ $EDITION == skl ]]; then
    check "SKL: SSH (AllowGroups wheel, sem root) e Cockpit web habilitados para professor/diretor" bash -c 'systemctl is-enabled sshd.service | grep -qx enabled && systemctl is-enabled cockpit.socket | grep -qx enabled && grep -q "AllowGroups wheel" /etc/ssh/sshd_config.d/50-tamagi-skl.conf && grep -q "PermitRootLogin no" /etc/ssh/sshd_config.d/50-tamagi-skl.conf'
else
    check "Cockpit só com o que o tamagi vm usa (sem storaged e networkmanager)" bash -c 'rpm -q cockpit cockpit-machines cockpit-podman >/dev/null && ! rpm -q cockpit-storaged >/dev/null 2>&1 && ! rpm -q cockpit-networkmanager >/dev/null 2>&1'
fi
check "WireGuard"                          command -v wg
check "Wi-Fi, banda larga móvel e Bluetooth no NetworkManager" rpm -q NetworkManager-wifi NetworkManager-wwan NetworkManager-bluetooth ModemManager
check "Tailscale pré-instalado e desligado por padrão (0 RAM parado; repo oficial desabilitado pós-build)" bash -c 'command -v tailscale >/dev/null && rpm -q tailscale >/dev/null && ! systemctl is-enabled tailscaled.service 2>/dev/null | grep -qx enabled && grep -q "tailscaled.service" /usr/libexec/tamagi/tailscale-setup'
check "pastas Windows/NAS (SMB) e MTP na imagem; NFS/SSHFS, OCR, adb e net-tools no catálogo" bash -c 'rpm -q gvfs-smb cifs-utils samba-client gvfs-mtp usb_modeswitch && ! rpm -q gvfs-nfs fuse-sshfs tesseract android-tools iperf3 nmap-ncat ifuse >/dev/null 2>&1 && for id in net-fs ocr android-tools net-tools; do grep -qP "^$id\t" /usr/share/tamagi/catalog.tsv || exit 1; done'
check "descoberta na rede local (mDNS)"    rpm -q avahi nss-mdns
check "impressão e scanner sem driver"     rpm -q cups cups-browsed ipp-usb sane-airscan
check "firmware de Wi-Fi (Intel, Realtek, MediaTek, Atheros, Broadcom)" rpm -q iwlwifi-mvm-firmware realtek-firmware mt7xxx-firmware atheros-firmware brcmfmac-firmware
check "firewall ativo por padrão"          systemctl is-enabled firewalld

section "Navegadores (Chromium nativo + extras) e WebApps"
if [[ $EDITION == skl ]]; then
    check "SKL: Chromium nativo SEM ChromeDriver e SEM OAuth2, com política de sessão (sem senhas, limpa login ao fechar, histórico 15 dias = 360h)" bash -c '
        rpm -q chromium >/dev/null
        ! rpm -q chromedriver >/dev/null 2>&1
        for p in firefox google-chrome-stable; do ! rpm -q "$p" >/dev/null 2>&1 || exit 1; done
        [[ -f /etc/chromium/tamagi.conf ]] && bash -n /etc/chromium/tamagi.conf
        CHROMIUM_FLAGS=""
        . /etc/chromium/tamagi.conf
        [[ $CHROMIUM_FLAGS != *"oauth2-client-id"* ]]
        jq -e ".PasswordManagerEnabled == false and .BrowserSignin == 0 and (.ClearBrowsingDataOnExitList | index(\"cookies_and_other_site_data\") != null) and .BrowsingDataLifetime[0].time_to_live_in_hours == 360" /etc/chromium/policies/managed/tamagi-skl.json >/dev/null
        for app in docs sheets slides gemini; do
            desktop-file-validate "/usr/share/applications/tamagi-skl-$app.desktop"
        done
        desktop-file-validate /etc/xdg/autostart/tamagi-skl-browser.desktop
        test -x /usr/libexec/tamagi/skl-browser-launch
    '
else
    check "Chromium + ChromeDriver nativos com OAuth2 Google, Lark WebApp e inglês padrão" bash -c '
        rpm -q chromium chromedriver >/dev/null
        for p in firefox google-chrome-stable; do ! rpm -q "$p" >/dev/null 2>&1 || exit 1; done
        [[ -f /etc/chromium/tamagi.conf ]] && bash -n /etc/chromium/tamagi.conf
        grep -q "/etc/chromium/tamagi.conf" /etc/chromium/chromium.conf
        grep -q -- "--oauth2-client-id=77185425430.apps.googleusercontent.com" /etc/chromium/tamagi.conf
        grep -q -- "--oauth2-client-secret=OTJgUOQcT7lO7GsGZq2G4IlT" /etc/chromium/tamagi.conf
        grep -q "ManagedOOMMemoryPressure=kill" /usr/lib/systemd/system/user@.service.d/20-tamagi-oomd.conf
        desktop-file-validate /usr/share/applications/tamagi-lark.desktop
        if [[ $(cat /usr/share/tamagi/default-lang 2>/dev/null || echo en) == en ]]; then
            grep -q "^LANG=en_US\.UTF-8" /etc/locale.conf
        fi
        TAMAGI_LANG=en tamagi-browser google-account status | grep -qi "enabled"
        tamagi-browser google-account off >/dev/null
        TAMAGI_LANG=en tamagi-browser google-account status | grep -qi "disabled"
        tamagi-browser google-account on >/dev/null
        TAMAGI_LANG=en tamagi-browser google-account status | grep -qi "enabled"
    '
fi
check "entrada genérica de navegador abre o padrão" bash -c 'grep -q "^Exec=/usr/bin/tamagi-browser %U" /usr/share/applications/tamagi-browser.desktop && desktop-file-validate /usr/share/applications/tamagi-browser.desktop && [[ ! -e /usr/share/applications/google-chrome.desktop ]]'
check "navegador padrão: a entrada genérica aponta para tamagi-browser" grep -q 'x-scheme-handler/https=tamagi-browser.desktop' /etc/xdg/mimeapps.list
check "caixa dos navegadores é Fedora"    grep -q 'fedora-toolbox:45' /usr/bin/tamagi-browser
check "tamagi-browser: Chromium nativo ativo por padrão e outros navegadores opcionais" bash -c '
    o=$(TAMAGI_LANG=en tamagi-browser list)
    ! tamagi-browser install opera-gx --native 2>/dev/null || exit 1
    ! tamagi-browser remove chromium 2>/dev/null || exit 1
    for b in chrome firefox vivaldi chromium opera-gx; do grep -q "^  $b " <<<"$o" || exit 1; done
    tamagi-browser installed | grep -qx "chromium:native"
    grep -qx "Default: chromium" <<<"$o"
    grep -q -- "--flatpak|--native|--box" /usr/bin/tamagi-browser
    ! tamagi-browser default firefox 2>/dev/null
'
check "tamagi-browser recusa navegador desconhecido" bash -c '! tamagi-browser install opera 2>/dev/null'
check "opções de aceleração de vídeo"       bash -c 'source <(sed -n "/^chromium_flags()/,/^}/p" /usr/bin/tamagi-browser); FLAGS_FILE=/nonexistent; chromium_flags | grep -q AcceleratedVideoEncoder'
check "OnlyOffice na lista de Flatpaks"    grep -qE '^flathub +org.onlyoffice.desktopeditors ' /usr/share/tamagi/flatpaks.list
check "remote cosmic-flatpak configurado"  grep -q '^Url=https://apt.pop-os.org/cosmic/' /etc/flatpak/remotes.d/cosmic.flatpakrepo
check "lista de Flatpaks só usa flathub e cosmic" bash -c '! grep -vE "^(#|$|flathub |cosmic )" /usr/share/tamagi/flatpaks.list | grep -q .'
check "tamagi-pkg e o repositório do Omarchy saíram (pacotes de outras distros: caixas)" bash -c '! command -v tamagi-pkg && [[ ! -e /usr/share/tamagi/omarchy && ! -e /usr/share/applications/tamagi-sideload.desktop ]] && ! grep -q sideload /etc/xdg/mimeapps.list'
check "web app (compartilhado, isolado e presets) gera .desktop válido e limpa ao remover" bash -c '
    tamagi-webapp install "Meu Teste" https://exemplo.invalid >/dev/null
    desktop-file-validate ~/.local/share/applications/webapp-meu-teste.desktop
    grep -q "Exec=tamagi-browser app --app=" ~/.local/share/applications/webapp-meu-teste.desktop
    tamagi-webapp list | grep -q "Meu Teste"
    tamagi-webapp install "Iso Test" https://exemplo.invalid --isolated >/dev/null
    desktop-file-validate ~/.local/share/applications/webapp-iso-test.desktop
    grep -q "^X-Tamagi-WebApp-Mode=isolated$" ~/.local/share/applications/webapp-iso-test.desktop
    grep -q -- "--user-data-dir=" ~/.local/share/applications/webapp-iso-test.desktop
    tamagi-webapp presets | grep -q "^  gmail "
    tamagi-webapp remove "Meu Teste" >/dev/null
    tamagi-webapp remove "Iso Test" --purge >/dev/null
    [[ ! -e ~/.local/share/tamagi/webapp-profiles/webapp-iso-test ]]
    [[ -z $(tamagi-webapp list) ]]
'

section "Rust"
check "COSMIC (desktop em Rust) instalado"  rpm -q cosmic-session cosmic-comp cosmic-term cosmic-files cosmic-edit
check "sudo e coreutils do shell são os do sistema (sem troca por versões em Rust)" bash -lic '[[ ! -e /usr/libexec/tamagi/rust-bin ]] && [[ $(type -P sudo) != *rust-bin* && $(type -P ls) != *rust-bin* ]] && ! rpm -q sudo-rs uutils-coreutils >/dev/null 2>&1'

section "Máquinas virtuais (nativo, sob demanda)"
if [[ $EDITION == skl ]]; then
    check "SKL: sem stack de VMs na imagem (libvirt/QEMU omitidos)" bash -c '! rpm -q libvirt-daemon-kvm cockpit-machines >/dev/null 2>&1'
else
    check "tamagi vm: Cockpit Machines, libvirt e QEMU/KVM embutidos na imagem DxAI (socket activation + NoCoW)" bash -c 'TAMAGI_LANG=en tamagi-vm status | grep -q "built in" && systemctl is-enabled cockpit.socket virtqemud.socket virtnetworkd.socket | grep -c enabled | grep -qx 3 && grep -q "virtqemud.socket" /usr/bin/tamagi-vm && grep -q "+C" /usr/lib/tmpfiles.d/tamagi-libvirt.conf && tamagi-vm --help | grep -q "tamagi vm install"'
fi
check "tamagi-layer: --repo, --unit, /opt e serviço de boot das camadas" bash -c 'tamagi-layer --help | grep -q -- "--repo id=baseurl" && tamagi-layer --help | grep -q -- "--unit" && grep -q "usr/lib/opt" /usr/bin/tamagi-layer && systemctl is-enabled tamagi-layer-units.service | grep -qx enabled && ! tamagi-layer build x --repo "ruim" cowsay 2>/dev/null && ! tamagi-layer build x --unit "ruim" cowsay 2>/dev/null'
check "administradores gerenciam VMs sem senha (polkit)" grep -q 'org.libvirt.unix.manage' /usr/share/polkit-1/rules.d/50-tamagi-libvirt.rules
check "tmpfiles gerado é bem formado" bash -c '[[ $(grep -c "^d " /usr/lib/tmpfiles.d/tamagi-autovar.conf) -ge 3 ]] && ! grep -vE "^(#.*|d /var/(lib|log|spool)/[^ ]+ [0-7]{3,4} [a-zA-Z0-9_-]+ [a-zA-Z0-9_-]+ -)$" /usr/lib/tmpfiles.d/tamagi-autovar.conf | grep -q .'

section "Atualização automática"
check "imagem do sistema: timer do bootc ligado, o do rpm-ostree não" bash -c 'systemctl is-enabled tamagi-system-update.timer | grep -qx enabled && systemctl is-enabled tamagi-system-status.service | grep -qx enabled && systemctl is-enabled tamagi-layer-rebuild.service | grep -qx enabled && ! systemctl is-enabled rpm-ostreed-automatic.timer 2>/dev/null | grep -qx enabled && systemd-analyze verify /usr/lib/systemd/system/tamagi-system-update.timer /usr/lib/systemd/system/tamagi-system-update.service /usr/lib/systemd/system/tamagi-layer-rebuild.service 2>&1 | grep -v "bootc-status-updated" | grep -c . | grep -qx 0'
check "Flatpaks: só o timer do usuário (não há Flatpak de sistema)" bash -c '! systemctl is-enabled flatpak-system-update.timer 2>/dev/null | grep -qx enabled && systemctl --global is-enabled flatpak-user-update.timer | grep -qx enabled'
check "sem remotes Flatpak do Fedora: flatpak-add-fedora-repos mascarado e limpeza ligada" bash -c '[[ $(systemctl is-enabled flatpak-add-fedora-repos.service 2>/dev/null) == masked ]] && systemctl is-enabled tamagi-flatpak-cleanup.service | grep -qx enabled'
check "ldconfig, appstream e mandb rodam uma vez por imagem, com cache pré-gerado e guarda live" bash -c 'test -s /etc/ld.so.cache && grep -qx "ConditionFileNotEmpty=" /usr/lib/systemd/system/ldconfig.service.d/10-tamagi.conf && grep -qx "ConditionPathExists=!/run/tamagi-live" /usr/lib/systemd/system/ldconfig.service.d/10-tamagi.conf && for u in ldconfig fedora-atomic-desktop-appstream-cache-refresh fedora-atomic-desktop-mandb-update; do systemctl cat $u.service | grep -q "once-per-image check" || { echo $u; exit 1; }; done'
check "once-per-image: roda na primeira vez, pula na segunda, roda de novo com imagem nova" bash -c 'd=/var/lib/tamagi/once-per-image; rm -f $d/teste; /usr/libexec/tamagi/once-per-image check teste && /usr/libexec/tamagi/once-per-image done teste && ! /usr/libexec/tamagi/once-per-image check teste && echo outra > $d/teste && /usr/libexec/tamagi/once-per-image check teste; r=$?; rm -f $d/teste; exit $r'
check "subsistemas e caixas: timer do usuário ligado" bash -c 'systemctl --global is-enabled tamagi-autoupdate.timer | grep -qx enabled'
check "units do timer são válidas"       systemd-analyze verify --user /usr/lib/systemd/user/tamagi-autoupdate.timer /usr/lib/systemd/user/tamagi-autoupdate.service
check "tamagi-update status lista os três (pt e en)" bash -c '[[ $(tamagi-update status | grep -c "próxima:") -eq 3 && $(TAMAGI_LANG=en tamagi-update status | grep -c "next:") -eq 3 ]]'
check "tamagi-update --auto roda sem caixas nem rede" bash -c 'tamagi-update --auto </dev/null'
check "quarentena de digests: leitura e status" bash -c 'q=/tmp/q.list; printf "sha256:bad001\n" > "$q"; TAMAGI_QUARANTINE="$q" tamagi-update quarantine | grep -q "sha256:bad001" && TAMAGI_QUARANTINE="$q" tamagi-update status | grep -q "quarentena: 1" && rm -f "$q"'

section "Camadas sobre a imagem (tamagi-layer)"
check "comando, ajuda e ferramentas"      bash -c 'tamagi-layer --help | grep -q "tamagi-layer add" && command -v mkfs.erofs fsck.erofs systemd-sysext >/dev/null && mkfs.erofs --help 2>&1 | grep -q file-contexts'
check "camadas entram no /usr a cada boot" systemctl is-enabled systemd-sysext.service
check "sem camadas: lista vazia e nada desatualizado" bash -c 'tamagi-layer list | grep -q "Nenhuma camada" && [[ -z $(tamagi-layer stale) ]]'
check "nomes e pacotes inválidos são recusados" bash -c '! tamagi-layer build "Ruim Nome" cowsay 2>/dev/null && ! tamagi-layer build ok "x;rm" 2>/dev/null && ! tamagi-layer build ok 2>/dev/null'
check "pacote que já está na imagem: nada a fazer" bash -c 'o=$(tamagi-layer build ja bash -o /tmp/ja.raw 2>&1); [[ $? -eq 3 ]] && grep -q "já está na imagem" <<<"$o" && [[ ! -e /tmp/ja.raw ]]'
check "constrói uma camada só com os arquivos novos" bash -c 'rm -rf /tmp/cam && mkdir /tmp/cam && tamagi-layer build teste cowsay -o /tmp/cam/teste.raw >/dev/null 2>&1 && fsck.erofs --extract=/tmp/cam/x /tmp/cam/teste.raw >/dev/null 2>&1 && test -x /tmp/cam/x/usr/bin/cowsay && grep -qx "ID=fedora" /tmp/cam/x/usr/lib/extension-release.d/extension-release.teste && grep -q "^cowsay " /tmp/cam/x/usr/share/tamagi/layers/teste.packages && [[ ! -e /tmp/cam/x/usr/share/rpm && ! -e /tmp/cam/x/usr/bin/bash && ! -e /tmp/cam/x/etc ]]'
check "a imagem não foi tocada pela construção" bash -c '! command -v cowsay && ! rpm -q cowsay >/dev/null'
check "boot com falha desliga as camadas"  bash -c 'mkdir -p /var/lib/extensions && cp /tmp/cam/teste.raw /var/lib/extensions/ && /usr/lib/tamagi/boot-check/red.d/40-tamagi-camadas.sh >/dev/null && [[ ! -e /var/lib/extensions/teste.raw && -e /var/lib/tamagi/layers-off/teste.raw ]] && tamagi-layer list | grep -q "teste.*desligada"; rc=$?; rm -rf /var/lib/tamagi/layers-off /var/lib/extensions; exit $rc'
check "tamagi.toml: camada entra no plano e pede senha" bash -c 'printf "[layers]\nferramentas = [\"strace\", \"ltrace\"]\n" > /tmp/cam.toml && tamagi sys diff /tmp/cam.toml | grep -q "camada ferramentas: strace ltrace   \[pede senha\]" && printf "[layers]\n\"Ruim\" = \"x\"\n" > /tmp/camruim.toml && ! tamagi sys diff /tmp/camruim.toml 2>/dev/null'

check "o agente local sabe do tamagi-layer e não sugere rpm-ostree install" bash -c 'grep -q "tamagi-layer add" /usr/share/tamagi/agent/system.md && ! grep -q "rpm-ostree install <pacote>\` (exige" /usr/share/tamagi/agent/system.md && ! command -v tamagi-app'

section "Inovação 1: o sistema num arquivo (tamagi sys apply)"
check "comando tamagi"                    command -v tamagi
check "export gera TOML válido com as 11 seções (em inglês, incluindo mcp)" bash -c 'tamagi sys export /tmp/t.toml >/dev/null && python3 -c "import tomllib; d = tomllib.load(open(\"/tmp/t.toml\", \"rb\")); assert set(d) == {\"appearance\", \"browsers\", \"flatpak\", \"mise\", \"agents\", \"mcp\", \"catalog\", \"boxes\", \"layers\", \"webapps\", \"services\"}, d.keys()"'
check "diff do próprio estado: nada a fazer" bash -c 'tamagi sys diff /tmp/t.toml | grep -q "Nada a fazer"'
cat > /tmp/quero.toml <<'TOML'
[appearance]
theme = "nord"
[browsers]
install = ["firefox", "chrome:native"]
[flatpak]
apps = ["org.gimp.GIMP"]
[mise]
tools = ["node@lts"]
[agents]
install = ["codex"]
[catalog]
install = ["steam"]
[boxes]
dev = "arch"
[webapps]
WhatsApp = "https://web.whatsapp.com"
[services]
security = "strict"
TOML
check "diff lista o que falta e marca o que pede senha" bash -c 'o=$(tamagi sys diff /tmp/quero.toml); for x in "tema: nord" "navegador: firefox (flatpak)" "navegador: chrome (native)   \[pede senha\]" "Flatpak: org.gimp.GIMP" "mise: node@lts" "agente: codex" "catálogo: steam   \[pede senha\]" "caixa dev (arch)" "web app: WhatsApp" "segurança: strict   \[pede senha\]"; do grep -q "$x" <<<"$o" || { echo "faltou: $x"; exit 1; }; done'
printf '[mcp]\nservers = ["memory", "git"]\n' > /tmp/mcp-want.toml
check "seção [mcp] entra no plano do tamagi sys diff" bash -c 'o=$(tamagi sys diff /tmp/mcp-want.toml) && grep -q "MCP: memory" <<<"$o" && grep -q "MCP: git" <<<"$o"'
printf '[appearance]\ntheme = "gruvbox"\n[webapps]\nExemplo = "https://exemplo.invalid"\n' > /tmp/so-local.toml
check "apply executa e fica idempotente"  bash -c 'tamagi sys apply -y /tmp/so-local.toml >/dev/null && [[ $(tamagi-theme current) == gruvbox ]] && test -f ~/.local/share/applications/webapp-exemplo.desktop && tamagi sys diff /tmp/so-local.toml | grep -q "Nada a fazer"; rc=$?; tamagi-webapp remove Exemplo >/dev/null; tamagi-theme set ipe >/dev/null; exit $rc'
check "apply sem terminal e sem -y não altera nada" bash -c '! tamagi sys apply /tmp/quero.toml </dev/null >/dev/null 2>&1 && [[ $(tamagi-theme current) != nord ]]'
printf '[browsers]\ninstall = ["opera"]\n[boxes]\nx = "gentoo"\n[inventado]\na = 1\n' > /tmp/ruim.toml
check "arquivo com erros é recusado com a lista" bash -c 'o=$(tamagi sys diff /tmp/ruim.toml 2>&1); [[ $? -ne 0 ]] && grep -q "navegador desconhecido: opera" <<<"$o" && grep -q "gentoo" <<<"$o" && grep -q "inventado" <<<"$o"'

section "Perfis isolados do navegador e chave de segurança"
check "cria perfis com atalho próprio no menu" bash -c 'export XDG_DATA_HOME=/tmp/perfis; rm -rf /tmp/perfis; tamagi-browser profile add trabalho chrome >/dev/null && tamagi-browser profile add pessoal firefox >/dev/null && desktop-file-validate /tmp/perfis/applications/tamagi-chrome-trabalho.desktop && grep -qx "Exec=/usr/bin/tamagi-browser chrome --profile trabalho %U" /tmp/perfis/applications/tamagi-chrome-trabalho.desktop && grep -qx "StartupWMClass=tamagi-chrome-trabalho" /tmp/perfis/applications/tamagi-chrome-trabalho.desktop && [[ -d /tmp/perfis/tamagi/browser-profiles/chrome-trabalho && -d /tmp/perfis/tamagi/browser-profiles/firefox-pessoal ]]'
check "lista os perfis; nome inválido é recusado" bash -c 'export XDG_DATA_HOME=/tmp/perfis; o=$(tamagi-browser profile list) && grep -q "^chrome *trabalho$" <<<"$o" && grep -q "^firefox *pessoal$" <<<"$o" && ! tamagi-browser profile add "Ruim Nome" 2>/dev/null && ! tamagi-browser profile add "../x" 2>/dev/null'
check "remover perfil exige confirmação num terminal" bash -c 'export XDG_DATA_HOME=/tmp/perfis; ! tamagi-browser profile remove trabalho </dev/null >/dev/null 2>&1 && [[ -d /tmp/perfis/tamagi/browser-profiles/chrome-trabalho ]]'
check "diagnóstico da chave de segurança roda sem chave e sem caixa" bash -c 'o=$(tamagi-browser fido 2>&1); grep -q "Chaves de segurança vistas pelo sistema" <<<"$o"'
check "modo banco e Warsaw saíram da imagem" bash -c '! command -v tamagi-bank && ! command -v tamagi-warsaw && [[ ! -e /usr/share/tamagi/warsaw && ! -e /usr/share/applications/tamagi-bank.desktop ]] && ! grep -rqi warsaw /usr/bin/tamagi-* /usr/libexec/tamagi /usr/share/ublue-os/just/60-custom.just'

section "Inovação 3: boot que se conserta e se explica"
check "verificação de saúde do boot ligada (sem greenboot)" bash -c 'systemctl is-enabled tamagi-boot-check.service | grep -qx enabled && ! rpm -q greenboot >/dev/null'
check "só as verificações do Tamagi (nenhuma exige rede)" bash -c '[[ $(ls /usr/lib/tamagi/boot-check/required.d/) == 10-tamagi-essencial.sh ]]'
check "imagem nova que falha: tenta de novo, depois volta; imagem já aprovada não volta" bash -c 'D=$(mktemp -d); mkdir -p $D/c/required.d $D/c/wanted.d $D/c/red.d $D/s; printf "#!/bin/sh\nexit 1\n" > $D/c/required.d/1.sh; printf "#!/bin/sh\necho red >> $D/red\n" > $D/c/red.d/40-tamagi-camadas.sh; cp $D/c/red.d/40-tamagi-camadas.sh $D/c/red.d/50-tamagi-relatorio.sh; chmod +x $D/c/*/*.sh; B="env TAMAGI_BOOT_CHECK_DRYRUN=1 TAMAGI_BOOT_CHECK_DIR=$D/c TAMAGI_BOOT_STATE=$D/s TAMAGI_BOOT_ID=novo TAMAGI_BOOT_HAS_ROLLBACK=1 /usr/libexec/tamagi/boot-check"; $B | grep -q "trying once more" && $B | grep -q "going back" && [[ ! -e $D/s/good ]] && echo novo > $D/s/good && $B | grep -q "no automatic way back" && printf "#!/bin/sh\nexit 0\n" > $D/c/required.d/1.sh && rm $D/s/good && $B | grep -qx "boot-check: ok" && [[ $(cat $D/s/good) == novo ]]'
check "relatório de falha é gravado"       bash -c 'mkdir -p /var/lib/tamagi && bash /usr/lib/tamagi/boot-check/red.d/50-tamagi-relatorio.sh && ls /var/lib/tamagi/boot-failures/*.txt >/dev/null && grep -q "Verificações de saúde" /var/lib/tamagi/boot-failures/*.txt'
check "tamagi-boot-report mostra o pendente e marca como visto" bash -c 'chmod a+r /var/lib/tamagi/boot-failures/*.txt; a=$(tamagi-boot-report show </dev/null); b=$(tamagi-boot-report show </dev/null); grep -q "voltou para a imagem anterior" <<<"$a" && grep -q "Nenhuma falha de boot pendente" <<<"$b"'
if [[ $EDITION == skl ]]; then
    check "SKL: mise omitido da imagem escolar" bash -c '! command -v mise >/dev/null 2>&1'
else
    check "DxAI: mise nativo no host (shims no PATH) e isolado por caixa nas devboxes" bash -c 'command -v mise >/dev/null && mise --version | grep -qE "^[0-9]+\." && grep -q "mise/shims" /usr/lib/environment.d/50-tamagi-path.conf && grep -q "mise activate bash" /etc/profile.d/tamagi.sh && grep -q "install_mise" /usr/libexec/tamagi/devbox-provision && grep -q "MISE_DATA_DIR=.*mise-\$_box" /usr/libexec/tamagi/devbox-provision && grep -q "ensure_host_node" /usr/libexec/tamagi/agent-manager'
fi
check "tamagi pkg --native chrome e tamagi pkg remove (navegador + camada + agentes)" bash -c '
    D=/tmp/inst-nat; rm -rf "$D"; mkdir -p "$D/bin"
    printf "%s\n" "#!/bin/bash" "echo browser:\$*" >> "$D/bin/tamagi-browser"
    printf "%s\n" "#!/bin/bash" "echo layer:\$*" >> "$D/bin/tamagi-layer"
    chmod +x "$D/bin/"*
    PATH="$D/bin:$PATH" TAMAGI_INSTALL_NO_FLATHUB=1 TAMAGI_LANG=en tamagi-install search chrome | grep -q "\[Native / Flatpak\]"
    PATH="$D/bin:$PATH" TAMAGI_LANG=en tamagi-install --native chrome | grep -q "browser:install chrome --native"
    PATH="$D/bin:$PATH" TAMAGI_LANG=en tamagi-remove chrome | grep -q "browser:remove chrome"
    PATH="$D/bin:$PATH" TAMAGI_LANG=en tamagi pkg remove chrome | grep -q "browser:remove chrome"
    rm -rf "$D"
'
check ".local/bin do usuário no PATH de toda sessão (agentes instalados no usuário)" bash -lc 'case ":$PATH:" in *"/.local/bin:"*) exit 0 ;; *) exit 1 ;; esac'


section "Segurança e desempenho"
check "sysctl de endurecimento na imagem"   grep -q '^kernel.kptr_restrict = 2' /usr/lib/sysctl.d/60-tamagi-hardening.conf
check "sysctls são aceitos pelo systemd-sysctl" bash -c 'out=$(/usr/lib/systemd/systemd-sysctl --cat-config 2>&1); echo "$out" | grep -q "kernel.yama.ptrace_scope = 1" && echo "$out" | grep -q "vm.max_map_count = 2147483642"'
check "argumentos de kernel (bootc kargs.d) são TOML válido" python3 -c "import tomllib; d = tomllib.load(open('/usr/lib/bootc/kargs.d/10-tamagi.toml', 'rb')); assert 'init_on_alloc=1' in d['kargs']"
check "MAC aleatório por rede no NetworkManager" grep -q 'wifi.cloned-mac-address=stable' /usr/lib/NetworkManager/conf.d/50-tamagi-privacy.conf
check "hardening extra (USBGuard/hardened_malloc) opcional no catálogo" bash -c '! rpm -q usbguard hardened_malloc >/dev/null 2>&1 && grep -qP "^hardening-extra\t" /usr/share/tamagi/catalog.tsv && test ! -e /etc/ld.so.preload'
check "tamagi-harden status"                bash -c 'tamagi-harden status | grep -q "Modo.*equilibrado"'
check "zram: zstd, metade da RAM"           grep -q 'compression-algorithm = zstd' /usr/lib/systemd/zram-generator.conf.d/10-tamagi.conf
check "regras udev de E/S são válidas"      bash -c 'udevadm verify /usr/lib/udev/rules.d/60-tamagi-ioschedulers.rules'
if [[ $EDITION != skl ]]; then
    check "base Bazzite de jogos na imagem DxAI (steam-devices, gamescope, mangohud, gamemode, Mesa/Vulkan 32-bit); Steam sob demanda" bash -c 'rpm -q steam-devices gamescope mangohud gamemode mesa-vulkan-drivers.i686 >/dev/null 2>&1 && ! command -v steam >/dev/null 2>&1'
    section "Agentes de terceiros"
    check "Claude Code executa"                claude --version
    check "Claude Code se atualiza sozinho (mora em ~/.local)" bash -lc '[[ -z ${DISABLE_AUTOUPDATER:-} ]]'
fi

section "Scripts"
for f in /usr/bin/tamagi-* /usr/libexec/tamagi/*; do
    [[ -f $f ]] || continue
    # Os programas em Rust (tamagi-layer, agent-jail) são binários: os testes deles rodam no build.
    head -c 4 "$f" | grep -q "ELF" && continue
    if [[ $f == *.py ]] || head -1 "$f" | grep -q python; then
        check "sintaxe $(basename "$f")" python3 -c "import ast,sys; ast.parse(open(sys.argv[1]).read())" "$f"
    else
        check "sintaxe $(basename "$f")" bash -n "$f"
    fi
done
check "ferramentas em Rust: tamagi, tamagi-why, tamagi-layer, tamagi-doctor, tamagi-install, tamagi-boot e agent-jail são binários estáticos" bash -c 'for b in /usr/bin/tamagi /usr/bin/tamagi-why /usr/bin/tamagi-layer /usr/bin/tamagi-doctor /usr/bin/tamagi-install /usr/libexec/tamagi/tamagi-boot /usr/libexec/tamagi/agent-jail; do file -L $b | grep -Eq "static(ally|-pie) linked" || { echo $b; exit 1; }; done; tamagi | grep -q "tamagi update" && tamagi | grep -q "tamagi sys" && tamagi nosuchcommand 2>/dev/null; [[ $? -ne 0 ]]'
check "instalação rápida: runtime adiantado, Flatpaks em segundo plano, etapas numeradas, aviso de runtimes" bash -c 'D=/tmp/inst; rm -rf $D; mkdir -p $D/bin $D/h $D/run; printf "%s\n" "#!/bin/bash" "case \"\$1\" in remote-add) exit 0 ;; remote-info) case \"\${@: -1}\" in *VLC) echo \" Runtime: org.kde.Platform/x86_64/5.15-25.08\" ;; *) echo \" Runtime: org.freedesktop.Platform/x86_64/26.08\" ;; esac ;; info) grep -qx \"\${@: -1}\" $D/state ;; install) for a in \"\$@\"; do case \$a in runtime/*|org.*|com.*) echo \$a >> $D/state ;; esac; done ;; list) grep -v ^runtime/ $D/state ;; uninstall) sed -i \"\\|^\${@: -1}\$|d\" $D/state ;; esac" > $D/bin/flatpak; printf "%s\n" "#!/bin/bash" "echo vm-ok" > $D/bin/tamagi-vm; printf "%s\n" "#!/bin/bash" "exit 1" > $D/bin/nmcli; chmod +x $D/bin/*; : > $D/state; export PATH=$D/bin:$PATH HOME=$D/h XDG_CACHE_HOME=$D/h/.cache XDG_RUNTIME_DIR=$D/run TAMAGI_LANG=en; tamagi-install prefetch && grep -qx "runtime/org.freedesktop.Platform/x86_64/26.08" $D/state && o=$(tamagi-install --actions flatpak:flathub:org.videolan.VLC flatpak:flathub:com.obsproject.Studio "run:tamagi-vm install" 2>&1) && grep -qE "\[1/2\].*Flatpak" <<<"$o" && grep -qE "\[2/2\].*tamagi-vm install" <<<"$o" && grep -q "org.kde.Platform 5.15-25.08 .*to download" <<<"$o" && grep -q "org.freedesktop.Platform 26.08 .*already here" <<<"$o" && grep -q "in the background" <<<"$o" && grep -q "vm-ok" <<<"$o" && grep -qx "org.videolan.VLC" $D/state && : > $D/state && tamagi-install prefetch && tamagi-install prefetch-cleanup && [[ ! -s $D/state ]] && grep -q "tamagi-install prefetch" /usr/bin/tamagi-firstrun'
check "atualização só com imagem assinada: política exige uma das duas chaves do Tamagi para ghcr.io/wes-tamagi" bash -c 'jq -e ".transports.docker[\"ghcr.io/wes-tamagi\"][0] | .type == \"sigstoreSigned\" and .keyPaths == [\"/etc/pki/containers/tamagi.pub\", \"/etc/pki/containers/tamagi-2.pub\"] and .signedIdentity.type == \"matchRepository\"" /etc/containers/policy.json >/dev/null && grep -q "BEGIN PUBLIC KEY" /etc/pki/containers/tamagi.pub && grep -q "BEGIN PUBLIC KEY" /etc/pki/containers/tamagi-2.pub && ! cmp -s /etc/pki/containers/tamagi.pub /etc/pki/containers/tamagi-2.pub && grep -q "use-sigstore-attachments: true" /etc/containers/registries.d/tamagi.yaml'
check "100% Flathub (sem fedora-flatpak), RPM Fusion Free desabilitado pós-build e /etc drift journal ativo" bash -c 'test -s /etc/flatpak/remotes.d/flathub.flatpakrepo && ! ls /etc/flatpak/remotes.d/fedora*.flatpakrepo >/dev/null 2>&1 && rpm -q ffmpeg mesa-va-drivers-freeworld >/dev/null && ! grep -q "^enabled=1" /etc/yum.repos.d/*rpmfusion*.repo /usr/lib/yum.repos.d/*rpmfusion*.repo 2>/dev/null && systemctl is-enabled tamagi-etc-drift.timer | grep -qx enabled && test -x /usr/libexec/tamagi/etc-drift'
if [[ $EDITION == skl ]]; then
    check "SKL: papéis de parede ilustrados extras removidos" bash -c '[[ ! -d /usr/share/backgrounds/tamagi/art ]]'
else
    check "papéis de parede ilustrados (10 em art) + papéis espaciais COSMIC sem marca" bash -c '[[ $(ls /usr/share/backgrounds/tamagi/art/*.jpg | wc -l) -eq 10 ]] && for f in /usr/share/backgrounds/tamagi/art/*.jpg; do file "$f" | grep -q "2560x1440" || { echo "$f"; exit 1; }; done && ! find /usr/share/backgrounds/tamagi -maxdepth 1 \( -iname "*logo*" -o -iname "*pop*" -o -iname "*system76*" \) | grep -q .'
fi
check "a janela de primeiro login abre flutuante (regra de exceção ao tiling)" bash -c 'grep -q "appid: \"org.tamagi.Welcome\"" /usr/share/cosmic/com.system76.CosmicSettings.WindowRules/v1/tiling_exception_defaults'
check "primeiro login: janela nativa (libcosmic) com o assistente em texto de reserva" bash -c 'test -x /usr/bin/tamagi-welcome && file -L /usr/bin/tamagi-welcome | grep -q "ELF 64-bit" && ! ldd /usr/bin/tamagi-welcome | grep -q "not found" && grep -q "tamagi-welcome && exit 0" /usr/libexec/tamagi/firstrun-launch && grep -q "popup tamagi-firstrun" /usr/libexec/tamagi/firstrun-launch && grep -q "^Exec=tamagi-welcome --force" /usr/share/applications/org.tamagi.Welcome.desktop && mkdir -p /tmp/wl/tamagi && touch /tmp/wl/tamagi/firstrun-done && XDG_CONFIG_HOME=/tmp/wl timeout 10 tamagi-welcome'
check "um comando só: sem ujust nem scripts legados de TPM; tamagi setup cobre tudo" bash -c '! command -v ujust && command -v just >/dev/null && ! rpm -q ublue-os-just >/dev/null 2>&1 && [[ ! -e /usr/share/ublue-os/just && ! -e /usr/libexec/luks-enable-tpm2-autounlock ]] && for a in secure-boot-key tpm-unlock firmware bios clean lazyvim switch-image; do tamagi-setup help | grep -q "tamagi-setup $a" || exit 1; done; ! tamagi-setup switch-image "x; y" 2>/dev/null && test -f /etc/pki/akmods/certs/akmods-ublue.der && ! grep -rn "ujust" /usr/bin/tamagi* /usr/libexec/tamagi /usr/share/tamagi/actions.toml'

section "llama.cpp"
if [[ $EDITION == skl ]]; then
    check "SKL: llama.cpp omitido da imagem escolar" bash -c '! command -v llama-server >/dev/null 2>&1'
else
    check "llama-server executa"              llama-server --version
    check "backend Vulkan presente"           bash -c 'ls /usr/lib64/ggml/*vulkan*'
    check "variantes de CPU presentes"        bash -c 'ls /usr/lib64/ggml/*cpu*haswell* /usr/lib64/ggml/*cpu*x64*'
    check "sem bibliotecas faltando"          bash -c '! ldd /usr/bin/llama-server | grep -q "not found"'
    check "interface web embutida"            bash -c 'grep -qa "<!doctype html\|<!DOCTYPE html" /usr/bin/llama-server /usr/lib64/libllama*.so* /usr/lib64/*.so 2>/dev/null || llama-server --help | grep -q -- "--ui"'
    check "presets: 11 modelos (Gemma 4, Ornith 1.5, Mistral/Devstral, Qwen 3.8/3.6)" bash -c '[[ $(tamagi-ai presets | grep -cE "^(mini|leve|mistral|agente|medio|devstral|qwen38|codigo|ornith|grande|workstation) ") -eq 11 ]]'
    check "presets em inglês funcionam"          bash -c 'TAMAGI_LANG=en tamagi-ai presets | grep -q "ROLE" && TAMAGI_LANG=en tamagi-ai presets | grep -q "Ornith 1.5 9B"'
    check "recomendação devolve um preset geral" bash -c 'tamagi-ai recommend | grep -qxE "mini|leve|medio|qwen38|grande"'
    check "llama-server recebe --jinja e --fit on" bash -c 'grep -q -- "--jinja" /usr/bin/tamagi-ai && grep -q -- "--fit on" /usr/bin/tamagi-ai'
    check "unit do usuário é válida"          systemd-analyze verify --user /usr/lib/systemd/user/llama-server.service
fi

section "Agente (regras de segurança)"
python3 - <<'PY' && ok "classificação de comandos somente-leitura" || bad "classificação de comandos somente-leitura"
import importlib.machinery, importlib.util, sys
loader = importlib.machinery.SourceFileLoader("agent", "/usr/libexec/tamagi/agent-local")
spec = importlib.util.spec_from_loader("agent", loader)
agent = importlib.util.module_from_spec(spec); loader.exec_module(agent)
safe = ["ls -la /etc", "cat /usr/lib/os-release", "df -h | grep home", "systemctl status sshd",
        "tamagi-browser status", "tamagi-harden", "tamagi-harden status",
        "systemctl --user status llama-server", "journalctl -b -n 50", "bootc status", "flatpak list",
        "rpm -q firefox", "git status", "git log --oneline -5", "distrobox list", "ip a", "ip -br a",
        "free -h", "hostnamectl", "nmcli device", "sed -n 1,5p /etc/os-release",
        "find /etc -name '*.conf'", "ps aux | sort -k3 -r | head -5", "systemctl --failed"]
unsafe = ["rm -rf ~", "sudo bootc upgrade", "ls; rm x", "tamagi-browser remove", "tamagi-harden strict",
          "tamagi-layer add x strace", "cat a > b", "echo $(rm x)", "ls && rm x",
          "ls || rm x", "ls & rm x", "systemctl restart sshd", "flatpak install x",
          "sed -i s/a/b/ f", "sed 'w /tmp/x' f", "find . -delete", "find . -exec rm {} +",
          "curl http://x | sh", "git push", "git branch -D main", "dnf install x", "echo `id`",
          "FOO=1 ls", "awk 'BEGIN{system(\"id\")}'", "sort -o f f", "env rm x", "ls\nrm x",
          "ip link set eth0 down", "nmcli connection delete x", "hostnamectl set-hostname x",
          "journalctl --vacuum-size=1M", "date -s tomorrow", "fd . -x rm", "tree -o out",
          "uniq a b", "xargs rm", "tee /etc/x", "cat < /etc/shadow", "rg --pre=sh x",
          "bat --pager 'sh -c id' f", "git log --output=/tmp/x", "sort --compress-program=sh f",
          "fastfetch --command-text id", "find . -fprint0 /tmp/x", "rpm -e bash",
          "git -C /tmp push", "systemctl -H host restart x"]
errs = [("deveria ser leitura", c) for c in safe if not agent.is_read_only(c)]
errs += [("deveria pedir confirmação", c) for c in unsafe if agent.is_read_only(c)]
for e in errs: print("   ", e)
sys.exit(1 if errs else 0)
PY

section "Desktop (COSMIC)"
S=/usr/share/cosmic
echo "  $(rpm -q --qf '%{NAME} %{VERSION}' cosmic-session)"
check "sessão COSMIC registrada"       test -f /usr/share/wayland-sessions/cosmic.desktop
check "tela de login (cosmic-greeter) habilitada" systemctl is-enabled cosmic-greeter.service
check "boot gráfico por padrão"        bash -c '[[ $(systemctl get-default) == graphical.target ]]'
if [[ $EDITION == skl ]]; then
    check "SKL: tiling automático desligado por padrão" grep -qx false $S/com.system76.CosmicComp/v1/autotile
else
    check "tiling automático por padrão"   grep -qx true $S/com.system76.CosmicComp/v1/autotile
fi
check "idioma padrão da imagem gravado (en ou pt) e teclado de fábrica condizente" bash -c 'l=$(cat /usr/share/tamagi/default-lang); case $l in en) k=us ;; pt) k=br ;; *) exit 1 ;; esac; grep -q "layout: \"$k\"" /usr/share/cosmic/com.system76.CosmicComp/v1/xkb_config && [[ $(tamagi-lang list | head -1 | cut -d" " -f1) == "$l" ]]'
check "papel de parede padrão: brasão (mon) do Ipê" bash -c 'grep -q tamagi/ipe-mon.png /usr/share/cosmic/com.system76.CosmicBackground/v1/all && test -s /usr/share/backgrounds/tamagi/ipe-mon.png'
check "5 temas (Ipê, Ipê Claro, Tokyo Night, Nord, Gruvbox) com 9 papéis de parede PNG enxutos em 2560px" bash -c '[[ $(tamagi-theme list | tr "\n" " ") == "gruvbox ipe ipe-light nord tokyo-night " && $(ls /usr/share/backgrounds/tamagi/*.png | wc -l) -eq 9 ]]'
check "cada tema tem o seu tema do COSMIC (.ron) e a miniatura, junto dos do COSMIC" bash -c 'for t in $(tamagi-theme list); do f=/usr/share/cosmic-themes/tamagi-$t.ron; grep -q "^    accent: Some" "$f" && grep -q "^    palette: " "$f" && file "/usr/share/cosmic-themes/tamagi-$t.png" | grep -q "768 x 432" || { echo "$t"; exit 1; }; done; tamagi-theme list | grep -qx ipe'
check "papéis de parede só em PNG, na pasta que o COSMIC lista" bash -c '! ls /usr/share/backgrounds/tamagi/*.svg >/dev/null 2>&1 && [[ ! -e /usr/share/tamagi/wallpapers.xml ]]'
check "tema padrão do COSMIC é o Ipê (escuro) e o Ipê Claro (claro); os terminais combinam" bash -c 'grep -qi "#16211C" /usr/share/cosmic/com.system76.CosmicTheme.Dark.Builder/v2/bg_color && grep -qi "#FAF6EC" /usr/share/cosmic/com.system76.CosmicTheme.Light.Builder/v2/bg_color && [[ -z $(XDG_CONFIG_HOME=/tmp/semtema tamagi-theme sync) ]] && [[ $(XDG_CONFIG_HOME=/tmp/semtema tamagi-theme current) == ipe ]]'
check "temas do COSMIC com os cantos do Material 3 (4/8/12/28 px, botões em pílula)" bash -c 'for f in /usr/share/cosmic-themes/tamagi-*.ron; do grep -q "radius_m: (12.0" "$f" && grep -q "radius_l: (28.0" "$f" || { echo "$f"; exit 1; }; done'
check "tema salvo que não existe mais volta para o Ipê" bash -c 'mkdir -p /tmp/velho/tamagi && echo kanagawa > /tmp/velho/tamagi/theme && [[ $(XDG_CONFIG_HOME=/tmp/velho tamagi-theme current) == ipe ]]'
for w in ipe ipe-mon ipe-seigaiha ipe-light ipe-light-mon ipe-light-seigaiha gruvbox-mon nord-mon tokyo-night-mon; do
    check "papel $w é PNG 2560px" bash -c "file /usr/share/backgrounds/tamagi/$w.png | grep -q '2560 x 1440'"
done
D=$S/com.system76.CosmicSettings.Shortcuts/v1/defaults
check "atalho Super+Enter abre o terminal" grep -q 'key: "Return"): System(Terminal)' $D
check "atalho Super+Espaço abre o menu iniciar (o botão do Tamagi)" grep -q '(modifiers: \[Super\], key: "space"): Spawn("/usr/libexec/tamagi/start-menu")' $D
check "menu iniciar pelo teclado: o applet aceita --toggle" bash -c 'grep -q "cosmic-ext-classic-menu-applet --toggle" /usr/libexec/tamagi/start-menu && cosmic-ext-classic-menu-applet --help | grep -q -- "--toggle"'
check "atalho Super+Alt+Espaço abre o lançador COSMIC" grep -q '(modifiers: \[Super, Alt\], key: "space"): System(Launcher)' $D
check "atalho Super+W fecha a janela"  grep -q '(modifiers: \[Super\], key: "w"): Close' $D
check "atalho Super+A abre a paleta de comandos" grep -q '(modifiers: \[Super\], key: "a"): Spawn("tamagi-menu")' $D
check "sem combinações de teclas duplicadas" bash -c "[[ -z \$(grep -oE '^\s*\(modifiers: \[[^]]*\](, key: \"[^\"]+\")?\)' $D | sed 's/ //g' | sort | uniq -d) ]]"
check "arquivo de atalhos termina bem formado" bash -c "tail -1 $D | grep -qx '}'"
check "janelas do Tamagi usam Ptyxis (com fallback cosmic-term) e ícone do menu válido" bash -c 'grep -q "exec ptyxis --new-window" /usr/libexec/tamagi/popup && desktop-file-validate /usr/share/applications/org.tamagi.Menu.desktop && grep -qx "Icon=tamagi-menu" /usr/share/applications/org.tamagi.Menu.desktop && [[ ! -e /usr/share/applications/tamagi-popup.desktop ]] && grep -q "appid: \"org.tamagi.Menu\"" '"$S"'/com.system76.CosmicSettings.WindowRules/v1/tiling_exception_defaults'
check "tamagi-menu (GTK4 + actions.toml) lista ações em pt e en" bash -c 'python3 -c "import gi; gi.require_version(\"Gtk\",\"4.0\"); gi.require_version(\"Adw\",\"1\"); from gi.repository import Gtk, Adw" && tamagi-menu --list | grep -q "^ai.setup " && TAMAGI_LANG=en tamagi-menu --json | jq -e "length > 20 and any(.[]; .id == \"ai.setup\" and (.title | contains(\"AI model\")))"'
check "tamagi-agent carrega tamagi_action do actions.toml" bash -c 'python3 -c "import importlib.machinery, importlib.util; l=importlib.machinery.SourceFileLoader(\"a\",\"/usr/libexec/tamagi/agent-local\"); s=importlib.util.spec_from_loader(\"a\",l); m=importlib.util.module_from_spec(s); l.exec_module(m); assert \"ai.status\" in m.AGENT_ACTIONS and any(t[\"function\"][\"name\"]==\"tamagi_action\" for t in m.TOOLS)"'
check "regras de janela terminam bem formadas" bash -c "tail -1 $S/com.system76.CosmicSettings.WindowRules/v1/tiling_exception_defaults | grep -qx ']'"
check "dock enxuto: navegador genérico, arquivos, terminal" bash -c 'F=/usr/share/cosmic/com.system76.CosmicAppList/v1/favorites; grep -q "\"tamagi-browser\"" $F && grep -q CosmicFiles $F && ! grep -q "google-chrome\|claude-code" $F'
for fav in $(grep -oE '"[^"]+"' $S/com.system76.CosmicAppList/v1/favorites | tr -d '"'); do
    check "favorito $fav existe" bash -c "test -f '/usr/share/applications/$fav.desktop' || grep -qE '^(flathub|cosmic) +$fav ' /usr/share/tamagi/flatpaks.list"
done
check "portal do COSMIC"               rpm -q xdg-desktop-portal-cosmic
if [[ $EDITION == skl ]]; then
    check "SKL: primeiro login abre Chromium escolar (sem assistente firstrun nem flatpak-setup automático)" bash -c 'test ! -e /etc/xdg/autostart/tamagi-firstrun.desktop && test -f /etc/xdg/autostart/tamagi-skl-browser.desktop && ! systemctl --global is-enabled tamagi-flatpak-setup.service 2>/dev/null | grep -qx enabled'
else
    check "primeiro login: só o assistente do Tamagi abre sozinho" bash -c 'test ! -e /etc/xdg/autostart/com.system76.CosmicInitialSetup.desktop && test -f /etc/xdg/autostart/tamagi-firstrun.desktop && command -v cosmic-initial-setup'
fi
check "janelas do Tamagi usam Ptyxis (sem alacritty nem foot)" bash -c '! command -v alacritty && ! command -v foot && grep -q "exec ptyxis --new-window" /usr/libexec/tamagi/popup && ! grep -qE "Alacritty|foot" /etc/xdg/xdg-terminals.list'
check "papel de parede é do COSMIC: o tema só troca as cores de um papel do Tamagi" bash -c 'B=~/.config/cosmic/com.system76.CosmicBackground/v1/all; rm -f $B; tamagi-theme set gruvbox >/dev/null && grep -q gruvbox-mon.png $B && sed -i "s|gruvbox-mon.png|ipe-seigaiha.png|" $B && tamagi-theme set ipe-light >/dev/null && grep -q ipe-light-seigaiha.png $B && sed -i "s|Path(\".*\")|Path(\"/usr/share/backgrounds/default.jxl\")|" $B && tamagi-theme set gruvbox >/dev/null && grep -q default.jxl $B; r=$?; rm -f $B; tamagi-theme set ipe >/dev/null; exit $r'
check "terminais seguem o tema escolhido no COSMIC (tamagi theme sync)" bash -c 'export XDG_CONFIG_HOME=/tmp/sync XDG_DATA_HOME=/tmp/sync-data; rm -rf /tmp/sync /tmp/sync-data; XDG_CONFIG_HOME=/tmp/sync HOME=/tmp/sync cosmic-settings appearance import /usr/share/cosmic-themes/tamagi-nord.ron >/dev/null 2>&1; tamagi-theme sync | grep -q nord && [[ $(tamagi-theme current) == nord ]] && test -s /tmp/sync-data/org.gnome.Ptyxis/palettes/tamagi.palette && [[ -z $(tamagi-theme sync) ]] && systemctl --global is-enabled tamagi-theme-sync.path | grep -qx enabled && systemd-analyze verify --user /usr/lib/systemd/user/tamagi-theme-sync.path /usr/lib/systemd/user/tamagi-theme-sync.service'
check "tamagi-theme aplica um tema (e sincroniza Papirus-Dark/Light)" bash -c 'tamagi-theme set gruvbox >/dev/null && grep -q gruvbox-mon.png ~/.config/cosmic/com.system76.CosmicBackground/v1/all && grep -q "Papirus-Dark" ~/.config/cosmic/com.system76.CosmicTk/v1/icon_theme && [[ $(tamagi-theme current) == gruvbox ]]'
for t in $(tamagi-theme list); do
    check "tema $t é um arquivo válido"     bash -c "source /usr/share/tamagi/themes/$t.conf && [[ \${#PALETTE[@]} -eq 16 && \${#WALL[@]} -eq 5 ]]"
done
section "Serviços"
if [[ $EDITION != skl ]]; then
    check "flatpak-setup habilitado (--user global)" systemctl --global is-enabled tamagi-flatpak-setup.service
fi
check "units do sistema são válidas"      bash -c 'systemd-analyze verify /usr/lib/systemd/system/tamagi-firstboot.service && test -s /usr/lib/systemd/user/tamagi-flatpak-setup.service && grep -q "^ExecStart=/usr/libexec/tamagi/flatpak-setup" /usr/lib/systemd/user/tamagi-flatpak-setup.service'
check "provisionamento no primeiro boot (formato Combustion): serviço ligado, só com a marca do instalador" bash -c 'systemctl is-enabled tamagi-combustion.service >/dev/null && grep -q "^ConditionPathExists=/etc/tamagi/first-boot" /usr/lib/systemd/system/tamagi-combustion.service && test -x /usr/libexec/tamagi/combustion && bash -n /usr/libexec/tamagi/combustion && /usr/libexec/tamagi/combustion && [[ ! -e /var/lib/tamagi/combustion-done ]]'
if [[ $EDITION == skl ]]; then
    check "SKL: sem GNOME Disks nem cosmic-ext-storage; trava de 2 abas/1 janela presente" bash -c '! command -v gnome-disks >/dev/null 2>&1 && ! command -v cosmic-ext-storage >/dev/null 2>&1 && test -f /usr/share/tamagi/skl-tab-guard/manifest.json && test -f /usr/share/tamagi/skl-tab-guard/background.js'
else
    check "armazenamento nativo: GNOME Disks (gnome-disk-utility) + rclone na imagem (sem cosmic-ext-storage)" bash -c 'command -v gnome-disks >/dev/null && command -v rclone >/dev/null && ! command -v cosmic-ext-storage >/dev/null 2>&1 && ! command -v cosmic-player >/dev/null 2>&1'
fi
check "imagem enxuta: ferramentas de terminal vivem na caixa de desenvolvimento, que as exporta ao ser criada" bash -c 'for c in nvim lazygit gh uv glow hyperfine direnv yq shellcheck; do ! command -v $c >/dev/null 2>&1 || { echo "ainda na imagem: $c"; exit 1; }; done; P=/usr/libexec/tamagi/devbox-provision; for t in neovim lazygit gh uv glow hyperfine direnv yq ShellCheck; do grep -q "$t" $P || { echo "a caixa não instala $t"; exit 1; }; done; grep -q "distrobox-export --bin" /usr/bin/tamagi-devbox && grep -q "export_tools \"\$name\"" /usr/bin/tamagi-devbox && test -x /usr/libexec/tamagi/dev-tool && grep -q "dev-tool nvim" /usr/share/cosmic/com.system76.CosmicSettings.Shortcuts/v1/defaults && grep -q "dev-tool lazygit" /usr/share/cosmic/com.system76.CosmicSettings.Shortcuts/v1/defaults'
check "lista de Flatpaks não está vazia"  grep -qE '^[a-z]' /usr/share/tamagi/flatpaks.list
check "atualização: não baixa na bateria nem em rede limitada" bash -c 'grep -qx ON_BATTERY=skip /usr/lib/tamagi/update.conf && grep -qx ON_METERED=skip /usr/lib/tamagi/update.conf && bash -n /usr/libexec/tamagi/system-update && command -v skopeo'
check "distrobox-create não gera volume :ro:rslave (podman 6)" bash -c '! grep -qE ":ro\\$\\{rslave\\}|:ro:rslave" /usr/bin/distrobox-create && grep -q ":ro,rslave" /usr/bin/distrobox-create'
check "botões do gum com cores legíveis"  bash -c '. /usr/share/tamagi/gum.env && [[ $GUM_CONFIRM_SELECTED_FOREGROUND == "#11111b" ]] && grep -q gum.env /usr/libexec/tamagi/popup /etc/profile.d/tamagi.sh'
check "TPM: serviço que amarra a chave ao PCR 7 ligado" bash -c 'systemctl is-enabled tamagi-tpm-seal.service | grep -qx enabled && test -x /usr/libexec/tamagi/tpm-seal'
check "TPM: PCR 7 só com Secure Boot ligado; PIN sem PCR; tamagi setup tpm-unlock pin|auto|off|reseal nativo" bash -c 'grep -q "SecureBoot-8be4df61" /usr/libexec/tamagi/tpm-seal && grep -q -- "--tpm2-with-pin=yes" /usr/libexec/tamagi/tpm-unlock && ! grep -q luks-enable-tpm2-autounlock /usr/bin/tamagi-setup && tamagi-setup tpm-unlock 2>&1 | grep -q "pin|auto|off|reseal" && bash -n /usr/libexec/tamagi/tpm-unlock'
check "menu de boot: editor desligado (instalação nova pelo bootc e anteriores pelo serviço)" bash -c 'grep -qx "editor no" /usr/lib/bootc/install/systemd-boot/loader.conf && systemctl is-enabled tamagi-loader-conf.service | grep -qx enabled && d=$(mktemp -d) && mkdir -p $d/loader && printf "timeout 3\neditor yes\n" > $d/loader/loader.conf && TAMAGI_ESP=$d /usr/libexec/tamagi/loader-conf >/dev/null && grep -qx "editor no" $d/loader/loader.conf && grep -qx "timeout 3" $d/loader/loader.conf && [[ $(grep -c editor $d/loader/loader.conf) -eq 1 ]]'
check "Plymouth desenha no simpledrm (pedido de senha/PIN visível sem esperar o driver de vídeo)" grep -q "plymouth.use-simpledrm=1" /usr/lib/bootc/kargs.d/20-tamagi-boot.toml
check "modelos de IA fora dos snapshots, sem compressão; VMs do usuário sem CoW" bash -c 'grep -qx "    .local/share/tamagi/models" /usr/libexec/tamagi/home-snapshots && grep -q "compression none" /usr/libexec/tamagi/home-snapshots && grep -q "chattr +C" /usr/libexec/tamagi/home-snapshots'
check "idiomas: toda entrada de menu com [pt_BR] tem [pt] (o lançador do COSMIC lê [pt])" bash -c 'for f in /usr/share/applications/tamagi-*.desktop /usr/share/applications/org.tamagi.Menu.desktop /usr/share/tamagi/desktop/claude-code.desktop /usr/share/tamagi/desktop/agy.desktop; do [[ $(grep -c "\[pt_BR\]=" "$f") -eq $(grep -c "\[pt\]=" "$f") ]] || { echo "$f"; exit 1; }; done'
check "menu sem entradas do que não está instalado (Claude Code e agy entram ao instalar)" bash -c '[[ ! -e /usr/share/applications/claude-code.desktop && ! -e /usr/share/applications/agy.desktop ]] && desktop-file-validate /usr/share/tamagi/desktop/claude-code.desktop /usr/share/tamagi/desktop/agy.desktop && grep -q "menu_entry claude-code" /usr/libexec/tamagi/agent-manager'
check "idiomas: locales pt_BR, en_US, es_ES e fr_FR" bash -c 'for l in pt_BR en_US es_ES fr_FR; do locale -a | grep -qix "$l.utf8" || exit 1; done'
check "idiomas: todas as frases das ferramentas traduzidas (en/es/fr)" python3 /tests/i18n-strings.py / --check
check "idiomas: a camada de tradução responde em es e fr" bash -c 'unset TAMAGI_LANG; [[ $(LANG=es_ES.UTF-8 bash -c ". /usr/libexec/tamagi/i18n; t Quit") == Salir && $(LANG=fr_FR.UTF-8 bash -c ". /usr/libexec/tamagi/i18n; t Quit") == Quitter && $(LANG=de_DE.UTF-8 bash -c ". /usr/libexec/tamagi/i18n; t Quit") == Quit && $(LANG=pt_BR.UTF-8 bash -c ". /usr/libexec/tamagi/i18n; t Quit") == Sair ]]'
check "idiomas: atalhos em inglês"         bash -c 'TAMAGI_LANG=en tamagi-keys | grep -q "close window"'
check "idiomas: tamagi-lang lista os quatro" bash -c '[[ $(tamagi-lang list | wc -l) -eq 4 ]]'
check "idiomas: entradas do menu traduzidas" bash -c 'for f in /usr/share/applications/tamagi-agent.desktop /usr/share/applications/org.tamagi.Menu.desktop /usr/share/applications/tamagi-browser.desktop /usr/share/tamagi/desktop/claude-code.desktop /usr/share/tamagi/desktop/agy.desktop; do grep -q "\[fr\]=" $f && grep -q "\[es\]=" $f || exit 1; done'
check "fontes: Roboto Flex + Inter + Noto Sans/Serif (hintslight) e ícones Papirus-Dark" bash -c '[[ $(fc-match -f "%{family}" sans-serif) == "Roboto Flex" ]] && grep -q "Roboto Flex" /usr/share/cosmic/com.system76.CosmicTk/v1/interface_font && grep -q "Papirus-Dark" /usr/share/cosmic/com.system76.CosmicTk/v1/icon_theme && fc-match -s sans-serif | head -10 | grep -q "Noto Sans" && grep -q "font-name=.Roboto Flex 11" /etc/dconf/db/local.d/10-tamagi-fonts'
check "Roboto Flex: um arquivo só (variável), com a licença OFL" bash -c '[[ $(fc-list "Roboto Flex" file | sort -u | wc -l) -eq 1 ]] && grep -q "SIL OPEN FONT LICENSE" /usr/share/licenses/roboto-flex/OFL.txt'
check "barra: uma só, embaixo, apps no centro" bash -c 'P=/usr/share/cosmic/com.system76.CosmicPanel.Panel/v1; grep -q "^\[\"Panel\"\]$" /usr/share/cosmic/com.system76.CosmicPanel/v1/entries && grep -qx Bottom $P/anchor && grep -q CosmicAppList $P/plugins_center && grep -q CosmicAppletTime $P/plugins_wings'
if [[ $EDITION == skl ]]; then
    check "SKL: barra minimalista sem workspaces/tiling/bluetooth (só iniciar + apps escolares + relógio/rede/áudio/energia)" bash -c 'P=/usr/share/cosmic/com.system76.CosmicPanel.Panel/v1; grep -q CosmicAppList $P/plugins_center && grep -q CosmicAppletTime $P/plugins_wings && ! grep -qE "CosmicPanelWorkspacesButton|CosmicAppletTiling|CosmicAppletBluetooth" $P/plugins_wings'
else
    check "menu iniciar: applet instalado, primeiro botão da barra, layout Retro" bash -c 'P=/usr/share/cosmic/com.system76.CosmicPanel.Panel/v1; M=/usr/share/cosmic/com.championpeak87.cosmic-ext-classic-menu/v1; command -v cosmic-ext-classic-menu-applet && grep -q "^Some((\[\"com.championpeak87.cosmic-ext-classic-menu\"" $P/plugins_wings && ! grep -q CosmicPanelAppButton $P/plugins_wings && grep -qx Retro $M/menu_layout && grep -qx Places $M/side_panel'
fi
check "menu iniciar: ícone próprio do Tamagi, sem texto" bash -c 'M=/usr/share/cosmic/com.championpeak87.cosmic-ext-classic-menu/v1; f=$(tr -d "\"" < $M/button_icon); [[ $f == /usr/share/tamagi/menu/tamagi-start-symbolic.svg && -s $f ]] && grep -qx IconOnly $M/applet_button_style && rsvg-convert "$f" -o /tmp/menu-icon.png && [[ -s /tmp/menu-icon.png ]]'
check "Arquivos: grade, pastas primeiro" bash -c 'f=/usr/share/cosmic/com.system76.CosmicFiles/v1/tab; grep -q "view: Grid" $f && grep -q "folders_first: true" $f'
check "menu iniciar 1.1.0: menu focado (já abre com o teclado)" bash -c 'grep -qx true /usr/share/cosmic/com.championpeak87.cosmic-ext-classic-menu/v1/focused_menu && grep -aq focused_menu /usr/bin/cosmic-ext-classic-menu-applet && [[ -x /usr/bin/cosmic-ext-classic-menu-settings ]]'
check "terminal padrão: Ptyxis (Super+Enter e xdg-terminal-exec, fallback cosmic-term; sem foot)" bash -c 'grep -q "Terminal: \"/usr/libexec/tamagi/term\"" /usr/share/cosmic/com.system76.CosmicSettings.Shortcuts/v1/system_actions && grep -q "exec cosmic-term" /usr/libexec/tamagi/term && command -v ptyxis && grep -q "exec ptyxis --new-window" /usr/libexec/tamagi/term && [[ $(head -1 /etc/xdg/xdg-terminals.list) == org.gnome.Ptyxis.desktop ]] && ! grep -q "foot.desktop" /etc/xdg/xdg-terminals.list && grep -q com.system76.CosmicTerm.desktop /etc/xdg/xdg-terminals.list'
check "sandbox (Landlock): leitura não lê segredo, não grava, não conecta" bash -c 'S=/usr/libexec/tamagi/sandbox; mkdir -p $HOME/.ssh $HOME/Documents; echo CHAVE > $HOME/.ssh/id_test; echo ok > $HOME/Documents/a; $S -- cat $HOME/Documents/a | grep -qx ok && ! $S -- cat $HOME/.ssh/id_test && ! $S -- touch $HOME/Documents/b && $S -- python3 -c "
import socket, sys
try: socket.create_connection((\"127.0.0.1\", 9), 2)
except PermissionError: sys.exit(0)
except OSError: sys.exit(1)
sys.exit(1)"'
check "sandbox (Landlock): usuario grava em subpasta mas não mexe em segredo" bash -c 'S=/usr/libexec/tamagi/sandbox; mkdir -p $HOME/.ssh $HOME/Documents; echo CHAVE > $HOME/.ssh/id_test; $S --mode usuario -- sh -c "echo x > $HOME/Documents/c" && $S --mode usuario -- cat $HOME/.ssh/id_test | grep -qx CHAVE && ! $S --mode usuario -- rm -f $HOME/.ssh/id_test && ! $S --mode usuario -- mv $HOME/.ssh/id_test $HOME/Documents/x && [[ -f $HOME/.ssh/id_test ]]'
check "agente: comandos de leitura passam pelo sandbox" bash -c 'grep -q "sandbox = \"leitura\"" /usr/libexec/tamagi/agent-local && grep -q /usr/libexec/tamagi/sandbox /usr/libexec/tamagi/agent-local'
check "snapshots da pasta pessoal: snapper, serviço e timers" bash -c 'systemctl is-enabled snapper-timeline.timer && systemctl is-enabled snapper-cleanup.timer && systemctl is-enabled tamagi-home-snapshots.service && bash -n /usr/libexec/tamagi/home-snapshots && grep -q -- --btrfs-subvolume-home /usr/libexec/tamagi/firstboot'
check "tamagi-snap: estado sem snapshots e recusa caminho fora da pasta pessoal" bash -c 'tamagi-snap | grep -qiE "desligad|off" && ! tamagi-snap versions /etc/passwd 2>/dev/null'
check "Windows: Bottles oferecido no primeiro login e .exe/.msi abrem nele" bash -c 'grep -qE "^flathub +com.usebottles.bottles" /usr/share/tamagi/flatpaks.list && grep -q "^application/x-ms-dos-executable=com.usebottles.bottles.desktop" /etc/xdg/mimeapps.list && grep -q "^application/x-msi=com.usebottles.bottles.desktop" /etc/xdg/mimeapps.list'
check "Windows: tamagi-windows fixa a imagem do dockur e não instala nada sozinho" bash -c 'grep -q "^IMAGE=\"docker.io/dockurr/windows:[0-9]" /usr/bin/tamagi-windows && tamagi-windows status | grep -qiE "não instalado|not installed" && [[ ! -e $HOME/.config/containers/systemd/tamagi-windows.container ]]'
check "tamagi-db: lista vazia e recusa banco desconhecido" bash -c 'tamagi-db list | grep -qiE "nenhum|no database" && ! tamagi-db add oracle 2>/dev/null'
check "tamagi-mcp: Quadlets .image, runners isolados (--cap-drop=ALL) e sync para Claude/Gemini/OpenCode/Codex" bash -c '
    H=/tmp/mcp-test; rm -rf "$H"; mkdir -p "$H"
    export HOME="$H" XDG_CONFIG_HOME="$H/.config" XDG_DATA_HOME="$H/.local/share" TAMAGI_MCP_OFFLINE=1
    tamagi-mcp list | grep -q "memory"
    tamagi-mcp add --no-pull memory git >/dev/null
    test -f "$H/.config/containers/systemd/tamagi-mcp-memory.image"
    grep -q "^AutoUpdate=registry" "$H/.config/containers/systemd/tamagi-mcp-memory.image"
    grep -q -- "--cap-drop=ALL" "$H/.local/share/tamagi/mcp/bin/tamagi-mcp-memory"
    jq -e ".mcpServers[\"tamagi-memory\"].command" "$H/.gemini/settings.json" >/dev/null
    jq -e ".mcpServers[\"tamagi-git\"].command" "$H/.claude.json" >/dev/null
    jq -e ".mcp[\"tamagi-memory\"].enabled == true" "$H/.config/opencode/opencode.json" >/dev/null
    grep -q "\[mcp_servers.tamagi-memory\]" "$H/.codex/config.toml"
    [[ $(tamagi-mcp installed | tr "\n" " ") == "memory git " ]]
    tamagi-mcp status --json | jq -e "(.servers | map(select(.enabled)) | length) == 2" >/dev/null
    tamagi-mcp remove memory git >/dev/null
    [[ -z $(tamagi-mcp installed) ]]
    ! jq -e ".mcpServers" "$H/.gemini/settings.json" >/dev/null
    rm -rf "$H"
'
check "tamagi-capture: ajuda, menu sem tela e aviso limpo para OCR sob demanda" bash -c 'tamagi-capture --help | grep -q OCR && tamagi-capture </dev/null | grep -q "tamagi-capture text " && TAMAGI_CAPTURE_PRINT=1 tamagi-capture text 2>&1 | grep -q "tamagi install ocr"'
check "atalhos enxutos: Shift/Alt+Print, Super+Shift+G/D/O/M, Super+Ctrl+P (sem emoji/remind/windows)" bash -c 'D=/usr/share/cosmic/com.system76.CosmicSettings.Shortcuts/v1/defaults; grep -q "(modifiers: \[Shift\], key: \"Print\"): Spawn(\"tamagi-capture text\")" $D && grep -q "(modifiers: \[Alt\], key: \"Print\"): Spawn(\"tamagi-capture record\")" $D && for k in g d o m; do grep -q "(modifiers: \[Super, Shift\], key: \"$k\"): Spawn" $D || exit 1; done && ! grep -q "tamagi-remind\|/emoji\|tamagi-windows" $D && grep -q "open-or-install org.onlyoffice.desktopeditors" $D && [[ $(grep -c "key: \"Print\")" $D) -ge 3 ]]'
check "tamagi-theme: cores do Neovim (LazyVim) em todos os temas" bash -c 'for th in $(tamagi-theme list); do tamagi-theme nvim $th > /tmp/n.lua && grep -q "colorscheme" /tmp/n.lua || exit 1; done; tamagi-theme nvim ipe | grep -q base16 && mkdir -p $HOME/.config/nvim/lua/plugins && tamagi-theme set gruvbox >/dev/null && grep -q "colorscheme = \"gruvbox\"" $HOME/.config/nvim/lua/plugins/tamagi-theme.lua; r=$?; rm -rf $HOME/.config/nvim; tamagi-theme set ipe >/dev/null; exit $r'
check "linguagens: presets do mise na caixa (tamagi dev box lang)" bash -c 'o=$(tamagi-devbox lang </dev/null); grep -q "^  ruby " <<<"$o" && grep -q "^  dotnet " <<<"$o" && ! tamagi-devbox lang "x;rm" 2>/dev/null'
check "tamagi-font: troca a fonte do COSMIC" bash -c 'tamagi-font list | grep -qx "JetBrains Mono" && alt=$(tamagi-font list | grep -vx "JetBrains Mono" | head -1) && [[ -n $alt ]] && GSETTINGS_BACKEND=memory tamagi-font set "$alt" >/dev/null && grep -q "$alt" $HOME/.config/cosmic/com.system76.CosmicTk/v1/monospace_font && GSETTINGS_BACKEND=memory tamagi-font reset >/dev/null && grep -q "JetBrains Mono" $HOME/.config/cosmic/com.system76.CosmicTk/v1/monospace_font'
check "VMs (libvirt e Windows) ficam fora dos snapshots da pasta pessoal" bash -c 'grep -q ".local/share/libvirt/images" /usr/libexec/tamagi/home-snapshots && grep -q ".local/share/tamagi-windows" /usr/libexec/tamagi/home-snapshots && grep -q ".local/share/flatpak" /usr/libexec/tamagi/home-snapshots && grep -q ".local/share/Steam" /usr/libexec/tamagi/home-snapshots && grep -q "prune_space" /usr/libexec/tamagi/home-snapshots && grep -q "flock" /usr/libexec/tamagi/system-update && grep -q "pre_update_snapshot" /usr/bin/tamagi-update'
check "tamagi-update, snapper preservado e autorebuild de layer estão corretos" bash -c '
    for cmd in check drop rollback history "--soft-reboot"; do
        grep -q -- "$cmd" /usr/bin/tamagi-update || exit 1
    done
    grep -q "delete-deployment" /usr/libexec/tamagi/system-update
    grep -q "prep-soft-reboot" /usr/libexec/tamagi/system-update
    grep -q "composefs-gc" /usr/libexec/tamagi/system-update
    grep -q ".snapshots.tamagi-preserve" /usr/libexec/tamagi/home-snapshots
    grep -q -- "--no-create-home" /usr/libexec/tamagi/firstboot
    bash -n /usr/libexec/tamagi/layer-autorebuild
    systemctl cat tamagi-layer-rebuild.service >/dev/null 2>&1
'
check "Plymouth: tema tamagi padrão, com o brasão animado e a senha do disco" bash -c 'T=/usr/share/plymouth/themes/tamagi; [[ $(plymouth-set-default-theme) == tamagi ]] && [[ $(ls $T/throbber-*.png | wc -l) -ge 30 ]] && for f in watermark entry bullet lock; do [[ -s $T/$f.png ]] || exit 1; done && grep -q "^ModuleName=two-step" $T/tamagi.plymouth'
check "Plymouth: tema dentro do initramfs e boot gráfico (rhgb quiet)" bash -c 'img=$(ls /usr/lib/modules/*/initramfs.img); lsinitrd "$img" | grep -q "plymouth/themes/tamagi/throbber-0001.png" && grep -q "\"rhgb\"" /usr/lib/bootc/kargs.d/20-tamagi-boot.toml'
check "marcas: sem logos do Fedora; os-release com o logo e os endereços do Tamagi" bash -c '! rpm -q fedora-logos >/dev/null && rpm -q generic-logos >/dev/null && grep -qx "LOGO=tamagi" /usr/lib/os-release && ! grep -E "^(HOME|DOCUMENTATION|SUPPORT)_URL=" /usr/lib/os-release | grep -qi fedoraproject && grep -qx "ID=fedora" /usr/lib/os-release && test -s /usr/share/icons/hicolor/scalable/apps/tamagi.svg && test -s /usr/share/doc/tamagi/NOTICE'
check "Antigravity: o catálogo instala a CLI (agy) pelo instalador do Google; a IDE fica fora do catálogo" bash -c 'grep -qP "^antigravity\t.*\tagent:antigravity\tAntigravity CLI" /usr/share/tamagi/catalog.tsv && grep -q "antigravity.google/cli/install.sh" /usr/libexec/tamagi/agent-manager && grep -q "install_agy && menu_entry agy" /usr/libexec/tamagi/agent-manager && ! grep -q "tamagi-antigravity install" /usr/libexec/tamagi/agent-manager'
check "Antigravity IDE: sob demanda na caixa de navegadores" bash -c '! rpm -q antigravity >/dev/null 2>&1 && [[ ! -e /etc/yum.repos.d/antigravity.repo ]] && grep -q "app antigravity" /usr/bin/tamagi-antigravity && grep -q "antigravity-rpm" /usr/libexec/tamagi/browser-provision && desktop-file-validate /usr/share/tamagi/desktop/agy.desktop && grep -q "install chrome --box" /usr/bin/tamagi-antigravity && grep -q "\.local/bin" /usr/lib/environment.d/50-tamagi-path.conf'
check "primeiro boot: só Flatpaks essenciais (--user); OnlyOffice e Loupe pré-instalados" bash -c 'L=/usr/share/tamagi/flatpaks.list; grep -qE "^flathub +org.onlyoffice.desktopeditors +essencial +Office" $L && grep -qE "^flathub +org.gnome.Loupe +essencial" $L && ! grep -qE "^(flathub|cosmic) +[^ ]+ +recomendado" $L && grep -E "^(flathub|cosmic) " $L | awk "NF < 6 {bad=1} END {exit bad}" && grep -q "kind:-essencial" /usr/libexec/tamagi/flatpak-setup && grep -q -- "--user" /usr/libexec/tamagi/flatpak-setup && ! grep -q -- "--system" /usr/libexec/tamagi/flatpak-setup && [[ ! -e /usr/libexec/tamagi/apps-setup ]]'
check "catálogo único de opcionais (tamagi pkg): itens essenciais, Bazzite, drivers/ROCm, Lark, Sniffnet e Flathub" bash -c 'C=/usr/share/tamagi/catalog.tsv; for id in opera-gx whatsapp lark firefox chrome vivaldi virt-manager windows steam lutris wine devbox-fedora devbox-ubuntu devbox-opensuse devbox-arch claude antigravity codex clio qwen opencode openclaw local-ai rocm video-amd video-intel video-nvidia codecs-extra lact openrgb solaar disk-tools sniffnet wireshark vpn-extra net-fs hardening-extra; do grep -qP "^$id\t" $C || { echo "falta $id"; exit 1; }; done; ! grep -vE "^#" $C | awk -F"\t" "NF != 5 || \$4 !~ /^(flatpak|gaming|agent|run):/ {bad=1} END {exit !bad}" && o=$(TAMAGI_LANG=en tamagi-install list) && grep -q "^  steam .*\[Games\]" <<<"$o" && grep -q "^  rocm .*\[Drivers\]" <<<"$o" && grep -q "^  okular " <<<"$o" && ! TAMAGI_INSTALL_NO_FLATHUB=1 tamagi-install inexistente 2>/dev/null && grep -q "flathub.org/api/v2/search" /usr/bin/tamagi-install'
check "primeiro login com apps COSMIC/criativos escolhidos pelo usuário: toda linha de basics.list está no catálogo e não é navegador" bash -c 'B=/usr/share/tamagi/basics.list; n=0; while read -r state id _; do [[ $state == on || $state == off ]] || continue; n=$((n+1)); grep -qP "^$id\t(?!Browsers)" /usr/share/tamagi/catalog.tsv || grep -qE "^(flathub|cosmic) +$id +opcional " /usr/share/tamagi/flatpaks.list || { echo "fora do catálogo: $id"; exit 1; }; done < $B; (( n >= 15 )) && grep -q "^on " $B && for app in org.kde.okular org.videolan.VLC org.inkscape.Inkscape org.gimp.GIMP dev.mariinkys.Cedilla com.francescogaglione.chronos com.github.flxzt.rnote app.drey.KeyRack org.gnome.Snapshot app.drey.Warp com.system76.Popsicle io.github.seadve.Kooha com.belmoussaoui.Authenticator sniffnet; do grep -qE "^(on|off) +$app" $B || { echo "falta app: $app"; exit 1; }; done'
check "primeiro login enxuto: navegador, o básico e pronto (tema, barra e tiling ficam no COSMIC)" bash -c 'F=/usr/bin/tamagi-firstrun; ! grep -q "Which profile?" $F && ! grep -q "tamagi apply" $F && grep -q "opera-gx  Opera GX" $F && grep -q "basics.list" $F && grep -q "Which browser?" $F && grep -q "tamagi-install --actions" $F && ! grep -qE "tamagi-theme (set|pick)|tamagi-panel|mise use|tamagi-ai setup" $F'
check "agentes de IA sob demanda: catálogo, digest conferido e npm na caixa" bash -c 'o=$(tamagi-agent list); for a in claude antigravity codex clio qwen opencode gemini openclaw local; do grep -q "^  $a " <<<"$o" || exit 1; done; grep -q "SyntheticAutonomicMind/CLIO" /usr/libexec/tamagi/agent-manager && grep -qw perl /usr/libexec/tamagi/devbox-provision && grep -q "sha256sum -c" /usr/libexec/tamagi/agent-manager && grep -q "npm install -g --prefix" /usr/libexec/tamagi/agent-manager && ! tamagi-agent install inexistente 2>/dev/null'
check "gamescope com HDR: sessão de jogos na tela de login, opções e detecção pelo EDID" bash -c 'F=/usr/share/tamagi/layer-overlays/gaming/usr/share/wayland-sessions/tamagi-gaming.desktop; grep -qx "Exec=/usr/libexec/tamagi/gamescope-session" $F && [[ ! -e /usr/share/wayland-sessions/tamagi-gaming.desktop ]] && grep -q -- "--overlay gaming" /usr/bin/tamagi-gaming && export TAMAGI_ASSUME_HDR=1 && export XDG_CONFIG_HOME=/tmp/gs && rm -rf /tmp/gs && tamagi-gaming hdr | grep -q "^HDR  *: on" && tamagi-gaming hdr nits 300 >/dev/null && grep -qx "SDR_NITS=300" /tmp/gs/tamagi/gamescope.conf && /usr/libexec/tamagi/gamescope-session --print-args 1 | grep -qx -- "--hdr-enabled" && tamagi-gaming hdr off >/dev/null && ! /usr/libexec/tamagi/gamescope-session --print-args 1 | grep -q hdr && ! tamagi-gaming hdr nits abc 2>/dev/null && python3 -I /usr/libexec/tamagi/hdr-displays; [[ $? -le 1 ]]'
check "inglês é o idioma de origem: sem idioma definido sai inglês; pt, es e fr são catálogos" bash -c '[[ ! -e /usr/share/tamagi/i18n/en.tsv ]] && for l in pt es fr; do test -s /usr/share/tamagi/i18n/$l.tsv && test -s /usr/share/tamagi/keys.$l.txt || exit 1; done; env -u TAMAGI_LANG -u LANG -u LC_ALL -u LC_MESSAGES tamagi | grep -q "one command for the whole system" && TAMAGI_LANG=pt tamagi | grep -q "um comando para o sistema inteiro" && TAMAGI_LANG=de tamagi-layer --help | grep -q "Fedora packages" && TAMAGI_LANG=fr tamagi-install list | grep -q "\[Jeux\]" && ! grep -lE "\bt[l]? \"[^\"]*[áéíóúâêôãõç]" /usr/bin/tamagi-* | grep -q .'
check "perfis prontos: ai, dev, gamer e office são TOML válido e entram no plano" bash -c 'for f in ai dev gamer office; do python3 -c "import tomllib,sys; tomllib.load(open(sys.argv[1],\"rb\"))" /usr/share/tamagi/profiles/$f.toml || exit 1; done; tamagi sys profiles | grep -q "tamagi sys apply ai" && o=$(tamagi sys diff ai) && grep -q "agente: codex" <<<"$o" && grep -q "caixa dev (fedora)" <<<"$o" && tamagi sys diff gamer | grep -q "catálogo: steam gaming-tools   \[pede senha\]" && ! tamagi sys diff http://exemplo.invalid/x.toml 2>/dev/null && ! tamagi sys diff inexistente 2>/dev/null'
check "arquivo antigo com [packages] é lido, com aviso, e o resto vale" bash -c 'printf "[packages]\ndeb = [\"x\"]\n[appearance]\ntheme = \"nord\"\n" > /tmp/velho.toml; o=$(tamagi sys diff /tmp/velho.toml 2>&1) && grep -q "não existe mais" <<<"$o" && grep -q "tema: nord" <<<"$o"'
check "agente preso ao projeto: só a pasta do projeto, sem chaves, sistema somente-leitura, ponto de retorno no git" bash -c 'H=/tmp/jaula; rm -rf $H; mkdir -p $H/.ssh $H/proj $H/outro && echo SEGREDO > $H/.ssh/id && cd $H/proj && git init -q . && git -c user.email=t@t -c user.name=t commit -q --allow-empty -m i && echo v1 > a && git add a && export HOME=$H && o=$(TAMAGI_LIBEXEC=/usr/libexec/tamagi tamagi-agent jail --host -- sh -c "cat ~/.ssh/id 2>/dev/null; ls ~/outro 2>/dev/null; cat a; echo novo > b; touch /usr/zz 2>/dev/null || echo ro" 2>&1) || { grep -qiE "namespace|permission|operation not permitted" <<<"$o" && git for-each-ref refs/tamagi/before-agent | grep -q . && exit 0; exit 1; }; grep -qx v1 <<<"$o" && grep -qx ro <<<"$o" && ! grep -q SEGREDO <<<"$o" && test -f $H/proj/b && git for-each-ref refs/tamagi/before-agent | grep -q . && cd $H && ! tamagi-agent jail --host -- true 2>/dev/null && ! tamagi-agent run inexistente 2>/dev/null && grep -q "bubblewrap" /usr/libexec/tamagi/devbox-provision'
if [[ $EDITION != skl ]]; then
    check "llm-scripts: opcional, usa o llama.cpp da imagem (sem compilar)" bash -c 'grep -qP "^llm-scripts\t" /usr/share/tamagi/catalog.tsv && bash -n /usr/libexec/tamagi/llm-scripts-setup && grep -q "llama-vulkan/bin" /usr/libexec/tamagi/llm-scripts-setup && grep -q "LLM_BUILD_DIR=" /usr/libexec/tamagi/llm-scripts-setup && test -x /usr/bin/llama-server'
fi
check "jaula montada onde roda: ajudante em Rust (estático), esconde as pastas pessoais, protege os hooks do git, recusa estado fora da home" bash -c 'J=/usr/libexec/tamagi/agent-jail; file -L $J | grep -Eq "static(ally|-pie) linked" && grep -q "JAIL_BOX=.*run/host/usr/libexec/tamagi" /usr/libexec/tamagi/agent-manager && mkdir -p /tmp/jp/.git/hooks && o=$(HOME=/tmp/jh $J --print --root /tmp/jp --cwd /tmp/jp -- true) && grep -qx -- "--unshare-pid" <<<"$o" && grep -qx "/tmp/jp/.git/hooks" <<<"$o" && grep -qx "/tmp/jh" <<<"$o" && ! HOME=/tmp/jp $J --print --root /tmp/jp -- true 2>/dev/null && ! HOME=/tmp/jh $J --print --root /tmp/jp --state ../fora -- true 2>/dev/null && ! $J --print --root /tmp/jp --nope -- true 2>/dev/null'
if [[ $EDITION != skl ]]; then
    check "Cockpit só em localhost, sem desligar a conferência de certificado" bash -c 'D=/usr/lib/systemd/system/cockpit.socket.d/10-tamagi.conf; grep -qx "ListenStream=" $D && grep -qx "ListenStream=127.0.0.1:9090" $D && [[ $(grep -c "^ListenStream=" $D) == 3 ]] && ! grep -q "ignore-certificate-errors" /usr/bin/tamagi-vm && grep -q "http://localhost:9090/machines" /usr/bin/tamagi-vm'
fi
check "instalador próprio na imagem (tamagi-welcome --install) e bootc 1.17.5 ou mais novo" bash -c 'tamagi-welcome --help | grep -q -- "--install" && v=$(rpm -q --qf "%{VERSION}" bootc) && [[ $(printf "%s\n" 1.17.5 "$v" | sort -V | head -1) == 1.17.5 ]] && ! command -v tik >/dev/null && ! rpm -q zenity >/dev/null 2>&1'
check "tamagi sys status: o que roda em segundo plano, em texto e JSON" bash -c 'F=/tmp/fakectl; printf "%s\n" "#!/bin/bash" "case \"\$*\" in *is-active*fstrim*) echo activating;; *is-active*snapper-timeline*) echo active;; *is-active*) echo inactive;; *snapper*) echo exited;; *) echo running;; esac" > $F; chmod +x $F; o=$(TAMAGI_LANG=en tamagi sys status) && grep -q "Nothing is running in the background" <<<"$o" && o=$(TAMAGI_LANG=pt TAMAGI_STATUS_SYSTEMCTL=$F tamagi sys status) && grep -q "Fazendo o TRIM do SSD" <<<"$o" && ! grep -q "snapshot" <<<"$o" && [[ $(TAMAGI_STATUS_SYSTEMCTL=$F tamagi sys status --json | jq -r ".running | join(\",\")") == fstrim.service ]] && while IFS=$(printf "\t") read -r _ _ what; do grep -qP "^\Q$what\E\t" /usr/share/tamagi/i18n/pt.tsv || { echo "sem tradução: $what"; exit 1; }; done < <(sed -n "/^JOBS=/,/^\$/p" /usr/bin/tamagi-status | grep -P "\t")'
check "egress-guard: permite APIs de IA, registries e llama.cpp local; bloqueia domínios desconhecidos e IPs crus" bash -c '
    G=/usr/libexec/tamagi/egress-guard
    test -x "$G"
    "$G" --check-host api.anthropic.com:443 | grep -qx ALLOW
    "$G" --check-host crates.io:443 | grep -qx ALLOW
    "$G" --check-host 127.0.0.1:9931 | grep -qx ALLOW
    ! "$G" --check-host evil-exfil.example.com:443 >/dev/null
    ! "$G" --check-host 1.1.1.1:443 >/dev/null
    "$G" --check-host custom.internal:443 --allow custom.internal | grep -qx ALLOW
'
check "tamagi sys shield lease: prazo, lista fechada, vencimento e devolução (simulado)" bash -c 'export TAMAGI_LEASE_DIR=/tmp/leases TAMAGI_LEASE_DRYRUN=1 TAMAGI_LANG=en; rm -rf $TAMAGI_LEASE_DIR; H=/usr/libexec/tamagi/lease-helper; systemctl is-enabled tamagi-lease-expire.service >/dev/null && tamagi sys shield lease | grep -q "Nothing is on lease" && tamagi sys shield lease vm 30 | grep -q "On lease until" && tamagi sys shield lease port 8080 >/dev/null && [[ $($H list | wc -l) == 2 ]] && ! $H grant shell 5 2>/dev/null && ! $H grant vm 99999 2>/dev/null && ! $H grant port 5 70000 2>/dev/null && TAMAGI_LEASE_NOW=$(( $(date +%s) + 2400 )) $H expire | grep -qx "expired: vm" && [[ $($H list | cut -d" " -f1) == port-8080 ]] && tamagi sys shield lease end port 8080 | grep -q "remove-port=8080/tcp" && [[ -z $($H list) ]] && ! grep -q "security_driver" $H'
if [[ $EDITION != skl ]]; then
    check "tamagi vm: acesso total opcional (pen-drives e discos externos) e console Cockpit Machines" bash -c 'tamagi-vm --help | grep -q "tamagi vm access" && ! grep -q "security_driver" /usr/bin/tamagi-vm && [[ $(grep -c "label=disable\"" /usr/libexec/tamagi/gpu-args) == 1 ]] && grep -q "nvidia/version ]]; then echo \"--security-opt=label=disable" /usr/libexec/tamagi/gpu-args && grep -q ":/work:z" /usr/bin/tamagi-try && test -f /etc/libvirt/qemu/networks/autostart/default.xml && test -f /usr/share/applications/tamagi-vm.desktop && grep -q "zram-size = min(ram, 32768)" /usr/lib/systemd/zram-generator.conf.d/10-tamagi.conf && jq -e ".HighEfficiencyModeEnabled == true" /etc/chromium/policies/managed/tamagi-chromeos.json >/dev/null && [[ ! -e /usr/bin/tamagi-config && ! -e /etc/tamagi/system.toml ]] && ! tamagi-vm access inexistente 2>/dev/null'
fi
if [[ $EDITION != skl ]]; then
    check "GPU nos contêineres e VS Code com Dev Containers opcional" bash -c '/usr/libexec/tamagi/gpu-args | grep -q -- "--group-add=keep-groups" && grep -q "mesa-vulkan-drivers" /usr/libexec/tamagi/devbox-provision && tamagi-devbox --help | grep -q "tamagi dev box gpu" && grep -qP "^vscode\t" /usr/share/tamagi/catalog.tsv && grep -q "packages.microsoft.com/yumrepos/vscode" /usr/libexec/tamagi/vscode-setup && grep -q "\"dev.containers.dockerPath\": \"podman\"" /usr/libexec/tamagi/vscode-setup && command -v podman-compose && ! command -v brew'
fi
check "uma Noto Sans só (a variável): duas cópias da família faziam o texto sumir" bash -c 'rpm -q google-noto-sans-vf-fonts >/dev/null && ! rpm -q google-noto-sans-fonts >/dev/null 2>&1 && fc-match -f "%{file}" "Noto Sans" | grep -qi "vf\|variable\|\[wght\]"'
check "btrfs com a manutenção do MicroOS/Aeon: scrub e balance em timers" bash -c 'rpm -q btrfsmaintenance >/dev/null && systemctl is-enabled btrfs-scrub.timer btrfs-balance.timer | grep -c enabled | grep -qx 2'
check "TRIM: fstrim mensal, fora da bateria (o btrfs faz o descarte contínuo)" bash -c 'systemctl is-enabled fstrim.timer >/dev/null && grep -qx "OnCalendar=monthly" /usr/lib/systemd/system/fstrim.timer.d/10-tamagi.conf && grep -q "ConditionACPower=true" /usr/lib/systemd/system/fstrim.service.d/10-tamagi.conf && [[ $(grep -v "^#" /etc/sysconfig/btrfsmaintenance | grep -c "BTRFS_TRIM_PERIOD=\"none\"") -eq 1 ]]' 
check "lista de atalhos cabe na janela (paginada quando é maior)" bash -c 'grep -q "gum pager" /usr/bin/tamagi-keys && TAMAGI_LANG=en tamagi-keys | grep -q "start menu"'
check "Ptyxis recebe a paleta do tema (tamagi.palette)" bash -c 'export XDG_DATA_HOME=/tmp/pty XDG_CONFIG_HOME=/tmp/ptyc; tamagi-theme set nord >/dev/null 2>&1; f=/tmp/pty/org.gnome.Ptyxis/palettes/tamagi.palette; grep -qx "Name=Tamagi" $f && grep -qx "Background=#2e3440" $f && [[ $(grep -c "^Color15=" $f) -eq 2 ]]'
check "Docker e lazydocker opcionais (Podman continua o padrão)" bash -c '! command -v dockerd && ! command -v lazydocker && for id in docker lazydocker; do grep -qP "^$id\t" /usr/share/tamagi/catalog.tsv || exit 1; done; grep -q "download.docker.com/linux/fedora" /usr/libexec/tamagi/docker-setup && grep -q "groupadd -r docker" /usr/libexec/tamagi/docker-setup && grep -q "sha256sum -c" /usr/libexec/tamagi/lazydocker-setup && grep -q "podman.sock" /usr/libexec/tamagi/lazydocker-setup'
check "catálogo: VLC, OBS Studio e apps do COSMIC opcionais, por Flatpak" bash -c 'ids=$(tamagi-install ids); for id in vlc obs-studio tasks forecast calculator examine cosmic-tweaks popsicle; do grep -qx "$id" <<<"$ids" || { echo "falta $id"; exit 1; }; done; [[ $(grep -c "opcional    COSMIC " /usr/share/tamagi/flatpaks.list) -ge 6 ]]; ! rpm -q vlc obs-studio >/dev/null 2>&1'
check "tamagi try: caixa descartável, pacote inválido recusado" bash -c 'tamagi-try --help | grep -q "disposable" && ! tamagi-try "x;rm" 2>/dev/null && ! tamagi-try --distro gentoo htop 2>/dev/null'
check "tamagi dev db: URL, cliente e snapshots" bash -c 'tamagi-db --help | grep -q "tamagi dev db snapshot" && ! tamagi-db url postgres 2>/dev/null && TAMAGI_LANG=en tamagi-db snapshots | grep -q "No snapshots"'
check "novidades: lê o diff do rpm-ostree e destaca kernel e COSMIC" bash -c 'printf "Upgraded:\n  kernel-core 7.3.4-300.fc45.x86_64 -> 7.3.6-300.fc45.x86_64\n  cosmic-comp 1.11.0-2.fc45 -> 1.12.0-1.fc45\nAdded:\n  snapper-0.13.0-5.fc45.x86_64\n" > /tmp/d.txt && o=$(TAMAGI_LANG=pt /usr/libexec/tamagi/update-changes --texto /tmp/d.txt --curto) && grep -q "Kernel: 7.3.4 → 7.3.6" <<<"$o" && grep -q "COSMIC" <<<"$o" && grep -q "1 novos" <<<"$o" && grep -q update-changes /usr/bin/tamagi-update'
check "Abrir isolado: comando, janela e entrada no Abrir com" bash -c 'bash -n /usr/bin/tamagi-isolate && tamagi-isolate --help | grep -q "no network" && desktop-file-validate /usr/share/applications/tamagi-isolate.desktop && grep -q "application/vnd.appimage" /usr/share/applications/tamagi-isolate.desktop'
check "Abrir isolado: sem rede e sem a pasta pessoal (se o contêiner permitir userns)" bash -c 'mkdir -p $HOME/segredo && echo x > $HOME/segredo/a; out=$(tamagi-isolate --no-gui -- sh -c "ls $HOME/segredo 2>&1; cat /proc/net/dev | wc -l" 2>&1) || { grep -qiE "namespace|permission|operation not permitted" <<<"$out" && exit 0; exit 1; }; ! grep -q "^a$" <<<"$out"'
check "kernel do Bazzite (ogc) é o padrão, com os módulos" bash -c '[[ $(cat /usr/share/tamagi/kernel-flavor) == ogc ]] && ls /usr/lib/modules | grep -q ogc'
check "Secure Boot: guarda ligado e só cancela no caso perigoso" bash -c 'grep -q secureboot-guard /usr/libexec/tamagi/system-update && G=/usr/libexec/tamagi/secureboot-guard; printf "x" > /tmp/k; printf "Fedora Secure Boot Signer" > /tmp/kf; TAMAGI_SB_DRYRUN=1 TAMAGI_SB_ENABLED=1 TAMAGI_SB_KEY=0 TAMAGI_STAGED_VMLINUZ=/tmp/k $G >/dev/null; [[ $? == 3 ]] && TAMAGI_SB_DRYRUN=1 TAMAGI_SB_ENABLED=1 TAMAGI_SB_KEY=1 TAMAGI_STAGED_VMLINUZ=/tmp/k $G && TAMAGI_SB_DRYRUN=1 TAMAGI_SB_ENABLED=0 TAMAGI_SB_KEY=0 TAMAGI_STAGED_VMLINUZ=/tmp/k $G && TAMAGI_SB_DRYRUN=1 TAMAGI_SB_ENABLED=1 TAMAGI_SB_KEY=0 TAMAGI_STAGED_VMLINUZ=/tmp/kf $G'
check "voltar no tempo: junta snapshots e sistema; tamagi sys rewind repassa" bash -c 'mkdir -p $HOME/.snapshots/7/snapshot/.config/tamagi && printf "<snapshot><date>2026-11-20 13:00:00</date></snapshot>" > $HOME/.snapshots/7/info.xml && touch $HOME/.snapshots/7/snapshot/.config/tamagi/tamagi.toml && tamagi-rewind list | grep -qE "snapshot 7, tamagi.toml" && tamagi sys rewind --help | grep -q "timeline"; r=$?; rm -rf $HOME/.snapshots; exit $r'
check "modo privacidade: ajudante simulado, regra do polkit e Super+Ctrl+P" bash -c 'TAMAGI_PRIVACY_DRYRUN=1 /usr/libexec/tamagi/privacy-helper on >/dev/null && test -s /usr/share/polkit-1/rules.d/50-tamagi-privacy.rules && grep -q "privacy-helper" /usr/share/polkit-1/rules.d/50-tamagi-privacy.rules && grep -q "(modifiers: \[Super, Ctrl\], key: \"p\"): Spawn(\"tamagi-privacy\")" /usr/share/cosmic/com.system76.CosmicSettings.Shortcuts/v1/defaults && tamagi-privacy --help | grep -q microphone'
check "shadow workspaces (CoW): isolamento da árvore ao vivo, diff, apply com save-point e drop" bash -c '
    H=/tmp/shw; rm -rf "$H"; mkdir -p "$H/proj" "$H/s1"
    export HOME="$H" XDG_DATA_HOME="$H/.local/share" TAMAGI_LANG=en
    cd "$H/proj" && git init -q . && git -c user.email=t@t -c user.name=t commit -q --allow-empty -m init
    echo "original" > app.txt && git add app.txt && git -c user.email=t@t -c user.name=t commit -q -m v1
    # Verifica montagem transparente --shadow e --egress-sock no agent-jail
    p=$(/usr/libexec/tamagi/agent-jail --print --root "$H/proj" --cwd "$H/proj" --shadow "$H/s1" --egress-sock /tmp/eg.sock -- true)
    grep -qx "$H/s1" <<<"$p" && grep -qx "TAMAGI_AGENT_SHADOW" <<<"$p" && grep -qx -- "--unshare-net" <<<"$p"
    # Ciclo completo: cria sombra CoW, modifica na sombra sem sujar o projeto real, tira diff e aplica
    mkdir -p "$H/.local/share/tamagi/shadows/test1"
    cp -a --reflink=auto "$H/proj" "$H/.local/share/tamagi/shadows/test1/work"
    printf "SHADOW_ID=test1\nSHADOW_ROOT=%s/proj\nSHADOW_AGENT=claude\nSHADOW_CREATED=2026-11-24\n" "$H" > "$H/.local/share/tamagi/shadows/test1/meta.env"
    echo "speculative-change" > "$H/.local/share/tamagi/shadows/test1/work/app.txt"
    echo "new-file" > "$H/.local/share/tamagi/shadows/test1/work/added.txt"
    [[ $(cat "$H/proj/app.txt") == "original" && ! -e "$H/proj/added.txt" ]]
    tamagi-agent shadow list | grep -q "test1.*2 changed files"
    tamagi-agent shadow diff test1 | grep -q "+speculative-change"
    tamagi-agent shadow apply test1 >/dev/null
    [[ $(cat "$H/proj/app.txt") == "speculative-change" && $(cat "$H/proj/added.txt") == "new-file" ]]
    git -C "$H/proj" for-each-ref refs/tamagi/before-agent | grep -q .
    tamagi-agent shadow list | grep -q "No active shadow workspaces"
    rm -rf "$H"
'
check "tamagi-menu: residente pelo autostart" bash -c 'desktop-file-validate /etc/xdg/autostart/tamagi-menu-service.desktop && grep -q "^Exec=tamagi-menu --service" /etc/xdg/autostart/tamagi-menu-service.desktop && env -u WAYLAND_DISPLAY -u DISPLAY tamagi-menu --service'
if [[ $EDITION == skl ]]; then
    check "SKL barra: camada, medidas fixas (margin 0) e Dock intacto" bash -c 'P=/usr/share/cosmic/com.system76.CosmicPanel.Panel/v1; grep -qx Top $P/layer && grep -qx 0 $P/margin && grep -qx 2 $P/padding && grep -qx 4 $P/spacing && ! rpm -V cosmic-panel | grep "CosmicPanel\.Dock/" | grep -q "^..5"'
else
    check "barra: camada e medidas certas, Dock intacto" bash -c 'P=/usr/share/cosmic/com.system76.CosmicPanel.Panel/v1; grep -qx Top $P/layer && grep -qx 4 $P/margin && grep -qx 2 $P/padding && grep -qx 4 $P/spacing && ! rpm -V cosmic-panel | grep "CosmicPanel\.Dock/" | grep -q "^..5"'
fi
check "terminal e editor usam JetBrains Mono" bash -c 'for a in CosmicTerm CosmicEdit; do grep -qx "\"JetBrains Mono\"" /usr/share/cosmic/com.system76.$a/v1/font_name || exit 1; done'
check "agente de IA responde no idioma do sistema" bash -c 'grep -q "Responda sempre em" /usr/libexec/tamagi/agent-local && ! grep -q "português do Brasil," /usr/share/tamagi/agent/system.md'
check "tamagi-panel troca o estilo da barra" bash -c 'tamagi-panel set cosmic >/dev/null && grep -q Dock ~/.config/cosmic/com.system76.CosmicPanel/v1/entries && grep -qx Top ~/.config/cosmic/com.system76.CosmicPanel.Panel/v1/anchor && tamagi-panel set classico >/dev/null && grep -qx None ~/.config/cosmic/com.system76.CosmicPanel.Panel/v1/plugins_center && tamagi-panel set centro >/dev/null && [[ $(tamagi-panel current) == centro ]] && ! tamagi-panel set nada 2>/dev/null'
if [[ $EDITION != skl ]]; then
    check "Claude Code fora da imagem: atalho que baixa no primeiro uso, e no menu" bash -c '[[ $(stat -c %s /usr/bin/claude) -lt 20000 ]] && head -1 /usr/bin/claude | grep -q bash && bash -n /usr/bin/claude && grep -q "sha256sum -c" /usr/bin/claude && desktop-file-validate /usr/share/tamagi/desktop/claude-code.desktop && grep -q tamagi-agent /usr/share/cosmic/com.system76.CosmicAppList/v1/favorites'
fi
check "tmpfiles cria /var/log/tuned (rótulo SELinux do tuned-ppd)" bash -c 'systemd-tmpfiles --cat-config 2>/dev/null | grep -qE "^d /var/log/tuned "'
check "mcelog é pulado fora de processadores Intel" bash -c 'systemctl cat mcelog.service | grep -q "ExecCondition=.*GenuineIntel"'
check "desempenho: writeback fixo, nmi_watchdog=0, NVMe none, boot enxuto" bash -c 'grep -q "^vm.dirty_bytes = 268435456" /usr/lib/sysctl.d/61-tamagi-performance.conf && grep -q "^kernel.nmi_watchdog = 0" /usr/lib/sysctl.d/61-tamagi-performance.conf && grep -q "nvme.*=\"none\"" /usr/lib/udev/rules.d/60-tamagi-ioschedulers.rules && ! systemctl is-enabled NetworkManager-wait-online.service 2>/dev/null | grep -qx enabled && ! systemctl is-enabled dnf-makecache.timer 2>/dev/null | grep -qx enabled && ! systemctl is-enabled virtqemud.service 2>/dev/null | grep -qx enabled'
check "tamagi-devbox: Fedora, Ubuntu, openSUSE e Arch; clientes de banco, mise e ponte para o navegador" bash -c 'o=$(tamagi-devbox distros); [[ $(wc -l <<<"$o") -eq 4 ]] && for d in fedora ubuntu opensuse arch; do grep -q "^  $d " <<<"$o" || exit 1; done; P=/usr/libexec/tamagi/devbox-provision; head -1 $P | grep -qx "#!/usr/bin/env bash" && grep -q "sqlite-devel" $P && grep -q "nodejs npm" $P && grep -q "postgresql mariadb valkey" $P && grep -q "distrobox-host-exec xdg-open" $P && grep -q "PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH" $P && ! tamagi-devbox create gentoo 2>/dev/null'
check "tamagi-perf: telemetria, JSON, polkit, perfil ai (scx_rustland) e unit tamagi-scx válida" bash -c 'tamagi-perf --json | jq -e ".profile and .kernel and .vm" && test -x /usr/libexec/tamagi/perf-helper && test -x /usr/libexec/tamagi/scx-launch && grep -q "scx_rustland" /usr/libexec/tamagi/scx-launch && grep -q "enable --now tamagi-scx.service" /usr/libexec/tamagi/perf-helper && grep -q "perf-helper" /usr/share/polkit-1/rules.d/50-tamagi-perf.rules && systemd-analyze verify /usr/lib/systemd/system/tamagi-scx.service'
check "tamagi-workspace: detecta Rust/Node/SQLite e gera tamagi-workspace.toml" bash -c 'w=/tmp/ws-test; rm -rf "$w"; mkdir -p "$w"; printf "[dependencies]\nrusqlite = \"0.32\"\n" > "$w/Cargo.toml"; printf "{\"scripts\":{\"dev\":\"vite\"}}\n" > "$w/package.json"; tamagi-workspace -C "$w" init >/dev/null && test -f "$w/tamagi-workspace.toml" && tamagi-workspace -C "$w" status --json | jq -e ".configured == true and .distro == \"fedora\" and (.stack | index(\"rust\") != null) and (.stack | index(\"node\") != null) and (.stack | index(\"sqlite\") != null)"; rc=$?; rm -rf "$w"; exit $rc'
check "tamagi-doctor: 13 verificações em JSON + --report Markdown" bash -c 'tamagi-doctor --json | jq -e "(.checks | length) == 13 and all(.checks[]; .id and .status and .summary)" && tamagi-doctor --report | grep -q "Tamagi Soak-Test Report"'
check "tamagi-gaming: nativo sob demanda (como no Bazzite, via tamagi-layer multilib) + Flatpak + status" bash -c 'tamagi-gaming status | grep -q "gamescope" && grep -q -- "--rpmfusion" /usr/bin/tamagi-layer && grep -q "%{NAME}\.%{ARCH}" /usr/bin/tamagi-layer && grep -q "ldconfig" /usr/bin/tamagi-layer && grep -q "mesa-vulkan-drivers.i686" /usr/bin/tamagi-gaming && ! tamagi-gaming native inexistente 2>/dev/null'
check "gum-multi: tradução de Espaço (puro, colado a seta, Kitty press/release) e home-snapshots" bash -c 'test -x /usr/libexec/tamagi/gum-multi && python3 -c "import importlib.util, importlib.machinery; s=importlib.util.spec_from_loader(\"gm\", importlib.machinery.SourceFileLoader(\"gm\", \"/usr/libexec/tamagi/gum-multi\")); m=importlib.util.module_from_spec(s); s.loader.exec_module(m); assert m.translate_space(b\" \x1b[B\") == b\"\t\x1b[B\"; assert m.translate_space(b\"\x1b[32;1:1u\x1b[32;1:3u\") == b\"\t\"; assert m.translate_space(b\"\x1b[27;1;32~\") == b\"\t\"" && grep -q "ensure_user_dir" /usr/libexec/tamagi/home-snapshots'
check "i18n completo: todos os comandos obedecem TAMAGI_LANG (en/es/fr/locale) e ~/.config/tamagi/lang" bash -c 'TAMAGI_LANG=en_US.UTF-8 tamagi-harden status | grep -q "Mode" && TAMAGI_LANG=en tamagi-layer list | grep -q "No layers" && TAMAGI_LANG=en tamagi-browser --help | grep -q "the default browser" && TAMAGI_LANG=es tamagi-devbox distros | grep -q "predeterminad" && TAMAGI_LANG=fr tamagi-doctor --help | grep -q "Diagnostic" && mkdir -p "$HOME/.config/tamagi" && echo en > "$HOME/.config/tamagi/lang" && env -u TAMAGI_LANG -u LC_ALL -u LC_MESSAGES LANG=pt_BR.UTF-8 tamagi-harden status | grep -q "Mode"; rc=$?; rm -f "$HOME/.config/tamagi/lang"; exit $rc'

section "Primeiro boot (o que o instalador deixa para o sistema novo)"
check "serviço habilitado"                systemctl is-enabled tamagi-firstboot.service
mkdir -p /tmp/fb /var/home
printf 'USER=maria\nFULLNAME=Maria da Silva\nPASSWORD_HASH=%s\nXKB_LAYOUT=us\nXKB_VARIANT=intl\n' "$(openssl passwd -6 segredo)" > /tmp/fb/install.conf
check "cria o usuário pedido no instalador" bash -c 'TAMAGI_INSTALL_CONF=/tmp/fb/install.conf /usr/libexec/tamagi/firstboot && id -nG maria | grep -qw wheel'
check "nome completo e senha gravados"    bash -c 'getent passwd maria | grep -q "Maria da Silva" && getent shadow maria | cut -d: -f2 | grep -q "^[$]6[$]"'
check "teclado escolhido vale para o usuário" bash -c 'grep -q "layout: \"us\"" ~maria/.config/cosmic/com.system76.CosmicComp/v1/xkb_config && grep -q "variant: \"intl\"" ~maria/.config/cosmic/com.system76.CosmicComp/v1/xkb_config && [[ $(stat -c %U ~maria/.config/cosmic/com.system76.CosmicComp/v1/xkb_config) == maria ]]'
check "arquivo de respostas é apagado"    test ! -e /tmp/fb/install.conf
if [[ $EDITION == skl ]]; then
    mkdir -p /tmp/fb
    printf 'EDITION=skl\nUSER=prof\nFULLNAME=Professor\nPASSWORD_HASH=%s\nSTUDENT_USER=aluno\nSKL_HOMEPAGE=https://escola.example.edu\nXKB_LAYOUT=br\n' "$(openssl passwd -6 segredo)" > /tmp/fb/skl.conf
    check "SKL firstboot: cria admin (wheel) + aluno (sem sudo), autologin e homepage" bash -c 'TAMAGI_INSTALL_CONF=/tmp/fb/skl.conf /usr/libexec/tamagi/firstboot && id -nG prof | grep -qw wheel && ! id -nG aluno | grep -qw wheel && grep -q "user = \"aluno\"" /etc/greetd/cosmic-greeter.toml && jq -e ".HomepageLocation == \"https://escola.example.edu\"" /etc/chromium/policies/managed/tamagi-skl.json >/dev/null'
fi
mkdir -p /tmp/fb; printf 'USER=Root; rm -rf /\n' > /tmp/fb/install.conf
check "recusa nome de usuário inválido"   bash -c '! TAMAGI_INSTALL_CONF=/tmp/fb/install.conf /usr/libexec/tamagi/firstboot 2>/dev/null'


echo
echo "Resultado: $pass ok, $fail falha(s)"
exit $(( fail > 0 ))
