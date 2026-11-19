# Tamagi

**Tamagi 1.0 "Kazunobu M"** — distro Linux atômica e imutável para uso diário e
desenvolvimento no Brasil. O host é uma imagem [bootc](https://github.com/bootc-dev/bootc)
sobre o Fedora 45 com o desktop [COSMIC](https://github.com/pop-os/cosmic-epoch)
(escrito em Rust). Navegadores, pacotes de outras distros e o Warsaw vivem em
caixas [distrobox](https://distrobox.it/); a IA é local, com [llama.cpp](https://llama-cpp.com/).

O sistema inteiro é um contêiner OCI: `podman build` gera a imagem, a máquina
troca de versão de forma atômica e a anterior fica guardada para rollback.

Veja também: [por que mais uma distro?](#por-que-mais-uma-distro) e os
[agradecimentos](#agradecimentos).

## Três ideias próprias

**1. O sistema num arquivo.** `~/.config/tamagi/tamagi.toml` descreve a
máquina: tema, navegadores, Flatpaks, ferramentas do mise, pacotes dos
subsistemas, caixas, web apps e serviços.

```bash
tamagi init      # cria o arquivo a partir do estado atual
tamagi diff      # o que falta para a máquina ficar como o arquivo
tamagi apply     # faz o que falta
```

Copie o arquivo para outro computador com Tamagi e rode `tamagi apply`.
Ele só acrescenta, nunca remove, e avisa o que pede senha.

**2. Modo banco descartável.** `tamagi-bank` (ou "Banco" no lançador) abre o
Chrome com perfil limpo e o Warsaw numa caixa criada na hora e apagada ao
fechar a janela. O perfil fica em memória; a identificação e a autoridade
certificadora do Warsaw nascem e morrem com a sessão. A primeira vez monta um
modelo (alguns minutos); depois cada sessão abre em segundos.
`tamagi-bank --manter-registro` guarda só a identificação do Warsaw, para o
banco reconhecer o aparelho.

**3. Boot que se conserta e se explica.** A cada boot, o
[greenboot](https://github.com/fedora-iot/greenboot) roda as verificações de
saúde do Tamagi (`/usr/lib/greenboot/check`). Se uma atualização não passa, o
computador reinicia e, na terceira falha seguida, o GRUB volta sozinho para a
imagem anterior. No login seguinte, `tamagi-boot-report` mostra o que quebrou
e, havendo um modelo de IA local, o agente explica em português.

## Construir e testar

Precisa de `podman` e `just`. Nada aqui pede root.

```bash
just build          # imagem do sistema (~20 min na primeira vez)
just test           # 272 verificações dentro da imagem
just test-desktop   # sobe a sessão COSMIC sem monitor e salva output/desktop.png
just test-browser   # cria de verdade a caixa de navegadores neste computador (--warsaw, --bank)
just test-agent     # baixa o preset mini (2,8 GB) e testa o agente de IA
just lint
```

## Instalar

O instalador é o [tik](https://github.com/sysrich/tik) com um módulo do Tamagi
que chama o `bootc install`. A ISO é gerada sem root:

```bash
just build-iso      # output/tamagi.iso (6,9 GiB: cabe num pendrive de 8 GB)
just test-iso       # instala sozinho numa VM e confere o sistema instalado
just test-rollback  # simula uma atualização ruim e confere o rollback automático
just test-upgrade   # atualização automática de ponta a ponta (registry local, imagem nova, reinício, rollback)
just run-iso        # abre a ISO numa VM com janela, para usar o instalador
sudo bash live/write-usb.sh /dev/disk/by-id/usb-… --yes   # grava num pendrive e confere
```

A ISO inicia (só UEFI) numa sessão COSMIC live e abre o instalador, em
**português (Brasil), inglês (EUA), espanhol ou francês**: a primeira pergunta
é o idioma, que vale para o instalador, para o sistema instalado e para as
ferramentas do Tamagi. Depois: teclado, fuso horário, a sua conta, criptografar
ou não o disco (LUKS2) e a confirmação do disco que será apagado. O sistema é
copiado da própria mídia, sem rede. Quem quiser só experimentar fecha a
janela; o instalador fica no lançador.

**Instalação tudo-ou-nada:** antes de apagar o disco o instalador confere o
tamanho dele e a imagem embutida inteira (SHA-256 de cada camada); mídia com
defeito é recusada com o disco intacto. Se a cópia parar no meio, as partições
criadas são limpas: o disco nunca fica com um sistema pela metade que pareça
instalado. O resultado é um sistema atômico (deployment bootc/ostree): cada
atualização entra como uma imagem inteira ao lado da atual, e a anterior fica
guardada para rollback.

**Disco criptografado sem digitar senha:** se o computador tem chip TPM 2.0,
o instalador oferece guardar a chave do disco nele (`systemd-cryptenroll`).
No primeiro boot o `tamagi-tpm-seal.service` amarra essa chave ao PCR 7
(estado do Secure Boot). O computador liga direto e o disco continua ilegível fora daquela
máquina; a senha criada na instalação fica de reserva (troca de placa-mãe,
atualização de firmware). `just test-iso --tpm` confere isso com um TPM
virtual.

No primeiro login abre o assistente do Tamagi (tema, barra, tiling, navegador, caixa
de desenvolvimento, IA, Android, Warsaw). O assistente inicial do COSMIC não
abre sozinho, porque repete as perguntas do instalador; ele continua no
lançador.

Como funciona por dentro:

- `live/Containerfile` parte da imagem do sistema e acrescenta o tik (fixado
  por tag e commit), o módulo `live/tik/modules/pre/10-tamagi`, um initramfs
  live e o usuário `live`.
- O módulo faz as perguntas e substitui a etapa do tik que gravaria uma
  imagem de disco por `/usr/libexec/tamagi-install`: particiona (ESP, `/boot`
  ext4, raiz btrfs com zstd, LUKS2 e chave no TPM se pedidos) e roda
  `bootc install to-filesystem --source-imgref oci:…` com a imagem embutida.
- O usuário é criado no primeiro boot (`tamagi-firstboot.service`), com o
  sistema já rodando.
- `tamagi.autoinstall=/dev/sdX` na linha do kernel instala sem perguntas
  (é o que `just test-iso` usa).

Para o sistema instalado receber atualizações, publique a imagem num registry
(`.github/workflows/build.yml` faz isso no GHCR) e gere a ISO com
`TAMAGI_TARGET_IMGREF=ghcr.io/SEU_USUARIO/tamagi:latest just build-iso`. Numa
máquina já instalada: `ujust switch-image ghcr.io/SEU_USUARIO/tamagi:latest`.
Em quem já roda Fedora atomic/Bazzite/Bluefin: `sudo bootc switch <imagem>`.

## Navegadores (caixa Ubuntu 26.04 LTS)

O host não traz navegador. O Chrome vive numa caixa distrobox Ubuntu 26.04 LTS, criada
no primeiro uso; ela enxerga a placa de vídeo, o áudio, a webcam, as chaves de
segurança USB e a sua pasta pessoal.

```bash
tamagi-browser                    # abre o navegador padrão (cria a caixa na 1ª vez)
tamagi-browser install firefox    # também: brave, edge, vivaldi
tamagi-browser default firefox    # navegador padrão do sistema
tamagi-browser status             # caixa, navegadores, VA-API e Vulkan
tamagi-browser chrome --safe      # sem as opções de aceleração
```

- O Chrome abre com decodificação e codificação de vídeo por hardware
  (VA-API), rasterização por GPU e sem a lista de bloqueio de GPUs. Para
  mudar: uma opção por linha em `~/.config/tamagi/browser-flags.conf`.
- NVIDIA com driver proprietário: a caixa é criada com `--nvidia` (as
  bibliotecas vêm do host).
- A entrada "Google Chrome" do menu e os web apps (`tamagi-webapp`) usam essa
  caixa. Outros navegadores aparecem no menu pelo `distrobox-export`.

Medido na máquina de desenvolvimento (Ryzen 7 5700, RX 7600): dentro da caixa
o `vainfo` lista H.264, HEVC, VP9 e AV1 (decodificação) e H.264, HEVC e AV1
(codificação); o Chrome 158 usou o `VaapiVideoDecoder` para H.264, VP9 e AV1,
com WebGL e WebGPU ligados e o sandbox do Chrome ativo (sem `--no-sandbox`).

## Internet banking (Warsaw)

```bash
ujust warsaw ligar      # mostra a licença, pede aceite, instala na caixa e sobe
ujust warsaw            # estado e teste da porta 30900
ujust warsaw desligar   # para o serviço   ·   ujust warsaw remover
```

O Warsaw **não vem na imagem** e não roda no host. Quando você aceita a licença
da Topaz, o `.deb` oficial é baixado do site do fabricante, conferido por
SHA-256 e instalado na caixa dos navegadores. A caixa não tem systemd como
init; o serviço é só o binário `core`, que o `tamagi-browser` sobe junto com o
navegador. Como a caixa usa a rede do host, o site do banco fala com ele em
`127.0.0.1:30900` — desde que o navegador seja o da caixa.

O Warsaw gera uma autoridade certificadora local. O Tamagi a registra no banco
NSS do seu usuário enquanto o Warsaw está ligado (sem isso o Chrome recusa a
conexão) e a remove ao desligar.

## Pacotes deb, rpm, Arch e Omarchy

```bash
tamagi-pkg install ./programa.deb     # ou clique duas vezes no .deb/.rpm
tamagi-pkg install --arch yazi        # pacman; AUR se a caixa tiver paru/yay
tamagi-pkg install --omarchy hype     # do repositório do Omarchy
tamagi-pkg search --omarchy oma
tamagi-pkg install --rpm htop
tamagi-pkg run --deb <comando>
tamagi-pkg list | remove | update | export | enter | status
```

Cada formato tem um subsistema (um distrobox com o gerenciador nativo), criado
no primeiro uso; apps gráficos vão para o menu do host. O subsistema `deb` é
Ubuntu 26.04 LTS. O do Arch já nasce com o repositório `[omarchy]`
(`https://pkgs.omarchy.org/stable`, 264 pacotes) à frente dos do Arch; a chave
pública do Omarchy vem na imagem e é conferida pela impressão digital. Outro
canal: `tamagi-pkg omarchy rc`.

O distrobox **não** é uma barreira de segurança: o que roda na caixa enxerga a
sua pasta pessoal.

**A caixa perde desempenho?** Não de forma mensurável. Medido na máquina de
desenvolvimento, host contra caixa Arch com as opções do distrobox (duas
rodadas): SHA-256 de 4 GB 2,32 s × 2,34 s; zstd de 1 GB 2,38 s × 2,39 s; 400
execuções do `mise` 1,87 s × 1,87 s; 20 mil arquivos na pasta pessoal 1,06 s ×
1,10 s. A única diferença é escrever muitos arquivos pequenos na raiz da
própria caixa (overlay): 1,51 s contra 1,06 s. Não há virtualização: é o mesmo
kernel, a mesma CPU e o mesmo disco.

## mise: linguagens, ferramentas e tarefas

O [mise](https://mise.jdx.dev/) é o padrão do Tamagi para o que ele se propõe:
versões de linguagens e ferramentas, variáveis de ambiente por projeto e
tarefas. Roda nativo no host (não numa caixa), instala tudo no seu usuário e
não precisa de root.

```bash
mise use -g node@lts python@latest   # global, para o seu usuário
mise use go@1.24                     # só neste projeto (grava mise.toml)
mise run build                       # tarefas do projeto   ·   mise ls
```

- Ativado no shell interativo; os *shims* entram no `PATH` de toda sessão,
  então IDEs e apps gráficos também enxergam as ferramentas.
- `/etc/mise/config.toml` liga a leitura de `.nvmrc`, `.python-version` e
  afins e a instalação automática do que o projeto declara.
- O assistente de primeiro login oferece as linguagens mais comuns, e a
  atualização automática mantém tudo em dia.

## Máquinas virtuais

O virt-manager vem nativo no host, com QEMU/KVM, firmware UEFI (edk2-ovmf),
TPM virtual (swtpm, para Windows 11), pastas compartilhadas (virtiofs),
virt-viewer, virt-install e guestfs-tools. Os daemons do libvirt sobem por
socket, só quando usados, e quem é administrador gerencia as VMs sem digitar a
senha. `ujust vms` abre o virt-manager.

## Atualização automática

Tudo se atualiza sozinho, todo dia:

| O quê | Como |
|---|---|
| Sistema (imagem) | baixada e preparada em segundo plano; entra no próximo boot, com a anterior guardada |
| Flatpaks | timers do sistema e do usuário |
| Caixas distrobox e mise | `tamagi-autoupdate.timer` (não roda na bateria) |

`tamagi-update status` mostra os quatro timers e avisa se há imagem nova
esperando o reinício; `ujust autoupdate off` desliga e `on` religa.
`tamagi-update` continua atualizando tudo na hora.

A imagem só é baixada em rede não tarifada (regra da base Universal Blue) e a
troca acontece no reinício: nada muda no sistema em uso. Se a imagem nova não
passar na verificação de saúde do boot, o computador volta sozinho para a
anterior. `just test-upgrade` prova o ciclo inteiro numa VM.

**Publique antes de gerar a ISO.** O sistema instalado segue a imagem do
registry (`TAMAGI_TARGET_IMGREF`). Se a ISO for gerada de um build local mais
novo que o publicado, a primeira atualização automática troca o sistema pela
imagem do registry — mais antiga. A ordem certa: `git push` (o CI publica no
GHCR), e só então `just build-iso`.

## Jogos

`gamescope`, MangoHud, GameMode e as regras de controles (`steam-devices`) já
vêm na imagem. O resto é sob demanda:

```bash
ujust setup-gaming       # Steam, Lutris, Heroic, ProtonUp-Qt, Bottles (Flatpak)
```

## Segurança

O padrão é o nível **equilibrado**: sysctls de endurecimento, argumentos de
kernel (`init_on_alloc`, `slab_nomerge`, `randomize_kstack_offset`…), MAC
aleatório por rede Wi-Fi, firewall e SELinux ligados — nada que quebre jogos,
distrobox, Flatpak ou GPU.

```bash
tamagi-harden status
tamagi-harden strict           # SMT desligado em CPUs vulneráveis, lockdown do kernel, ptrace só root…
tamagi-harden balanced         # desfaz
tamagi-harden usb on           # USBGuard: só os aparelhos USB já conhecidos
tamagi-harden malloc on        # hardened_malloc em todos os programas
ujust setup-luks-tpm-unlock    # desbloqueio do disco por TPM
```

## Rust

- **Desktop:** COSMIC 1.11 inteiro (compositor, painel, terminal, arquivos,
  editor, loja, player, tela de login).
- **Kernel:** Fedora 7.3, `CONFIG_RUST=y`, Binder em Rust, sched_ext.
- **Terminal:** `sudo` é o sudo-rs e `ls`, `cp`, `mv` etc. são os do uutils no
  shell interativo (os originais continuam em `/usr/bin` para serviços e
  scripts; desligar: `touch ~/.config/tamagi/no-rust-defaults`). Também:
  alacritty, ripgrep, fd, bat, eza, zoxide, dust, delta, hyperfine,
  difftastic, uv, starship, mise.

As ferramentas `tamagi-*` são bash e Python.

## IA

```bash
tamagi-ai setup          # recomenda um modelo pelo hardware, baixa e liga o servidor
ia "por que meu boot está lento?"    # agente do sistema, 100% local
tamagi-ai chat | code    # interface web do llama.cpp | opencode no modelo local
claude                   # Claude Code, já instalado (também no menu e na dock)
```

- **llama.cpp v1.0.0** compilado na imagem com Vulkan e variantes de CPU
  escolhidas em tempo de execução; serve em `127.0.0.1:9931` (API OpenAI e
  Anthropic), carrega o modelo sob demanda e o libera após 15 min parado.
- **`tamagi-agent`** (alias `ia`): agente próprio que conhece o sistema.
  Comandos somente-leitura rodam direto; o resto pede confirmação.
- **mise** já ativado no shell: `mise use -g node@lts python@3.13`.

| Preset | Modelo | Tamanho | Para |
|---|---|---|---|
| `mini` | Gemma 4 E2B | 2,8 GB | qualquer máquina |
| `leve` | Gemma 4 E4B | 4,6 GB | 8–16 GB de RAM |
| `medio` | Gemma 4 12B | 7,2 GB | 16 GB+ de RAM ou GPU de 8–12 GB |
| `grande` | Qwen 3.6 35B (MoE) | 20,5 GB | 48 GB+ de RAM ou GPU de 24 GB |
| `codigo` | Qwen3 Coder 30B (MoE) | 18 GB | programação, 32 GB+ de RAM |

## O que vem instalado

- **Desktop:** COSMIC, com terminal, arquivos, editor, loja e player próprios;
  tiling ligado, teclado ABNT2.
- **Barra:** uma só, embaixo, com os apps no centro, a bandeja e o relógio à
  direita — o layout "Centered" do AnduinOS feito com os applets do COSMIC
  (flutuante, cantos arredondados, levemente translúcida).
  `tamagi-panel set centro|classico|cosmic` troca: `classico` põe os apps à
  esquerda; `cosmic` volta ao painel em cima + dock.
- **Idiomas:** português (Brasil), inglês (EUA), espanhol e francês, no
  sistema, no instalador, no assistente de primeiro login, no menu e nos
  atalhos. `tamagi-lang pick` troca depois de instalado.
- **Fontes:** Noto Sans na interface, Roboto Slab como fonte com serifa
  (documentos), JetBrains Mono no terminal.
- **Flatpak (primeiro boot):** OnlyOffice, Flatseal, Warehouse, Mission Center,
  Loupe, Papers, DistroShelf, COSMIC Tweaks, Examine e três applets do COSMIC.
- **Temas:** `tamagi-theme set <tema>` — tokyo-night, catppuccin, gruvbox,
  nord, everforest, rose-pine, kanagawa, matte-black, ristretto, latte. Trocam
  claro/escuro, papel de parede e a paleta das janelas do Tamagi (não as cores
  de destaque do COSMIC).
- **Papéis de parede:** oito estilos em cada tema, nas cores dele
  (`tamagi-theme wallpaper <estilo>`), desenhados por código com motivos do
  Japão do século XVII: `mon` (padrão; brasão no estilo dos kamon — um anel,
  *tamaki* 環, com três joias, *tama* 玉 — sobre ondas), `seigaiha` (ondas em
  escamas), `asanoha` (folha de cânhamo), `byobu` (biombo: nuvens, lua e
  monte), `shippo` (círculos entrelaçados), `hinode` (nascer do sol sobre o
  mar), `uroko` (escamas triangulares) e `ondas`.
- **Redes:** WireGuard, OpenVPN, OpenConnect, L2TP, PPTP, SSTP, IKEv2/IPsec,
  SSH, Tailscale (`ujust setup-tailscale`); SMB, NFS, SSHFS, MTP, iPhone,
  rclone; mDNS, impressoras e scanners sem driver.

## Atalhos (`Super+K` mostra todos)

| Atalho | Ação |
|---|---|
| `Super+Enter` / `Super+Espaço` | terminal / lançador |
| `Super+A` / `Super+Alt+Espaço` | ações rápidas / menu do sistema |
| `Super+Shift+A` | agente de IA |
| `Super+W` / `F` / `M` | fechar / tela cheia / maximizar |
| `Super+1…9`, `+Shift` | ir para o workspace / mandar a janela |
| `Super+Setas`, `+Shift` | foco / mover a janela |
| `Super+T` / `Super+Y` | janela flutuante / ligar-desligar tiling |
| `Super+B` / `E` | navegador / arquivos |

## Estrutura do repositório

```
Containerfile            estágios: llama.cpp → apps Omarchy → contexto → imagem
build_files/             scripts numerados do build; packages.txt
system_files/            árvore copiada para a raiz da imagem
  usr/bin/tamagi         o sistema num arquivo (init, diff, apply)
  usr/bin/tamagi-*       bank, boot-report, browser, warsaw, pkg, harden, ai, agent, update,
                         webapp, theme, menu, assist, keys, firstrun
  usr/libexec/tamagi/    browser-provision, warsaw-box, omarchy-repo (rodam dentro
                         das caixas), firstboot, popup
live/                    mídia de instalação: Containerfile, módulo do tik, tamagi-install,
                         build-iso.sh
tests/                   image.sh, desktop.sh, agent.sh, browser.sh, iso.sh + vm_check.py
                         (--luks, --en)
disk_config/             disco de VM pelo bootc-image-builder (alternativa, pede sudo)
```

## Por que mais uma distro?

*Um texto curto, inspirado em Max Weber.*

Weber observou que o mundo moderno troca o costume pela regra: tudo passa a
ser planejado, previsível e igual para todos. O Linux seguiu esse caminho.
Antes, cada computador era ajustado à mão pelo dono. Hoje existe o sistema
que não se altera: o mesmo em todas as máquinas, atualizado de uma vez e com
volta garantida se algo der errado. O Tamagi adota esse modelo porque ele
cumpre o que promete: o computador liga amanhã igual a hoje.

Weber também avisou do preço. Regra demais vira uma jaula: tudo funciona, mas
a pessoa já não decide nada. No sistema que não se altera, isso aparece
quando o programa do banco não instala ou o navegador de que você precisa não
existe ali. O Tamagi mantém o centro do sistema fechado e protegido, e em
volta dele deixa espaços livres onde você instala o que quiser. A regra
protege o sistema, mas não manda em quem usa.

Ele distinguia ainda quem age pela pureza do princípio de quem age pensando
nas consequências. Pelo princípio, a resposta seria "troque de banco". O
Tamagi fica com as consequências: as pessoas no Brasil precisam pagar contas
e declarar imposto hoje, com os bancos que existem. Então o programa do banco
roda, mas isolado, longe do sistema e dos seus arquivos.

Fazer mais uma distro quase nunca é o caminho mais barato; seria mais fácil
se adaptar a uma que já existe. O Tamagi existe por causa de uma ideia: um
sistema seguro e previsível não deveria obrigar ninguém a abrir mão da sua
língua, do seu banco e das suas ferramentas. Nenhuma das distros de que ele
descende foi feita para esse caso. Alguém precisava juntar as peças.

## Agradecimentos

O projeto agradece a todos os que tornaram esta distro possível.

- **Base: Fedora 45.** Pacotes e kernel são do Fedora, sem modificação. A
  imagem de partida é a `base-main` do
  [Universal Blue](https://universal-blue.org/), que é o Fedora com codecs,
  firmware e `ujust` já incluídos.
- **[Bazzite](https://bazzite.gg/):** ajustes de desempenho (memória, disco,
  rede), gamescope, MangoHud, GameMode e regras de controles. O kernel dele
  não é usado.
- **[Omarchy](https://omarchy.org/):** uso por teclado, temas, web apps
  e o repositório de pacotes, numa caixa Arch.
- **[Ubuntu](https://ubuntu.com/) 26.04 LTS** nas caixas `deb` e de
  navegadores: o Chrome vem do repositório apt do Google e o Firefox do da
  Mozilla (no Ubuntu eles só existem como snap, que não roda em contêiner).
- **[COSMIC](https://github.com/pop-os/cosmic-epoch)** (System76): o desktop.
- **Instalador:** [tik](https://github.com/sysrich/tik), do openSUSE Aeon.
- **[GrapheneOS](https://grapheneos.org/):** o hardened_malloc (empacotado
  pelo [secureblue](https://secureblue.dev/)) e o modelo de segurança.
- **Outros:** bootc, distrobox, llama.cpp, mise e Rust for Linux.

Nenhum desses projetos endossa o Tamagi.

## Licença

O código deste repositório é BSD de 3 cláusulas (veja `LICENSE`). A imagem
construída reúne software de terceiros, cada um com a sua licença (Fedora,
COSMIC, llama.cpp, tik e outros). Chrome e Warsaw são proprietários e não
estão aqui nem na imagem: são baixados na máquina de quem usa, dos sites dos
fabricantes; o Warsaw, só depois do aceite da licença da Topaz.

## Limites conhecidos

- Só x86_64 e só UEFI.
- O tik lista discos internos; discos USB ficam de fora como destino (padrão
  do tik). A instalação apaga o disco inteiro: não há dual boot assistido.
- Idiomas: o sistema, o instalador, o assistente de primeiro login, o menu
  (`tamagi-menu`), as ações rápidas, os atalhos e as entradas do lançador
  existem em pt-BR, en-US, es e fr. As mensagens das ferramentas de linha de
  comando (`tamagi-browser`, `tamagi-pkg`, `tamagi-ai`, `tamagi-update`,
  `tamagi-harden`, `ujust`…) ainda saem só em português; a camada de tradução
  (`/usr/libexec/tamagi/i18n`, catálogos em `/usr/share/tamagi/i18n`) já está
  pronta para elas.
- Desbloqueio por TPM: a chave é selada no PCR 7 (estado do Secure Boot).
  Ligar ou desligar o Secure Boot, ou trocar as chaves dele, faz o boot voltar
  a pedir a senha do disco — é o comportamento esperado, não um defeito.
- Testado em VM (QEMU/KVM), não em máquina física: a instalação automática
  sem e com LUKS (`just test-iso`), o instalador gráfico passo a passo até o
  login no sistema instalado, e `tamagi-harden strict` com reinício.
- A caixa de navegadores e o Warsaw foram testados na máquina de
  desenvolvimento (`just test-browser`), com GPU AMD. Intel e NVIDIA seguem o
  mesmo caminho, mas não foram medidos. O acesso a um banco de verdade não foi
  testado: só a porta, o certificado e o navegador aceitando a conexão local.
- A tela de login usa o papel de parede padrão do COSMIC, não o do Tamagi.
- HDR não existe no COSMIC; VRR e `gamescope` embutido têm falhas conhecidas
  no compositor.
- O Chrome pode pedir permissão para o site do banco acessar a rede local
  (é a conversa com o Warsaw em `127.0.0.1`): permita.
- O Warsaw é baixado por um SHA-256 fixo; quando a Topaz troca o arquivo, o
  `tamagi-warsaw` avisa e pede confirmação até a imagem atualizar o valor.
- O Claude Code tem versão fixa no `Containerfile` (`CLAUDE_CODE_VERSION`):
  só muda quando esse número é trocado.
- A imagem traz locales e traduções só de inglês, português, espanhol e
  francês. Android (Waydroid), Qwen Code, Antigravity, omacalc/omawrite e os
  escalonadores scx saíram da imagem para reduzir manutenção e tamanho.
- O rollback automático só desfaz; ele não impede que a mesma imagem com
  defeito volte na atualização automática seguinte (`ujust autoupdate off`
  pausa). `ujust rollback` continua valendo para voltar à mão.
- O modo banco sem `--manter-registro` se apresenta ao banco como um aparelho
  novo a cada sessão; alguns bancos pedem para autorizar o aparelho de novo.
- NVIDIA com driver proprietário: a base atual não traz o driver. É preciso
  trocar `BASE_IMAGE` por uma base que o traga; não foi testado.

> **Licenças:** Chrome e Warsaw são proprietários e não fazem parte da
> imagem — são baixados na sua máquina, dos sites dos fabricantes.
