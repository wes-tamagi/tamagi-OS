# RETURN.md — Estado do Projeto Tamagi OS

Última atualização: 2026-11-24 (ISO da v1.3 fechada, branch `english-cli-minimal`).

ISO local: `output/tamagi.iso` (5,8 GB; 5.822.881.792 bytes)
SHA-256: `c2778eca57f2a243c00e92d9a11f564fcc8350645a3a2ab2a707b2a2f4694dd6`
(também em `output/tamagi.iso.sha256`). Imagem do sistema: 7,4 GB.

> **Instale com o Secure Boot desligado no firmware.** O sistema instalado usa o
> systemd-boot ainda sem assinatura; a cadeia assinada (shim do Fedora → systemd-boot
> com a chave Tamagi) vem na próxima versão. A ISO inicia com o Secure Boot ligado,
> mas o sistema instalado não iniciaria.

### Validação (cadeia completa, 0 falhas)
| Passo | Resultado |
|---|---|
| `just lint` | ok (845 textos, 0 problemas) |
| `just build` | ok |
| `just test` | 429 ok, 0 falhas |
| `just test-desktop` | ok |
| `just test-installer-ui` | ok: 17 telas no tema escuro e no claro (`output/installer-shots/`) |
| `just build-iso` | ok |
| `just test-iso` | ok: ESP de 2 GiB (disco de 64 GiB), `editor no`, `plymouth.use-simpledrm=1` |
| `tests/iso.sh --tpm` | ok: Secure Boot desligado → chave do TPM sem PCR; 2º boot abre sozinho |
| `tests/iso.sh --tpm-pin` | ok: o boot pede o **PIN** (não a senha); chave com PIN e sem PCR |
| `just test-preserve-home` | ok |
| `just test-upgrade` | ok: atualização + volta automática; `editor no` continua depois da atualização |

Nada disso foi exercitado em máquina real.

### Teste em máquina real (para o Wes)
1. Grave a ISO (`live/write-usb.sh` ou Fedora Media Writer) e confira o SHA-256 acima.
2. No firmware: **Secure Boot desligado**, modo UEFI.
3. Instale escolhendo **criptografia** e, na tela "Desbloqueio ao ligar", **Chip TPM + PIN**
   (PIN de 6+ caracteres).
4. Reinicie: o pedido do **PIN** tem de aparecer na hora (sem tela preta). Digite o PIN.
5. Reinicie mais 2 vezes e, se possível, atualize o firmware/BIOS ou rode
   `tamagi update`: o PIN tem de continuar funcionando, sem pedir a senha longa.
6. Mande: `systemd-analyze`, `systemd-analyze blame | head -20`,
   `journalctl -b -p err --no-pager`, `systemctl --failed`,
   `flatpak remotes --show-details` e `tamagi setup tpm-unlock status`.

Esta máquina (instalação antiga, GRUB): o TPM recusa desde a atualização de 23/11
(chave em PCR 7+14). Para corrigir sem reinstalar:
`sudo systemd-cryptenroll --wipe-slot=tpm2 --tpm2-device=auto --tpm2-pcrs= --tpm2-with-pin=yes /dev/disk/by-uuid/b36cd9df-d937-4356-97a7-f2f83f1cc402`

---

## O que mudou desde `e6bfd7a`

### Boot mais rápido
- `ldconfig`, cache do appstream e `mandb` rodam uma vez por imagem nova
  (`/usr/libexec/tamagi/once-per-image`), não a cada boot.
- Sem os remotes "Fedora Flatpaks": `flatpak-add-fedora-repos.service` mascarado;
  `tamagi-flatpak-cleanup` apaga os antigos.

### Instalador (próprio, opção D)
- 8 passos: Boas-vindas com o idioma → Teclado e fuso → Conta → **Disco → Criptografia**
  → Revisão → Instalando → Pronto.
- Desbloqueio ao ligar: **TPM + PIN** (padrão), TPM automático ou só a senha.
  Mantendo uma pasta criptografada, só pergunta como abre.
- Material 3: listas segmentadas, revisão sem rolagem, botão vermelho só ao apagar,
  disco pequeno esmaecido, barra de 8 px. Senha de outro disco nunca reaproveitada.

### TPM e boot
- TPM + PIN sem PCR: atualização de firmware, shim ou MOK nunca trava o disco
  (o erro visto aqui e no Aeon). Automático: PCR 7 só com o Secure Boot ligado.
- `tamagi setup tpm-unlock pin|auto|off|reseal|status` nativo (só o disco do sistema).
- `editor no` no systemd-boot; `plymouth.use-simpledrm=1` (pedido de senha/PIN visível).

### Partições
- ESP de 4 GiB (discos ≥ 80 GiB) ou 2 GiB; sem `var-flatpak`.
- Reinstalar mantendo os arquivos mantém as VMs (discos, definição, NVRAM, TPM virtual).
- Modelos de IA fora dos snapshots, sem compressão e com checksum; VMs do usuário sem CoW.

### Visual
- Tema padrão **Ipê** + novo **Ipê Claro**; 5 temas (Ipê, Ipê Claro, Tokyo Night, Nord,
  Gruvbox); cantos do Material 3 nos temas do COSMIC; 40 papéis de parede (eram 88).
- Fonte da interface **Roboto Flex** (commit `739e06d`, sha256 conferido), Noto Sans de reserva.
- Assistente do primeiro login com as peças Material 3 do instalador.

### Imagem mais enxuta
- VPNs raras (L2TP, PPTP, SSTP, strongSwan, Libreswan, SSH) no catálogo
  (`tamagi install vpn-extra`); Cockpit sem `storaged` e `networkmanager`.
  Virtualização e IA continuam na base.

### Processo
- `AGENTS.md` (modelo do Dakota), `gemini-serv.md` (CEO) ↔ `claude-serve.md` (CTO),
  `just test-installer-ui`, `tests/iso.sh --tpm-pin`.

---

## Decisões registradas
- Base: Universal Blue `base-main` (Fedora 45 estável) + kernel OGC.
- Instalador próprio (opção D); Agama, Anaconda WebUI, tik, tunaOS, Readymade, Calamares
  e distinst avaliados e descartados (nenhum instala bootc + composefs + systemd-boot
  com btrfs como o nosso).
- Snapper (não Timeshift) para a pasta pessoal; a raiz fica com o bootc.
- Virtualização e pilha de IA/contêineres ficam na imagem.

## Próximos passos
1. Aviso/bloqueio no instalador quando o Secure Boot estiver ligado.
2. Cadeia Secure Boot assinada + reamarrar o TPM automático (base: Dakota).
3. Canais `testing` → `stable` no CI, canal `next`.
4. Ícones (Colloid ou Papirus) — decisão de fonte externa pendente.
5. Flathub "de sistema" vindo de `/etc/flatpak/remotes.d` (mover os `.flatpakrepo`).
6. Testar `tamagi install vpn-extra` (NetworkManager precisa reiniciar para ver os plugins).
7. Enxugar o initramfs (244 MB).
