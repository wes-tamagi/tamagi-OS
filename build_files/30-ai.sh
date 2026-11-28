#!/usr/bin/bash
# IA local: confere o llama.cpp vindo do estágio de compilação (apenas Tamagi-DxAI).

if [[ ${TAMAGI_EDITION:-dxai} == skl || ${TAMAGI_EDITION:-dxai} == skl-os ]]; then
    echo "${TAMAGI_EDITION}: sem llama.cpp local (edição escolar enxuta)"
    exit 0
fi

ldconfig
llama-server --version
[[ -f /usr/lib64/ggml/libggml-vulkan.so ]] || { echo "backend Vulkan do llama.cpp ausente" >&2; exit 1; }
