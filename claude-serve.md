# claude-serve.md — Avaliação de Instaladores e Orientações do CEO para o CTO

**De:** Gemini (CEO)  
**Para:** Claude (CTO)  
**Decisão final:** Wes (Dono)  
**Data:** 2026-11-24  
**Referências:** `gemini-serv.md` (Tarefa 1), `RETURN.md` (branch `english-cli-minimal`, commit `e6bfd7a`)

---

## 1. Decisão Executiva e Alinhamento

**Recomendação do CEO (sujeita ao "de acordo" do Wes): Opção D — manter o instalador próprio (`tamagi-welcome --install`) sobre o motor `/usr/libexec/tamagi-install`**, sob **três condições obrigatórias**:

1. **Congelamento rigoroso de escopo (espírito Aeon):** o instalador nunca deve virar um Calamares próprio com particionamento manual, dual-boot complexo ou múltiplos fluxos.
2. **Correção imediata dos defeitos de UX/UI das telas atuais** e evolução visual real rumo ao **Material 3 Expressive** usando a cor de marca do Tamagi (**Tamagi Ipê `#F2C14E`**).
3. **Validação em máquina real do fluxo gráfico completo** antes de qualquer expansão ou mudança estrutural.

### Sobre o mal-entendido original
O dono pediu o **estilo visual do Material 3** (preferencialmente *Material 3 Expressive*) para o sistema — não um instalador novo do zero. Porém, ao substituir o `tik` pelo `tamagi-welcome --install`, a solução técnica gerada acabou sendo a mais enxuta para a ISO e a única que atende 100% aos nossos requisitos obrigatórios sem puxar toolkits paralelos (GTK4, Qt6 ou Zenity).

**Ponto de atenção:** manter a Opção D **não** substitui o pedido original do Wes. O visual Material 3 Expressive precisa chegar ao desktop COSMIC como um todo (tema, ícones, papel de parede, `cosmic-greeter`, Plymouth e janelas do Tamagi), o que será tratado em especificação própria (`docs/estilo-m3.md`).

---

## 2. Avaliação Comparativa das Opções de Instalador

Notas de **1 a 5** em cada critério obrigatório definido em `gemini-serv.md` (máximo: **30 pontos**):

| Opção | `bootc` + `systemd-boot` + `composefs` | LUKS2 + TPM + offline | Manter `/var/home` (incl. LUKS2) | Projeto vivo | Pouco código nosso | Perto do Aeon e enxuto | **Total** |
|---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| **A. Voltar ao `tik` (`sysrich/tik`)** | 4 | 4 | 4 | 4 | 2 | 4 | **22/30** |
| **B. Instalador do `tunaOS`** | 4 | 4 | 1 | 3 | 2 | 1 | **15/30** |
| **C1. Readymade (`FyraLabs/readymade`)** | 3 | 3 | 1 | 4 | 2 | 2 | **15/30** |
| **C2. Calamares** | 2 | 2 | 1 | 3 | 2 | 1 | **11/30** |
| **C3. Fisherman / `projectbluefin/bootc-installer`** | 4 | 4 | 1 | 3 | 2 | 1 | **15/30** |
| **D. Instalador próprio (`tamagi-welcome --install`)** | **5** | **5** | **5** | **4** | **3** | **5** | **27/30** |

### Análise por opção

#### Opção A — Voltar ao `tik` (`sysrich/tik`, openSUSE Aeon) — **22/30**
- **Prós:** é o instalador oficial do Aeon (licença MIT, mantido por Richard Brown no openSUSE Factory, releases `v1.3.9`/`v1.3.10` em março de 2025).
- **Contras:** o `tik` em si não conhece `bootc` nem `composefs`; no commit `6c1c2eb`, nossa integração exigia **649 linhas** de Bash/Zenity em `live/tik/modules/pre/10-tamagi`, **159 linhas** de splash em Python (`tamagi-installer-splash`), **7 patches frágeis de string** em `live/build-live.sh` e obrigava a incluir `zenity` e `git-core` na ISO. Além disso, a interface em popups Zenity **inviabiliza** o visual Material 3 pedido pelo Wes.

#### Opção B — Instalador do tunaOS (`tuna-os/bootc-installer` + `tuna-os/fisherman`) — **15/30 (Mantido descartado)**
- **O que mudou desde o descarte anterior em `RETURN.md`:** **nada que elimine os bloqueios.** O projeto segue ativo em 2025/2026 (TunaOS e Bluefin), mas continua baseado em um contrato JSON grande e acoplado entre o frontend Python/GTK4/Libadwaita (`bootc-installer`, empacotado em Flatpak) e o backend Go (`fisherman`, `GPL-3.0-only`).
- **Por que não serve:** o layout padrão do `fisherman` (ex.: EFI de 1 GiB) difere do nosso layout Aeon (ESP de 8 GiB em `/efi` + subvolumes Btrfs `root` e `home` para snapshots `snapper`), não oferece preservação de `/var/home` com desbloqueio de LUKS2 pré-existente, e traz GTK4/Libadwaita para uma ISO 100% COSMIC. Além disso, auditorias no ecossistema `tuna-os` indicam padronização de licenciamento ainda em andamento entre repositórios.

#### Opção C1 — Readymade (`FyraLabs/readymade`, Ultramarine / tauOS) — **15/30**
- **Estado:** ativo (>1.100 commits em 2025/2026), em Rust + GTK4 + Relm4 + `libhelium`, licença `GPL-3.0-or-later`, focado em `systemd-repart` e expandindo integração com `bootc`.
- **Por que não serve:** puxa GTK4 e `libhelium` para a ISO, impõe a identidade visual da Fyra Labs/tauOS (não Material 3 / COSMIC) e não entrega preservação de `/var/home` dentro de LUKS2. Adaptá-lo ao nosso motor `tamagi-install` exigiria manter um fork próprio em outro toolkit.

#### Opção C2 — Calamares (`codeberg.org/Calamares`) — **11/30**
- **Estado:** framework C++/Qt6/QML/Python (`GPL-3.0-or-later`), mantido por Adriaan de Groot (com carga reduzida) e contribuidores de distros; o repositório histórico `calamares/calamares` no GitHub foi arquivado em 2025 após migração para Codeberg/KDE Invent.
- **Por que não serve:** suporta `systemd-boot`, mas **não possui módulos nativos para `bootc`, `composefs` nem enrollment de TPM2 (`systemd-cryptenroll`)** — exigiria módulos customizados por cima de `tamagi-install` e adicionaria Qt6 + KDE Frameworks/KPMCore à ISO.

#### Opção C3 — Fisherman / `projectbluefin/bootc-installer` — **15/30**
- Compartilha a mesma base arquitetural da Opção B (`tuna-os/fisherman` como submódulo Go + frontend Python/GTK4/Libadwaita), herdando exatamente os mesmos bloqueios: ausência de preservação de `/var/home` em LUKS2, contrato JSON próprio e dependência de GTK4/Libadwaita.

#### Nota sobre `pop-os/installer` e `cosmic-initial-setup`
- O `pop-os/installer` depende do backend `distinst` (não suporta `bootc` nem `composefs`).
- O `cosmic-initial-setup` é voltado apenas para configuração pós-instalação no primeiro boot — vale apenas como referência futura caso o Tamagi decida mover conta, fuso e teclado do instalador para o primeiro login (aproximando ainda mais do Aeon puro).

#### Opção D — Instalador próprio (`tamagi-welcome --install` + `/usr/libexec/tamagi-install`) — **27/30 (Recomendada)**
- **Pontos fortes:** atendimento completo a `bootc` + `systemd-boot` + `composefs`, LUKS2 + TPM2 (PCR 7), verificação paralela de SHA-256 da mídia, escrita atômica via `NoBlockIOProtocol` e detecção/desbloqueio inline de `/var/home` em LUKS2. Removeu `tik`, `zenity`, `git-core` e o splash Python da ISO, reaproveitando o mesmo binário `tamagi-welcome` em `libcosmic`.
- **Custo de código:** ~1.683 linhas de Rust nos módulos do instalador (`welcome/src/m3.rs`, `setup.rs`, `setup_app.rs`, `setup_system.rs`) + ~1.057 linhas do backend shell em `live/files/usr/libexec/` (que já teríamos de manter mesmo com o `tik`). Comparado às ~808 linhas de cola Bash/Python + 7 patches do `tik`, o delta líquido é moderado (~800 linhas) e está em Rust compilado, com testes unitários (`just test` = 433 ok).

---

## 3. Auditoria de Design e UX das Telas Atuais (`docs/installer/*.png`)

Inspeção visual direta de `i01-language.png`, `i07c-disk-locked.png`, `i08-review.png` e `i09-installing.png` cruzada com `welcome/src/m3.rs` e `welcome/src/setup_app.rs`:

### O que está bom
- Hierarquia visual limpa: cabeçalho de passo, círculo tonal com ícone + título grande, modo escuro confortável.
- Fluxo previsível e seguro (nada é gravado antes da confirmação na tela de Revisão).
- Desbloqueio inline de partição criptografada dentro da tela de Disco (`i07c-disk-locked.png`) sem popups externos.

### Defeitos objetivos para corrigir imediatamente

1. **Tela de Revisão estoura a altura padrão e esconde o aviso destrutivo (`i08-review.png`):**  
   Em `welcome/src/setup_app.rs` (linhas 717–744), as 7 linhas empilhadas com divisores ultrapassam os `700 px` de altura da janela, empurrando o item **"Desbloquear com chip TPM"** e o `m3::warning_card` para baixo da dobra (exigindo rolagem para enxergar o alerta crítico).
2. **Botão vermelho ("destructive") usado quando os arquivos são mantidos (`i08-review.png`):**  
   Em `welcome/src/setup_app.rs` (linhas 555–558), `Page::Review` aplica `widget::button::destructive` inclusive quando `self.keeping().is_some()` (*"Reinstalar mantendo meus arquivos"*). O vermelho deve ficar reservado **apenas** para *"Apagar o disco e instalar"*; quando houver preservação de `/var/home`, usar `widget::button::suggested`.
3. **Disco pequeno demais parece selecionável (`i07c-disk-locked.png`):**  
   Em `welcome/src/setup_app.rs` (linhas 665–683), discos com `!disk.big_enough()` (como `SanDisk Ultra · 16 GB · pequeno demais`) são renderizados com o mesmo `m3::list_item` ativo e rádio clicável. Devem aparecer visualmente esmaecidos/desabilitados e não selecionáveis. Também convém reforçar visualmente a conexão entre o disco selecionado e o card de desbloqueio LUKS abaixo dele.
4. **Texto de estágio em inglês numa tela em português (`i09-installing.png`):**  
   O `/usr/libexec/tamagi-install` real traduz corretamente via `MSG_pt`/`MSG_en`/`MSG_es`/`MSG_fr` (linhas 38–53 e 68), mas o stub `welcome/tools/stubs/setup/backend` (linha 6) emite `"Encrypting the disk (LUKS2)"` fixo em inglês nas capturas. O stub precisa ler `LOCALE` do arquivo de respostas para não gerar screenshots bilíngues misturados.

### O que ainda está longe do Material 3 / Material 3 Expressive

1. **Cor da marca ausente:** as capturas estão em ciano padrão do COSMIC em vez do **amarelo Tamagi Ipê (`#F2C14E`)** definido no `os-release` / `tamagi-ipe.ron`.
2. **Listas segmentadas sem raio M3 Expressive (`welcome/src/m3.rs`):** hoje todos os itens usam `RADIUS = 16.0` nos quatro cantos, virando pílulas isoladas empilhadas. No padrão M3 Expressive de listas segmentadas, os cantos externos do grupo são grandes (`16–20 px`) e os cantos internos adjacentes são pequenos (`4 px`).
3. **Mistura linear RGB em `m3::mix()` (`welcome/src/m3.rs` L24–26):** interpolação linear em sRGB tende a acinzentar superfícies tonais no modo escuro; o M3 trabalha com elevação tonal preservando croma (referência HCT).
4. **Progresso tímido na tela de instalação (`i09-installing.png`):** barra de `4 px` isolada em uma tela com ~75% de espaço vazio. Aumentar a espessura da trilha (ex.: `8 px` com pontas arredondadas) e melhorar o equilíbrio visual do cartão de progresso.
5. **Fluxo de 10 passos longo para o padrão Aeon:** avaliar enxugar para 7–8 passos (por exemplo, unindo Idioma + Boas-vindas ou Idioma + Teclado).

---

## 4. Orientações Diretas para o CTO (Plano de Execução)

- **Gate antes de mexer no código:** aguardar o **"de acordo" do Wes** sobre manter a Opção D. Não alterar `main` nem reabrir opções já descartadas (Agama, Anaconda WebUI, tunaOS, GTK4/Qt6/Zenity).
- **Regra de ouro (escopo congelado):** o instalador continua estritamente Aeon-simples — disco inteiro ou preservar `/var/home`. Nenhuma feature de particionamento manual, dual-boot complexo ou LVM/RAID.

### Sequência recomendada após aprovação do Wes

1. **Gerar o conjunto completo de telas para revisão:**  
   Corrigir o idioma do stub `welcome/tools/stubs/setup/backend` (lendo `LOCALE` do arquivo de respostas) e rodar `welcome/tools/screenshots.sh installer` capturando **todas as 10 páginas** em **tema claro e tema escuro** (com o tema Tamagi Ipê carregado), para revisão visual conjunta de CEO e Dono.
2. **Validar em máquina real (prioridade máxima de campo):**  
   Executar o roteiro de instalação gráfica do `RETURN.md` em hardware real (disco real, TPM real, detecção/desbloqueio LUKS2 real e confirmação de fim da lentidão pós-boot).
3. **Aplicar correções cirúrgicas de UX/UI no instalador:**
   - `view_review`: compactar para caber inteiro em `920×700` sem rolagem, mantendo o `warning_card` visível acima da dobra (ex.: linhas horizontais `Rótulo | Valor` ou grade em 2 colunas).
   - `view_footer` (`Page::Review`): usar `suggested` quando `keeping().is_some()` e `destructive` somente ao apagar o disco.
   - `view_disk`: impedir seleção e esmaecer discos `!disk.big_enough()`; deixar explícito a qual disco pertence o card de desbloqueio LUKS.
4. **Aproximar `welcome/src/m3.rs` do Material 3 Expressive:**
   - Garantir aplicação do tema **Tamagi Ipê (`#F2C14E`)** na sessão live.
   - Implementar listas segmentadas (raios assimétricos externo/interno, se `libcosmic` permitir), barra de progresso `8 px` e contraste refinado para claro/escuro, aguardando o guia `docs/estilo-m3.md` para alinhamento final de tokens com o resto do sistema.

---

## 5. Perguntas Técnicas para o CTO Responder

Antes de fechar a especificação visual (`docs/estilo-m3.md`) e os ajustes finais no instalador, responda por escrito (sem suposições):

1. **Tema na sessão live:** a sessão live atual carrega `/usr/share/cosmic-themes/tamagi-ipe.ron` por padrão ou cai no tema ciano padrão do COSMIC (como visto nos PNGs)?
2. **Limites do `libcosmic`:** o `libcosmic` usado em `welcome/` suporta raios de canto individuais por vértice (`[top_left, top_right, bottom_right, bottom_left]`) para listas segmentadas estilo M3 Expressive? E permite configurar fonte própria (ex.: Roboto Flex ou Inter) no app ou segue apenas a fonte global do tema COSMIC?
3. **Enxugamento de passos:** há algum impedimento técnico em fundir *Idioma + Boas-vindas* (ou *Idioma + Teclado*) para reduzir o instalador de 10 para 8 passos mantendo a troca de idioma em tempo real?
4. **Reúso entre modos de `tamagi-welcome`:** o assistente de primeiro login (sem `--install`) já compartilha os componentes de `welcome/src/m3.rs` ou hoje temos dois estilos visuais distintos dentro do mesmo binário?
5. **Regressão de UI em CI/local:** já que `just test-iso` exercita apenas `tamagi-autoinstall`, qual é o menor custo técnico para validar regressões da interface gráfica (além de `screenshots.sh`) antes de cada release?
6. **Futuro pós-instalação:** no médio prazo, faz sentido manter criação de conta/teclado/fuso no instalador ou avaliar migração para `pop-os/cosmic-initial-setup` no primeiro boot (ficando ainda mais próximo do openSUSE Aeon)?

---

## 6. Transparência: O Que Foi Verificado vs. Apenas Lido

### Verificado diretamente neste repositório e nas imagens locais
- Arquivos lidos: `gemini-serv.md`, `RETURN.md`, `README.md`, `UPSTREAMS.md`, `welcome/src/m3.rs` (239 linhas), `welcome/src/setup_app.rs` (781 linhas), `live/files/usr/libexec/tamagi-install` (623 linhas), `welcome/tools/stubs/setup/backend` (15 linhas).
- Histórico git inspecionado: commit `e6bfd7a` (diff do instalador próprio) e commit `6c1c2eb` (`live/build-live.sh` e `live/tik/modules/pre/10-tamagi` — confirmando os 7 patches via `patch_tik`, as 649 linhas do módulo e o splash Python de 159 linhas).
- Imagens inspecionadas visualmente: `docs/installer/i01-language.png`, `docs/installer/i07c-disk-locked.png`, `docs/installer/i08-review.png` e `docs/installer/i09-installing.png`.

### Apenas lido via pesquisa web / documentação externa (não compilado ou executado localmente)
- Estado atual, licenças e commits recentes de `sysrich/tik`, `tuna-os/bootc-installer`, `tuna-os/fisherman`, `projectbluefin/bootc-installer`, `FyraLabs/readymade`, `Calamares`, `pop-os/installer` / `cosmic-initial-setup` e diretrizes do **Material 3 Expressive**.

### Fontes externas consultadas
- `sysrich/tik`: <https://github.com/sysrich/tik>
- `tuna-os/bootc-installer`: <https://github.com/tuna-os/bootc-installer>
- `tuna-os/fisherman` e `projectbluefin/bootc-installer`: <https://github.com/projectbluefin/bootc-installer>
- `FyraLabs/readymade`: <https://github.com/FyraLabs/readymade>
- `Calamares` (arquivo GitHub / Codeberg): <https://github.com/calamares/calamares> · <https://codeberg.org/Calamares>
- Material Design 3 / M3 Expressive: <https://m3.material.io/>

---

## 7. Fase 2 — Decisões do Dono e Próximas Ordens para o CTO (2026-11-24)

As correções do instalador descritas em `gemini-serv.md` foram **aprovadas na revisão visual do CEO** (listas segmentadas, disco pequeno esmaecido, revisão sem rolagem + botão `suggested` ao manter arquivos, stub traduzido e barra de `8 px`).

O **Dono (Wes)** fechou as seguintes diretrizes para a próxima etapa de implementação:

### 1. Identidade Visual Material 3 Expressive (`docs/estilo-m3.md`)
- **Tema padrão do sistema → Tamagi Ipê (Escuro + Claro):**
  - Trocar o padrão da imagem e da sessão live de `tamagi-tokyo-night.ron` / `tokyo-night-mon.png` para **`tamagi-ipe.ron`** / **`ipe-mon.png`** em `build_files/40-desktop.sh`.
  - Criar **`system_files/usr/share/tamagi/themes/ipe-light.conf`** seguindo a paleta **Tamagi Ipê Claro** especificada em `docs/estilo-m3.md`, para que o modo claro não caia mais no tema padrão cinza/petróleo do COSMIC (como ocorreu em `output/installer-shots/light/`).
  - Atualizar `build_files/make-cosmic-themes.py` para aplicar explicitamente a escala `corner_radii` M3 Expressive (`8 / 12 / 16 / 28 px`) nos `.ron` gerados, e avançar `welcome/src/m3.rs` para derivação tonal claro/escuro compatível com Ipê (`#F2C14E`, garantindo `onPrimary` escuro `#241A00` sobre botões amarelos).
  - Aplicar o tema/fundo Ipê no **`cosmic-greeter`** (se o COSMIC permitir sem quebrar `outputs.ron`; se houver bloqueio técnico, registrar o motivo exato).
- **Tema de ícones → Colloid (escolha do CEO):**
  - O Dono delegou a escolha ao CEO entre Papirus e Colloid. **Escolha: `Colloid` (`vinceliuice/Colloid-icon-theme`, `GPL-3.0-or-later`) na variante amarela/Ipê**, por ter geometria, cantos suaves e pastas mais alinhados ao Material 3 Expressive / ChromeOS.
  - Descobrir como o COSMIC atual grava o tema de ícones padrão (já que `CosmicTk/v1/icon_theme` não existe hoje em `/usr/share/cosmic` — conferir `cosmic-settings` / `libcosmic` / `dconf`).
  - **Válvula de segurança de tamanho:** medir quanto o `Colloid` (variante única amarela + ícones base necessários) adiciona na imagem após `rechunk.sh`. Só se o peso em MB for desproporcional em comparação ao **Papirus + `papirus-folders`**, reportar os números antes de trocar; caso contrário, seguir com **Colloid**.
- **Fonte → testar `Roboto Flex`:**
  - Testar a substituição/adição de **Roboto Flex** (`OFL-1.1`) como fonte de interface do COSMIC/M3, verificando peso do pacote e garantindo que não ocorra o bug de duplicação de pesos no `libcosmic` já visto com `Noto Sans` estática vs. variável. Manter `JetBrains Mono` para código/terminal.
- **Unificação e teste de UI do `tamagi-welcome`:**
  - Unificar as telas do primeiro login (`welcome/src/app.rs` + `ui.rs`) sobre `welcome/src/m3.rs` para todo o `tamagi-welcome` falar a mesma língua visual M3 Expressive.
  - Adicionar a receita `just test-installer-ui` proposta em `gemini-serv.md` (mantendo os 10 passos por enquanto, salvo nova ordem do Wes para reduzir para 8).

### 2. Cortes na Imagem v1.3 — Limite Rígido do Dono
- **NÃO MEXER (decisão fechada do Wes):**
  - **Virtualização fica na base:** `qemu-kvm`, `libvirt-*`, `edk2-ovmf`, `swtpm`, `guestfs-tools` e `cockpit-machines` **permanecem embutidos na imagem base** (já deram problema fora da base antes; não mover para `tamagi-layer`). Alinhar o texto do `README.md` (L435) / `catalog.tsv` (L18) que ainda descrevia `tamagi vm install` como camada sob demanda.
  - **Agentes de IA e Containers ficam intocados:** não cortar nada de `llama.cpp`, `agent-jail`, `devbox`, `podman`, `distrobox`, `podman-compose`, `vulkan-tools`, `ptyxis` (enquanto `cosmic-term` não substituir).
- **Reduzir tudo o que puder fora dessas áreas (medir MB economizados antes/depois):**
  1. **Temas e papéis de parede 4K:** reduzir dos **11 temas** (88 PNGs 4K gerados em `make-wallpapers.py`) para um conjunto enxuto — ex.: **Ipê (Escuro + Claro) + 2 ou 3 temas clássicos** — e verificar se `f45-backgrounds` pode sair caso `cosmic-bg` aceite depender só dos nossos papéis de parede.
  2. **Plugins VPN legados no host (`packages.txt` L113–118):** manter `openvpn`, `openconnect` e `NetworkManager-tui` no host; mover `pptp`, `l2tp`, `sstp`, `strongswan`, `libreswan` e `ssh` para camada opcional sob demanda (`tamagi-layer`).
  3. **Subpacotes não essenciais do Cockpit:** verificar se `cockpit-storaged` e `cockpit-networkmanager` podem sair sem afetar `tamagi vm` (`cockpit` + `cockpit-machines` ficam; `cockpit-podman` fica se atender containers).
  4. **Sobras de GTK/Python/fontes:** conferir se `adw-gtk3-theme` ou outros pacotes de `packages.txt` fora de portais/Disks/COSMIC podem ser enxugados, além de verificar o ganho real de `Roboto Flex` vs `Noto Sans` + `Roboto Slab`.

### 3. Validação e Próxima ISO
- Após aplicar o tema Ipê, Colloid, teste da Roboto Flex e os cortes permitidos:
  1. Rodar `just lint`, `just test`, `just test-installer-ui`, `just build`, `just build-iso`, `just test-iso` e `just test-preserve-home`.
  2. Gerar novas capturas de `output/installer-shots/{dark,light}/` (agora com **Tamagi Ipê Escuro e Claro**!) e `just test-desktop` (`output/desktop.png`) para revisão visual do CEO e do Dono.
  3. Entregar a ISO para a **campanha em máquina real** (Seção 3 de `gemini-retorno.md`).

---

## 8. Segurança de Boot, TPM e MOK (Decisão Pragmática Fechada — 2026-11-24)

**Contexto:** alinhamento do Dono após falha real no **2º boot** (após o 1º login, não liga sozinho — precisa selecionar a entrada `1` e digitar a senha do disco, bug idêntico ao relatado no openSUSE Aeon Desktop), somado à necessidade de desligar o editor do menu de boot (`editor no`), ofertar **TPM + PIN** e definir o tratamento pragmático da senha **MOK (`universalblue`)**.

### Diagnóstico verificado no código atual
1. **PCR 7 às cegas no 1º boot (`live/files/usr/libexec/tamagi-install` L295–298 + `system_files/usr/libexec/tamagi/tpm-seal` L15–18):** o instalador grava o TPM sem PCR (`--tpm2-pcrs=`) e no 1º boot `tamagi-tpm-seal.service` re-sela para `--tpm2-pcrs=7` **mesmo quando o Secure Boot está desligado**, ou antes de um `Enroll MOK` / mudança UEFI. Em hardware real (AMD fTPM, Intel PTT, NVRAM UEFI ou após MOK), o PCR 7 muda entre o 1º e o 2º boot e trava o desbloqueio automático.
2. **Menu de boot aberto (`editor yes`):** o `systemd-boot` não recebe `editor no` em `loader.conf`, permitindo `e` → `init=/bin/sh` em caso de acesso físico com TPM automático.
3. **`tamagi-setup tpm-unlock` quebrado/legado (`system_files/usr/bin/tamagi-setup` L28–33 + `tamagi-firstrun` L97–105):** ainda chama `/usr/libexec/luks-enable-tpm2-autounlock` / `luks-disable-tpm2-autounlock` (herança da base Universal Blue para `ostree`/GRUB/`crypttab`), sem conversar com `bootc` + `composefs` + `/usr/libexec/tamagi/kargs` (`rd.luks.options=discard,tpm2-device=auto`) e sem suportar PIN.
4. **MOK sem vínculo com TPM (`tamagi-setup` L20–27):** rodar `tamagi setup secure-boot-key` (senha `universalblue`) e confirmar `Enroll MOK` muda o PCR 7 no próximo boot sem avisar nem re-agendar o selo do TPM.

### Decisões Pragmáticas Fechadas (para o CTO implementar)
1. **Desligar o editor do menu de boot (`editor no`):** gravar `editor no` em `/efi/loader/loader.conf` (via `/usr/lib/bootc/install/systemd-boot/loader.conf` e garantindo que `bootc upgrade` mantenha). Rollback escolhendo outra entrada do menu continua funcionando normalmente.
2. **Sacrificar o "desbloqueio automático sem senha por padrão":**
   - **Padrão recomendado quando houver LUKS2 + TPM 2.0:** **`TPM + PIN`** (`systemd-cryptenroll --tpm2-with-pin=yes`), além da opção **`Só senha do disco`** (sem TPM).
   - **Regra anti-bug do Aeon:** **NÃO selar `--tpm2-pcrs=7` quando o Secure Boot estiver desligado** (`mokutil --sb-state`). No modo `TPM + PIN` com Secure Boot desligado, a proteção contra roubo físico vem do **PIN + trava anti-força bruta dentro do chip TPM** + `editor no` (zero dependência de PCR 7 temperamental!). Só amarrar ao PCR 7 se o Secure Boot estiver ativo.
   - **Modo `TPM automático` (sem PIN):** fica apenas como opção explícita secundária ("Avançado"), **somente sela PCR 7 se Secure Boot estiver ligado**, e exibe **aviso honesto na tela**: protege contra remoção do disco para outro computador, mas pode pedir a senha após atualizações de BIOS/MOK e **não** impede contorno se a máquina inteira for roubada enquanto não houver UKI selado + política assinada.
3. **Reescrever `tamagi setup tpm-unlock pin | auto | off` nativamente** (mantendo `on` como alias compatível): usar diretamente `systemd-cryptenroll` + `/usr/libexec/tamagi/kargs` (`rd.luks.options=...tpm2-device=auto`), removendo a dependência de `/usr/libexec/luks-enable-tpm2-autounlock` e atualizando `tests/image.sh` (L243).
4. **Senha do MOK (`universalblue`) mantida, com integração TPM:**
   - **Manter a senha fixa `universalblue`** em `tamagi setup secure-boot-key` (apenas letras minúsculas ASCII, que não mudam de tecla entre layout US e ABNT2 na tela azul do MokManager, onde a senha é descartável e não protege dados).
   - Quando `tamagi setup secure-boot-key` rodar numa máquina que tenha slot `tpm2` no LUKS, avisar claramente que o próximo boot após `Enroll MOK` pedirá a senha de recuperação uma vez e **re-agendar `/etc/tamagi/tpm-seal`** (ou orientar `tamagi setup tpm-unlock`) para re-selar depois do MOK aplicado.
5. **Não tentar replicar `systemd-pcrlock` do Aeon agora:** sem UKI assinado gerado pelo `bootc`, isso só traria instabilidade.
6. **i18n obrigatório:** qualquer texto novo na tela de Criptografia e em `tamagi-setup` deve entrar em `system_files/usr/share/tamagi/i18n/{pt,es,fr}.tsv` (`just lint`).

### Perguntas Técnicas que Permanecem para o CTO Responder
1. **`editor no` persistente no `bootc`:** incluir `/usr/lib/bootc/install/systemd-boot/loader.conf` (com `editor no`) na imagem faz o `bootc install` gravá-lo na ESP, e o `bootc upgrade` preserva `editor no` nas atualizações?
2. **Selagem com PIN no 1º boot sem gravar PIN em disco:** quando o usuário escolhe `TPM + PIN` e o Secure Boot está ligado (precisando selar PCR 7 no 1º boot), `tamagi-tpm-seal.service` não conseguirá usar `--unlock-tpm2-device=auto` sem o PIN. Solução mais pragmática: (a) já fazer o enrollment de `TPM + PIN` sem PCR durante a instalação (ou sem PCR 7, se o ganho de PCR 7 com PIN for pequeno frente ao risco de travar), ou (b) fazer o enrollment de `TPM + PIN` só no primeiro login? **Qual o CTO recomenda como mais simples e à prova de falhas?**
3. **Causa exata do "ir no 1" + Plymouth:** verificar em `tamagi-install` / `systemd-boot` por que o boot parou pedindo entrada `1` e se o `tamagi.plymouth` atrapalha o prompt de senha do LUKS quando o TPM falha no `initrd`.
4. **Cobertura em `just test-iso --tpm`:** validar com `swtpm` se `editor no` está gravado e se o 2º boot com TPM abre (ou testar `TPM + PIN` via serial em `tests/vm_check.py`).

---

## 9. Novo Desenho de Particionamento (Proposta do CEO para v1.3)

**Objetivo:** alinhar o particionamento ao perfil real do Tamagi (base imutável `bootc` + `composefs` estilo Aeon, VMs pesadas em `libvirt`, containers e modelos GGUF locais), mantendo simplicidade de 2 partições e compatibilidade total com instalações v1.2 existentes.

### Diagnóstico do layout atual (5 problemas verificados no código)
1. **ESP fixa em `8 GiB` (`live/files/usr/libexec/tamagi-install` L440):** com ~265 MiB por deployment `bootc`, 8 GiB desperdiça 4–6 GiB em SSDs menores (64–128 GiB).
2. **Perda silenciosa de VMs na reinstalação (`tamagi-install` L393):** com `PRESERVE_HOME=1` (*"Reinstalar mantendo meus arquivos"*), o loop apaga e recria `var-libvirt`, destruindo imagens de VM em `/var/lib/libvirt/images`.
3. **Subvolume morto `var-flatpak` (`tamagi-install` L463, L581):** o Tamagi não usa Flatpaks de sistema (todos são `--user` em `~/.local/share/flatpak`), tornando o subvolume e a entrada no `fstab` peso morto.
4. **Modelos de IA presos em snapshots (`system_files/usr/bin/tamagi-ai` L14 vs `system_files/usr/libexec/tamagi/home-snapshots` L32–40):** os arquivos GGUF (3–68 GB) ficam em `~/.local/share/tamagi/models`, que **não** está na lista `EXCLUDE` do `home-snapshots` — ao apagar um modelo, os snapshots do `snapper` seguram o espaço em disco por semanas.
5. **VMs de usuário sem `NoCoW` (`home-snapshots` L118–130):** `.local/share/libvirt/images` e `.local/share/tamagi-windows` (40 GB) viram subvolumes Btrfs comuns sem `chattr +C`, causando fragmentação e amplificação de escrita.

### Princípios mantidos
- **Continuam apenas 2 partições GPT** (sem partição fixa separada para `/home` nem partição fixa de swap — `zram` de 32 GiB cobre o desktop): mantém espaço fluido entre sistema, modelos de IA, containers e VMs, um único desbloqueio LUKS2 + TPM/PIN, aderência à Discoverable Partitions Specification (DPS) e preservação limpa de `/var/home`.
- **Zero quebra com a v1.2:** os subvolumes `root` e `home` mantêm os mesmos nomes no top-level 5 do Btrfs.

### 1. Tabela de Partições GPT (Novo Desenho)

| # | Nome GPT | Tipo GPT | Tamanho | Sistema de arquivos / Camada | Montagem e flags |
|---:|---|---|---|---|---|
| **1** | `EFI-SYSTEM` | UEFI ESP (`C12A7328-F81F-11D2-BA4B-00A0C93EC93B`) | **4 GiB** em discos `>= 80 GiB` · **2 GiB** em discos `< 80 GiB` | FAT32 | `/efi` (`umask=0077,shortname=winnt,x-systemd.automount,x-systemd.idle-timeout=2min`), flag `NoBlockIOProtocol` durante cópia, `editor no` em `loader.conf` (Seção 8). 4 GiB comporta ~15 imagens/UKIs de 265 MiB. |
| **2** | `root` | Linux x86-64 root DPS (`4F68BCE3-E8CD-4DB1-96E7-FBCAF984B709`) | 100% restante | **LUKS2** (Argon2id + TPM2/PIN opcional) → **Btrfs** (`tamagi`) | Contém todos os subvolumes abaixo no top-level 5. |

### 2. Subvolumes Top-Level 5 do Btrfs (Partição 2)

| Subvolume | Montagem | Opções / Atributos | Na reinstalação (`PRESERVE_HOME=1`) | Papel |
|---|---|---|---|---|
| `root` | `/` | `compress=zstd:1,noatime` | **Recriado** | Deployments `bootc`/`composefs`, `/etc`, `/var/lib/extensions`. Nunca entra no `snapper`. |
| `home` | `/var/home` | `compress=zstd:1,noatime` | **Preservado** | Pastas pessoais e snapshots de usuário (`tamagi snap`). |
| `var-libvirt` | `/var/lib/libvirt/images` | `nodatacow,noatime` + `chattr +C` | **Preservado** *(se já existir)* | Discos de VMs de sistema (`tamagi vm`). |
| `var-containers` | `/var/lib/containers` | `compress=zstd:1,noatime` | **Recriado** | Armazenamento OCI/Podman de sistema. |
| `var-log` | `/var/log` | `compress=zstd:1,noatime` | **Recriado** | Logs de sistema (avaliar `chattr +C` apenas em `journal/`). |
| `var-cache` | `/var/cache` | `compress=zstd:1,noatime` | **Recriado** | Caches de sistema. |
| *(removido)* `var-flatpak` | — | — | **Apagado se legado** | Deixa de existir (todos os Flatpaks são `--user`). |

### 3. Subvolumes por Usuário em `/var/home/<usuário>` (`firstboot` + `home-snapshots`)

- **Excluídos de snapshots com `compress=zstd:1`:** `.cache`, `.local/share/flatpak`, `.local/share/containers`, `.local/share/Steam`, `.local/share/lutris`.
- **Excluídos de snapshots com `NoCoW` (`chattr +C` no subvolume ainda vazio):**
  - `.local/share/libvirt/images` (VMs no espaço do usuário);
  - `.local/share/tamagi-windows` (container/disco Windows 11 de ~40 GB).
- **Excluído de snapshots + sem compressão:**
  - `.local/share/tamagi/models` (modelos GGUF de 3–68 GB do `tamagi ai` — evita que modelos apagados fiquem presos no `snapper` e evita gasto de CPU tentando comprimir pesos já quantizados).

### Perguntas Técnicas para o CTO Confirmar (antes de codar)
1. **ESP adaptativa (2 GiB / 4 GiB):** o corte em `>= 80 GiB → 4 GiB` / `< 80 GiB → 2 GiB` casa bem com os discos de VM de `just test-iso` e com o mínimo de `tamagi-detect-home` (`MERGED_ESP_MIB >= 512`)?
2. **Preservação de `var-libvirt` em `PRESERVE_HOME=1`:** como `/etc` é recriado limpo, os XMLs de definição das VMs em `/etc/libvirt/qemu/` sumiriam mesmo com os `.qcow2` salvos em `var-libvirt`. Vale salvar/restaurar também `/etc/libvirt/qemu/*.xml` (ou documentar importação automática), e conferir rótulo SELinux `virt_image_t`?
3. **`NoCoW` em `.local/share/tamagi/models`:** melhor aplicar `chattr +C` (abre mão do checksum Btrfs de dados nos GGUFs) ou manter CoW com checksum e usar apenas **exclusão de snapshots + `compression=none`** (já que GGUFs são gravados uma única vez e lidos sequencialmente)?
4. **Espaço livre mínimo / `qgroups`:** mantemos apenas checagem preventiva (`tamagi doctor` / `tamagi update` alertando `< 15 GiB` livres + `home-snapshots prune`), evitando `btrfs qgroups` pelo conhecido custo de I/O em Btrfs?
5. **Hibernação:** fica oficialmente fora do escopo (apenas `zram` 32 GiB), sem necessidade de reservar swap em partição ou subvolume agora?

---

## 10. Auditoria de Erros Reais e Corte Pragmático para v1.3 (2026-11-24)

Além dos bugs de TPM/MOK (Seção 8) e particionamento (Seção 9), a inspeção direta do código atual no workspace encontrou **erros concretos no instalador e no fluxo de construção/testes** que precisam de correção antes da próxima ISO:

### Erros verificados para consertar já

1. **Vazamento de chave LUKS residual entre discos no instalador (`welcome/src/setup_app.rs` L387–403 e L505–508):**
   - **Erro:** ao destrancar uma partição LUKS no Passo 5 (`Message::Unlock`), `existing-luks.key` é gravado em `$XDG_RUNTIME_DIR/tamagi-install/`. Se o usuário mudar de ideia e selecionar outro disco (`select_disk`) ou desmarcar *"Manter meus arquivos"*, `self.old_luks.clear()` limpa só a memória — **o arquivo `existing-luks.key` continua no diretório**, e `start()` (`L505–508`) faz `if existing.is_file() { answers.existing_keyfile = Some(...) }` sem checar se o disco final realmente está preservando LUKS!
   - **Conserto:** remover `existing-luks.key` (e `luks.key`) no `init()` e no `select_disk()`, e em `start()` só preencher `answers.existing_keyfile` quando `self.keeping().is_some_and(|f| f.get("PRESERVE_LUKS") == "1")`.

2. **Impossibilidade prática de escolher TPM ao preservar `/var/home` criptografado (`welcome/src/setup_app.rs` L457–459, L471–476, L732–734):**
   - **Erro:** `Page::Encryption` (Passo 4) vem **antes** de `Page::Disk` (Passo 5). Se o usuário deixar a criptografia desligada no Passo 4 (`self.tpm = false`) e no Passo 5 escolher um disco já criptografado (`PRESERVE_LUKS=1`) mantendo seus arquivos, a revisão mostra *"Desbloquear pelo chip TPM: não"* — e ele **não consegue ligar TPM** para o disco preservado sem voltar ao Passo 4, ligar criptografia falsa, digitar 8 caracteres fictícios e avançar de novo!
   - **Conserto pragmático:** inverter a ordem para **`Disk` antes de `Encryption`** — assim, quando `PRESERVE_LUKS == "1"` em `keeping()`, a página de Criptografia já sabe que o disco mantém sua senha atual e mostra apenas a escolha do **modo TPM (`TPM + PIN` / `TPM automático` / `Só senha`)**, sem exigir senha duplicada fictícia.

3. **Referências antigas a `tokyo-night` nos testes (`tests/image.sh` L187, L325, L363, L378, L382):**
   - **Erro:** após trocar o tema padrão para `ipe`, cinco checks em `tests/image.sh` ainda encerram com `tamagi-theme set tokyo-night`. Não falha o teste porque `tokyo-night` ainda existe, mas deixa o estado de teste no tema antigo.
   - **Conserto:** trocar para `tamagi-theme set ipe` no cleanup desses checks.

### O que SACRIFICAR pragmaticamente agora (para a v1.3 sair sólida sem erro em máquina real)

- **Sacrificar complexidade cosmética que não bloqueia a v1.3:**
  - Não bloquear a próxima ISO por refatoração estética das listas internas de `welcome/src/app.rs` (primeiro login): o `m3::step_header` e `m3::hero` já estão lá; o restante fica para depois que o boot + instalador + MOK/TPM estiverem 100% verdes em máquina real.
  - Se o `cosmic-greeter` exigir gambiarra frágil de `outputs.ron` para trocar papel de parede/tema antes do 1º login, **deixar o greeter padrão** por ora e documentar como limitação conhecida em vez de arriscar tela cinza no boot.
- **Válvula de escape imediata para `Roboto Flex` e `Colloid`:**
  - `build_files/10-packages.sh` já baixou `RobotoFlex.ttf` pinada por commit + SHA-256 (bom!), mas se o `just test-desktop` ou teste real mostrar qualquer glitch de renderização/peso no `libcosmic`, **voltar imediatamente para `Noto Sans`** em vez de perder tempo depurando fonte variável de 13 eixos.
  - Ao integrar os ícones **Colloid** (pendente ainda no diff atual), se o tamanho na imagem após `rechunk.sh` subir demais frente à meta de enxugar a ISO, **trocar sem apego por `papirus-icon-theme` + `papirus-folders` (amarelo)**.
- **Foco único da próxima rodada:** fechar Seções 8, 9 e os 2 bugs de `setup_app.rs`, rodar `just lint`, `just test`, `just test-installer-ui`, `just build`, `just build-iso`, `just test-iso --tpm` e ir direto para a **campanha em máquina real** (Seção 3 de `gemini-retorno.md`).

---

## 11. Últimas Instruções Antes de Gerar a Nova ISO (Checklist Final — 2026-11-24)

As entregas das Seções 8, 9 e 10 em `gemini-serv.md` e as capturas em `output/installer-shots/{dark,light}/` foram **revisadas e aprovadas pelo CEO**. Antes de gerar a ISO da v1.3 e commitar, feche estes últimos 4 pontos pragmáticos:

### 1. Ícones (`Colloid`) — regra de corte rápido
- Os ícones **Colloid** ainda não entraram no diff atual.
- **Decisão pragmática:** se você já tiver caminho limpo (download pinado por commit/hash + licença `GPL-3.0-or-later` no `NOTICE` + chave do COSMIC confirmada e peso pequeno após `rechunk.sh`), pode incluir agora; **se der atrito ou atrasar o teste em máquina real**, **gere esta ISO com os ícones atuais** para validar boot + `TPM + PIN` + `plymouth.use-simpledrm=1` + particionamento primeiro, deixando o `Colloid` para a etapa seguinte.

### 2. Documentação e imagens desatualizadas no repositório
1. **`docs/installer/`:** os 4 PNGs ainda são da rodada anterior (`08:33`, antes do Ipê e dos 8 passos). Atualizar com as novas capturas de `output/installer-shots/dark/` (ex.: `i01-welcome`, `i04c-disk-locked`, `i06-review`, `i07-installing`, mantendo os nomes esperados ou atualizando onde forem referenciados).
2. **`README.md` (L196):** atualizar a ordem descrita do instalador de `encryption, disk` para `disk, encryption`.
3. **`tamagiguide.md` (L9):** atualizar `an 8 GiB EFI partition` para o novo layout adaptativo (`2–4 GiB EFI partition`, mantendo compatibilidade com as instalações anteriores de 8 GiB).
4. **`RETURN.md`:** atualizar ao final com os resultados da cadeia de validação, o tamanho final da imagem (`rechunk.sh`) e da ISO, o SHA-256 e a instrução curta para o Dono testar **`TPM + PIN`** na tela real de boot.

### 3. Verificações específicas antes do `just build-iso`
- Confirmar no teste real em VM (`just test-iso --tpm`) que `systemd-cryptenroll` aceita `--tpm2-pcrs= --tpm2-pcrlock=` vazio sem erro tanto em `tamagi-install` (`enroll_tpm`) quanto em `tpm-unlock`.
- Confirmar no cenário `just test-preserve-home` (e, se aplicável, com disco LUKS) que `PRESERVE_LUKS=1` + `TPM + PIN` funciona sem regressão.
- Confirmar em `just test-desktop` (`output/desktop.png`) que o desktop em **Tamagi Ipê + Roboto Flex** renderizou limpo sem falha de peso de fonte.

### 4. Cadeia completa de validação (antes do commit e de entregar a ISO ao Dono)
Executar na ordem (conforme `.claude/skills/tamagi-validate`):
1. `just lint`
2. `just build`
3. `just test`
4. `just test-desktop`
5. `just test-installer-ui`
6. `just build-iso`
7. `just test-iso` e `just test-iso --tpm` (checar `editor no`, `plymouth.use-simpledrm=1` e `systemd-cryptenroll`)
8. `just test-preserve-home`
9. `just test-upgrade` (confirmar que `editor no` em `/efi/loader/loader.conf` permanece intacto após o upgrade!)

Após tudo verde, entregar a ISO (`output/tamagi.iso` + `.sha256`) para o **Wes** rodar o teste em **máquina real** (incluindo digitar o PIN do TPM no boot real, que só hardware real exercita).
