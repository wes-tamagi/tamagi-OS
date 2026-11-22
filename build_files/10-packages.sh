#!/usr/bin/bash
# Instala os pacotes de packages.txt e as ferramentas sem pacote no Fedora.

# COPRs só para o build: o que vem deles está marcado em packages.txt.
#   secureblue/packages                        hardened_malloc
COPRS=(secureblue/packages)
for copr in "${COPRS[@]}"; do dnf5 -y copr enable "$copr"; done

mapfile -t PACKAGES < <(sed -e 's/#.*//' -e 's/[[:space:]]//g' /ctx/build_files/packages.txt | grep -v '^$')
dnf5 -y install --setopt=install_weak_deps=False "${PACKAGES[@]}"

# A imagem não consulta COPR depois de pronta: tudo se atualiza pelo rebuild.
for copr in "${COPRS[@]}"; do dnf5 -y copr disable "$copr"; done

# distrobox 1.8.2.5 x podman 6: em host somente-leitura (o caso do Tamagi) o
# distrobox-create monta cada pasta da raiz com a opção ":ro:rslave", e o
# podman 6 recusa ("incorrect volume format"): nenhuma caixa era criada. As
# opções de um volume se separam por vírgula. Conferido: se uma versão nova
# do distrobox mudar este trecho, o build avisa em vez de a correção sumir.
DBX=/usr/bin/distrobox-create
if grep -qF ':ro${rslave}"' "$DBX"; then
    sed -i 's/:ro${rslave}"/:ro,rslave"/' "$DBX"
    echo "distrobox-create: volume somente-leitura corrigido (:ro,rslave)"
fi
if grep -qE ':ro\$\{rslave\}|:ro:rslave' "$DBX"; then
    echo "distrobox-create ainda gera ':ro:rslave' (o podman 6 recusa)" >&2; exit 1
fi
sh -n "$DBX"

# Nenhum navegador vem na imagem: o usuário escolhe (Flatpak, nativo ou caixa)
# no primeiro login ou com `tamagi browser install`.
dnf5 -y remove firefox firefox-langpacks 2>/dev/null || true
# Um comando só: `tamagi`. O `ujust` da base sai; o que ele fazia de útil está
# em `tamagi setup` e `tamagi update`. O `just` fica: é ferramenta de
# desenvolvimento (o Justfile deste repositório, entre outros).
dnf5 -y remove --no-autoremove ublue-os-just
if command -v ujust >/dev/null 2>&1; then echo "ujust ainda está na imagem" >&2; exit 1; fi

# Ferramentas sem pacote no Fedora: binários oficiais, com checksum conferido.
tmp=$(mktemp -d)
cd "$tmp" || exit 1

curl -fsSLO "https://github.com/starship/starship/releases/download/${STARSHIP_VERSION}/starship-x86_64-unknown-linux-gnu.tar.gz"
curl -fsSLO "https://github.com/starship/starship/releases/download/${STARSHIP_VERSION}/starship-x86_64-unknown-linux-gnu.tar.gz.sha256"
echo "$(cat starship-x86_64-unknown-linux-gnu.tar.gz.sha256)  starship-x86_64-unknown-linux-gnu.tar.gz" | sha256sum -c -
tar -xzf starship-x86_64-unknown-linux-gnu.tar.gz starship
install -Dm755 starship /usr/bin/starship

# O mise (linguagens e ferramentas de desenvolvimento) não vive mais no host:
# é instalado dentro da caixa de desenvolvimento (devbox-provision), onde há
# compiladores e bibliotecas. O host imutável fica sem runtimes.

cd /
rm -rf "$tmp"
