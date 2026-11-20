# vim: set ft=make :
# Receitas do Tamagi (aparecem em `ujust --list`).

# IA local (llama.cpp): setup, presets, pull, list, use, status, chat...
ai *ARGS:
    @tamagi-ai {{ ARGS }}

# Conversar com o agente de IA local
agent *ARGS:
    @tamagi-agent {{ ARGS }}

# Trocar o tema do desktop (sem argumento abre a lista)
theme NAME="":
    #!/usr/bin/bash
    if [[ -n "{{ NAME }}" ]]; then tamagi-theme set "{{ NAME }}"; else tamagi-theme pick; fi

# Ligar ou desligar o tiling automático de janelas (on/off)
tiling ACTION="toggle":
    #!/usr/bin/bash
    set -euo pipefail
    file="${XDG_CONFIG_HOME:-$HOME/.config}/cosmic/com.system76.CosmicComp/v1/autotile"
    now=$(cat "$file" 2>/dev/null || cat /usr/share/cosmic/com.system76.CosmicComp/v1/autotile)
    case "{{ ACTION }}" in
        on)  new=true ;;
        off) new=false ;;
        *)   [[ $now == true ]] && new=false || new=true ;;
    esac
    mkdir -p "$(dirname "$file")"
    echo "$new" > "$file"
    echo "Tiling: $new"

# Navegadores na caixa Ubuntu 26.04 LTS: setup, install <navegador>, status, default, update
browser *ARGS:
    @tamagi-browser {{ ARGS }}

# Segurança: status, strict, balanced, usb on|off, malloc on|off
harden *ARGS:
    @tamagi-harden {{ ARGS }}

# Jogos sob demanda: Steam, Lutris, Heroic, ProtonUp-Qt e MangoHud por Flatpak
setup-gaming:
    #!/usr/bin/bash
    set -euo pipefail
    echo "gamescope, MangoHud, GameMode e as regras de controles já vêm na imagem."
    apps=$(gum choose --no-limit --header "O que instalar? (espaço marca, Enter confirma)" \
        --selected "Steam" "Steam" "Lutris" "Heroic (Epic/GOG)" "ProtonUp-Qt" "Bottles")
    ids=()
    while read -r app; do
        case "$app" in
            Steam)        ids+=(com.valvesoftware.Steam) ;;
            Lutris)       ids+=(net.lutris.Lutris) ;;
            Heroic*)      ids+=(com.heroicgameslauncher.hgl) ;;
            ProtonUp-Qt)  ids+=(net.davidotek.pupgui2) ;;
            Bottles)      ids+=(com.usebottles.bottles) ;;
        esac
    done <<<"$apps"
    (( ${#ids[@]} )) || { echo "Nada escolhido."; exit 0; }
    flatpak install -y flathub "${ids[@]}"
    echo "Pronto. MangoHud dentro do Steam Flatpak: flatpak install flathub org.freedesktop.Platform.VulkanLayer.MangoHud"

# Criar uma caixa de desenvolvimento distrobox (fedora, arch, ubuntu, opensuse)
devbox DISTRO="":
    #!/usr/bin/bash
    set -euo pipefail
    declare -A IMAGES=(
        [fedora]=ghcr.io/ublue-os/fedora-toolbox:latest
        [arch]=ghcr.io/ublue-os/arch-toolbox:latest
        [ubuntu]=ghcr.io/ublue-os/ubuntu-toolbox:latest
        [opensuse]=quay.io/toolbx-images/opensuse-toolbox:tumbleweed
    )
    distro="{{ DISTRO }}"
    [[ -n $distro ]] || distro=$(gum choose --header "Distro da caixa" "${!IMAGES[@]}")
    [[ -n ${IMAGES[$distro]:-} ]] || { echo "distro desconhecida: $distro (use: ${!IMAGES[*]})" >&2; exit 1; }
    # A primeira caixa se chama 'dev'; as seguintes levam o nome da distro.
    name=dev
    if distrobox list --no-color 2>/dev/null | awk -F'|' 'NR>1 {gsub(/ /, "", $2); print $2}' | grep -qx dev; then
        name=$distro
    fi
    distrobox create --yes --name "$name" --image "${IMAGES[$distro]}"
    echo "Caixa '$name' criada. Entre com: distrobox enter $name"

# Instalar outros agentes de IA no seu usuário (codex, gemini); Claude Code e opencode já vêm na imagem
install-agent AGENT="":
    #!/usr/bin/bash
    set -euo pipefail
    agent="{{ AGENT }}"
    [[ -n $agent ]] || agent=$(gum choose --header "Qual agente?" codex gemini)
    case "$agent" in
        codex)  mise use -g node@lts "npm:@openai/codex" ;;
        gemini) mise use -g node@lts "npm:@google/gemini-cli" ;;
        *) echo "agente desconhecido: $agent (use: codex, gemini)" >&2; exit 1 ;;
    esac
    echo "Instalado. Abra um terminal novo e rode: $agent"

# Atualizar tudo: imagem do sistema, Flatpaks, navegadores, subsistemas deb/rpm/Arch (com AUR) e mise
update *ARGS:
    @tamagi-update {{ ARGS }}

# Atualização automática: status, on ou off (sistema, Flatpaks, subsistemas, mise)
autoupdate ACTION="status":
    #!/usr/bin/bash
    case "{{ ACTION }}" in
        on|off) tamagi-update auto "{{ ACTION }}" ;;
        *)      tamagi-update status ;;
    esac

# O sistema num arquivo: init, export, diff, apply (~/.config/tamagi/tamagi.toml)
sistema *ARGS:
    @tamagi {{ ARGS }}

# Verificação de saúde do boot e relatórios de atualizações desfeitas
boot-report *ARGS:
    @tamagi-boot-report {{ ARGS }}

# Máquinas virtuais: abre o virt-manager (KVM nativo, com UEFI, TPM e virtiofs)
vms:
    @setsid -f virt-manager >/dev/null 2>&1

# Atualizar só os pacotes do Arch (pacman + AUR)
update-arch:
    @tamagi-update --arch

# Ligar o Tailscale (VPN mesh) neste computador
setup-tailscale:
    sudo systemctl enable --now tailscaled
    sudo tailscale up

# Pacotes deb, rpm, Arch e Omarchy em subsistemas (ex.: ujust pkg install --omarchy hype)
pkg *ARGS:
    @tamagi-pkg {{ ARGS }}

# Transformar um site em app (ex.: ujust webapp install WhatsApp https://web.whatsapp.com)
webapp *ARGS:
    @tamagi-webapp {{ ARGS }}

# Instalar a configuração LazyVim no Neovim (como no Omarchy)
setup-lazyvim:
    #!/usr/bin/bash
    set -euo pipefail
    dir="${XDG_CONFIG_HOME:-$HOME/.config}/nvim"
    if [[ -e $dir ]]; then echo "Você já tem uma configuração em $dir; nada foi alterado."; exit 0; fi
    git clone --depth 1 https://github.com/LazyVim/starter "$dir"
    echo "LazyVim instalado. Abra o nvim: os plugins são baixados na primeira vez."

# Refazer a configuração inicial (tema, tiling, navegador, distrobox, IA, Android)
tamagi-setup:
    @tamagi-firstrun --force

# Mostrar os atalhos de teclado
keys:
    @tamagi-keys

# Voltar para a imagem anterior do sistema no próximo boot
rollback:
    sudo bootc rollback

# Passar a seguir outra imagem (ex.: ujust switch-image ghcr.io/voce/tamagi:latest)
switch-image REF:
    sudo bootc switch {{ REF }}
