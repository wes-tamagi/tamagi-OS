# Tamagi — imagem bootc sobre o Universal Blue (base-main) com o desktop COSMIC,
# distrobox e IA local (llama.cpp). A imagem é enxuta: navegadores, agentes de
# IA, jogos, máquinas virtuais e ferramentas de desenvolvimento são opcionais
# e entram pela escolha do usuário (`tamagi install`).
#
#   podman build -t tamagi .          (ou: just build)

ARG FEDORA_VERSION=45
ARG BASE_IMAGE=ghcr.io/ublue-os/base-main
# Kernel: ogc (padrão: Open Gaming Collective, assinado pelo
# Universal Blue) | main (o da base, assinado pelo Fedora) | ark (kernel-ark,
# o do Fedora Rawhide).
# Veja build_files/12-kernel.sh, em especial a nota sobre Secure Boot.
ARG KERNEL_FLAVOR=ogc
# Módulos prontos do Universal Blue: o mesmo sabor do kernel (ogc ou main). O
# ark não tem módulos lá; com ele, passe AKMODS_FLAVOR=main (o Justfile faz).
ARG AKMODS_FLAVOR=${KERNEL_FLAVOR}

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

# ── Estágio 1b: menu iniciar (cosmic-ext-classic-menu 1.1.0) ───────────────
# O COPR para na 1.0.0, cujo menu só recebe o teclado depois de um clique. A
# 1.1.0 traz o "menu focado" (focused_menu): abriu, já dá para digitar e usar
# as setas. Versão fixa: uma nova só entra trocando CLASSIC_MENU_REF.
FROM quay.io/fedora/fedora:${FEDORA_VERSION} AS rustdesk
ARG CLASSIC_MENU_REF=1.1.0
RUN dnf -y install --setopt=install_weak_deps=False \
        git-core cargo rust just gcc pkgconf-pkg-config \
        libxkbcommon-devel wayland-devel fontconfig-devel freetype-devel \
    && dnf clean all
ENV CARGO_HOME=/cargo CARGO_TERM_COLOR=never CARGO_NET_GIT_FETCH_WITH_CLI=true
# Menu iniciar (o justfile do projeto instala no mesmo lugar que o RPM do COPR).
RUN git clone --depth 1 --branch "${CLASSIC_MENU_REF}" \
        https://github.com/championpeak87/cosmic-ext-classic-menu /src/menu \
    && cd /src/menu && just build-release --locked \
    && just rootdir=/out install \
    && strip /out/usr/bin/cosmic-ext-classic-menu-* \
    && install -Dm644 LICENSE /out/usr/share/licenses/cosmic-ext-classic-menu/LICENSE

# ── Estágio 1c: as ferramentas do Tamagi escritas em Rust (rust/) ───────────
# tamagi (a entrada única), tamagi-layer (camadas nativas) e agent-jail (o
# isolamento dos agentes de IA). Estáticas (glibc embutida): o agent-jail também roda dentro das caixas de
# desenvolvimento (Ubuntu, Arch, openSUSE), onde a glibc é outra. Os testes
# unitários rodam aqui: se falharem, a imagem não é construída.
FROM quay.io/fedora/fedora:${FEDORA_VERSION} AS tamagitools
RUN dnf -y install --setopt=install_weak_deps=False cargo rust gcc glibc-static \
    && dnf clean all
ENV CARGO_HOME=/cargo CARGO_TERM_COLOR=never CARGO_TARGET_DIR=/target
COPY rust /src
WORKDIR /src
RUN cargo test --release --locked \
    && RUSTFLAGS="-C target-feature=+crt-static" cargo build --release --locked --target x86_64-unknown-linux-gnu \
    && install -Dm755 /target/x86_64-unknown-linux-gnu/release/agent-jail /out/usr/libexec/tamagi/agent-jail \
    && install -Dm755 /target/x86_64-unknown-linux-gnu/release/tamagi /out/usr/bin/tamagi \
    && install -Dm755 /target/x86_64-unknown-linux-gnu/release/tamagi-layer /out/usr/bin/tamagi-layer

# ── Kernel e módulos prontos e assinados do Universal Blue (ublue-os/akmods) ──
# build_files/12-kernel.sh troca o kernel (se o sabor pedir) e instala os módulos.
FROM ghcr.io/ublue-os/akmods:${AKMODS_FLAVOR}-${FEDORA_VERSION} AS akmods

# ── Estágio 2: contexto de build (scripts + arquivos) ──────────────────────
FROM scratch AS ctx
COPY build_files /build_files
COPY system_files /system_files
COPY README.md /README.md
COPY NOTICE /NOTICE

# ── Estágio 3: a imagem do sistema ─────────────────────────────────────────
FROM ${BASE_IMAGE}:${FEDORA_VERSION}

ARG FEDORA_VERSION=45
ARG IMAGE_NAME=tamagi
ARG TAMAGI_VERSION=1.0
ARG TAMAGI_CODENAME="Kazunobu M"
ARG IMAGE_VENDOR=local
ARG IMAGE_REF=
ARG STARSHIP_VERSION=v1.27.0
ARG KERNEL_FLAVOR=ogc

COPY --from=llama /out/usr /usr
COPY --from=rustdesk /out/usr /usr
COPY --from=ctx /system_files /
COPY --from=tamagitools /out/usr /usr

RUN --mount=type=bind,from=ctx,source=/,target=/ctx \
    --mount=type=bind,from=akmods,source=/,target=/akmods \
    --mount=type=cache,dst=/var/cache/libdnf5 \
    --mount=type=tmpfs,dst=/tmp \
    --mount=type=tmpfs,dst=/run \
    FEDORA_VERSION=${FEDORA_VERSION} IMAGE_NAME=${IMAGE_NAME} \
    TAMAGI_VERSION="${TAMAGI_VERSION}" TAMAGI_CODENAME="${TAMAGI_CODENAME}" \
    IMAGE_VENDOR=${IMAGE_VENDOR} IMAGE_REF=${IMAGE_REF} \
    STARSHIP_VERSION=${STARSHIP_VERSION} \
    KERNEL_FLAVOR=${KERNEL_FLAVOR} \
    /ctx/build_files/build.sh

# The version label is what bootc records at install time (rechunk.sh carries it over).
LABEL org.opencontainers.image.title="Tamagi" \
      org.opencontainers.image.version="${TAMAGI_VERSION}"

RUN bootc container lint
