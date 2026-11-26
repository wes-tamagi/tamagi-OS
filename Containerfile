# Tamagi — imagens bootc sobre Universal Blue (base-main) com COSMIC e kernel OGC.
# Duas edições a partir da mesma fonte (ARG TAMAGI_EDITION):
#   dxai  Tamagi-DxAI — estação atômica para agentes de IA, containers, VM e gaming base
#   skl   Tamagi-SKL  — edição escolar enxuta com painel travado, Chromium gerenciado e SSH admin
#
#   just build              (Tamagi-DxAI)
#   just build-skl          (Tamagi-SKL)

ARG FEDORA_VERSION=45
ARG BASE_IMAGE=ghcr.io/ublue-os/base-main
ARG TAMAGI_EDITION=dxai
ARG KERNEL_FLAVOR=ogc
ARG TAMAGI_DEFAULT_LANG=en
ARG AKMODS_FLAVOR=${KERNEL_FLAVOR}

# ── Estágio 1: compila o llama.cpp (CPU + Vulkan) ──────────────────────────
FROM quay.io/fedora/fedora:${FEDORA_VERSION} AS llama
ARG LLAMA_CPP_REF=v1.0.0
RUN dnf -y install --setopt=install_weak_deps=False \
        git-core cmake ninja-build clang llvm lld \
        vulkan-headers vulkan-loader-devel spirv-headers-devel glslc \
        openssl-devel && dnf clean all
RUN git clone --filter=blob:none --branch "${LLAMA_CPP_REF}" \
        https://github.com/ggml-org/llama.cpp /src
WORKDIR /src
RUN cmake -B build -G Ninja \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_C_COMPILER=clang \
        -DCMAKE_CXX_COMPILER=clang++ \
        -DCMAKE_EXE_LINKER_FLAGS="-fuse-ld=lld" \
        -DCMAKE_SHARED_LINKER_FLAGS="-fuse-ld=lld" \
        -DCMAKE_INTERPROCEDURAL_OPTIMIZATION=ON \
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

# ── Estágio 1b: apps COSMIC em Rust (classic-menu, welcome) ─────────────────
FROM quay.io/fedora/fedora:${FEDORA_VERSION} AS rustdesk
ARG CLASSIC_MENU_REF=1.1.0
RUN dnf -y install --setopt=install_weak_deps=False \
        git-core cargo rust just gcc clang clang-libs pkgconf-pkg-config \
        libxkbcommon-devel wayland-devel fontconfig-devel freetype-devel \
        openssl-devel \
    && dnf clean all
ENV CARGO_HOME=/cargo CARGO_TERM_COLOR=never CARGO_NET_GIT_FETCH_WITH_CLI=true
# Menu iniciar (1.1.0: focused_menu)
RUN git clone --depth 1 --branch "${CLASSIC_MENU_REF}" \
        https://github.com/championpeak87/cosmic-ext-classic-menu /src/menu \
    && cd /src/menu && just build-release --locked \
    && just rootdir=/out install \
    && strip /out/usr/bin/cosmic-ext-classic-menu-* \
    && install -Dm644 LICENSE /out/usr/share/licenses/cosmic-ext-classic-menu/LICENSE
# Janela de primeiro login e instalador (welcome/)
COPY welcome /src/welcome
RUN cd /src/welcome && CARGO_TARGET_DIR=/target-welcome cargo test --release --locked \
    && CARGO_TARGET_DIR=/target-welcome cargo build --release --locked \
    && install -Dm755 /target-welcome/release/tamagi-welcome /out/usr/bin/tamagi-welcome \
    && strip /out/usr/bin/tamagi-welcome

# ── Estágio 1c: ferramentas CLI do Tamagi em Rust estático (rust/) ─────────
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
    && install -Dm755 /target/x86_64-unknown-linux-gnu/release/tamagi-layer /out/usr/bin/tamagi-layer \
    && install -Dm755 /target/x86_64-unknown-linux-gnu/release/tamagi-doctor /out/usr/bin/tamagi-doctor \
    && install -Dm755 /target/x86_64-unknown-linux-gnu/release/tamagi-install /out/usr/bin/tamagi-install \
    && install -Dm755 /target/x86_64-unknown-linux-gnu/release/tamagi-why /out/usr/bin/tamagi-why \
    && install -Dm755 /target/x86_64-unknown-linux-gnu/release/tamagi-boot /out/usr/libexec/tamagi/tamagi-boot \
    && ln -sf tamagi-boot /out/usr/libexec/tamagi/once-per-image \
    && ln -sf tamagi-boot /out/usr/libexec/tamagi/loader-conf

# ── Kernel e módulos assinados do Universal Blue (ublue-os/akmods) ─────────
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
ARG TAMAGI_EDITION=dxai
ARG IMAGE_NAME=tamagi
ARG TAMAGI_VERSION=1.6
ARG TAMAGI_CODENAME="Kazunobu M"
ARG IMAGE_VENDOR=local
ARG IMAGE_REF=
ARG STARSHIP_VERSION=v1.27.0
ARG BOTTOM_VERSION=0.15.0
ARG BOTTOM_SHA256=051888e0e064aa037728efed3fcf77820714f8f4dc7bea2b0179d40c54400c26
ARG MISE_VERSION=v2026.11.9
ARG KERNEL_FLAVOR=ogc
ARG TAMAGI_DEFAULT_LANG=en

COPY --from=llama /out/usr /usr
COPY --from=rustdesk /out/usr /usr
COPY --from=ctx /system_files /
COPY --from=tamagitools /out/usr /usr

RUN --mount=type=bind,from=ctx,source=/,target=/ctx \
    --mount=type=bind,from=akmods,source=/,target=/akmods \
    --mount=type=cache,dst=/var/cache/libdnf5 \
    --mount=type=tmpfs,dst=/tmp \
    --mount=type=tmpfs,dst=/run \
    FEDORA_VERSION=${FEDORA_VERSION} TAMAGI_EDITION=${TAMAGI_EDITION} \
    IMAGE_NAME=${IMAGE_NAME} \
    TAMAGI_VERSION="${TAMAGI_VERSION}" TAMAGI_CODENAME="${TAMAGI_CODENAME}" \
    IMAGE_VENDOR=${IMAGE_VENDOR} IMAGE_REF=${IMAGE_REF} \
    STARSHIP_VERSION=${STARSHIP_VERSION} \
    BOTTOM_VERSION=${BOTTOM_VERSION} BOTTOM_SHA256=${BOTTOM_SHA256} \
    MISE_VERSION=${MISE_VERSION} \
    KERNEL_FLAVOR=${KERNEL_FLAVOR} TAMAGI_DEFAULT_LANG=${TAMAGI_DEFAULT_LANG} \
    /ctx/build_files/build.sh

LABEL org.opencontainers.image.title="Tamagi-${TAMAGI_EDITION}" \
      org.opencontainers.image.version="${TAMAGI_VERSION}"

RUN bootc container lint
