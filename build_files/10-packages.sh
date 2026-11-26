#!/usr/bin/bash
# Instala os pacotes da edição (dxai ou skl), Tailscale (repo oficial assinado)
# e binários oficiais verificados por SHA-256 (starship, bottom, Roboto Flex).

EDITION=${TAMAGI_EDITION:-dxai}
case "$EDITION" in
    dxai) PKG_FILE=/ctx/build_files/packages.txt ;;
    skl)  PKG_FILE=/ctx/build_files/packages-skl.txt ;;
    *)    echo "TAMAGI_EDITION inválido: $EDITION (use dxai ou skl)" >&2; exit 1 ;;
esac

mapfile -t PACKAGES < <(sed -e 's/#.*//' -e 's/[[:space:]]//g' "$PKG_FILE" | grep -v '^$')
dnf5 -y install --setopt=install_weak_deps=False "${PACKAGES[@]}"

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
# Player de vídeo do COSMIC é opcional (`tamagi install cosmic-player`).
dnf5 -y remove --no-autoremove cosmic-player
rpm -q cosmic-session cosmic-comp cosmic-files >/dev/null || { echo "remover cosmic-player levou parte do COSMIC junto" >&2; exit 1; }
# Um comando só: `tamagi` (sem ujust).
dnf5 -y remove --no-autoremove ublue-os-just
if command -v ujust >/dev/null 2>&1; then echo "ujust ainda está na imagem" >&2; exit 1; fi

# Edição Tamagi-SKL (Escola): sem loja gráfica (Flatpaks só via terminal),
# sem llama.cpp, sem chromedriver, sem ferramentas de IA/dev pesado.
if [[ $EDITION == skl ]]; then
    dnf5 -y remove --no-autoremove cosmic-store 2>/dev/null || true
    rpm -q cosmic-session cosmic-comp cosmic-files >/dev/null || { echo "remover cosmic-store levou parte do COSMIC junto" >&2; exit 1; }
    rm -rf /usr/bin/llama-* /usr/lib64/ggml /usr/lib64/libllama* /usr/lib64/libggml* \
           /usr/share/licenses/llama.cpp /usr/bin/claude \
           /usr/lib/systemd/user/llama-server.service
    echo "Tamagi-SKL: cosmic-store, llama.cpp e extras de IA removidos da imagem"
else
    command -v gnome-disks >/dev/null || { echo "gnome-disk-utility ausente no Tamagi-DxAI" >&2; exit 1; }
    command -v rclone >/dev/null || { echo "rclone ausente no Tamagi-DxAI" >&2; exit 1; }
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
if [[ $EDITION != skl ]]; then
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
