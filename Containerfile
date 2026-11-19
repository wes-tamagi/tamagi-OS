# Tamagi — imagem bootc (Fedora atomic / Universal Blue) com o desktop COSMIC,
# distrobox, Waydroid e IA local (llama.cpp + agentes). Navegadores e Warsaw
# não entram aqui: vivem numa caixa distrobox (tamagi-browser).
#
#   podman build -t tamagi .          (ou: just build)

ARG FEDORA_VERSION=45
ARG BASE_IMAGE=ghcr.io/ublue-os/base-main

# ── Estágio 1: compila o llama.cpp (CPU + Vulkan) ──────────────────────────
FROM quay.io/fedora/fedora:${FEDORA_VERSION} AS llama
ARG LLAMA_CPP_REF=v1.0.0
RUN dnf -y install --setopt=install_weak_deps=False \
        git-core cmake ninja-build gcc-c++ \
        vulkan-headers vulkan-loader-devel spirv-headers-devel glslc \
        openssl-devel && dnf clean all
# Clone sem blobs, mas com o histórico de commits: o build usa a contagem de
# commits para baixar a interface web pré-compilada da versão certa.
RUN git clone --filter=blob:none --branch "${LLAMA_CPP_REF}" \
        https://github.com/ggml-org/llama.cpp /src
WORKDIR /src
# GGML_NATIVE=OFF: binário portátil (o ggml escolhe AVX2/AVX-512 em runtime
# com GGML_CPU_ALL_VARIANTS, que exige o backend como biblioteca dinâmica).
RUN cmake -B build -G Ninja \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_INSTALL_PREFIX=/usr \
        -DCMAKE_INSTALL_LIBDIR=lib64 \
        -DGGML_NATIVE=OFF \
        -DGGML_BACKEND_DL=ON \
        -DGGML_BACKEND_DIR=/usr/lib64/ggml \
        -DGGML_CPU_ALL_VARIANTS=ON \
        -DGGML_VULKAN=ON \
        -DLLAMA_OPENSSL=ON \
        -DLLAMA_BUILD_TESTS=OFF \
        -DLLAMA_BUILD_EXAMPLES=OFF \
    && cmake --build build -j"$(nproc)" \
    && DESTDIR=/out cmake --install build --strip \
    && rm -rf /out/usr/include /out/usr/lib64/cmake /out/usr/lib64/pkgconfig \
    && install -Dm644 LICENSE /out/usr/share/licenses/llama.cpp/LICENSE

# ── Estágio 2: apps do Omarchy (Qt Quick, licença MIT) ─────────────────────
FROM quay.io/fedora/fedora:${FEDORA_VERSION} AS omaapps
ARG OMACALC_REF=v0.4.2
ARG OMAWRITE_REF=v0.7.5
RUN dnf -y install --setopt=install_weak_deps=False \
        git-core cmake make gcc-c++ qt6-qtbase-devel qt6-qtdeclarative-devel \
    && dnf clean all
RUN set -eux; \
    for app in "omacalc ${OMACALC_REF}" "omawrite ${OMAWRITE_REF}"; do \
        set -- $app; \
        git clone --depth 1 --branch "$2" "https://github.com/omacom/$1" "/src/$1"; \
        cmake -S "/src/$1" -B "/src/$1/build" -DCMAKE_BUILD_TYPE=Release; \
        cmake --build "/src/$1/build" -j"$(nproc)" --target "$1"; \
        install -Dm755 "/src/$1/build/$1" "/out/usr/bin/$1"; \
        strip "/out/usr/bin/$1"; \
        install -Dm644 "/src/$1/pkgbuild/$1.desktop" "/out/usr/share/applications/$1.desktop"; \
        install -Dm644 "/src/$1/pkgbuild/$1.svg" "/out/usr/share/icons/hicolor/scalable/apps/$1.svg"; \
        install -Dm644 "/src/$1/LICENSE" "/out/usr/share/licenses/$1/LICENSE"; \
        install -Dm644 "/src/$1/fonts/OFL.txt" "/out/usr/share/licenses/$1/OFL.txt"; \
    done

# ── Estágio 3: contexto de build (scripts + arquivos) ──────────────────────
FROM scratch AS ctx
COPY build_files /build_files
COPY system_files /system_files
COPY README.md /README.md

# ── Estágio 4: a imagem do sistema ─────────────────────────────────────────
FROM ${BASE_IMAGE}:${FEDORA_VERSION}

ARG FEDORA_VERSION=45
ARG IMAGE_NAME=tamagi
ARG TAMAGI_VERSION=1.0
ARG TAMAGI_CODENAME="Kazunobu M"
ARG IMAGE_VENDOR=local
ARG IMAGE_REF=
ARG OPENCODE_VERSION=v1.19.6
ARG OPENCODE_SHA256=8443113b3f3dfebf8b0fa91c4377e5519a62b9231732d9344a5df001e4a852b3
ARG STARSHIP_VERSION=v1.27.0
ARG CLAUDE_CODE_VERSION=latest
ARG QWEN_CODE_VERSION=latest
ARG MISE_VERSION=v2026.11.9

COPY --from=llama /out/usr /usr
COPY --from=omaapps /out/usr /usr
COPY --from=ctx /system_files /

RUN --mount=type=bind,from=ctx,source=/,target=/ctx \
    --mount=type=cache,dst=/var/cache/libdnf5 \
    --mount=type=tmpfs,dst=/tmp \
    --mount=type=tmpfs,dst=/run \
    FEDORA_VERSION=${FEDORA_VERSION} IMAGE_NAME=${IMAGE_NAME} \
    TAMAGI_VERSION="${TAMAGI_VERSION}" TAMAGI_CODENAME="${TAMAGI_CODENAME}" \
    IMAGE_VENDOR=${IMAGE_VENDOR} IMAGE_REF=${IMAGE_REF} \
    OPENCODE_VERSION=${OPENCODE_VERSION} OPENCODE_SHA256=${OPENCODE_SHA256} \
    STARSHIP_VERSION=${STARSHIP_VERSION} \
    MISE_VERSION=${MISE_VERSION} CLAUDE_CODE_VERSION=${CLAUDE_CODE_VERSION} \
    QWEN_CODE_VERSION=${QWEN_CODE_VERSION} \
    /ctx/build_files/build.sh

RUN bootc container lint
