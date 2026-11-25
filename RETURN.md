# RETURN.md — Estado do Projeto Tamagi OS

Última atualização: 2026-11-25 (v1.5 fechada na branch `feat/v1.5-dxai-skl`, commit `1e3b8db`).

## Artefatos da v1.5 (ISO Multi-Edição `Tamagi-DxAI` + `Tamagi-SKL`)

- **ISO local:** `output/imagens_iso/tamagi_nova.iso` (**7,1 GB** — duas edições completas deduplicadas em OCI)
- **Cópia verificada no SSD externo:** `/run/media/wesdt/Extreme SSD/tamagi_nova.iso` (`sha256sum -c`: **OK**)
- **SHA-256:** `3f72f002df2fad1de45a808d81a1dd26b44d55b268b8ac28dee914ab28feec27` (também em `output/imagens_iso/tamagi_nova.iso.sha256`)
- **Imagens OCI (`rechunk`):**
  - `localhost/tamagi:latest` (**Tamagi-DxAI**): **7,8 GB**
  - `localhost/tamagi-skl:latest` (**Tamagi-SKL**): **6,7 GB**

> **Instale com o Secure Boot desligado no firmware.** O instalador detecta automaticamente
> na 1ª tela se o Secure Boot está ativo e bloqueia o avanço antes de tocar no disco, pois
> o sistema instalado usa `systemd-boot` sem assinatura (Secure Boot ponta a ponta está no
> [ROADMAP-2.0.md](./ROADMAP-2.0.md)).

### Validação executada (cadeia completa pós-correções, 0 falhas)

| Passo | Resultado |
|---|---|
| `just lint` (ShellCheck + AST Python + i18n pt/es/fr) | ok (`864 textos + actions.toml, 0 problema(s)`) |
| `just build` (`Tamagi-DxAI` + testes unitários Rust + `bootc container lint` + `rechunk`) | ok (`Checks passed: 14`, `localhost/tamagi:latest` 7,8 GB) |
| `just test` (`tests/image.sh` em `Tamagi-DxAI`) | **428 ok, 0 falhas** |
| `just build-skl` (`Tamagi-SKL` + `bootc container lint` + `rechunk`) | ok (`Checks passed: 14`, `localhost/tamagi-skl:latest` 6,7 GB) |
| `just test-skl` (`tests/image.sh` em `Tamagi-SKL`) | **393 ok, 0 falhas** |
| `tests/desktop.sh` (sessão COSMIC headless em Xvfb) | ok (6/6 processos COSMIC ativos) |
| `just test-installer-ui` (`tamagi-welcome --install`) | ok: **19 telas** no tema escuro e no claro + respostas conferidas |
| `just build-iso` (`output/imagens_iso/tamagi_nova.iso`) | ok (7,1 GB multi-edição: 4,1 GB OCI deduplicado + 2,9 GB squashfs) |
| `just test-iso` (QEMU/KVM unattended `Tamagi-DxAI`) | ok: 0 falhas (instalação 1 min, 6,1 GiB em disco, boot 22s / 2º boot 11s, SELinux Enforcing, 0 units falhando) |
| `just test-iso-skl` (QEMU/KVM unattended `Tamagi-SKL`) | ok: 0 falhas (instalação 1 min, 5,4 GiB em disco, boot 19s, conta `aluno` autologin sem sudo, painel travado `chattr +i`, homepage `https://escola.tamagi.test`, política 15d=360h, SSH + Cockpit ativos) |
| Cópia + `sha256sum -c` em `/run/media/wesdt/Extreme SSD/` | ok (`tamagi_nova.iso: OK`) |

> **Período de testes em máquina real (soak test):** toda a validação acima rodou em
> contêineres e VMs QEMU/KVM. A fase atual é **testar a v1.5 em hardware físico real**
> durante uso diário e coletar achados para alimentar o ciclo da **v2.0 (Tachico B)** —
> veja [ROADMAP-2.0.md](./ROADMAP-2.0.md).

---

## Checklist de Testes em Máquina Real (v1.5 Soak Test)

### A. Instalação e Boot (Pendrive Real)
1. Grave a ISO:
   ```bash
   sudo bash live/write-usb.sh /dev/disk/by-id/usb-<seu-pendrive> --yes
   ```
   (ou grave a partir de `/run/media/wesdt/Extreme SSD/tamagi_nova.iso`).
2. Confirme no menu GRUB do pendrive as duas opções: **Tamagi-DxAI** e **Tamagi-SKL**.
3. Verifique boot rápido no modo Live sem travar em `ldconfig.service`, Wi-Fi/áudio/vídeo funcionando e aviso de Secure Boot caso esteja ligado.
4. Instale com **LUKS2 + Chip TPM + PIN** (ou sem criptografia em máquina de teste) e meça o tempo de boot com `systemd-analyze`.

### B. Rotina `Tamagi-DxAI` (Uso Diário, IA, Armazenamento e Jogos)
- [ ] **Sistema e saúde:** `systemctl --failed` vazio, `tamagi sys doctor` verde, `tamagi sys perf status` (testar perfil `ai` com `scx_rustland` e `latency` com `scx_rusty`).
- [ ] **CLI 10 comandos + novidades:** testar `tamagi why /usr/bin/bash`, `tamagi why 9090`, `tamagi ai run --shadow claude` (ou outro agente) e `tamagi ai flights list`.
- [ ] **First Login (`tamagi-welcome`):** multi-seleção dos apps Rust (`Papers`, `Cedilla`, `Chronos`, `Rnote`, `Key Rack`, `Snapshot`, `Warp`, `Impression`, `Kooha`, `Authenticator`, `Sniffnet`).
- [ ] **Armazenamento e rede:** abrir `COSMIC Storage` (`cosmic-ext-storage`) + `rclone`, `btm` (`Super+Shift+T`), `tamagi shield tailscale on|off`, e WebApp **Lark Suite**.
- [ ] **`tamagi install` expandido:** testar busca unificada (catálogo + Flathub), `tamagi install --native <pkg>`, e instalar opcionalmente `steam`, `rocm` ou driver de vídeo conforme a GPU da máquina.
- [ ] **Hardware real:** suspensão/retorno (suspend/resume), múltiplos monitores, escala fracionária, Bluetooth, áudio PipeWire, aceleração de vídeo no Chromium e temperatura/fan.

### C. Rotina `Tamagi-SKL` (Piloto Escolar / Laboratório)
- [ ] No boot do pendrive, escolher **Tamagi-SKL**; no instalador, configurar conta `admin`, conta `aluno` e URL inicial personalizada da escola.
- [ ] Confirmar após reiniciar: **login automático em `aluno` sem senha**, sem `sudo`, painel COSMIC minimalista **travado** (aluno não consegue mover/editar), sem `cosmic-store`.
- [ ] Confirmar abertura automática do Chromium na página da escola + atalhos **Google Docs, Planilhas, Apresentações e Gemini** + reprodução de vídeo/áudio (YouTube/Meet/Classroom).
- [ ] Testar política de privacidade: logar em um site, fechar o Chromium, reabrir → **login destruído e senha não salva**, mas **histórico mantido** (expiração configurada em 15 dias / 360h).
- [ ] De outro computador na mesma rede, testar monitoramento remoto:
  - `ssh <admin>@<ip>` → `sudo tamagi-skl status`, `sudo tamagi-skl history 15`, `sudo tamagi-skl homepage <url>`, `sudo tamagi-skl open <url>`, `sudo tamagi-skl reset-session`.
  - Cockpit web em `https://<ip>:9090` com a conta `admin`.

---

## Resumo do que entrou na v1.5 (`feat/v1.5-dxai-skl`)

1. **Duas edições na mesma ISO (`Tamagi-DxAI` e `Tamagi-SKL`):** construídas pelo mesmo `Containerfile` (`ARG TAMAGI_EDITION=dxai|skl`) e deduplicadas no `tamagi-oci/` da ISO.
2. **CLI `tamagi` reduzida para 10 comandos (sem aliases legados):** `ai`, `dev`, `install`, `update`, `rewind`, `layer`, `vm`, `shield`, `sys`, `why`.
3. **2 recursos únicos na CLI:** `tamagi why` (inspetor estático de proveniência em Rust) e `tamagi ai flights` (caixa-preta de auditoria para agentes em Shadow CoW + Egress Guard).
4. **Dieta + upgrades nativos no `Tamagi-DxAI`:** `cosmic-ext-storage` + `rclone` nativos no lugar de `gnome-disks`, `bottom` (`btm`) no lugar de `btop`, `tailscale` pré-instalado (desligado por padrão), base gaming Bazzite (`steam-devices`, `gamescope`, `mangohud`, `gamemode`, Mesa/Vulkan 64+32-bit), `scx_rustland` (perfil `ai`), Lark Suite WebApp pré-instalado e wallpapers enxutos (9 PNGs 2560px).
5. **Edição escolar `Tamagi-SKL`:** 2 contas (`admin` + `aluno` autologin sem sudo), painel minimalista travado (`chattr +i`), Chromium gerenciado (sem OAuth2/ChromeDriver, sem salvar senhas, limpa cookies ao fechar, histórico 15 dias), WebApps Google Docs/Planilhas/Apresentações/Gemini, SSH restrito ao grupo `wheel` + Cockpit + `tamagi-skl`.
6. **Correção crítica pega em VM real:** `HOME` indefinido sob systemd em `/usr/libexec/tamagi/i18n` e `/usr/libexec/tamagi/firstboot` (que abortava `tamagi-skl homepage` e `panel-lock` no 1º boot).

---

## Próximo Ciclo

O planejamento completo da próxima versão — **Tamagi 2.0 ("Tachico B")**, unindo foco em IA (Bluefin AI + inovação Tamagi), estabilidade/supply-chain nível TunaOS e evolução do COSMIC (Epoch 2 & 3) — está em **[ROADMAP-2.0.md](./ROADMAP-2.0.md)**.
