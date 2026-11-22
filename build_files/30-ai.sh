#!/usr/bin/bash
# IA local: confere o llama.cpp vindo do estágio de compilação. Os agentes
# (opencode, Claude Code, Codex, Qwen Code...) não vêm na imagem: são
# opcionais, instalados no usuário por `tamagi agents install`.

ldconfig
llama-server --version
[[ -f /usr/lib64/ggml/libggml-vulkan.so ]] || { echo "backend Vulkan do llama.cpp ausente" >&2; exit 1; }
