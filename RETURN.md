# RETURN.md — Estado do Projeto Tamagi OS

Última atualização: 2026-11-24 (ISO da v1.4 fechada, branch `feat/v1.4-immutable-launch`).

ISO local: `output/imagens_iso/tamagi.iso` (5,6 GiB; 5.965.824.000 bytes)
SHA-256: `764f4f53ba68ffd89b75caa9d463921a6b5c2d061839c98338de8e588eb3ca5d`
(também em `output/imagens_iso/tamagi.iso.sha256`). Imagem do sistema (`rechunk`): 7,39 GB.

> **Instale com o Secure Boot desligado no firmware.** O instalador agora detecta
> automaticamente na 1ª tela se o Secure Boot está ativo e bloqueia o avanço antes
> de tocar no disco (permitindo apenas experimentar o modo Live), pois o sistema
> instalado usa `systemd-boot` ainda sem assinatura.

### Validação (cadeia completa, 0 falhas)
| Passo | Resultado |
|---|---|
| `just lint` | ok (`865 textos + actions.toml, 0 problema(s)`) |
| `just build` (+ `rechunk.sh` + testes unitários Rust) | ok (`localhost/tamagi:latest` 7,39 GB) |
| `just test` | **422 ok, 0 falhas** |
| `just test-desktop` | ok (`output/desktop.png`, todos os 6 daemons COSMIC ativos) |
| `just test-installer-ui` | ok: **19 telas** no tema escuro e no claro (`output/installer-shots/`, incluindo alerta e bloqueio de Secure Boot) |
| `just build-iso` | ok (`output/imagens_iso/tamagi.iso` + `.sha256`) |
| `just test-iso` | ok: 0 execuções de `ldconfig`/`hwdb` na mídia live, `/etc/ld.so.cache` íntegro no host instalado, `tamagi-mcp` e `egress-guard` ativos |
| `tests/iso.sh output/imagens_iso/tamagi.iso --tpm` | ok: Secure Boot desligado → chave do TPM sem PCR; 2º boot abre sozinho |
| `tests/iso.sh output/imagens_iso/tamagi.iso --tpm-pin` | ok: o boot pede o **PIN** (não a senha); chave com PIN e sem PCR |
| `just test-preserve-home` | ok: `/var/home` e snapshot `#1` preservados de layout anterior |
| `just test-upgrade` | ok: atualização atômica + rollback automático em 85 s + quarentena |

> **Hardware real ainda pendente:** toda a cadeia acima foi validada em contêineres e VMs QEMU/KVM (`swtpm` + OVMF). Falta exercitar em máquina física real (pendrive USB real, TPM físico, Wi-Fi/GPU).

### Teste em máquina real (para o Wes)
1. Grave a ISO (`sudo bash live/write-usb.sh /dev/disk/by-id/usb-<seu-pendrive> --yes` — já aponta por padrão para `output/imagens_iso/tamagi.iso`) e confira o SHA-256 acima.
2. No firmware: **Secure Boot desligado**, modo UEFI (se deixar ligado, a 1ª tela do instalador deve exibir o cartão de aviso Material 3 e impedir a formatação).
3. Confirme que o boot do pendrive passa direto sem travar nem exibir job de `ldconfig.service`.
4. Instale escolhendo **criptografia** e **Chip TPM + PIN** (PIN de 6+ caracteres).
5. Reinicie 2–3 vezes e rode `systemd-analyze`, `systemctl --failed`, `tamagi doctor`, `tamagi mcp list` e `tamagi agents shadow list`.

---

## O que mudou nesta versão (`feat/v1.4-immutable-launch`)

### 1. Fim do travamento em `ldconfig.service` + Pipeline Live ISO dedicado
- **Mídia Live sem reconstrução de cache no pendrive (`live/build-live.sh`):** `/etc/ld.so.cache`, `/etc/udev/hwdb.bin` e o catálogo do journal são pré-gerados no build da ISO; na sessão live ficam mascarados `ldconfig.service`, `systemd-hwdb-update.service`, `systemd-journal-catalog-update.service`, `appstream`, `mandb`, Plymouth pós-switch-root e ~30 unidades/sockets exclusivos de sistema instalado (Cockpit, libvirt, Snapper, `boot-check`, timers Btrfs).
- **`squashfs` Live otimizado (`live/mkiso-inner.sh`):** `zstd` nível 15 para descompressão rápida em USB e exclusão de ROMs/binários de VM (`edk2`/`qemu`) apenas do `LiveOS/squashfs.img` (100% mantidos em `tamagi-oci/` para o sistema instalado).
- **Correção no sistema instalado (`ldconfig.service.d/10-tamagi.conf` + `90-finalize.sh`):** limpa também `ConditionFileNotEmpty=` (que impedia `once-per-image` de rodar após o 1º boot), adiciona `ConditionPathExists=!/run/tamagi-live`, sobe o timeout para `180s` e já entrega `/etc/ld.so.cache` pré-gerado na imagem.
- **Saída padronizada em `output/imagens_iso/`:** `Justfile`, `live/build-iso.sh` (gera `.sha256` automaticamente), `live/write-usb.sh` e `tests/iso.sh` usam `output/imagens_iso/tamagi.iso`.

### 2. Instalador mais seguro e inteligente (`welcome/` + `tamagi-install`)
- **Guarda de Secure Boot na abertura:** tanto a interface gráfica (`Page::Welcome`) quanto o motor `/usr/libexec/tamagi-install` detectam `SecureBoot=1` via efivar UEFI, exibem alerta claro em Material 3 e bloqueiam a instalação antes de tocar no disco (coberto em `just test-installer-ui`).
- **Reconexão automática de usuário:** ao detectar `/var/home` preservável num disco, o instalador já sugere automaticamente o usuário encontrado se o campo ainda estiver em branco.

### 3. Cortes pragmáticos e unificação em Rust estático
- **Fim do estágio Go (`go/` removido):** `tamagi-doctor` e `tamagi-install` foram reescritos como binários estáticos Rust dentro do workspace `rust/tamagi`, eliminando o compilador Go do `Containerfile` com paridade total de CLI, JSON e testes unitários.
- **Remoção de comandos e pacotes periféricos do host:** saíram da base `tamagi-remind`, `tamagi-recipe`, `tamagi-guest`, `emoji`, `tesseract*` (agora opcional em `tamagi install ocr`), `android-tools` (`tamagi install android-tools`), `iperf3`/`nmap-ncat`/`wol` (`tamagi install net-tools`), `ifuse`/`gvfs-afc`/`gvfs-goa`, os kmods `xone`/`xpadneo`, o sabor Rawhide `ark`, o script legado `luks-enable-tpm2-autounlock` e aliases TOML antigos em português; `foot.desktop` fica oculto no menu (`NoDisplay=true`), deixando apenas **Ptyxis** e **COSMIC Terminal** visíveis.

### 4. Recurso Extra: `tamagi mcp` sobre Podman Quadlets
- Novo comando `tamagi mcp` (`list|installed|add|remove|sync|status`) + seção declarativa `[mcp]` no `tamagi.toml` (`tamagi export|diff|apply`) e integração com `podman auto-update` no `tamagi update`.
- Servidores curados (`fetch`, `memory`, `git`, `postgres`, `playwright`, `filesystem`) geridos por unidades Quadlet `.image` em `~/.config/containers/systemd/` e executados sob demanda via `podman run --rm -i --cap-drop=ALL --security-opt=no-new-privileges`: **0 MB de RAM em repouso**, sem acesso à sua `$HOME`, e sincronizados automaticamente para **Claude Code**, **Antigravity / Gemini CLI**, **Codex** e **opencode**.

### 5. Inovação: Shadow Workspaces CoW + Egress Guard (`agent-jail` + `tamagi agents`)
- **`tamagi agents run --shadow <agente>` e `tamagi agents shadow list|run|diff|apply|drop`:** clona instantaneamente o projeto (incluindo `target/`, `node_modules/` e `.venv/`) via Copy-on-Write (`cp -a --reflink=auto` no Btrfs, 0 bytes extras iniciais) e o `agent-jail` monta a sombra sobre o **mesmo caminho canônico** do projeto — permitindo rodar múltiplos agentes em paralelo com caches de compilação quentes sem tocar um único arquivo da árvore real até `tamagi agents shadow apply <id>`.
- **Egress Guard (`/usr/libexec/tamagi/egress-guard` + `--egress-sock` no `agent-jail`):** isola a rede no kernel (`bwrap --unshare-net`) e faz ponte exclusivamente por socket Unix para um proxy com *allowlist* estrita (APIs de IA, registries de pacotes open-source e `llama.cpp` local em `127.0.0.1:9931`), bloqueando IPs crus e exfiltração de código.
