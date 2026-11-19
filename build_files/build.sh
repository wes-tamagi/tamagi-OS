#!/usr/bin/bash
# Orquestra o build do Tamagi dentro do contêiner (chamado pelo Containerfile).
set -euo pipefail

for script in /ctx/build_files/[0-9][0-9]-*.sh; do
    echo "::group::$(basename "$script")"
    bash -euo pipefail "$script"
    echo "::endgroup::"
done
