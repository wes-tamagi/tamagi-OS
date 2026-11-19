#!/usr/bin/bash
# Agente de IA de terceiros embutido: Claude Code.
# Vive em /usr (somente-leitura): atualiza junto com a imagem, não sozinho.

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

HOME="$tmp" claude --version
cd /
rm -rf "${tmp:?}"
