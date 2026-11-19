#!/usr/bin/bash
# Agentes de IA de terceiros embutidos: Claude Code, Qwen Code e Antigravity.
# Vivem em /usr (somente-leitura): atualizam junto com a imagem, não sozinhos.

tmp=$(mktemp -d)
cd "$tmp" || exit 1

# ── Claude Code: binário nativo oficial, com o checksum do manifesto ──
base=https://downloads.claude.ai/claude-code-releases
version=$CLAUDE_CODE_VERSION
[[ $version == latest ]] && version=$(curl -fsSL "$base/latest")
checksum=$(curl -fsSL "$base/$version/manifest.json" | jq -r '.platforms["linux-x64"].checksum')
[[ $checksum =~ ^[0-9a-f]{64}$ ]] || { echo "manifesto do Claude Code sem checksum" >&2; exit 1; }
curl -fsSL -o claude "$base/$version/linux-x64/claude"
echo "$checksum  claude" | sha256sum -c -
install -Dm755 claude /usr/bin/claude

# ── Qwen Code: pacote npm oficial (precisa de Node.js >= 22 no host) ──
dnf5 -y install --setopt=install_weak_deps=False nodejs nodejs-npm
HOME="$tmp" npm install --global --prefix /usr --no-fund --no-audit --loglevel=error \
    "@qwen-code/qwen-code@${QWEN_CODE_VERSION}"

# ── Antigravity: IDE agêntica do Google, do repositório RPM oficial ──
# O Google ainda não assina esse repositório (gpgcheck=0 é a instrução deles);
# a integridade depende só do HTTPS.
cat > /etc/yum.repos.d/antigravity.repo <<'REPO'
[antigravity-rpm]
name=Antigravity RPM Repository
baseurl=https://us-central1-yum.pkg.dev/projects/antigravity-auto-updater-dev/antigravity-rpm
enabled=1
gpgcheck=0
REPO
dnf5 -y install --setopt=install_weak_deps=False antigravity
sed -i 's/^enabled=1/enabled=0/' /etc/yum.repos.d/antigravity.repo

HOME="$tmp" claude --version
HOME="$tmp" qwen --version
rpm -q antigravity
cd /
rm -rf "${tmp:?}"
