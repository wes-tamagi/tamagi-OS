#!/usr/bin/bash
# IA local: confere o llama.cpp vindo do estágio de compilação e instala o
# opencode (agente de programação) apontável para o servidor local.

ldconfig
llama-server --version
[[ -f /usr/lib64/ggml/libggml-vulkan.so ]] || { echo "backend Vulkan do llama.cpp ausente" >&2; exit 1; }

tmp=$(mktemp -d)
cd "$tmp" || exit 1
# Variante "baseline": roda também em CPUs sem AVX2.
curl -fsSLO "https://github.com/anomalyco/opencode/releases/download/${OPENCODE_VERSION}/opencode-linux-x64-baseline.tar.gz"
echo "${OPENCODE_SHA256}  opencode-linux-x64-baseline.tar.gz" | sha256sum -c -
tar -xzf opencode-linux-x64-baseline.tar.gz
install -Dm755 opencode /usr/bin/opencode
# /root não existe durante o build (é /var/roothome); o opencode precisa de um HOME.
HOME="$tmp" opencode --version
cd /
rm -rf "$tmp"
