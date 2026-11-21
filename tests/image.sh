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

export HOME=/tmp/test-home XDG_RUNTIME_DIR=/tmp/test-run
mkdir -p "$HOME" "$XDG_RUNTIME_DIR"

section "Identidade"
check "os-release diz Tamagi"            grep -q '^NAME="Tamagi"' /usr/lib/os-release
check "versão 1.0, codinome Kazunobu M"  grep -q '^PRETTY_NAME="Tamagi 1.0 (Kazunobu M)"' /usr/lib/os-release
check "VERSION_ID continua o do Fedora"   grep -qE '^VERSION_ID=4[0-9]' /usr/lib/os-release
check "ID continua fedora"                grep -q '^ID=fedora' /usr/lib/os-release
check "image-info.json é JSON válido"     jq -e '."image-name"' /usr/share/ublue-os/image-info.json
check "root padrão é btrfs"               grep -q 'type = "btrfs"' /usr/lib/bootc/install/10-tamagi.toml

section "Ferramentas na imagem"
for cmd in distrobox podman flatpak bootc ujust foot alacritty cosmic-term cosmic-files git nvim rg fd bat \
           eza zoxide btop fastfetch gum lazygit starship mise jq dust delta hyperfine uv \
           sudo-rs uutils-coreutils gamescope mangohud gamemoderun usbguard \
           llama-server llama-cli opencode claude \
           tamagi-ai tamagi-agent tamagi-theme tamagi-menu tamagi-assist tamagi-keys tamagi-panel tamagi-lang \
           tamagi-firstrun tamagi-pkg tamagi-update tamagi-webapp \
           tamagi-browser tamagi-harden tamagi tamagi-layer tamagi-boot-report tamagi-snap snapper tamagi-windows tamagi-db tamagi-lembrete tamagi-capture tamagi-dev tamagi-font tesseract; do
    check "comando $cmd" command -v "$cmd"
done

section "Base e kernel"
kver=$(ls /usr/lib/modules | head -1); kconf=/usr/lib/modules/$kver/config
echo "  kernel $kver"
check "kernel com suporte a Rust"          grep -qx 'CONFIG_RUST=y' "$kconf"
check "kernel: binder (Android em contêiner), em C ou em Rust" grep -qxE 'CONFIG_ANDROID_BINDER_IPC(_RUST)?=y' "$kconf"
check "kernel é o do sabor escolhido no build" bash -c "f=\$(cat /usr/share/tamagi/kernel-flavor); case \$f in ogc) [[ $kver == *ogc* ]] ;; main) [[ $kver != *ogc* ]] ;; *) false ;; esac"
check "um kernel só, com initramfs" bash -c "[[ \$(ls /usr/lib/modules | wc -l) -eq 1 && -s /usr/lib/modules/$kver/initramfs.img && -s /usr/lib/modules/$kver/vmlinuz ]]"
check "kernel: aceleradores de IA (DRM_ACCEL)" grep -qx 'CONFIG_DRM_ACCEL=y' "$kconf"
check "kernel: NPU AMD Ryzen AI (amdxdna)" bash -c "find /usr/lib/modules/$kver -name 'amdxdna.ko*' | grep -q ."
check "kernel: NPU Intel (ivpu)"           bash -c "find /usr/lib/modules/$kver -name 'intel_vpu.ko*' | grep -q ."
check "kernel: sched_ext (escalonadores em Rust/BPF)" grep -qx 'CONFIG_SCHED_CLASS_EXT=y' "$kconf"
check "módulos do Universal Blue para este kernel (controles Xbox, câmera virtual)" bash -c "for m in xone_dongle hid-xpadneo v4l2loopback; do find /usr/lib/modules/$kver -name \"\$m.ko*\" | grep -q . || { echo \$m; exit 1; }; done"
check "módulos assinados; chave do Universal Blue na imagem (Secure Boot)" bash -c "test -s /etc/pki/akmods/certs/akmods-ublue.der && f=\$(find /usr/lib/modules/$kver -name 'v4l2loopback.ko*' | head -1) && modinfo -F signer \"\$f\" | grep -q ."

section "Redes e conectividade"
for vpn in openvpn openconnect l2tp pptp sstp strongswan libreswan ssh; do
    check "VPN $vpn no NetworkManager"     bash -c "ls /usr/lib/NetworkManager/VPN/ /etc/NetworkManager/VPN/ 2>/dev/null | grep -qi '$vpn'"
done
check "WireGuard"                          command -v wg
check "Wi-Fi, banda larga móvel e Bluetooth no NetworkManager" rpm -q NetworkManager-wifi NetworkManager-wwan NetworkManager-bluetooth ModemManager
check "Tailscale (desligado até você ativar)" bash -c 'command -v tailscale && ! systemctl is-enabled tailscaled 2>/dev/null | grep -qx enabled'
check "pastas Windows/NAS (SMB), NFS e SSHFS" bash -c 'rpm -q gvfs-smb cifs-utils nfs-utils fuse-sshfs samba-client && command -v wsdd'
check "iPhone e Android (arquivos e tethering)" bash -c 'rpm -q usbmuxd ifuse gvfs-afc gvfs-mtp android-tools usb_modeswitch'
check "descoberta na rede local (mDNS)"    rpm -q avahi nss-mdns
check "impressão e scanner sem driver"     rpm -q cups cups-browsed ipp-usb sane-airscan
check "firmware de Wi-Fi (Intel, Realtek, MediaTek, Atheros, Broadcom)" rpm -q iwlwifi-mvm-firmware realtek-firmware mt7xxx-firmware atheros-firmware brcmfmac-firmware
check "firewall ativo por padrão"          systemctl is-enabled firewalld

section "Navegadores (caixa Fedora) e apps"
check "nenhum navegador no host"            bash -c '! rpm -q google-chrome-stable firefox chromium 2>/dev/null | grep -qv "not installed"'
check "entrada do Chrome no menu aponta para a caixa" grep -q '^Exec=/usr/bin/tamagi-browser chrome %U' /usr/share/applications/google-chrome.desktop
check "navegador padrão é o Chrome da caixa" grep -q 'x-scheme-handler/https=google-chrome.desktop' /etc/xdg/mimeapps.list
check "caixa dos navegadores é Fedora"    grep -q 'fedora-toolbox:45' /usr/bin/tamagi-browser
check "tamagi-browser status sem a caixa"   bash -c 'tamagi-browser status | grep -q "ainda não criada"'
check "tamagi-browser recusa navegador desconhecido" bash -c '! tamagi-browser install opera 2>/dev/null'
check "opções de aceleração de vídeo"       bash -c 'source <(sed -n "/^chromium_flags()/,/^}/p" /usr/bin/tamagi-browser); FLAGS_FILE=/nonexistent; chromium_flags | grep -q AcceleratedVideoEncoder'
check "OnlyOffice na lista de Flatpaks"    grep -qx 'flathub org.onlyoffice.desktopeditors' /usr/share/tamagi/flatpaks.list
check "remote cosmic-flatpak configurado"  grep -q '^Url=https://apt.pop-os.org/cosmic/' /etc/flatpak/remotes.d/cosmic.flatpakrepo
check "lista de Flatpaks só usa flathub e cosmic" bash -c '! grep -vE "^(#|$|flathub |cosmic )" /usr/share/tamagi/flatpaks.list | grep -q .'
check ".deb e .rpm abrem no instalador de subsistema" grep -q 'application/vnd.debian.binary-package=tamagi-sideload.desktop' /etc/xdg/mimeapps.list
check "tamagi-pkg lista os 3 subsistemas" bash -c '[[ $(tamagi-pkg status | grep -cE "^(deb|rpm|arch) ") -eq 3 ]]'
check "subsistema deb é Ubuntu 26.04 LTS" bash -c 'tamagi-pkg status | grep -E "^deb " | grep -q "ubuntu:26.04"'
check "chave do repositório do Omarchy confere" bash -c '[[ $(gpg --show-keys --with-colons /usr/share/tamagi/omarchy/omarchy.gpg | awk -F: "\$1 == \"fpr\" {print \$10; exit}") == 40DFB630FF42BCFFB047046CF0134EE680CAC571 ]]'
check "web app gera .desktop válido"       bash -c 'tamagi-webapp install "Meu Teste" https://exemplo.invalid >/dev/null && desktop-file-validate ~/.local/share/applications/webapp-meu-teste.desktop && grep -q "Exec=tamagi-browser chrome --app=" ~/.local/share/applications/webapp-meu-teste.desktop && tamagi-webapp list | grep -q "Meu Teste" && tamagi-webapp remove "Meu Teste" >/dev/null && [[ -z $(tamagi-webapp list) ]]'

section "Rust"
check "COSMIC (desktop em Rust) instalado"  rpm -q cosmic-session cosmic-comp cosmic-term cosmic-files cosmic-edit
check "sudo do shell interativo é o sudo-rs" bash -c '/usr/libexec/tamagi/rust-bin/sudo --version | grep -qi sudo-rs'
check "ls do shell interativo é o do uutils" bash -c '/usr/libexec/tamagi/rust-bin/ls --version | grep -qi uutils'
check "shell interativo põe o Rust na frente do PATH" bash -lic '[[ $(command -v sudo) == /usr/libexec/tamagi/rust-bin/sudo && $(type -P cp) == /usr/libexec/tamagi/rust-bin/cp ]]'
check "originais continuam em /usr/bin (serviços e scripts)" bash -c '/usr/bin/ls --version | grep -q GNU && /usr/bin/sudo --version | grep -qv sudo-rs'
check "opt-out do Rust no shell"           bash -c 'mkdir -p ~/.config/tamagi && touch ~/.config/tamagi/no-rust-defaults && ! bash -lic "echo \$PATH" 2>/dev/null | grep -q rust-bin; rc=$?; rm -f ~/.config/tamagi/no-rust-defaults; exit $rc'
check "uutils preserva rótulos SELinux (ls -Z)" bash -c '/usr/libexec/tamagi/rust-bin/ls -Z / >/dev/null'

section "Máquinas virtuais (nativo)"
for cmd in virt-manager virt-viewer virt-install virsh qemu-img swtpm virt-customize; do
    check "comando $cmd" command -v "$cmd"
done
check "QEMU/KVM, firmware UEFI e virtiofs" bash -c 'rpm -q qemu-kvm edk2-ovmf virtiofsd libvirt-daemon-kvm && test -x /usr/libexec/virtiofsd'
check "daemons do libvirt sobem por socket" bash -c 'for s in virtqemud virtnetworkd virtstoraged virtnodedevd; do systemctl is-enabled $s.socket | grep -qx enabled || exit 1; done'
check "administradores gerenciam VMs sem senha (polkit)" grep -q 'org.libvirt.unix.manage' /usr/share/polkit-1/rules.d/50-tamagi-libvirt.rules
check "pastas de /var dos pacotes viram tmpfiles" bash -c 'grep -qE "^d /var/lib/libvirt/images " /usr/lib/tmpfiles.d/tamagi-autovar.conf || systemd-tmpfiles --cat-config | grep -qE "^d /var/lib/libvirt/images "'
check "tmpfiles gerado é bem formado" bash -c '[[ $(grep -c "^d " /usr/lib/tmpfiles.d/tamagi-autovar.conf) -gt 20 ]] && ! grep -vE "^(#.*|d /var/(lib|log|spool)/[^ ]+ [0-7]{3,4} [a-zA-Z0-9_-]+ [a-zA-Z0-9_-]+ -)$" /usr/lib/tmpfiles.d/tamagi-autovar.conf | grep -q .'

section "Atualização automática"
check "imagem do sistema: timer ligado (stage)" bash -c 'systemctl is-enabled rpm-ostreed-automatic.timer | grep -qx enabled && grep -q AutomaticUpdatePolicy=stage /etc/rpm-ostreed.conf'
check "Flatpaks: timers do sistema e do usuário" bash -c 'systemctl is-enabled flatpak-system-update.timer | grep -qx enabled && systemctl --global is-enabled flatpak-user-update.timer | grep -qx enabled'
check "subsistemas e mise: timer do usuário ligado" bash -c 'systemctl --global is-enabled tamagi-autoupdate.timer | grep -qx enabled'
check "units do timer são válidas"       systemd-analyze verify --user /usr/lib/systemd/user/tamagi-autoupdate.timer /usr/lib/systemd/user/tamagi-autoupdate.service
check "tamagi-update status lista os quatro (pt e en)" bash -c '[[ $(tamagi-update status | grep -c "próxima:") -eq 4 && $(TAMAGI_LANG=en tamagi-update status | grep -c "next:") -eq 4 ]]'
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
check "boot com falha desliga as camadas"  bash -c 'mkdir -p /var/lib/extensions && cp /tmp/cam/teste.raw /var/lib/extensions/ && /usr/lib/greenboot/red.d/40-tamagi-camadas.sh >/dev/null && [[ ! -e /var/lib/extensions/teste.raw && -e /var/lib/tamagi/layers-off/teste.raw ]] && tamagi-layer list | grep -q "teste.*desligada"; rc=$?; rm -rf /var/lib/tamagi/layers-off /var/lib/extensions; exit $rc'
check "tamagi.toml: camada entra no plano e pede senha" bash -c 'printf "[camadas]\nferramentas = [\"strace\", \"ltrace\"]\n" > /tmp/cam.toml && tamagi diff /tmp/cam.toml | grep -q "camada ferramentas: strace ltrace   \[pede senha\]" && printf "[camadas]\n\"Ruim\" = \"x\"\n" > /tmp/camruim.toml && ! tamagi diff /tmp/camruim.toml 2>/dev/null'

check "o agente local sabe do tamagi-layer e não sugere rpm-ostree install" bash -c 'grep -q "tamagi-layer add" /usr/share/tamagi/agent/system.md && ! grep -q "rpm-ostree install <pacote>\` (exige" /usr/share/tamagi/agent/system.md && ! command -v tamagi-app'

section "Inovação 1: o sistema num arquivo (tamagi apply)"
check "comando tamagi"                    command -v tamagi
check "export gera TOML válido com as 9 seções" bash -c 'tamagi export /tmp/t.toml >/dev/null && python3 -c "import tomllib; d = tomllib.load(open(\"/tmp/t.toml\", \"rb\")); assert set(d) == {\"aparencia\", \"navegadores\", \"flatpak\", \"mise\", \"pacotes\", \"caixas\", \"camadas\", \"webapps\", \"servicos\"}, d.keys()"'
check "diff do próprio estado: nada a fazer" bash -c 'tamagi diff /tmp/t.toml | grep -q "Nada a fazer"'
cat > /tmp/quero.toml <<'TOML'
[aparencia]
tema = "nord"
papel = "byobu"
[flatpak]
apps = ["org.gimp.GIMP"]
[mise]
ferramentas = ["node@lts"]
[pacotes]
omarchy = ["hype"]
[caixas]
dev = "arch"
[webapps]
WhatsApp = "https://web.whatsapp.com"
[servicos]
seguranca = "estrito"
TOML
check "diff lista o que falta e marca o que pede senha" bash -c 'o=$(tamagi diff /tmp/quero.toml); for x in "tema: nord" "papel de parede: byobu" "Flatpak: org.gimp.GIMP" "mise: node@lts" "pacote omarchy: hype" "caixa dev (arch)" "web app: WhatsApp" "segurança: estrito   \[pede senha\]"; do grep -q "$x" <<<"$o" || { echo "faltou: $x"; exit 1; }; done'
printf '[aparencia]\ntema = "gruvbox"\npapel = "asanoha"\n[webapps]\nExemplo = "https://exemplo.invalid"\n' > /tmp/so-local.toml
check "apply executa e fica idempotente"  bash -c 'tamagi apply -y /tmp/so-local.toml >/dev/null && [[ $(tamagi-theme current) == gruvbox ]] && grep -q gruvbox-asanoha.png ~/.config/cosmic/com.system76.CosmicBackground/v1/all && test -f ~/.local/share/applications/webapp-exemplo.desktop && tamagi diff /tmp/so-local.toml | grep -q "Nada a fazer"; rc=$?; tamagi-webapp remove Exemplo >/dev/null; tamagi-theme wallpaper mon >/dev/null; exit $rc'
check "apply sem terminal e sem -y não altera nada" bash -c '! tamagi apply /tmp/quero.toml </dev/null >/dev/null 2>&1 && [[ $(tamagi-theme current) != nord ]]'
printf '[navegadores]\ninstalar = ["opera"]\n[caixas]\nx = "gentoo"\n[inventado]\na = 1\n' > /tmp/ruim.toml
check "arquivo com erros é recusado com a lista" bash -c 'o=$(tamagi diff /tmp/ruim.toml 2>&1); [[ $? -ne 0 ]] && grep -q "navegador desconhecido: opera" <<<"$o" && grep -q "gentoo" <<<"$o" && grep -q "inventado" <<<"$o"'

section "Perfis isolados do navegador e chave de segurança"
check "cria perfis com atalho próprio no menu" bash -c 'export XDG_DATA_HOME=/tmp/perfis; rm -rf /tmp/perfis; tamagi-browser profile add trabalho >/dev/null && tamagi-browser profile add pessoal firefox >/dev/null && desktop-file-validate /tmp/perfis/applications/tamagi-chrome-trabalho.desktop && grep -qx "Exec=/usr/bin/tamagi-browser chrome --perfil trabalho %U" /tmp/perfis/applications/tamagi-chrome-trabalho.desktop && grep -qx "StartupWMClass=tamagi-chrome-trabalho" /tmp/perfis/applications/tamagi-chrome-trabalho.desktop && [[ -d /tmp/perfis/tamagi/browser-profiles/chrome-trabalho && -d /tmp/perfis/tamagi/browser-profiles/firefox-pessoal ]]'
check "lista os perfis; nome inválido é recusado" bash -c 'export XDG_DATA_HOME=/tmp/perfis; o=$(tamagi-browser profile list) && grep -q "^chrome *trabalho$" <<<"$o" && grep -q "^firefox *pessoal$" <<<"$o" && ! tamagi-browser profile add "Ruim Nome" 2>/dev/null && ! tamagi-browser profile add "../x" 2>/dev/null'
check "remover perfil exige confirmação num terminal" bash -c 'export XDG_DATA_HOME=/tmp/perfis; ! tamagi-browser profile remove trabalho </dev/null >/dev/null 2>&1 && [[ -d /tmp/perfis/tamagi/browser-profiles/chrome-trabalho ]]'
check "diagnóstico da chave de segurança roda sem chave e sem caixa" bash -c 'o=$(tamagi-browser fido 2>&1); grep -q "Chaves de segurança vistas pelo sistema" <<<"$o"'
check "modo banco e Warsaw saíram da imagem" bash -c '! command -v tamagi-bank && ! command -v tamagi-warsaw && [[ ! -e /usr/share/tamagi/warsaw && ! -e /usr/share/applications/tamagi-bank.desktop ]] && ! grep -rqi warsaw /usr/bin/tamagi-* /usr/libexec/tamagi /usr/share/ublue-os/just/60-custom.just'

section "Inovação 3: boot que se conserta e se explica"
check "greenboot instalado e ligado"       bash -c 'rpm -q greenboot && systemctl is-enabled greenboot-healthcheck.service | grep -qx enabled'
check "só as verificações do Tamagi (nenhuma exige rede)" bash -c '[[ $(ls /usr/lib/greenboot/check/required.d/) == 10-tamagi-essencial.sh ]] && ! ls /etc/greenboot/check/required.d/* 2>/dev/null | grep -q .'
check "GRUB do bootc conta as tentativas de boot" grep -q boot_counter /usr/lib/bootupd/grub2-static/configs.d/08_greenboot.cfg
check "relatório de falha é gravado"       bash -c 'mkdir -p /var/lib/tamagi && bash /usr/lib/greenboot/red.d/50-tamagi-relatorio.sh && ls /var/lib/tamagi/boot-failures/*.txt >/dev/null && grep -q "Verificações de saúde" /var/lib/tamagi/boot-failures/*.txt'
check "tamagi-boot-report mostra o pendente e marca como visto" bash -c 'chmod a+r /var/lib/tamagi/boot-failures/*.txt; a=$(tamagi-boot-report show </dev/null); b=$(tamagi-boot-report show </dev/null); grep -q "voltou para a imagem anterior" <<<"$a" && grep -q "Nenhuma falha de boot pendente" <<<"$b"'
check "shims do mise no PATH de toda sessão (apps gráficos e IDEs)" bash -lc 'case ":$PATH:" in *"/mise/shims:"*) exit 0 ;; *) exit 1 ;; esac'
check "configuração padrão do mise é lida" bash -c 'mise settings ls 2>/dev/null | grep -E "^not_found_auto_install +true +/etc/mise/config.toml"'
check "mise ativado no shell interativo"  bash -lic 'type -t mise | grep -qx function'


section "Segurança e desempenho"
check "sysctl de endurecimento na imagem"   grep -q '^kernel.kptr_restrict = 2' /usr/lib/sysctl.d/60-tamagi-hardening.conf
check "sysctls são aceitos pelo systemd-sysctl" bash -c 'out=$(/usr/lib/systemd/systemd-sysctl --cat-config 2>&1); echo "$out" | grep -q "kernel.yama.ptrace_scope = 1" && echo "$out" | grep -q "vm.max_map_count = 2147483642"'
check "argumentos de kernel (bootc kargs.d) são TOML válido" python3 -c "import tomllib; d = tomllib.load(open('/usr/lib/bootc/kargs.d/10-tamagi.toml', 'rb')); assert 'init_on_alloc=1' in d['kargs']"
check "MAC aleatório por rede no NetworkManager" grep -q 'wifi.cloned-mac-address=stable' /usr/lib/NetworkManager/conf.d/50-tamagi-privacy.conf
check "hardened_malloc instalado mas não global" bash -c 'test -f /usr/lib64/libhardened_malloc.so && test ! -e /etc/ld.so.preload'
check "hardened_malloc carrega num programa" bash -c 'LD_PRELOAD=/usr/lib64/libhardened_malloc.so /usr/bin/ls / >/dev/null'
check "USBGuard instalado e desligado"      bash -c 'rpm -q usbguard && ! systemctl is-enabled usbguard.service 2>/dev/null | grep -qx enabled'
check "tamagi-harden status"                bash -c 'tamagi-harden status | grep -q "Modo.*equilibrado"'
check "zram: zstd, metade da RAM"           grep -q 'compression-algorithm = zstd' /usr/lib/systemd/zram-generator.conf.d/10-tamagi.conf
check "regras udev de E/S são válidas"      bash -c 'udevadm verify /usr/lib/udev/rules.d/60-tamagi-ioschedulers.rules'
check "controles de jogo (steam-devices)"   rpm -q steam-devices

section "Agentes de terceiros"
check "Claude Code executa"                claude --version
check "auto-update do Claude desligado (binário em /usr)" bash -lc '[[ $DISABLE_AUTOUPDATER == 1 ]]'

section "Scripts"
for f in /usr/bin/tamagi-* /usr/libexec/tamagi/*; do
    [[ -f $f ]] || continue
    if head -1 "$f" | grep -q python; then
        check "sintaxe $(basename "$f")" python3 -c "import ast,sys; ast.parse(open(sys.argv[1]).read())" "$f"
    else
        check "sintaxe $(basename "$f")" bash -n "$f"
    fi
done
check "ujust lista as receitas do Tamagi" bash -c 'r=$(ujust --summary | tr " " "\n"); for x in devbox update-arch browser setup-gaming harden autoupdate vms sistema boot-report; do grep -qx "$x" <<<"$r" || exit 1; done'

section "llama.cpp"
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

section "Agente (regras de segurança)"
python3 - <<'PY' && ok "classificação de comandos somente-leitura" || bad "classificação de comandos somente-leitura"
import importlib.machinery, importlib.util, sys
loader = importlib.machinery.SourceFileLoader("agent", "/usr/bin/tamagi-agent")
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
check "tiling automático por padrão"   grep -qx true $S/com.system76.CosmicComp/v1/autotile
check "teclado ABNT2 por padrão"       grep -q 'layout: "br"' $S/com.system76.CosmicComp/v1/xkb_config
check "papel de parede padrão: brasão (mon)" bash -c 'grep -q tamagi/tokyo-night-mon.png /usr/share/cosmic/com.system76.CosmicBackground/v1/all && test -s /usr/share/backgrounds/tamagi/tokyo-night-mon.png'
check "8 estilos de papel de parede por tema (88 PNG)" bash -c '[[ $(ls /usr/share/backgrounds/tamagi/*.png | wc -l) -eq 88 ]]'
check "cada tema tem o seu tema do COSMIC (.ron)" bash -c 'for t in $(tamagi-theme list); do f=/usr/share/cosmic-themes/tamagi-$t.ron; grep -q "^    accent: Some" "$f" && grep -q "^    palette: " "$f" || { echo "$t"; exit 1; }; done; tamagi-theme list | grep -qx ipe'
for st in mon seigaiha asanoha byobu shippo hinode uroko; do
    check "estilo $st é SVG válido e PNG em 4K" bash -c "python3 -c 'import sys, xml.dom.minidom as m; m.parse(sys.argv[1])' /usr/share/backgrounds/tamagi/kanagawa-$st.svg && file /usr/share/backgrounds/tamagi/kanagawa-$st.png | grep -q '3840 x 2160'"
done
D=$S/com.system76.CosmicSettings.Shortcuts/v1/defaults
check "atalho Super+Enter abre o terminal" grep -q 'key: "Return"): System(Terminal)' $D
check "atalho Super+Espaço abre o tamagi-menu" grep -q '(modifiers: \[Super\], key: "space"): Spawn("tamagi-menu")' $D
check "atalho Super+Alt+Espaço abre o lançador COSMIC" grep -q '(modifiers: \[Super, Alt\], key: "space"): System(Launcher)' $D
check "atalho Super+W fecha a janela"  grep -q '(modifiers: \[Super\], key: "w"): Close' $D
check "atalho Super+A abre as ações rápidas" grep -q 'key: "a"): Spawn("/usr/libexec/tamagi/popup tamagi-assist")' $D
check "sem combinações de teclas duplicadas" bash -c "[[ -z \$(grep -oE '^\s*\(modifiers: \[[^]]*\](, key: \"[^\"]+\")?\)' $D | sed 's/ //g' | sort | uniq -d) ]]"
check "arquivo de atalhos termina bem formado" bash -c "tail -1 $D | grep -qx '}'"
check "janelas do Tamagi flutuam (tamagi-popup e org.tamagi.Menu)" bash -c 'grep -q "appid: \"tamagi-popup\"" '"$S"'/com.system76.CosmicSettings.WindowRules/v1/tiling_exception_defaults && grep -q "appid: \"org.tamagi.Menu\"" '"$S"'/com.system76.CosmicSettings.WindowRules/v1/tiling_exception_defaults'
check "tamagi-menu (GTK4 + actions.toml) lista ações em pt e en" bash -c 'python3 -c "import gi; gi.require_version(\"Gtk\",\"4.0\"); gi.require_version(\"Adw\",\"1\"); from gi.repository import Gtk, Adw" && tamagi-menu --list | grep -q "^ai.setup " && TAMAGI_LANG=en tamagi-menu --json | jq -e "length > 20 and any(.[]; .id == \"ai.setup\" and (.title | contains(\"AI model\")))"'
check "tamagi-agent carrega tamagi_action do actions.toml" bash -c 'python3 -c "import importlib.machinery, importlib.util; l=importlib.machinery.SourceFileLoader(\"a\",\"/usr/bin/tamagi-agent\"); s=importlib.util.spec_from_loader(\"a\",l); m=importlib.util.module_from_spec(s); l.exec_module(m); assert \"ai.status\" in m.AGENT_ACTIONS and any(t[\"function\"][\"name\"]==\"tamagi_action\" for t in m.TOOLS)"'
check "regras de janela terminam bem formadas" bash -c "tail -1 $S/com.system76.CosmicSettings.WindowRules/v1/tiling_exception_defaults | grep -qx ']'"
check "dock: Chrome da caixa e agente"  bash -c 'grep -q "\"google-chrome\"" /usr/share/cosmic/com.system76.CosmicAppList/v1/favorites && grep -q tamagi-agent /usr/share/cosmic/com.system76.CosmicAppList/v1/favorites'
for fav in $(grep -oE '"[^"]+"' $S/com.system76.CosmicAppList/v1/favorites | tr -d '"'); do
    check "favorito $fav existe"         test -f "/usr/share/applications/$fav.desktop"
done
check "portal do COSMIC"               rpm -q xdg-desktop-portal-cosmic
check "primeiro login: só o assistente do Tamagi abre sozinho" bash -c 'test ! -e /etc/xdg/autostart/com.system76.CosmicInitialSetup.desktop && test -f /etc/xdg/autostart/tamagi-firstrun.desktop && command -v cosmic-initial-setup'
check "configuração do alacritty é TOML válido" python3 -c "import tomllib; d = tomllib.load(open('/usr/share/tamagi/alacritty.toml', 'rb')); assert d['colors']['primary']['background'].startswith('#')"
check "tamagi-theme troca o estilo do papel de parede" bash -c 'tamagi-theme wallpaper byobu >/dev/null && grep -q tokyo-night-byobu.png ~/.config/cosmic/com.system76.CosmicBackground/v1/all && tamagi-theme wallpaper ondas >/dev/null && grep -q "tamagi/tokyo-night.png" ~/.config/cosmic/com.system76.CosmicBackground/v1/all && ! tamagi-theme wallpaper inexistente 2>/dev/null; tamagi-theme wallpaper mon >/dev/null'
check "tamagi-theme aplica um tema"     bash -c 'tamagi-theme set gruvbox >/dev/null && grep -q gruvbox-mon.png ~/.config/cosmic/com.system76.CosmicBackground/v1/all && python3 -c "import tomllib,os; tomllib.load(open(os.path.expanduser(\"~/.config/tamagi/alacritty.toml\"), \"rb\"))" && [[ $(tamagi-theme current) == gruvbox ]]'
for t in $(tamagi-theme list); do
    check "tema $t é um arquivo válido"     bash -c "source /usr/share/tamagi/themes/$t.conf && [[ \${#PALETTE[@]} -eq 16 && \${#WALL[@]} -eq 5 ]]"
done
check "11 temas"                          bash -c '[[ $(tamagi-theme list | wc -l) -eq 11 ]]'
for t in $(tamagi-theme list); do
    check "papel de parede $t é SVG válido" python3 -c "import sys, xml.dom.minidom as m; m.parse(sys.argv[1])" "/usr/share/backgrounds/tamagi/$t.svg"
done
section "Serviços"
check "flatpak-setup habilitado"          systemctl is-enabled tamagi-flatpak-setup.service
check "units do sistema são válidas"      systemd-analyze verify /usr/lib/systemd/system/tamagi-flatpak-setup.service /usr/lib/systemd/system/tamagi-firstboot.service
check "lista de Flatpaks não está vazia"  grep -qE '^[a-z]' /usr/share/tamagi/flatpaks.list
check "atualização automática (stage)"    grep -q 'AutomaticUpdatePolicy=stage' /etc/rpm-ostreed.conf
check "distrobox-create não gera volume :ro:rslave (podman 6)" bash -c '! grep -qE ":ro\\$\\{rslave\\}|:ro:rslave" /usr/bin/distrobox-create && grep -q ":ro,rslave" /usr/bin/distrobox-create'
check "botões do gum com cores legíveis"  bash -c '. /usr/share/tamagi/gum.env && [[ $GUM_CONFIRM_SELECTED_FOREGROUND == "#11111b" ]] && grep -q gum.env /usr/libexec/tamagi/popup /etc/profile.d/tamagi.sh'
check "TPM: serviço que amarra a chave ao PCR 7 ligado" bash -c 'systemctl is-enabled tamagi-tpm-seal.service | grep -qx enabled && test -x /usr/libexec/tamagi/tpm-seal'
check "idiomas: toda entrada de menu com [pt_BR] tem [pt] (o lançador do COSMIC lê [pt])" bash -c 'for f in /usr/share/applications/tamagi-*.desktop /usr/share/applications/claude-code.desktop /usr/share/applications/google-chrome.desktop; do [[ $(grep -c "\[pt_BR\]=" "$f") -eq $(grep -c "\[pt\]=" "$f") ]] || { echo "$f"; exit 1; }; done'
check "janelas utilitárias têm nome e ícone (tamagi-popup, tamagi-term)" bash -c 'for c in tamagi-popup tamagi-term; do desktop-file-validate /usr/share/applications/$c.desktop && grep -qx "StartupWMClass=$c" /usr/share/applications/$c.desktop || exit 1; done'
check "idiomas: locales pt_BR, en_US, es_ES e fr_FR" bash -c 'for l in pt_BR en_US es_ES fr_FR; do locale -a | grep -qix "$l.utf8" || exit 1; done'
check "idiomas: todas as frases das ferramentas traduzidas (en/es/fr)" python3 /tests/i18n-strings.py / --check
check "idiomas: a camada de tradução responde em es e fr" bash -c '[[ $(LANG=es_ES.UTF-8 bash -c ". /usr/libexec/tamagi/i18n; t Sair") == Salir && $(LANG=fr_FR.UTF-8 bash -c ". /usr/libexec/tamagi/i18n; t Sair") == Quitter && $(LANG=de_DE.UTF-8 bash -c ". /usr/libexec/tamagi/i18n; t Sair") == Quit && $(LANG=pt_BR.UTF-8 bash -c ". /usr/libexec/tamagi/i18n; t Sair") == Sair ]]'
check "idiomas: atalhos em inglês"         bash -c 'LANG=en_US.UTF-8 tamagi-keys | grep -q "close window"'
check "idiomas: tamagi-lang lista os quatro" bash -c '[[ $(tamagi-lang list | wc -l) -eq 4 ]]'
check "idiomas: entradas do menu traduzidas" bash -c 'for f in tamagi-agent tamagi-menu claude-code google-chrome; do grep -q "\[fr\]=" /usr/share/applications/$f.desktop && grep -q "\[es\]=" /usr/share/applications/$f.desktop || exit 1; done'
check "fontes: Noto Sans (interface) e Roboto Slab (serifa)" bash -c '[[ $(fc-match -f "%{family}" sans-serif) == "Noto Sans" && $(fc-match -f "%{family}" serif) == "Roboto Slab" ]] && grep -q "Noto Sans" /usr/share/cosmic/com.system76.CosmicTk/v1/interface_font'
check "barra: uma só, embaixo, apps no centro" bash -c 'P=/usr/share/cosmic/com.system76.CosmicPanel.Panel/v1; grep -q "^\[\"Panel\"\]$" /usr/share/cosmic/com.system76.CosmicPanel/v1/entries && grep -qx Bottom $P/anchor && grep -q CosmicAppList $P/plugins_center && grep -q CosmicAppletTime $P/plugins_wings'
check "menu iniciar: applet instalado, primeiro botão da barra, layout Retro" bash -c 'P=/usr/share/cosmic/com.system76.CosmicPanel.Panel/v1; M=/usr/share/cosmic/com.championpeak87.cosmic-ext-classic-menu/v1; command -v cosmic-ext-classic-menu-applet && grep -q "^Some((\[\"com.championpeak87.cosmic-ext-classic-menu\"" $P/plugins_wings && ! grep -q CosmicPanelAppButton $P/plugins_wings && grep -qx Retro $M/menu_layout && grep -qx Places $M/side_panel'
check "menu iniciar: ícone próprio do Tamagi, sem texto" bash -c 'M=/usr/share/cosmic/com.championpeak87.cosmic-ext-classic-menu/v1; f=$(tr -d "\"" < $M/button_icon); [[ $f == /usr/share/tamagi/menu/tamagi-start-symbolic.svg && -s $f ]] && grep -qx IconOnly $M/applet_button_style && rsvg-convert "$f" -o /tmp/menu-icon.png && [[ -s /tmp/menu-icon.png ]]'
check "Arquivos: grade, pastas primeiro" bash -c 'f=/usr/share/cosmic/com.system76.CosmicFiles/v1/tab; grep -q "view: Grid" $f && grep -q "folders_first: true" $f'
check "menu iniciar 1.1.0: menu focado (já abre com o teclado)" bash -c 'grep -qx true /usr/share/cosmic/com.championpeak87.cosmic-ext-classic-menu/v1/focused_menu && grep -aq focused_menu /usr/bin/cosmic-ext-classic-menu-applet && [[ -x /usr/bin/cosmic-ext-classic-menu-settings ]]'
check "terminal padrão: foot (Super+Enter e xdg-terminal-exec)" bash -c 'grep -q "Terminal: \"foot\"" /usr/share/cosmic/com.system76.CosmicSettings.Shortcuts/v1/system_actions && [[ $(head -1 /etc/xdg/xdg-terminals.list) == foot.desktop ]] && grep -q com.system76.CosmicTerm.desktop /etc/xdg/xdg-terminals.list'
check "foot: config do sistema válida, com o tema padrão" bash -c 'foot --check-config && grep -q "^background=1a1b26" /etc/xdg/foot/foot.ini && grep -q "JetBrains Mono" /etc/xdg/foot/foot.ini'
check "tamagi-theme grava o foot.ini do usuário e respeita o personalizado" bash -c 'tamagi-theme set nord >/dev/null && f=$HOME/.config/foot/foot.ini && foot --config "$f" --check-config && grep -q "Gerado por tamagi-theme (Nord)" "$f" && sed -i 1d "$f" && tamagi-theme set gruvbox >/dev/null && ! grep -q Gruvbox "$f"; r=$?; rm -f "$f"; tamagi-theme set tokyo-night >/dev/null; exit $r'
check "sandbox (Landlock): leitura não lê segredo, não grava, não conecta" bash -c 'S=/usr/libexec/tamagi/sandbox; mkdir -p $HOME/.ssh $HOME/Documents; echo CHAVE > $HOME/.ssh/id_test; echo ok > $HOME/Documents/a; $S -- cat $HOME/Documents/a | grep -qx ok && ! $S -- cat $HOME/.ssh/id_test && ! $S -- touch $HOME/Documents/b && $S -- python3 -c "
import socket, sys
try: socket.create_connection((\"127.0.0.1\", 9), 2)
except PermissionError: sys.exit(0)
except OSError: sys.exit(1)
sys.exit(1)"'
check "sandbox (Landlock): usuario grava em subpasta mas não mexe em segredo" bash -c 'S=/usr/libexec/tamagi/sandbox; mkdir -p $HOME/.ssh $HOME/Documents; echo CHAVE > $HOME/.ssh/id_test; $S --mode usuario -- sh -c "echo x > $HOME/Documents/c" && $S --mode usuario -- cat $HOME/.ssh/id_test | grep -qx CHAVE && ! $S --mode usuario -- rm -f $HOME/.ssh/id_test && ! $S --mode usuario -- mv $HOME/.ssh/id_test $HOME/Documents/x && [[ -f $HOME/.ssh/id_test ]]'
check "agente: comandos de leitura passam pelo sandbox" bash -c 'grep -q "sandbox = \"leitura\"" /usr/bin/tamagi-agent && grep -q /usr/libexec/tamagi/sandbox /usr/bin/tamagi-agent'
check "snapshots da pasta pessoal: snapper, serviço e timers" bash -c 'systemctl is-enabled snapper-timeline.timer && systemctl is-enabled snapper-cleanup.timer && systemctl is-enabled tamagi-home-snapshots.service && bash -n /usr/libexec/tamagi/home-snapshots && grep -q -- --btrfs-subvolume-home /usr/libexec/tamagi/firstboot'
check "tamagi-snap: estado sem snapshots e recusa caminho fora da pasta pessoal" bash -c 'tamagi-snap | grep -qiE "desligad|off" && ! tamagi-snap versoes /etc/passwd 2>/dev/null'
check "Windows: Bottles no primeiro boot e .exe/.msi abrem nele" bash -c 'grep -q "^flathub com.usebottles.bottles" /usr/share/tamagi/flatpaks.list && grep -q "^application/x-ms-dos-executable=com.usebottles.bottles.desktop" /etc/xdg/mimeapps.list && grep -q "^application/x-msi=com.usebottles.bottles.desktop" /etc/xdg/mimeapps.list'
check "Windows: tamagi-windows fixa a imagem do dockur e não instala nada sozinho" bash -c 'grep -q "^IMAGE=\"docker.io/dockurr/windows:[0-9]" /usr/bin/tamagi-windows && tamagi-windows status | grep -qiE "não instalado|not installed" && [[ ! -e $HOME/.config/containers/systemd/tamagi-windows.container ]]'
check "tamagi-db: lista vazia e recusa banco desconhecido" bash -c 'tamagi-db list | grep -qiE "nenhum|no database" && ! tamagi-db add oracle 2>/dev/null'
check "tamagi-lembrete: recusa tempo inválido" bash -c '! tamagi-lembrete amanhã x 2>/dev/null && tamagi-lembrete --help | grep -q 10m'
check "tamagi-theme também tema o btop" bash -c 'tamagi-theme set nord >/dev/null && grep -q "Gerado por tamagi-theme (Nord)" $HOME/.config/btop/themes/tamagi.theme && grep -qx "color_theme = \"tamagi\"" $HOME/.config/btop/btop.conf; r=$?; tamagi-theme set tokyo-night >/dev/null; exit $r'
check "OCR: Tesseract lê português de uma imagem" bash -c 'printf "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"900\" height=\"120\"><rect width=\"100%%\" height=\"100%%\" fill=\"white\"/><text x=\"20\" y=\"75\" font-family=\"Noto Sans\" font-size=\"48\">Atualização concluída</text></svg>" > /tmp/ocr.svg && rsvg-convert /tmp/ocr.svg -o /tmp/ocr.png && tesseract /tmp/ocr.png - -l por 2>/dev/null | grep -qi "concluída"'
check "tamagi-capture: ajuda e menu sem tela" bash -c 'tamagi-capture --help | grep -q OCR && tamagi-capture </dev/null | grep -q "tamagi-capture texto"'
check "atalhos do Omarchy: Shift/Alt+Print, Super+Shift+G/D/O/M/W, Super+Ctrl+E/R" bash -c 'D=/usr/share/cosmic/com.system76.CosmicSettings.Shortcuts/v1/defaults; grep -q "(modifiers: \[Shift\], key: \"Print\"): Spawn(\"tamagi-capture texto\")" $D && grep -q "(modifiers: \[Alt\], key: \"Print\"): Spawn(\"tamagi-capture gravar\")" $D && for k in g d o m w; do grep -q "(modifiers: \[Super, Shift\], key: \"$k\"): Spawn" $D || exit 1; done && grep -q "(modifiers: \[Super, Ctrl\], key: \"e\"): Spawn(\"/usr/libexec/tamagi/emoji\")" $D && [[ $(grep -c "key: \"Print\")" $D) -ge 3 ]]'
check "tamagi-theme: cores do Neovim (LazyVim) em todos os temas" bash -c 'for th in $(tamagi-theme list); do tamagi-theme nvim $th > /tmp/n.lua && nvim --headless -l /tmp/n.lua || exit 1; done; tamagi-theme nvim ipe | grep -q base16 && mkdir -p $HOME/.config/nvim/lua/plugins && tamagi-theme set kanagawa >/dev/null && grep -q "colorscheme = \"kanagawa\"" $HOME/.config/nvim/lua/plugins/tamagi-theme.lua; r=$?; rm -rf $HOME/.config/nvim; tamagi-theme set tokyo-night >/dev/null; exit $r'
check "tamagi-dev: presets só com binário pronto" bash -c 'tamagi-dev list | grep -q "^  ruby " && tamagi-dev list | grep -q "^  dotnet " && ! tamagi-dev add cobol 2>/dev/null'
check "tamagi-font: troca a fonte do foot e do COSMIC" bash -c 'tamagi-font list | grep -qx "JetBrains Mono" && GSETTINGS_BACKEND=memory tamagi-font set "Cascadia Code NF" >/dev/null && grep -q "^font=Cascadia Code NF:size=11" $HOME/.config/foot/foot.ini && grep -q "Cascadia Code NF" $HOME/.config/cosmic/com.system76.CosmicTk/v1/monospace_font && GSETTINGS_BACKEND=memory tamagi-font reset >/dev/null && grep -q "^font=JetBrains Mono" $HOME/.config/foot/foot.ini'
check "tamagi-menu: residente pelo autostart" bash -c 'desktop-file-validate /etc/xdg/autostart/tamagi-menu-service.desktop && grep -q "^Exec=tamagi-menu --service" /etc/xdg/autostart/tamagi-menu-service.desktop && env -u WAYLAND_DISPLAY -u DISPLAY tamagi-menu --service'
check "barra: camada e medidas certas, Dock intacto" bash -c 'P=/usr/share/cosmic/com.system76.CosmicPanel.Panel/v1; grep -qx Top $P/layer && grep -qx 4 $P/margin && grep -qx 2 $P/padding && grep -qx 4 $P/spacing && ! rpm -V cosmic-panel | grep -q "CosmicPanel\.Dock/"'
check "terminal e editor usam JetBrains Mono" bash -c 'for a in CosmicTerm CosmicEdit; do grep -qx "\"JetBrains Mono\"" /usr/share/cosmic/com.system76.$a/v1/font_name || exit 1; done'
check "agente de IA responde no idioma do sistema" bash -c 'grep -q "Responda sempre em" /usr/bin/tamagi-agent && ! grep -q "português do Brasil," /usr/share/tamagi/agent/system.md'
check "tamagi-panel troca o estilo da barra" bash -c 'tamagi-panel set cosmic >/dev/null && grep -q Dock ~/.config/cosmic/com.system76.CosmicPanel/v1/entries && grep -qx Top ~/.config/cosmic/com.system76.CosmicPanel.Panel/v1/anchor && tamagi-panel set classico >/dev/null && grep -qx None ~/.config/cosmic/com.system76.CosmicPanel.Panel/v1/plugins_center && tamagi-panel set centro >/dev/null && [[ $(tamagi-panel current) == centro ]] && ! tamagi-panel set nada 2>/dev/null'
check "Claude Code na imagem e no menu"   bash -c 'HOME=/tmp claude --version >/dev/null && desktop-file-validate /usr/share/applications/claude-code.desktop && grep -q claude-code /usr/share/cosmic/com.system76.CosmicAppList/v1/favorites'
check "distrobox.ini presente"            test -s /etc/distrobox/distrobox.ini
check "tmpfiles cria /var/log/tuned (rótulo SELinux do tuned-ppd)" bash -c 'systemd-tmpfiles --cat-config 2>/dev/null | grep -qE "^d /var/log/tuned "'
check "mcelog é pulado fora de processadores Intel" bash -c 'systemctl cat mcelog.service | grep -q "ExecCondition=.*GenuineIntel"'

section "Primeiro boot (o que o instalador tik deixa para o sistema novo)"
check "serviço habilitado"                systemctl is-enabled tamagi-firstboot.service
mkdir -p /tmp/fb /var/home
printf 'USER=maria\nFULLNAME=Maria da Silva\nPASSWORD_HASH=%s\nXKB_LAYOUT=us\nXKB_VARIANT=intl\n' "$(openssl passwd -6 segredo)" > /tmp/fb/install.conf
check "cria o usuário pedido no instalador" bash -c 'TAMAGI_INSTALL_CONF=/tmp/fb/install.conf /usr/libexec/tamagi/firstboot && id -nG maria | grep -qw wheel'
check "nome completo e senha gravados"    bash -c 'getent passwd maria | grep -q "Maria da Silva" && getent shadow maria | cut -d: -f2 | grep -q "^[$]6[$]"'
check "teclado escolhido vale para o usuário" bash -c 'grep -q "layout: \"us\"" ~maria/.config/cosmic/com.system76.CosmicComp/v1/xkb_config && grep -q "variant: \"intl\"" ~maria/.config/cosmic/com.system76.CosmicComp/v1/xkb_config && [[ $(stat -c %U ~maria/.config/cosmic/com.system76.CosmicComp/v1/xkb_config) == maria ]]'
check "arquivo de respostas é apagado"    test ! -e /tmp/fb/install.conf
mkdir -p /tmp/fb; printf 'USER=Root; rm -rf /\n' > /tmp/fb/install.conf
check "recusa nome de usuário inválido"   bash -c '! TAMAGI_INSTALL_CONF=/tmp/fb/install.conf /usr/libexec/tamagi/firstboot 2>/dev/null'


echo
echo "Resultado: $pass ok, $fail falha(s)"
exit $(( fail > 0 ))
