#!/usr/bin/bash
# Instala os pacotes de packages.txt e as ferramentas sem pacote no Fedora.

# COPRs só para o build: o que vem deles está marcado em packages.txt.
#   bieszczaders/kernel-cachyos-addons  scx-scheds, scx-tools (sched_ext)
#   secureblue/packages                 hardened_malloc
COPRS=(bieszczaders/kernel-cachyos-addons secureblue/packages)
for copr in "${COPRS[@]}"; do dnf5 -y copr enable "$copr"; done

mapfile -t PACKAGES < <(sed -e 's/#.*//' -e 's/[[:space:]]//g' /ctx/build_files/packages.txt | grep -v '^$')
dnf5 -y install --setopt=install_weak_deps=False "${PACKAGES[@]}"

# A imagem não consulta COPR depois de pronta: tudo se atualiza pelo rebuild.
for copr in "${COPRS[@]}"; do dnf5 -y copr disable "$copr"; done

# Navegadores não vivem no host (tamagi-browser cria a caixa AnduinOS).
dnf5 -y remove firefox firefox-langpacks 2>/dev/null || true

# Ferramentas sem pacote no Fedora: binários oficiais, com checksum conferido.
tmp=$(mktemp -d)
cd "$tmp" || exit 1

curl -fsSLO "https://github.com/starship/starship/releases/download/${STARSHIP_VERSION}/starship-x86_64-unknown-linux-gnu.tar.gz"
curl -fsSLO "https://github.com/starship/starship/releases/download/${STARSHIP_VERSION}/starship-x86_64-unknown-linux-gnu.tar.gz.sha256"
echo "$(cat starship-x86_64-unknown-linux-gnu.tar.gz.sha256)  starship-x86_64-unknown-linux-gnu.tar.gz" | sha256sum -c -
tar -xzf starship-x86_64-unknown-linux-gnu.tar.gz starship
install -Dm755 starship /usr/bin/starship

curl -fsSLO "https://github.com/jdx/mise/releases/download/${MISE_VERSION}/mise-${MISE_VERSION}-linux-x64"
curl -fsSLO "https://github.com/jdx/mise/releases/download/${MISE_VERSION}/SHASUMS256.txt"
grep -E " (\./)?mise-${MISE_VERSION}-linux-x64\$" SHASUMS256.txt | sha256sum -c -
install -Dm755 "mise-${MISE_VERSION}-linux-x64" /usr/bin/mise

cd /
rm -rf "$tmp"

# ── Rust como padrão no shell interativo ──
# sudo-rs e uutils não substituem os originais em /usr/bin (serviços e scripts
# do sistema continuam com os de sempre). Este diretório entra na frente do
# PATH só em shells interativos; quem não quiser:
#   touch ~/.config/tamagi/no-rust-defaults
RUST_BIN=/usr/libexec/tamagi/rust-bin
mkdir -p "$RUST_BIN"
ln -sf /usr/bin/sudo-rs "$RUST_BIN/sudo"
ln -sf /usr/bin/su-rs "$RUST_BIN/su"
ln -sf /usr/bin/visudo-rs "$RUST_BIN/visudo"
count=0
for uu in /usr/bin/uu_*; do
    [[ -x $uu ]] || continue
    name=$(basename "$uu"); name=${name#uu_}
    # Só substitui o que o host já tem com o mesmo nome (não inventa comandos).
    [[ -x /usr/bin/$name ]] || continue
    ln -sf "$uu" "$RUST_BIN/$name"
    count=$((count + 1))
done
(( count > 50 )) || { echo "uutils: esperava dezenas de utilitários, achei $count" >&2; exit 1; }
"$RUST_BIN/ls" --version | grep -qi uutils || { echo "ls em $RUST_BIN não é o do uutils" >&2; exit 1; }
"$RUST_BIN/sudo" --version | grep -qi 'sudo-rs' || { echo "sudo em $RUST_BIN não é o sudo-rs" >&2; exit 1; }
echo "Rust no shell interativo: sudo-rs e $count utilitários do uutils"
