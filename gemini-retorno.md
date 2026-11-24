# gemini-retorno.md — Respostas do CEO (Gemini)

**Autor:** Gemini (CEO)  
**Para:** Wes (Dono) e Claude (CTO)  
**Data:** 2026-11-24  
**Referência:** `gemini-serv.md` (branch `english-cli-minimal`, commit `e6bfd7a`)  
**Nota:** Nenhuma linha de código foi alterada nos branches `main` ou `english-cli-minimal`. As orientações diretas ao CTO para a Tarefa 1 também foram registradas em [claude-serve.md](file:///run/media/wesdt/wessd/Project/distroW/claude-serve.md).

---

## 1. Instalador: que caminho seguir

### Decisão / Recomendação
**Recomendo a Opção D — manter o instalador próprio (`tamagi-welcome --install`) sobre o motor `/usr/libexec/tamagi-install`**, sujeita ao "de acordo" do **Wes** e a **três condições obrigatórias**:

1. **Congelamento rigoroso de escopo (estilo Aeon):** nunca virar um instalador complexo (proibido particionamento manual, dual-boot guiado, LVM/RAID).
2. **Correção imediata dos defeitos de UX/UI encontrados nas telas** + aproximação real ao **Material 3 Expressive** com a cor de marca **Tamagi Ipê (`#F2C14E`)**.
3. **Validação em máquina real do fluxo gráfico completo** antes de qualquer expansão.

*(Esclarecimento ao dono: o pedido original era estilo visual Material 3 no sistema, não um instalador novo — porém a Opção D acabou entregando a solução mais enxuta da ISO e a única que já preserva `/var/home` dentro de LUKS2 sem toolkits extras. O visual Material 3 no desktop inteiro segue na Tarefa 2.)*

### Comparação nos 6 critérios obrigatórios (notas de 1 a 5)

| Opção | `bootc` + `systemd-boot` + `composefs` | LUKS2 + TPM + offline | Manter `/var/home` (incl. LUKS2) | Projeto vivo | Pouco código nosso | Perto do Aeon e enxuto | **Total** |
|---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| **A. Voltar ao `tik` (`sysrich/tik`)** | 4 | 4 | 4 | 4 | 2 | 4 | **22/30** |
| **B. Instalador do `tunaOS`** | 4 | 4 | 1 | 3 | 2 | 1 | **15/30** |
| **C1. Readymade (`FyraLabs/readymade`)** | 3 | 3 | 1 | 4 | 2 | 2 | **15/30** |
| **C2. Calamares** | 2 | 2 | 1 | 3 | 2 | 1 | **11/30** |
| **C3. Fisherman / `projectbluefin/bootc-installer`** | 4 | 4 | 1 | 3 | 2 | 1 | **15/30** |
| **D. Instalador próprio (`tamagi-welcome --install`)** | **5** | **5** | **5** | **4** | **3** | **5** | **27/30** |

**Por que o `tunaOS` continua descartado (o que mudou desde `RETURN.md`):**  
Nada mudou que remova os bloqueios: `tuna-os/bootc-installer` (Python/GTK4/Libadwaita em Flatpak) continua acoplado por um contrato JSON grande ao backend Go `tuna-os/fisherman` (`GPL-3.0-only`), usa layout de partições próprio (ex.: EFI de 1 GiB em vez do nosso ESP de 8 GiB + subvolumes `root`/`home` para `snapper`), **não** oferece preservação de `/var/home` dentro de LUKS2 pré-existente e adicionaria GTK4/Libadwaita a uma ISO 100% COSMIC. O mesmo vale para `projectbluefin/bootc-installer` / `fisherman`.

**Sobre as demais alternativas:**
- **A (`tik`):** exige ~649 linhas de módulo Bash/Zenity (`live/tik/modules/pre/10-tamagi` no commit `6c1c2eb`), ~159 linhas de splash Python, 7 patches frágeis de string em `live/build-live.sh`, e puxa `zenity` + `git-core` na ISO; além disso, Zenity impede o visual Material 3.
- **C1 (`Readymade`):** ativo em Rust + GTK4 + Relm4 + `libhelium` (`GPL-3.0-or-later`), focado em `systemd-repart` + `bootc`, mas impõe GTK4/`libhelium` e não preserva `/var/home` em LUKS2.
- **C2 (`Calamares`):** sem módulos nativos para `bootc`, `composefs` ou `systemd-cryptenroll` (TPM2), migrou do GitHub (arquivado em 2025) para Codeberg/KDE Invent e traria Qt6 + KDE Frameworks.
- **`pop-os/installer` / `cosmic-initial-setup`:** o primeiro usa `distinst` (sem `bootc`/`composefs`); o segundo é só pós-instalação (referência futura caso movamos conta/fuso/teclado para o 1º boot).

### Análise visual e de design das telas (`docs/installer/*.png`)
- **O que está bom:** hierarquia clara (passo + círculo tonal + título), modo escuro limpo, confirmação final antes de gravar, desbloqueio inline de LUKS2 em `i07c-disk-locked.png`.
- **Defeitos encontrados (corrigir prioritariamente):**
  1. `i08-review.png` (`welcome/src/setup_app.rs` L717–744): a revisão estoura os `700 px` de altura e esconde o item TPM e o `warning_card` abaixo da dobra (exigindo rolagem para ver o aviso destrutivo).
  2. `i08-review.png` (`setup_app.rs` L555–558): usa `widget::button::destructive` (vermelho) mesmo quando `keeping().is_some()` (*"Reinstalar mantendo meus arquivos"*). Deve usar `suggested` ao manter arquivos e `destructive` só ao apagar tudo.
  3. `i07c-disk-locked.png` (`setup_app.rs` L665–683): disco pequeno demais (`16 GB · pequeno demais`) aparece como item clicável ativo; deve ficar esmaecido e não selecionável, com vínculo visual mais claro entre o disco escolhido e o card de senha LUKS.
  4. `i09-installing.png` (`welcome/tools/stubs/setup/backend` L6): o stub de screenshots imprime `"Encrypting the disk (LUKS2)"` em inglês numa tela em português (o `/usr/libexec/tamagi-install` real traduz via `MSG_pt`, mas o stub precisa ler `LOCALE` do arquivo de respostas).
- **O que está longe do Material 3 Expressive:** cor ciano padrão em vez do amarelo **Tamagi Ipê `#F2C14E`**; itens de lista como pílulas isoladas de `16 px` em vez de listas segmentadas (`16 px` fora / `4 px` dentro); mistura linear sRGB em `m3::mix()`; barra de progresso de `4 px` muito fina em uma tela vazia (`8 px` recomendada); fluxo de 10 passos (avaliar reduzir para 7–8 passos).

### Perguntas técnicas para o CTO
Estão registradas na Seção 5 de [claude-serve.md](file:///run/media/wesdt/wessd/Project/distroW/claude-serve.md) (tema padrão da sessão live, suporte de `libcosmic` a raios por vértice e Roboto Flex, redução de passos, compartilhamento de `m3.rs` com o primeiro login, cobertura de regressão da UI além de `screenshots.sh` e eventual migração pós-instalação para `cosmic-initial-setup`), bem como na **Seção 8** de [claude-serve.md](file:///run/media/wesdt/wessd/Project/distroW/claude-serve.md) sobre segurança de boot e TPM (`editor no` no `systemd-boot`, oferta de **TPM + PIN** recomendado, **TPM automático** com aviso honesto sobre contorno físico e `tamagi setup tpm-unlock pin|auto|off`).

### Verificado de verdade vs. apenas lido / Fontes
- **Verificado diretamente:** `welcome/src/m3.rs`, `welcome/src/setup_app.rs`, `live/files/usr/libexec/tamagi-install`, `welcome/tools/stubs/setup/backend`, commits `6c1c2eb` e `e6bfd7a`, PNGs `i01-language.png`, `i07c-disk-locked.png`, `i08-review.png`, `i09-installing.png`.
- **Apenas lido (pesquisa web):** `sysrich/tik` (<https://github.com/sysrich/tik>), `tuna-os/bootc-installer` (<https://github.com/tuna-os/bootc-installer>), `projectbluefin/bootc-installer` / `fisherman` (<https://github.com/projectbluefin/bootc-installer>), `FyraLabs/readymade` (<https://github.com/FyraLabs/readymade>), `Calamares` (<https://codeberg.org/Calamares> / <https://github.com/calamares/calamares>), Material 3 Expressive (<https://m3.material.io/>).

---

## 2. Guia de estilo Material 3 do Tamagi

### Decisão / Entrega
Especificação completa entregue em [docs/estilo-m3.md](file:///run/media/wesdt/wessd/Project/distroW/docs/estilo-m3.md) e **aprovada/fechada com as decisões de 2026-11-24**:
- **Cores:** troca do padrão da imagem e da sessão live para **Tamagi Ipê** (Escuro `#16211C` + `#F2C14E` e novo **Tamagi Ipê Claro** `#FAF6EC` + `primaryContainer` `#FFE08D`, `primary` `#8B6200`).
- **Forma, tipografia e espaçamento:** escala M3 Expressive (`0/4/8/12/16/28/full px`), listas segmentadas (`16 px` fora, `4 px` dentro), grade de `4/8 px`, escala tipográfica (`Display 36 / Headline 28 / Title 18 / Body 16 / Label 14`), e **teste aprovado de `Roboto Flex`** (com `Noto Sans` variável como fallback seguro).
- **Ícones:** **Material Symbols** (`Apache-2.0`) apenas para glifos simbólicos próprios do Tamagi (não cobre apps/pastas/mimetypes sozinho), combinado com **Colloid (`GPL-3.0-or-later`)** em variante amarela/Ipê (**escolha do CEO** delegada pelo Wes; **Papirus + `papirus-folders`** só entra como fallback se Colloid pesar demais após `rechunk.sh`).
- **Ordem de aplicação:** (1) tema COSMIC Ipê claro/escuro + cantos → (2) ícones → (3) papel de parede Ipê padrão → (4) `cosmic-greeter` → (5) Plymouth (já em Ipê via `make-plymouth.py`) → (6) janelas próprias (`tamagi-welcome` unificado em `m3.rs` + `gum.env`/`foot`).
- **Limitações atuais do COSMIC** e **respostas/ordens para o CTO** registradas em [docs/estilo-m3.md](file:///run/media/wesdt/wessd/Project/distroW/docs/estilo-m3.md) e na Seção 7 de [claude-serve.md](file:///run/media/wesdt/wessd/Project/distroW/claude-serve.md).

### Verificado de verdade vs. apenas lido / Fontes
- **Verificado diretamente:** `system_files/usr/share/tamagi/themes/{ipe,latte}.conf`, `build_files/{make-cosmic-themes.py,make-plymouth.py,make-wallpapers.py,40-desktop.sh,50-branding.sh,packages.txt}`, `system_files/etc/fonts/conf.d/55-tamagi-fonts.conf`, `welcome/src/{m3.rs,ui.rs,app.rs}`, `README.md` L636–665 e L785.
- **Apenas lido (pesquisa web):** Material Design 3 / M3 Expressive (<https://m3.material.io/>), Google Material Symbols `Apache-2.0` (<https://github.com/google/material-design-icons>), temas Linux `Colloid` (<https://github.com/vinceliuice/Colloid-icon-theme>), `Tela` (<https://github.com/vinceliuice/Tela-icon-theme>), `Fluent` (<https://github.com/vinceliuice/Fluent-icon-theme>), `Papirus` (<https://github.com/PapirusDevelopmentTeam/papirus-icon-theme>), `MoreWaita` (<https://github.com/somepaulo/MoreWaita>), esquema `.ron` do COSMIC e `cosmic-greeter` (<https://github.com/pop-os/libcosmic>, <https://cosmic-themes.org/>).

---

## 3. Campanha de teste em máquina real

### Contexto verificado no repositório
No estado atual do workspace (`git diff` sobre `e6bfd7a`), o CTO já aplicou os drop-ins com `/usr/libexec/tamagi/once-per-image` para `ldconfig.service`, `fedora-atomic-desktop-appstream-cache-refresh.service` e `fedora-atomic-desktop-mandb-update.service`, além de mascarar `flatpak-add-fedora-repos.service` e criar `tamagi-flatpak-cleanup.service` em `build_files/90-finalize.sh`. Isso precisa ser confirmado em **hardware real**.

### Máquinas-alvo obrigatórias (mínimo 5 perfis)
1. **SSD SATA** (para confirmar efeito do escalonador `bfq` e I/O em disco SATA após o 1º boot).
2. **NVMe** (para baseline de velocidade e detecção no instalador).
3. **Notebook com Wi-Fi Intel** (validar firmware `iwlwifi`, `NetworkManager` sem `wait-online` e suspensão/retomada).
4. **GPU NVIDIA** (validar comportamento de boot/Wayland/COSMIC com a imagem padrão atual — lembrando que o `README.md` L780 registra que o driver proprietário NVIDIA não vem na imagem base; registrar se inicia normalmente em `nouveau`/fallback ou se trava).
5. **GPU AMD** (validar aceleração Wayland, Vulkan no `llama.cpp` e sessão COSMIC em bare metal).

*(Preferencialmente incluir ao menos uma máquina com **TPM 2.0 + Secure Boot** ativo e uma reinstalação com **`/var/home` existente em LUKS2**.)*

### Roteiro curto de execução (para quem for instalar)
1. **Gravar a ISO validada por SHA-256** (`sha256sum -c tamagi.iso.sha256` + `live/write-usb.sh` ou gravador equivalente).
2. **Boot do pendrive (Live):** conferir ausência de aviso `deprecated` (`rd.live.overlay…`), abertura automática do `tamagi-welcome --install`, Wi-Fi/rede e listagem correta dos discos (sem o pendrive como alvo).
3. **Instalação gráfica:** percorrer todas as telas (testar LUKS2 + TPM quando houver; ou preservação de `/var/home` se aplicável). Se falhar antes de concluir, **não reinicie ainda**: copie `$XDG_RUNTIME_DIR/tamagi-install/` para outro pendrive.
4. **1º Boot (sistema instalado):** cronometrar até a tela de login, conferir desbloqueio TPM/senha e aguardar as notificações do primeiro login (*"Installing the essential apps..."* → *"Your apps are ready"*). Abrir um terminal e rodar o **Bloco de Coleta (1º boot)**.
5. **Reiniciar para o 2º Boot:** conferir se a lentidão sumiu completamente e rodar imediatamente o **Bloco de Coleta (2º boot)**.

### Comandos obrigatórios (rodar após o 1º e após o 2º boot)
```bash
{
  echo "=== HOST ==="
  uname -r
  grep -E 'PRETTY_NAME|IMAGE_VERSION' /usr/lib/os-release
  lsblk -o NAME,TRAN,ROTA,SIZE,FSTYPE,MOUNTPOINTS
  echo "=== BOOT TIME ==="
  systemd-analyze
  echo "=== BLAME TOP 20 ==="
  systemd-analyze blame | head -20
  echo "=== CRITICAL CHAIN ==="
  systemd-analyze critical-chain
  echo "=== FAILED UNITS ==="
  systemctl --failed --no-pager
  echo "=== JOURNAL ERRORS ==="
  journalctl -b -p err --no-pager
  echo "=== FLATPAK REMOTES ==="
  flatpak remotes --show-details
  echo "=== TAMAGI STATUS ==="
  tamagi status
} > ~/tamagi-boot-report-"$(date +%Y%m%d-%H%M%S)".txt
```

### Modelo de relatório (copiar e preencher)
```markdown
## Relatório de Máquina Real — Tamagi OS
- **Testador / Data:**
- **Máquina / Perfil:** (ex.: Notebook Intel Wi-Fi + NVMe / Desktop AMD + SSD SATA / Desktop NVIDIA)
- **CPU / GPU / Wi-Fi / Disco:**
- **TPM 2.0 / Secure Boot:** (Sim/Não)
- **Cenário de instalação:** (Limpa / LUKS2+TPM / Preservar /var/home)

### Tempos de Boot
- **1º boot (`systemd-analyze`):**
- **2º boot (`systemd-analyze`):**
- **Sensação subjetiva de responsividade (1º vs 2º boot):**

### Checklist (✅ / ❌ + observação)
- [ ] Pendrive iniciou sem avisos obsoletos
- [ ] Instalador gráfico concluiu sem erros
- [ ] `systemctl --failed` retornou 0 unidades (1º e 2º boot)
- [ ] `ldconfig.service` passou sem falha
- [ ] `flatpak remotes --show-details` mostra apenas remotes `--user` (sem `fedora` / `fedora-testing` de sistema)
- [ ] Wi-Fi / Áudio / Rede / Aceleração gráfica funcionando

### O que falhou ou estranhou
- (colar aqui erros e anexar o arquivo `~/tamagi-boot-report-*.txt`)
```

### Verificado de verdade vs. apenas lido
- **Verificado diretamente:** `RETURN.md` L78–111, `build_files/90-finalize.sh` (diff atual), `system_files/usr/lib/systemd/system/{ldconfig.service.d,fedora-atomic-desktop-appstream-cache-refresh.service.d,fedora-atomic-desktop-mandb-update.service.d,tamagi-flatpak-cleanup.service}` e `system_files/usr/libexec/tamagi/once-per-image`, `README.md` L770–799.
- **Nota:** nenhum hardware real foi testado por mim nesta sessão (apenas leitura da campanha pendente e das correções recém-aplicadas).

---

## 4. Prioridades da versão 1.3

Com base em `RETURN.md`, `build_files/packages.txt`, `build_files/90-finalize.sh`, `system_files/usr/bin/tamagi-vm` e `README.md`, recomendo **5 prioridades máximas** para a v1.3, sempre trocando peso na imagem por enxugamento:

| # | Prioridade | Problema do usuário | Como saber que ficou pronto | O que **sai** da imagem para compensar |
|---|---|---|---|---|
| **1** | **Estabilizar boot em máquina real e fechar falhas pós-boot** | Boot lento e `ldconfig.service` falhando fora da VM; fluxo gráfico real ainda não validado em bare metal. | Campanha da Tarefa 3 concluída nas 5 classes de hardware, `systemctl --failed` = 0 no 1º e 2º boot, e `just test-iso` verde. | Remover unidades/configs legadas de compatibilidade que não precisarem ficar após migração limpa. *(Regra do Dono: `qemu-kvm`/`libvirt` ficam na base.)* |
| **2** | **Corrigir UX do instalador e lançar visual Material 3 Expressive (`Tamagi Ipê`)** | Tela de revisão esconde aviso abaixo da dobra, usa botão vermelho ao preservar dados, e o desktop abre em `Tokyo Night` ciano em vez da identidade Ipê + M3 pedida pelo dono. | Bugs de `setup_app.rs` e stub de idioma corrigidos; `Tamagi Ipê` (claro/escuro) + tema de ícones `Colloid` + teste `Roboto Flex` aplicados no desktop, live e greeter. | Reduzir os **11 temas** e **88 PNGs 4K gerados** (`make-wallpapers.py`) para um conjunto curado (ex.: **Ipê Escuro/Claro + 2–3 temas** = corte de ~48–56 PNGs 4K). |
| **3** | **Enxugar a imagem base sem tocar em IA, containers ou virtualização** | A imagem (`~6.8 GB` rechunked / ISO `5.6 GB`) ainda carrega extras fora do núcleo essencial que engordam a ISO. | Redução mensurável no tamanho da imagem e da ISO com `just test` e `just test-desktop` passando, mantendo 100% intactos IA, containers e VM. | Cortar **excesso de papéis de parede/temas**, **plugins VPN legados no host**, **subpacotes extras de Cockpit (`storaged`/`networkmanager`)** e eventuais sobras GTK/fontes fora de portais/Disks. |
| **4** | **CI automatizado de ISO + canais `:testing` e `:stable`** | `tests/iso.sh` roda só localmente e `:latest` recebe commits direto de `main`, aumentando risco de regressão de boot chegar ao usuário final. | Build + `tests/iso.sh` (ou smoke test equivalente) rodando em CI antes de promover `:testing` → `:stable`; inputs pinados por digest. | Eliminar builds/scripts manuais paralelos e a manutenção de variantes ad-hoc redundantes fora do fluxo único. |
| **5** | **Polir experiência de Agentes de IA & `tamagi` sem resíduos de idioma** | Usuários de IA ainda esbarram em logs raiz/opções em português (`home-snapshots`, `firstboot`, `secureboot-guard`, `tamagi panel set centro\|classico`) e faltam testes em bare metal de `tamagi apply ai` + `tamagi lease`. | Zero logs em português nos serviços de sistema; `tamagi apply ai` e `tamagi lease` validados em máquina real; mensagens claras de erro na jaula `agent-jail`. | Remover aliases/chaves antigas redundantes após janela de transição e limpar itens sobrepostos do catálogo. |

### O que eu cortaria do que já existe hoje (respeitando a trava do Dono: não mexer em IA, containers ou virtualização)

1. **Alinhamento de documentação da VM (`build_files/packages.txt` L138–165 vs `README.md` L435):**  
   O `README.md` (L435) e o `catalog.tsv` (L18) dizem que `tamagi vm install` instala virtualização numa camada reversível, mas `packages.txt` embute `qemu-kvm`, `libvirt-*`, `edk2-ovmf`, `swtpm`, `guestfs-tools` e `cockpit-*` na imagem base. **Decisão do Dono (2026-11-24):** `qemu-kvm` + `libvirt` + `cockpit-machines` **FICAM na base** (já houve problemas fora dela).
   - **Ação segura:** atualizar o texto do `README.md` / `catalog.tsv` para refletir a realidade, e avaliar apenas se **`cockpit-storaged`** e **`cockpit-networkmanager`** podem sair sem afetar `tamagi vm` nem containers (`cockpit-podman`).
2. **Terminais (`foot`, `ptyxis`, `cosmic-term`):**  
   Enquanto o fork `cosmic-term` não assumir containers (`UPSTREAMS.md`), **manter `ptyxis` + `foot`** (para não mexer em fluxo de containers). Quando o `cosmic-term` atingir paridade, cortar `ptyxis` e `foot` num único passo.
3. **Excesso de temas e papéis de parede 4K (`build_files/make-wallpapers.py` + `40-desktop.sh`):**  
   Hoje são gerados **88 PNGs em resolução 4K (3840×2160)** (8 estilos × 11 temas) mais 10 fotos em `art/` (~4,9 MB) e `f45-backgrounds` da dependência do `cosmic-bg`. Reduzir para **Ipê (Escuro + Claro) + 2 ou 3 temas clássicos** e avaliar com o CTO se `f45-backgrounds` pode sair reduz bastante o volume no `/usr`.
4. **Plugins VPN legados no host (`build_files/packages.txt` L111–118):**  
   São 8 pacotes `NetworkManager-*-gnome` na base. Manter apenas **`openvpn`**, **`openconnect`** e `NetworkManager-tui`; mover `pptp`, `l2tp`, `sstp`, `strongswan`, `libreswan` e `ssh` para camada opcional (`tamagi-layer`).
5. **Sobras GTK / fontes duplicadas:**  
   Conferir com o CTO se `adw-gtk3-theme` ou algum pacote fora de portais (`xdg-desktop-portal-gtk`) e `gnome-disk-utility` ainda é necessário, e ao testar **Roboto Flex**, verificar se `google-roboto-slab-fonts` ou pesos duplicados podem sair.

### Verificado de verdade vs. apenas lido
- **Verificado diretamente:** `RETURN.md`, `build_files/packages.txt` (168 linhas), `build_files/90-finalize.sh`, `build_files/40-desktop.sh`, `system_files/usr/bin/tamagi-vm`, `system_files/usr/share/tamagi/catalog.tsv`, `system_files/usr/share/tamagi/themes/*.conf`, `README.md` L435 e L793–795, `UPSTREAMS.md`.

---

## 5. Para quem é o Tamagi (posicionamento)

### Público principal
**Desenvolvedores, engenheiros e usuários avançados que trabalham diariamente com IA local e agentes de código autônomos** — e que querem dar autonomia real aos agentes sem entregar as chaves da máquina (`~/.ssh`, credenciais, navegador, outros repositórios ou a integridade do sistema operacional).

### As 3 frases que o `README.md` deve dizer logo no começo
1. **Tamagi is an atomic, self-healing COSMIC Linux built for running AI coding agents and local models safely.**
2. **Every agent runs jailed to your current project — blind to your SSH keys, tokens, and browser profiles — with automatic Git, Btrfs, and database rollback points before it touches a file.**
3. **The OS stays minimal and immutable on `bootc` + `composefs`, while devboxes, native sysext layers, browsers, and VMs are reversible and strictly opt-in.**

### Comparação honesta: o que os outros fazem melhor que nós

| Projeto | O que eles fazem melhor que o Tamagi hoje |
|---|---|
| **openSUSE Aeon** | Maturidade de campo e disciplina minimalista extrema: base muito menor, fluxo de 1º boot (`gnome-initial-setup`) enxuto de verdade, migração/backup integrada no `tik` e anos de testes em hardware real com Tumbleweed + `snapper`. |
| **Bluefin** | Experiência pronta para desenvolvedor tradicional/cloud-native ("macOS para Linux"): integração polida com VS Code Dev Containers, Homebrew maduro, documentação de comunidade ampla, imagens testadas em CI e suporte pronto de fábrica a hardware/NVIDIA. |
| **Bazzite** | Líder absoluto em jogos e portáteis: sessão SteamOS/Game Mode madura, dezenas de patches para controles e handhelds, drivers NVIDIA pré-empacotados em variantes dedicadas, captura/VR e comunidade massiva testando cada kernel OGC. |
| **tunaOS** | Foco corporativo em Enterprise Linux (AlmaLinux 10 / CentOS Stream) com ciclo de vida longo, imagens dedicadas com NVIDIA/CUDA prontas para trabalho de IA/VFX, `bootc-migrate` e instalação sem reparticionar dentro do Windows (`wootc`). |
| **Ultramarine** | Polimento "funciona direto da caixa" para usuário comum (codecs, drivers, variantes de desktop prontas, repositório Terra) e um instalador moderno próprio (`Readymade`) com preview público e equipe dedicada de empacotamento. |

**Onde o Tamagi se diferencia:** nenhum desses cinco une, em um único sistema COSMIC enxuto sobre `bootc` + `composefs` + `systemd-boot`, **jaula de agentes por projeto (`bubblewrap` + Landlock + snapshots automáticos pré-agente)**, **`tamagi lease` (permissões com prazo de validade)**, **camadas `systemd-sysext` reversíveis (`tamagi layer`)** e **Chromium nativo com OAuth2 + `llama.cpp` compilado com Vulkan**.

### Verificado de verdade vs. apenas lido / Fontes
- **Verificado diretamente:** `README.md` (826 linhas completas), `system_files/usr/share/tamagi/catalog.tsv`, `Containerfile`.
- **Apenas lido (pesquisa web):** características e posicionamento de Aeon, Bluefin (<https://projectbluefin.io/>), Bazzite (<https://github.com/ublue-os/bazzite>), tunaOS (<https://tunaos.org/>) e Ultramarine (<https://ultramarine-linux.org/>).

---

## 6. Vigiar o que vem de cima

| Upstream | O que acompanhar | Frequência | Onde a novidade aparece primeiro (link) | Por que importa para o Tamagi |
|---|---|---|---|---|
| **COSMIC (`cosmic-epoch`)** | Lançamentos pontuais `1.0.x` / novas epochs, mudanças no `libcosmic` (`CosmicTk` / `.ron` de temas) e melhorias em `cosmic-comp` / `cosmic-greeter` | **Semanal** (toda segunda-feira) | Releases: <https://github.com/pop-os/cosmic-epoch/releases> · Commits/PRs de compositor: <https://github.com/pop-os/cosmic-comp> | Quebra de chaves `.ron` (como `CosmicTk/interface_font` já vigiada em `40-desktop.sh`), estabilidade Wayland/multi-monitor e novidades de personalização M3. |
| **HDR no `cosmic-comp`** | Discussão técnica, PRs ligados e mudança de status da issue de rastreamento oficial | **Quinzenal** (ou por notificação da issue) | Issue oficial: <https://github.com/pop-os/cosmic-comp/issues/1384> | Decisão fechada: o desktop espera o HDR oficial no `cosmic-comp`; quando entrar em review final, podemos testar em hardware real. |
| **`bootc` e `composefs`** | Releases semanais do `bootc` (`1.17.x+`), mudanças no `--composefs-backend`, `bootc install reset` e `composefs-rs` | **Semanal** | `bootc` releases (toda segunda-feira): <https://github.com/bootc-dev/bootc/releases> · `composefs`: <https://github.com/composefs/composefs/releases> · `composefs-rs`: <https://github.com/composefs/composefs-rs/releases> | `RETURN.md` aponta que `tamagi reset` está adiado porque `bootc install reset` ainda não funciona em `composefs`; também afeta contagem/rollback em `systemd-boot`. |
| **Universal Blue `base-main`** | Mudanças no `Containerfile`, serviços ligados na base Fedora (como ocorreu com `flatpak-add-fedora-repos` e `ldconfig`), transição de versão do Fedora (`45` → `46`) | **Semanal / antes de cada tag de ISO** | Repositório e commits: <https://github.com/ublue-os/main/commits/main> · Releases/packages: <https://github.com/ublue-os/main> | Toda mudança de serviço ou pacote na base `base-main` entra silenciosamente no nosso próximo `just build` (como causou o atraso de boot corrigido agora). |
| **Kernel OGC e `akmods`** | Novas versões do kernel `ogc`, módulos assinados (`xone`, `xpadneo`, `v4l2loopback`) e notas de regressão usadas pelo Bazzite estável | **Semanal** | Build/releases OGC: <https://github.com/OpenGamingCollective/kernel-packages/releases> · Árvore Linux OGC: <https://github.com/OpenGamingCollective/linux> · Módulos UB: <https://github.com/ublue-os/akmods> · Referência estável: <https://github.com/ublue-os/bazzite/releases> | Garante que nossa atualização de kernel OGC + `akmods` continue compatível com Secure Boot (`universalblue.der`) e sem regressões de drivers em bare metal. |

### Pergunta técnica para o CTO
- Em vez de vigiar tudo manualmente, faz sentido habilitar **Renovate** ou um workflow semanal simples (como anotado da prática do `utah` em `UPSTREAMS.md`) para abrir PRs automáticos quando o digest de `base-main` ou a versão do `bootc` mudar?

### Verificado de verdade vs. apenas lido / Fontes
- **Verificado diretamente:** `UPSTREAMS.md`, `Containerfile`, `build_files/05-latest.sh` / `40-desktop.sh` / `90-finalize.sh`, `RETURN.md`.
- **Apenas lido (pesquisa web):** cadência de releases do `bootc` (<https://github.com/bootc-dev/bootc/releases>), `composefs` / `composefs-rs` (<https://github.com/composefs/composefs>), `pop-os/cosmic-epoch` (<https://github.com/pop-os/cosmic-epoch/releases>), `ublue-os/main` (<https://github.com/ublue-os/main>), `OpenGamingCollective/kernel-packages` (<https://github.com/OpenGamingCollective/kernel-packages/releases>) e `ublue-os/akmods` (<https://github.com/ublue-os/akmods>).
