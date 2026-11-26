# Tamagi — tarefas de desenvolvimento das imagens (Tamagi-DxAI e Tamagi-SKL). `just` lista tudo.

image := env("TAMAGI_IMAGE", "localhost/tamagi")
skl_image := env("TAMAGI_SKL_IMAGE", "localhost/tamagi-skl")
tag := env("TAMAGI_TAG", "latest")
fedora := env("FEDORA_VERSION", "45")
kernel := env("KERNEL_FLAVOR", "ogc")
akmods := kernel
target := env("TAMAGI_TARGET_IMGREF", "ghcr.io/wes-tamagi/tamagi:latest")
skl_target := env("TAMAGI_SKL_TARGET_IMGREF", "ghcr.io/wes-tamagi/tamagi-skl:latest")
iso := "output/imagens_iso/tamagi_nova.iso"
bib := "quay.io/centos-bootc/bootc-image-builder:latest"

_default:
    @just --list

# Constrói a imagem OCI do Tamagi-DxAI (edição regular: IA, containers, VM, gaming base)
build *ARGS:
    podman build --pull=newer \
        --build-arg FEDORA_VERSION={{ fedora }} \
        --build-arg TAMAGI_EDITION=dxai \
        --build-arg IMAGE_NAME=tamagi-dxai \
        --build-arg IMAGE_REF={{ image }} \
        --build-arg KERNEL_FLAVOR={{ kernel }} \
        --build-arg TAMAGI_DEFAULT_LANG={{ env("TAMAGI_DEFAULT_LANG", "en") }} \
        --build-arg AKMODS_FLAVOR={{ akmods }} \
        -t {{ image }}:{{ tag }}-layers {{ ARGS }} .
    if [ "${TAMAGI_RECHUNK:-1}" = 1 ]; then bash rechunk.sh {{ image }}:{{ tag }}-layers {{ image }}:{{ tag }}; else podman tag {{ image }}:{{ tag }}-layers {{ image }}:{{ tag }}; fi

# Constrói a imagem OCI do Tamagi-SKL (edição escolar enxuta: painel travado, Chromium gerenciado, SSH admin)
build-skl *ARGS:
    podman build --pull=newer \
        --build-arg FEDORA_VERSION={{ fedora }} \
        --build-arg TAMAGI_EDITION=skl \
        --build-arg IMAGE_NAME=tamagi-skl \
        --build-arg IMAGE_REF={{ skl_image }} \
        --build-arg KERNEL_FLAVOR={{ kernel }} \
        --build-arg TAMAGI_DEFAULT_LANG={{ env("TAMAGI_DEFAULT_LANG", "en") }} \
        --build-arg AKMODS_FLAVOR={{ akmods }} \
        -t {{ skl_image }}:{{ tag }}-layers {{ ARGS }} .
    if [ "${TAMAGI_RECHUNK:-1}" = 1 ]; then bash rechunk.sh {{ skl_image }}:{{ tag }}-layers {{ skl_image }}:{{ tag }} "$(pwd)/output/rechunk-skl"; else podman tag {{ skl_image }}:{{ tag }}-layers {{ skl_image }}:{{ tag }}; fi

# Constrói ambas as edições (Tamagi-DxAI + Tamagi-SKL)
build-all: build build-skl

# Testes da imagem Tamagi-DxAI
test:
    podman run --rm -v ./tests:/tests:ro,Z {{ image }}:{{ tag }} bash /tests/image.sh

# Testes da imagem Tamagi-SKL
test-skl:
    podman run --rm -v ./tests:/tests:ro,Z {{ skl_image }}:{{ tag }} bash /tests/image.sh

# Teste de ponta a ponta do agente local
test-agent:
    mkdir -p .cache/models
    podman run --rm -v ./tests:/tests:ro,Z -v ./.cache/models:/models:Z \
        {{ image }}:{{ tag }} bash /tests/agent.sh

# Sobe a sessão COSMIC num X virtual, confere os componentes e salva output/desktop.png
test-desktop:
    mkdir -p output
    podman run --rm -v ./tests:/tests:ro,Z -v ./output:/out:Z {{ image }}:{{ tag }} bash /tests/desktop.sh

# Percorre o instalador sem monitor (tema escuro e claro)
test-installer-ui:
    bash welcome/tools/test-installer-ui.sh {{ image }}:{{ tag }}

# Testa a caixa de navegadores localmente
test-browser:
    bash tests/browser.sh

# Abre um shell dentro da imagem DxAI
shell:
    podman run --rm -it {{ image }}:{{ tag }} bash

# Lint dos scripts e strings i18n do repositório
lint:
    podman run --rm -v .:/src:ro,Z -w /src quay.io/fedora/fedora:{{ fedora }} bash -c '\
        dnf -y -q install --setopt=install_weak_deps=False ShellCheck python3 >/dev/null && \
        shellcheck -x -S warning build_files/*.sh tests/*.sh live/*.sh welcome/tools/test-installer-ui.sh welcome/tools/stubs/setup/backend welcome/tools/stubs/setup/detect \
            system_files/etc/profile.d/tamagi.sh \
            $(grep -lE "^#!/usr/bin/(env )?bash" system_files/usr/bin/* system_files/usr/libexec/tamagi/* live/files/usr/libexec/*) \
        && python3 -c "import ast, sys; [ast.parse(open(p).read()) for p in sys.argv[1:]]" \
            system_files/usr/libexec/tamagi/agent-local system_files/usr/bin/tamagi-menu \
            system_files/usr/bin/tamagi-snap system_files/usr/bin/tamagi-capture system_files/usr/bin/tamagi-rewind \
            system_files/usr/bin/tamagi-workspace \
            system_files/usr/libexec/tamagi/egress-guard \
            system_files/usr/libexec/tamagi/tamagi_i18n.py system_files/usr/libexec/tamagi/gum-multi \
            system_files/usr/libexec/tamagi/update-changes \
        && ! grep -rnE "tamagi (agents|devbox|db) " system_files/usr/bin/ system_files/usr/libexec/tamagi/ \
        && python3 tests/i18n-strings.py system_files --check \
        && echo "lint ok"'

# ── Instalador: ISO live multi-edição (Tamagi-DxAI + Tamagi-SKL) ──

build-live *ARGS:
    podman build --build-arg IMAGE={{ image }}:{{ tag }} \
        --build-arg TARGET_IMGREF={{ target }} \
        --build-arg SKL_TARGET_IMGREF={{ skl_target }} \
        -t {{ image }}-live:{{ tag }} {{ ARGS }} -f live/Containerfile .

# Gera output/imagens_iso/tamagi_nova.iso com Tamagi-DxAI e Tamagi-SKL deduplicadas em tamagi-oci/
build-iso: build-live
    mkdir -p output/imagens_iso
    bash live/build-iso.sh {{ image }}:{{ tag }} {{ image }}-live:{{ tag }} {{ iso }} {{ skl_image }}:{{ tag }}

# Instala a ISO numa VM (DxAI por padrão; passe --skl para testar Tamagi-SKL)
test-iso *ARGS:
    bash tests/iso.sh {{ iso }} {{ ARGS }}

test-iso-skl:
    bash tests/iso.sh {{ iso }} --skl

test-preserve-home:
    bash tests/iso.sh {{ iso }} --preserve-home

test-upgrade:
    bash tests/iso.sh {{ iso }} --upgrade

run-iso:
    bash tests/iso.sh {{ iso }} --interactive

run-vm:
    bash tests/iso.sh {{ iso }} --boot-disk

_to-root:
    podman save {{ image }}:{{ tag }} | sudo podman load

build-qcow2: _to-root
    mkdir -p output
    sudo podman run --rm -it --privileged --pull=newer \
        --security-opt label=type:unconfined_t \
        -v ./disk_config/disk.toml:/config.toml:ro \
        -v ./output:/output \
        -v /var/lib/containers/storage:/var/lib/containers/storage \
        {{ bib }} --type qcow2 --rootfs btrfs --use-librepo=True {{ image }}:{{ tag }}
    sudo chown -R "$(id -u):$(id -g)" output

clean:
    rm -rf output .cache
