# ROADMAP — Tamagi 2.0 ("Tachico B")

**Status:** Planejamento aprovado para início após período de testes em máquina real da v1.5 (`Tamagi-DxAI` + `Tamagi-SKL`).  
**Codinome:** **Tachico B**  
**Direção estratégica:** Manter a identidade enxuta e agêntica do Tamagi (equilíbrio Aeon + Bluefin Dakota), combinando **três pilares**:
1. **IA Local e Agêntica aberta e sem atrito** (inspirada na arquitetura do [Bluefin AI](https://docs.projectbluefin.io/ai/) + inovações nativas do Tamagi).
2. **Estabilidade, determinismo e cadeia de suprimentos nível [TunaOS](https://github.com/tuna-os/tunaos-packages/blob/main/ROADMAP.md)**.
3. **COSMIC de próxima geração** acompanhando e potencializando o [COSMIC Epoch 2 e Epoch 3](https://system76.com/blog/post/cosmic-epoch-2-and-3-roadmap).

---

## Fase 0 — Período de Estabilização da v1.5 em Hardware Real (Agora)

Antes de abrir novas funcionalidades na 2.0, a v1.5 (`feat/v1.5-dxai-skl`) roda em máquina real (ver checklist em [RETURN.md](./RETURN.md)). Só abrimos a branch da 2.0 quando fecharmos os achados reais de campo:

| Critério de saída da Fase 0 | Meta |
|---|---|
| Uso diário em hardware real (`Tamagi-DxAI`) | Zero regressões críticas em boot, suspend/resume, GPU/Vulkan, áudio, Wi-Fi/BT, `bootc upgrade` e `tamagi-layer` |
| Piloto em máquina real (`Tamagi-SKL`) | Validar autologin `aluno`, painel travado em uso contínuo, vídeo/áudio em aulas (YouTube/Meet/Classroom) e gestão remota via SSH/Cockpit |
| Lista fechada de dores reais do COSMIC + hardware | Registrar falhas concretas de compositor, escala fracionária, multi-monitor, impressoras ou jogos para priorizar nos marcos da 2.0 |

---

## Pilar 1 — IA Local e Agêntica (Foco Principal)

**Referência:** [Bluefin AI (`docs.projectbluefin.io/ai/`)](https://docs.projectbluefin.io/ai/) — modelo *"Bring Your Own LLM"*, padrões abertos da Agentic AI Foundation (AAIF) e CNCF, aceleração de GPU automática e ferramentas estilo RHEL Lightspeed (`linux-mcp-server`).

### 1.1 Endpoint Local Unificado e Aceleração Automática de Hardware
- **API OpenAI-compatível unificada (`127.0.0.1:9931`):** manter `llama.cpp` (Vulkan + CPU multi-variante) como padrão zero-overhead no host e adicionar **RamaLama** (via Podman) como backend opcional gerenciado por `tamagi ai local` para rodar modelos GGUF/OCI em contêiner com detecção automática de GPU (AMD ROCm, NVIDIA CUDA, Intel Vulkan) sem sujar o sistema base.
- **Suporte real a NPUs (AMD Ryzen AI `amdxdna` e Intel Core Ultra `ivpu`):** já presentes no kernel OGC da v1.5 — expor detecção em `tamagi sys doctor` e perfil de inferência assistido em `tamagi ai local recommend`.
- **Troca limpa Local ↔ Cloud ("Bring Your Own LLM"):** perfil único em `~/.config/tamagi/ai.toml` que sincroniza automaticamente provedor/endpoint entre `tamagi ai ask`, agentes CLI e servidores MCP.

### 1.2 Agentes da Agentic AI Foundation + "Tamagi Lightspeed" (`linux-mcp-server`)
- **Novos agentes curados no catálogo (`tamagi ai agents`):**
  - **Goose (`block-goose-cli`)** — agente principal open-source da AAIF com suporte nativo a MCP e modelos locais/remotos.
  - **Crush** (Charm) e **Mistral Vibe** como opções de terminal além de Claude Code, Codex, Gemini CLI, Antigravity (`agy`), Qwen Code, OpenCode e CLIO.
- **`tamagi-linux-mcp` (inspirado no `linux-mcp-server` / Bluespeed):** servidor MCP **estritamente somente-leitura** rodando em Podman Quadlet (`--cap-drop=ALL`, 0 MB idle) que expõe diagnósticos estruturados (`tamagi sys doctor --json`, `tamagi why`, `journalctl` filtrado, `bootc status`, métricas `sched_ext`) para qualquer agente (Goose, Claude, Gemini, local `llama.cpp`) investigar falhas do sistema com segurança.
- **Podman AI Lab opcional no catálogo:** para quem prefere interface gráfica local para testar modelos e playgrounds sem pesar na imagem base.

### 1.3 Evolução das Inovações Exclusivas do Tamagi
- **`tamagi ai flights` v2 (Caixa-Preta de Agentes):** adicionar resumo de diff por arquivo, domínios bloqueados/permitidos pelo Egress Guard, duração, código de saída e comando `tamagi ai flights replay <id>` / exportação em Markdown/JSON para auditoria.
- **Shadow Workspaces CoW paralelos:** permitir rodar 2+ agentes simultâneos em sombras Btrfs distintas sobre o mesmo repositório (`tamagi ai run --shadow --name a1 claude`, `--name a2 codex`) com comparação lado a lado (`tamagi ai shadow compare a1 a2`) antes do `apply`.
- **Egress Guard declarativo por projeto:** ler seção `[egress]` de `tamagi-workspace.toml` para permitir domínios extras apenas naquele projeto (ex.: staging privado da empresa) sem abrir a allowlist global.

---

## Pilar 2 — Estabilidade e Cadeia de Suprimentos Nível TunaOS

**Referência:** [TunaOS Packages Roadmap (`tuna-os/tunaos-packages/ROADMAP.md`)](https://github.com/tuna-os/tunaos-packages/blob/main/ROADMAP.md) — fábrica única determinística, promoção exata dos bytes aprovados no portão (*promote what you gate*), zero COPRs não-fixados, paridade por edição como gate automático, SBOM/SLSA e verificação de *upstream shadowing*.

### 2.1 Promote-What-You-Gate & Canais de Atualização (`stable` / `testing`)
- **Construir uma vez, testar, promover o mesmo digest:** no CI (GitHub Actions / GHCR), gerar a imagem candidata (`:testing`), rodar toda a bateria (`lint` → `test` → `test-skl` → `test-installer-ui` → `test-iso` → `test-iso-skl` → `test-upgrade`) e **re-taguear/assinar exatamente o mesmo digest SHA-256** para `:stable` — nunca reconstruir na publicação.
- **Canais explícitos (`stable` e `testing`):** `Tamagi-DxAI` e `Tamagi-SKL` publicam canais separados (`:dxai-stable`, `:dxai-testing`, `:skl-stable`, `:skl-testing`), com troca atômica via `tamagi sys setup switch-image`.

### 2.2 Portão de Paridade por Edição e Detector de Upstream Shadowing
- **Contrato de fechamento por edição (Package Closure Gate):** substituir checagens ad-hoc por manifestos verificados em CI que garantem o fechamento exato de cada edição (ex.: `Tamagi-SKL` nunca recebe acidentalmente `cosmic-store`, `chromedriver`, `llama-server` ou `libvirt`; `Tamagi-DxAI` nunca perde `cosmic-ext-storage`, `rclone`, `btm`, `gamescope` ou `bootc` lint).
- **Detector de *Upstream Shadowing*:** script de CI que compara qualquer pacote ou binário vindo de fora dos repositórios oficiais do Fedora contra a versão upstream do Fedora, alertando imediatamente se algum pin nosso ficar atrás de uma correção de segurança.
- **Aposentadoria de COPRs não-fixados:** migrar qualquer dependência restante de COPR pessoal (como `lact` ou `hardening-extra` no catálogo opcional) para releases oficiais com SHA-256/assinatura verificada ou pacotes promovidos com atestação própria.

### 2.3 Assinatura, SBOM/SLSA, Rotação de Chaves e Secure Boot Completo
- **SBOM + SLSA Provenance em cada release:** anexar SBOM (Syft/SPDX) e atestação SLSA assinados via `cosign` às imagens OCI (`DxAI` e `SKL`) e à ISO multi-edição.
- **Documentação de rotação de chaves e *disaster recovery*:** formalizar procedimento de rotação entre `tamagi.pub` / `tamagi-2.pub` (já suportado na `policy.json`) com teste automatizado de rollback e quarentena.
- **Secure Boot assinado de ponta a ponta:** viabilizar boot com **Secure Boot LIGADO** no sistema instalado (assinatura de `systemd-boot`/UKI ou cadeia `shim` + MOK do Tamagi + integração automática com TPM PCR 7), removendo a restrição de desligar Secure Boot da v1.5.
- **Determinismo verificado (`SOURCE_DATE_EPOCH`):** fixar timestamps e validar em CI que dois builds com os mesmos insumos não geram divergência indevida nas camadas do `rechunk`.

### 2.4 Estabilidade Operacional do `Tamagi-SKL` (Laboratórios e Escolas)
- **Gestão de frota leve:** suporte opcional a um inventário declarativo (`skl-fleet.toml`) para o professor/TI aplicar `homepage`, `open`, `history`, `lock`/`unlock` e janelas de atualização em 10–40 máquinas via SSH/Tailscale de uma só vez.
- **Reset automático de sessão por inatividade / troca de turma:** timer opcional (configurável em `/etc/tamagi/skl.conf`) que limpa cookies/login entre aulas mantendo o histórico auditável de 15 dias (360h).

---

## Pilar 3 — Melhorias no COSMIC (Alinhado ao Epoch 2 e Epoch 3)

**Referência:** [System76 COSMIC Epoch 2 and 3 Roadmap](https://system76.com/blog/post/cosmic-epoch-2-and-3-roadmap) — foco em desempenho do `libcosmic`/Iced, polimento visual, protocolos Wayland, HDR nativo, acessibilidade e apps nativos em Rust.

### 3.1 Adoção Curada do COSMIC Epoch 2 (Curto/Médio Prazo)
- **Rebase reativo do `libcosmic`/Iced (-60% a -80% de CPU + menor VRAM):** atualizar `tamagi-welcome` e `cosmic-ext-storage` para a nova base reativa do `libcosmic` assim que estabilizar no Fedora, reduzindo consumo de bateria em notebooks e latência de UI.
- **Polimento visual com sobriedade Material 3 / Ipê:**
  - Adotar **Frosted Glass (blur)** e **Vulkan Renderer** do `cosmic-comp` com opção leve (*Reduced Motion / No Blur* automático no `Tamagi-SKL` e em GPUs integradas antigas).
  - Integrar **CuteCOSMIC Qt6 Platform Theme** na imagem `DxAI` para que apps Qt6 sigam automaticamente a paleta **Ipê / Ipê Claro** e as fontes `Roboto Flex` + `JetBrains Mono`.
- **Produtividade e fluxos nativos:**
  - Habilitar e documentar `COSMIC Clipboard Manager`, **Workspaces nomeados**, atalhos padrão do Epoch 2 (`Super+D`, `Alt+\``, `Alt+Space`), diálogo nativo de **Impressoras**, **Firmware Manager** e **Tiling Window Exceptions** diretamente no COSMIC Settings.
  - Avaliar substituição de ferramentas externas quando os nativos do Epoch 2 atingirem paridade real (`COSMIC Screenshot` com vídeo/marcação vs `tamagi-capture`; `COSMIC Image Viewer` e `COSMIC Reader` vs Flatpaks Loupe/Papers).

### 3.2 Adoção Curada do COSMIC Epoch 3 (Alvo Final da 2.0 "Tachico B")
- **HDR nativo no `cosmic-comp` + Night Light + Restore Session:**
  - Com HDR nativo no compositor, jogos e mídia rodam em HDR diretamente dentro da sessão COSMIC (simplificando o fluxo do `tamagi install gaming` / `gamescope`).
  - **Restore Session** opcional para dev workstations (reabrir janelas e posições após reboot de `bootc upgrade`!).
  - **SVG Cursors** com tamanho ajustável e recursos de acessibilidade (legendas ao vivo, flash notifications, sticky/slow keys).
- **Theming inteligente & Applets dinâmicos:**
  - Aproveitar **Hotloading de Applets** do `cosmic-panel` e criar um **applet nativo Tamagi em `libcosmic`** (leve, opcional na barra) para status em 1 clique: modelo local `llama.cpp`, voos/shadows de agentes ativos, leases temporários com contagem regressiva e perfil `sched_ext` (`ai`/`balanced`/`latency`).
  - Integrar **Auto-match de accent/tema pelo wallpaper** com a nossa paleta Ipê e manter sincronia com `Ptyxis`, `foot` e Neovim.
- **Enxugamento adicional de dependências GTK/Python:**
  - **Migrar `tamagi-menu` (`Super+A`) de Python+GTK4 para `libcosmic` puro (Rust):** elimina o runtime GTK4/libadwaita/PyGObject da paleta de comandos, corta o tempo de abertura para <30 ms e alinha 100% da UI própria do Tamagi em `libcosmic`.
  - Quando a **GUI nativa de permissões Flatpak** (`xdg-permission-store`) e o **`COSMIC Keyring`** amadurecerem no Epoch 3, reavaliar a remoção de `Flatseal` dos essenciais e enxugar ainda mais a imagem base.

---

## Cronograma em Marcos — Tamagi 2.0 ("Tachico B")

| Marco | Foco | Entregáveis concretos |
|---|---|---|
| **M0 — Soak Test v1.5** | Hardware real | Relatório de bugs/ajustes de máquina real (`DxAI` + `SKL`), correções pontuais v1.5.x |
| **M1 — Base TunaOS** | Estabilidade & CI | Pipeline *promote-what-you-gate* (`:testing` → `:stable`), Package Closure Gate por edição, detector de *upstream shadowing*, SBOM + SLSA + rotação de chaves |
| **M2 — COSMIC Epoch 2 + Rust UI** | Desktop & polimento | `libcosmic` reativo em `tamagi-welcome`/`cosmic-ext-storage`, Frosted/Vulkan renderer, CuteCOSMIC Qt6, **`tamagi-menu` reescrito em `libcosmic`**, revisão de apps nativos Epoch 2 |
| **M3 — IA 2.0 (Bluefin AI + Tamagi)** | IA local & agêntica | `tamagi-linux-mcp` (read-only Lightspeed), Goose + RamaLama opcional, detecção NPU (`amdxdna`/`ivpu`), `tamagi ai flights` v2, Shadow CoW paralelo e Egress Guard por projeto |
| **M4 — Tachico B RC & Release** | Epoch 3 + Secure Boot | Secure Boot assinado ponta a ponta (TPM PCR 7), COSMIC Epoch 3 (HDR nativo, Night Light, Restore Session, applet Tamagi hotload), `Tamagi-SKL` fleet CLI, ISO multi-edição 2.0 validada em VM + hardware real |

---

## Critérios de Aprovação da 2.0 ("Tachico B" Release Gate)

Uma release da 2.0 só é promovida para `:stable` se cumprir **todos** os critérios abaixo:
1. `just lint`, `just test` (`DxAI`), `just test-skl` (`SKL`), `just test-desktop`, `just test-installer-ui`, `just test-iso`, `just test-iso-skl`, `just test-preserve-home` e `just test-upgrade` passando com **0 falhas** no mesmo digest promovido.
2. **Package Closure Gate** verde para `Tamagi-DxAI` e `Tamagi-SKL`, com **0 alertas** de *upstream shadowing* sem justificativa explícita.
3. Imagens OCI e ISO assinadas (`cosign`) com **SBOM + atestação SLSA** verificáveis.
4. Validação documentada em **hardware físico real** (não apenas VM) para ambas as edições (`DxAI` + `SKL`), incluindo Secure Boot + TPM+PIN e atualização atômica entre dois releases reais.

---

## Governança deste Roadmap (Regra de Atualidade)

Inspirado na regra `#506` do TunaOS: sempre que um marco ou item acima for concluído, cancelado ou substituído, sua linha deve ser atualizada no **mesmo PR/commit** que entrega a mudança — mantendo este arquivo sempre como espelho fiel do estado real do projeto.
