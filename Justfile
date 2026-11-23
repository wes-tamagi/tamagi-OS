# Tamagi — tarefas de desenvolvimento da imagem.   `just` lista tudo.

image := env("TAMAGI_IMAGE", "localhost/tamagi")
tag := env("TAMAGI_TAG", "latest")
fedora := env("FEDORA_VERSION", "45")
# Kernel: ogc (padrão: Open Gaming Collective) | main (o da base) | ark (o do Fedora Rawhide).
# Ex.: KERNEL_FLAVOR=main TAMAGI_TAG=main just build
kernel := env("KERNEL_FLAVOR", "ogc")
akmods := if kernel == "ark" { "main" } else { kernel }
# Para onde o sistema instalado olha para se atualizar (vazio: fica na imagem local).
target := env("TAMAGI_TARGET_IMGREF", "")
bib := "quay.io/centos-bootc/bootc-image-builder:latest"

_default:
    @just --list

# Constrói a imagem OCI do sistema (rootless, sem sudo), em dois passos:
#   1. o build normal, com cache de camadas, em <tag>-layers (~9,3 GB);
#   2. o rechunk (rechunk.sh): a mesma imagem refeita só com o conteúdo real,
#      em camadas por conteúdo (~6,8 GB; atualizações baixam só o que mudou).
# A base é uma imagem ostree: o que o build troca ou remove dela (kernel,
# idiomas) continuaria ocupando espaço sem o passo 2. Para pular o rechunk
# enquanto se desenvolve: TAMAGI_RECHUNK=0 just build
build *ARGS:
    podman build --pull=newer \
        --build-arg FEDORA_VERSION={{ fedora }} \
        --build-arg IMAGE_REF={{ image }} \
        --build-arg KERNEL_FLAVOR={{ kernel }} \
        --build-arg TAMAGI_DEFAULT_LANG={{ env("TAMAGI_DEFAULT_LANG", "en") }} \
        --build-arg AKMODS_FLAVOR={{ akmods }} \
        -t {{ image }}:{{ tag }}-layers {{ ARGS }} .
    if [ "${TAMAGI_RECHUNK:-1}" = 1 ]; then bash rechunk.sh {{ image }}:{{ tag }}-layers {{ image }}:{{ tag }}; else podman tag {{ image }}:{{ tag }}-layers {{ image }}:{{ tag }}; fi

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

# Lint dos scripts do repositório (o shellcheck não vem mais na imagem: é instalado no contêiner do lint)
lint:
    podman run --rm -v .:/src:ro,Z -w /src quay.io/fedora/fedora:{{ fedora }} bash -c '\
        dnf -y -q install --setopt=install_weak_deps=False ShellCheck python3 >/dev/null && \
        shellcheck -x -S warning build_files/*.sh tests/*.sh live/*.sh live/tik/modules/*/* \
            system_files/etc/profile.d/tamagi.sh \
            $(grep -lE "^#!/usr/bin/(env )?bash" system_files/usr/bin/* system_files/usr/libexec/tamagi/* live/files/usr/libexec/*) \
        && python3 -c "import ast, sys; [ast.parse(open(p).read()) for p in sys.argv[1:]]" \
            system_files/usr/bin/tamagi-agent system_files/usr/bin/tamagi-menu \
            system_files/usr/bin/tamagi-snap system_files/usr/bin/tamagi-capture system_files/usr/bin/tamagi-rewind \
            system_files/usr/bin/tamagi-workspace system_files/usr/bin/tamagi-doctor \
            system_files/usr/libexec/tamagi/tamagi_i18n.py system_files/usr/libexec/tamagi/gum-multi \
            system_files/usr/libexec/tamagi/update-changes live/files/usr/libexec/tamagi-installer-splash \
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

# Testa preservação de /var/home a partir de layout GRUB+ostree anterior (sem interação)
test-preserve-home:
    bash tests/iso.sh output/tamagi.iso --preserve-home

# Atualização automática de ponta a ponta numa VM (registry local, imagem nova, reinício) e volta automática de uma imagem ruim
test-upgrade:
    bash tests/iso.sh output/tamagi.iso --upgrade

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
