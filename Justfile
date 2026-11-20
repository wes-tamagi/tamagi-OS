# Tamagi — tarefas de desenvolvimento da imagem.   `just` lista tudo.

image := env("TAMAGI_IMAGE", "localhost/tamagi")
tag := env("TAMAGI_TAG", "latest")
fedora := env("FEDORA_VERSION", "45")
# Kernel: main (padrão, o da base) | ogc. Ex.: KERNEL_FLAVOR=ogc TAMAGI_TAG=ogc just build
kernel := env("KERNEL_FLAVOR", "main")
# Para onde o sistema instalado olha para se atualizar (vazio: fica na imagem local).
target := env("TAMAGI_TARGET_IMGREF", "")
bib := "quay.io/centos-bootc/bootc-image-builder:latest"

_default:
    @just --list

# Constrói a imagem OCI do sistema (rootless, sem sudo)
build *ARGS:
    podman build --pull=newer \
        --build-arg FEDORA_VERSION={{ fedora }} \
        --build-arg IMAGE_REF={{ image }} \
        --build-arg KERNEL_FLAVOR={{ kernel }} \
        -t {{ image }}:{{ tag }} {{ ARGS }} .

# Testes da imagem: arquivos, serviços, COSMIC, Rust, hardening, primeiro boot, llama.cpp
test:
    podman run --rm -v ./tests:/tests:ro,Z {{ image }}:{{ tag }} bash /tests/image.sh

# Teste de ponta a ponta: baixa o preset mini (2,8 GB na 1ª vez) e faz o agente usar uma ferramenta
test-agent:
    mkdir -p .cache/models
    podman run --rm -v ./tests:/tests:ro,Z -v ./.cache/models:/models:Z \
        {{ image }}:{{ tag }} bash /tests/agent.sh

# Sobe a sessão COSMIC num X virtual, confere os componentes e salva output/desktop.png
test-desktop:
    mkdir -p output
    podman run --rm -v ./tests:/tests:ro,Z -v ./output:/out:Z {{ image }}:{{ tag }} bash /tests/desktop.sh

# Testa a caixa de navegadores de verdade neste computador (distrobox): Chrome, VA-API, Vulkan
test-browser:
    bash tests/browser.sh

# Abre um shell dentro da imagem (para inspecionar)
shell:
    podman run --rm -it {{ image }}:{{ tag }} bash

# Lint dos scripts do repositório (usa o shellcheck que vem na imagem)
lint:
    podman run --rm -v .:/src:ro,Z -w /src {{ image }}:{{ tag }} bash -c '\
        shellcheck -x -S warning build_files/*.sh tests/*.sh live/*.sh live/tik/modules/*/* \
            system_files/etc/profile.d/tamagi.sh \
            $(grep -lE "^#!/usr/bin/(env )?bash" system_files/usr/bin/* system_files/usr/libexec/tamagi/* live/files/usr/libexec/*) \
        && python3 -c "import ast; [ast.parse(open(p).read()) for p in ('system_files/usr/bin/tamagi-agent', 'system_files/usr/bin/tamagi-menu', 'system_files/usr/bin/tamagi')]" \
        && python3 tests/i18n-strings.py system_files --check \
        && echo "lint ok"'

# ── Instalador: ISO live com o tik + bootc (tudo rootless, sem sudo) ──────

# Constrói a imagem do ambiente live (o sistema + tik + initramfs live)
build-live *ARGS:
    podman build --build-arg IMAGE={{ image }}:{{ tag }} \
        --build-arg TARGET_IMGREF={{ target }} \
        -t {{ image }}-live:{{ tag }} {{ ARGS }} -f live/Containerfile .

# Gera output/tamagi.iso: ambiente live + a imagem do sistema embutida (instala sem rede)
build-iso: build-live
    bash live/build-iso.sh {{ image }}:{{ tag }} {{ image }}-live:{{ tag }} output/tamagi.iso

# Instala a ISO numa VM sem ninguém clicar (disco output/vm-test.qcow2) e confere o boot do sistema instalado
test-iso *ARGS:
    bash tests/iso.sh output/tamagi.iso {{ ARGS }}

# Atualização automática de ponta a ponta numa VM (registry local, imagem nova, reinício, rollback)
test-upgrade:
    bash tests/iso.sh output/tamagi.iso --upgrade

# Depois de `just test-iso`: simula uma atualização ruim e confere o rollback automático
test-rollback:
    bash tests/iso.sh output/tamagi.iso --rollback

# Abre a ISO numa VM com janela, para usar o instalador de verdade
run-iso:
    bash tests/iso.sh output/tamagi.iso --interactive

# Inicia a VM pelo disco já instalado (output/vm-test.qcow2)
run-vm:
    bash tests/iso.sh output/tamagi.iso --boot-disk

# ── Alternativa sem instalador: disco de VM pelo bootc-image-builder (pede sudo) ──

# Copia a imagem do seu usuário para o podman do root
_to-root:
    podman save {{ image }}:{{ tag }} | sudo podman load

# Gera um disco de VM (output/qcow2/disk.qcow2) com usuário tamagi/tamagi — pede sudo
build-qcow2: _to-root
    mkdir -p output
    sudo podman run --rm -it --privileged --pull=newer \
        --security-opt label=type:unconfined_t \
        -v ./disk_config/disk.toml:/config.toml:ro \
        -v ./output:/output \
        -v /var/lib/containers/storage:/var/lib/containers/storage \
        {{ bib }} --type qcow2 --rootfs btrfs --use-librepo=True {{ image }}:{{ tag }}
    sudo chown -R "$(id -u):$(id -g)" output

# Apaga artefatos gerados
clean:
    rm -rf output .cache
