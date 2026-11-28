#!/usr/bin/bash
# Instala os pacotes da edição (dxai ou skl), Tailscale (repo oficial assinado)
# e binários oficiais verificados por SHA-256 (starship, bottom, Roboto Flex).

EDITION=${TAMAGI_EDITION:-dxai}
case "$EDITION" in
    dxai)       PKG_FILE=/ctx/build_files/packages.txt ;;
    skl|skl-os) PKG_FILE=/ctx/build_files/packages-skl.txt ;;
    *)          echo "TAMAGI_EDITION inválido: $EDITION (use dxai, skl ou skl-os)" >&2; exit 1 ;;
esac

mapfile -t PACKAGES < <(sed -e 's/#.*//' -e 's/[[:space:]]//g' "$PKG_FILE" | grep -v '^$')
dnf5 -y install --setopt=install_weak_deps=False --allowerasing "${PACKAGES[@]}"

# Subpacotes opcionais de variantes escura/clara do Papirus (quando separados no Fedora).
dnf5 -y install --setopt=install_weak_deps=False papirus-icon-theme-dark papirus-icon-theme-light 2>/dev/null || true

# Codecs multimídia completos via RPM Fusion Free (+ Steam nativo em DxAI via
# RPM Fusion Nonfree; ambos desabilitados após o build para determinismo em runtime).
rpmfusion_rpms=("https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-${FEDORA_VERSION}.noarch.rpm")
if [[ $EDITION == dxai ]]; then
    rpmfusion_rpms+=("https://mirrors.rpmfusion.org/nonfree/fedora/rpmfusion-nonfree-release-${FEDORA_VERSION}.noarch.rpm")
fi
dnf5 -y install --setopt=install_weak_deps=False "${rpmfusion_rpms[@]}"
dnf5 -y install --setopt=install_weak_deps=False --allowerasing \
    ffmpeg mesa-va-drivers-freeworld gstreamer1-plugins-bad-free gstreamer1-plugins-ugly-free
if [[ $EDITION == dxai ]]; then
    # Steam nativo + bibliotecas 32-bit de áudio/rede/gamescope para compatibilidade
    # máxima com GE-Proton, gamescope e MangoHud (estilo Bazzite).
    dnf5 -y install --setopt=install_weak_deps=False --allowerasing \
        steam pipewire-alsa.i686 alsa-plugins-pulseaudio.i686
    dnf5 -y install --setopt=install_weak_deps=False gamescope-libs gamescope-libs.i686 2>/dev/null || true
fi
for r in /etc/yum.repos.d/*rpmfusion*.repo /usr/lib/yum.repos.d/*rpmfusion*.repo; do
    [[ -f $r ]] && sed -i 's/^enabled=1/enabled=0/' "$r"
done

# 100% Flathub: remove quaisquer remotes Fedora Flatpak e garante o remote oficial do Flathub.
rm -f /etc/flatpak/remotes.d/fedora*.flatpakrepo
install -d /etc/flatpak/remotes.d
curl -fsSL -o /etc/flatpak/remotes.d/flathub.flatpakrepo https://dl.flathub.org/repo/flathub.flatpakrepo

# Tailscale pré-disponível (repositório oficial assinado; desabilitado após o
# build para a imagem não consultar repos externos em runtime; serviço desligado
# por padrão até o usuário ativar).
curl -fsSL -o /etc/yum.repos.d/tailscale.repo https://pkgs.tailscale.com/stable/fedora/tailscale.repo
dnf5 -y install --setopt=install_weak_deps=False tailscale
sed -i 's/^enabled=1/enabled=0/' /etc/yum.repos.d/tailscale.repo
command -v tailscale >/dev/null || { echo "tailscale não instalado" >&2; exit 1; }

# distrobox 1.8.2.5 x podman 6: corrige opção ":ro:rslave" -> ":ro,rslave".
DBX=/usr/bin/distrobox-create
if [[ -f $DBX ]]; then
    if grep -qF ':ro${rslave}"' "$DBX"; then
        sed -i 's/:ro${rslave}"/:ro,rslave"/' "$DBX"
        echo "distrobox-create: volume somente-leitura corrigido (:ro,rslave)"
    fi
    if grep -qE ':ro\$\{rslave\}|:ro:rslave' "$DBX"; then
        echo "distrobox-create ainda gera ':ro:rslave' (o podman 6 recusa)" >&2; exit 1
    fi
    sh -n "$DBX"
fi

# Navegador padrão é Chromium nativo; Firefox sai da imagem.
dnf5 -y remove firefox firefox-langpacks 2>/dev/null || true
# Player de vídeo do COSMIC é opcional (`tamagi install cosmic-player`);
# terminais do sistema são Ptyxis + foot leve (sem cosmic-term; usa rpm -e --nodeps
# porque cosmic-session declara Requires: cosmic-term mesmo com Terminal trocado).
dnf5 -y remove --no-autoremove cosmic-player 2>/dev/null || true
rpm -e --nodeps cosmic-term 2>/dev/null || true
rpm -q cosmic-session cosmic-comp cosmic-files >/dev/null || { echo "remover cosmic-player/cosmic-term levou parte do COSMIC junto" >&2; exit 1; }
if command -v cosmic-term >/dev/null 2>&1; then echo "cosmic-term ainda está na imagem" >&2; exit 1; fi
# Um comando só: `tamagi` (sem ujust).
dnf5 -y remove --no-autoremove ublue-os-just 2>/dev/null || true
if command -v ujust >/dev/null 2>&1; then echo "ujust ainda está na imagem" >&2; exit 1; fi

# Edição Tamagi-SKL / SKL (Escola): sem loja gráfica (Flatpaks só via terminal),
# sem llama.cpp, sem chromedriver, sem ferramentas de IA/dev pesado.
if [[ $EDITION == skl || $EDITION == skl-os ]]; then
    dnf5 -y remove --no-autoremove cosmic-store 2>/dev/null || true
    rpm -q cosmic-session cosmic-comp cosmic-files >/dev/null || { echo "remover cosmic-store levou parte do COSMIC junto" >&2; exit 1; }
    rm -rf /usr/bin/llama-* /usr/lib64/ggml /usr/lib64/libllama* /usr/lib64/libggml* \
           /usr/share/licenses/llama.cpp /usr/bin/claude /usr/bin/opencode \
           /usr/lib/systemd/user/llama-server.service
    echo "$EDITION: cosmic-store, llama.cpp e extras de IA removidos da imagem"
else
    command -v gnome-disks >/dev/null || { echo "gnome-disk-utility ausente no Tamagi-DxAI" >&2; exit 1; }
    command -v rclone >/dev/null || { echo "rclone ausente no Tamagi-DxAI" >&2; exit 1; }
    command -v steam >/dev/null || { echo "steam nativo ausente no Tamagi-DxAI" >&2; exit 1; }
fi

# Ferramentas sem pacote no Fedora: binários oficiais com SHA-256 conferido.
tmp=$(mktemp -d)
cd "$tmp" || exit 1

curl -fsSLO "https://github.com/starship/starship/releases/download/${STARSHIP_VERSION}/starship-x86_64-unknown-linux-gnu.tar.gz"
curl -fsSLO "https://github.com/starship/starship/releases/download/${STARSHIP_VERSION}/starship-x86_64-unknown-linux-gnu.tar.gz.sha256"
echo "$(cat starship-x86_64-unknown-linux-gnu.tar.gz.sha256)  starship-x86_64-unknown-linux-gnu.tar.gz" | sha256sum -c -
tar -xzf starship-x86_64-unknown-linux-gnu.tar.gz starship
install -Dm755 starship /usr/bin/starship

# bottom (btm): monitor de sistema TUI em Rust (substitui btop).
curl -fsSLO "https://github.com/ClementTsang/bottom/releases/download/${BOTTOM_VERSION}/bottom_x86_64-unknown-linux-gnu.tar.gz"
echo "${BOTTOM_SHA256}  bottom_x86_64-unknown-linux-gnu.tar.gz" | sha256sum -c -
tar -xzf bottom_x86_64-unknown-linux-gnu.tar.gz btm
install -Dm755 btm /usr/bin/btm

# mise (somente Tamagi-DxAI): gerenciador de runtimes em ~/.local/share/mise
# (sem root e sem tocar em /usr) para agentes Node/Python, MCPs e projetos.
if [[ $EDITION != skl && $EDITION != skl-os ]]; then
    curl -fsSLO "https://github.com/jdx/mise/releases/download/${MISE_VERSION}/mise-${MISE_VERSION}-linux-x64"
    curl -fsSLO "https://github.com/jdx/mise/releases/download/${MISE_VERSION}/SHASUMS256.txt"
    grep -E " (\./)?mise-${MISE_VERSION}-linux-x64\$" SHASUMS256.txt | sha256sum -c -
    install -Dm755 "mise-${MISE_VERSION}-linux-x64" /usr/bin/mise
fi

# Roboto Flex (fonte da interface Material 3; commit pinado + SHA-256; OFL-1.1).
ROBOTO_FLEX_COMMIT=739e06dc46ebb14cddd88b9768a6c1504d4677f6
ROBOTO_FLEX_URL="https://raw.githubusercontent.com/googlefonts/roboto-flex/$ROBOTO_FLEX_COMMIT"
curl -fsSL -o RobotoFlex.ttf "$ROBOTO_FLEX_URL/fonts/RobotoFlex%5BGRAD,XOPQ,XTRA,YOPQ,YTAS,YTDE,YTFI,YTLC,YTUC,opsz,slnt,wdth,wght%5D.ttf"
curl -fsSL -o OFL.txt "$ROBOTO_FLEX_URL/OFL.txt"
sha256sum -c - <<'SUMS'
94a7ea95ccee28c54885a507e3cc0a534ce41ec61d413935df0e07261a7ffe63  RobotoFlex.ttf
454ec26838f2b8f2e2588e759dd3742577e0d305d3359a7b81c86641fcab7d21  OFL.txt
SUMS
install -Dm644 RobotoFlex.ttf /usr/share/fonts/roboto-flex/RobotoFlex.ttf
install -Dm644 OFL.txt /usr/share/licenses/roboto-flex/OFL.txt

cd /
rm -rf "$tmp"
