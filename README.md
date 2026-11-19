# Tamagi

**Tamagi 1.0 "Kazunobu M"** — distro Linux atômica e imutável para uso diário e
desenvolvimento no Brasil. O host é uma imagem [bootc](https://github.com/bootc-dev/bootc)
sobre o Fedora 45 com o desktop [COSMIC](https://github.com/pop-os/cosmic-epoch)
(escrito em Rust). Navegadores, pacotes de outras distros e o Warsaw vivem em
caixas [distrobox](https://distrobox.it/); Android roda pelo
[Waydroid](https://waydro.id/); a IA é local, com [llama.cpp](https://llama-cpp.com/).

O sistema inteiro é um contêiner OCI: `podman build` gera a imagem, a máquina
troca de versão de forma atômica e a anterior fica guardada para rollback.

## De onde vem cada ideia

| Origem | O que o Tamagi usa |
|---|---|
| [bootc](https://github.com/bootc-dev/bootc) / Universal Blue | Sistema como imagem OCI, atualização atômica com rollback, `ujust`, codecs e firmware |
| [COSMIC](https://github.com/pop-os/cosmic-epoch) + [cosmic-flatpak](https://github.com/pop-os/cosmic-flatpak) | Desktop em Rust com tiling nativo; o remote `cosmic` da System76 já vem configurado, ao lado do Flathub |
| [Bazzite](https://bazzite.gg/) | Ajustes de desempenho (sysctl, zram, escalonador de E/S, BBR), sched_ext (`scx_lavd`), gamescope, MangoHud, GameMode, regras de controles; Steam e afins sob demanda |
| [Omarchy](https://omarchy.org/) | Fluxo por teclado, temas com um comando, **omacalc** e **omawrite**, web apps, LazyVim, mise — e o **repositório de pacotes do Omarchy** (`pkgs.omarchy.org`) pronto numa caixa Arch |
| [AnduinOS](https://www.anduinos.com/) | A caixa dos navegadores e o subsistema `.deb` são AnduinOS (Ubuntu sem snap), pela imagem oficial `aiursoft/anduinos` |
| [GrapheneOS](https://grapheneos.org/) | Endurecimento em dois níveis (`tamagi-harden`): sysctls e argumentos de kernel, MAC aleatório por rede, hardened_malloc e USBGuard opcionais |
| [astrOS](https://astros-linux.org/astros/) | Recursos opcionais ligados por um comando (Android, jogos, escalonador) em vez de tudo ativo; COSMIC como desktop único |
| [Rust for Linux](https://rust-for-linux.com/) | Kernel com `CONFIG_RUST=y`: o Binder que o Waydroid usa é o driver em Rust |
| [tik](https://github.com/sysrich/tik) (openSUSE Aeon) | Instalador de poucas perguntas, aqui com um módulo que instala pelo `bootc` |
| Vanilla OS (`apx`) | Subsistemas por formato de pacote: `tamagi-pkg` |

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
just run-iso        # abre a ISO numa VM com janela, para usar o instalador
```

A ISO inicia (só UEFI) numa sessão COSMIC live e abre o instalador, em
**português ou inglês (en_US)**: a primeira pergunta é o idioma, que vale para
o instalador e para o sistema instalado. Depois: teclado, fuso horário, a sua conta, criptografar ou não o disco
(LUKS2) e a confirmação do disco que será apagado. O sistema é copiado da
própria mídia, sem rede, em cerca de 3 minutos numa VM. Quem quiser só
experimentar fecha a janela; o instalador fica no lançador.

No primeiro login abre o assistente do Tamagi (tema, tiling, navegador, caixa
de desenvolvimento, IA, Android, Warsaw). O assistente inicial do COSMIC não
abre sozinho, porque repete as perguntas do instalador; ele continua no
lançador.

Como funciona por dentro:

- `live/Containerfile` parte da imagem do sistema e acrescenta o tik (fixado
  por tag e commit), o módulo `live/tik/modules/pre/10-tamagi`, um initramfs
  live e o usuário `live`.
- O módulo faz as perguntas e substitui a etapa do tik que gravaria uma
  imagem de disco por `/usr/libexec/tamagi-install`: particiona (ESP, `/boot`
  ext4, raiz btrfs com zstd, LUKS2 se pedido) e roda
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

## Navegadores (caixa AnduinOS)

O host não traz navegador. O Chrome vive numa caixa distrobox AnduinOS, criada
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
AnduinOS. O do Arch já nasce com o repositório `[omarchy]`
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

## Android (Waydroid)

```bash
ujust setup-waydroid     # baixa o Android (~1 GB), ajusta firewall e SELinux, liga o serviço
tamagi-waydroid open     # ou o ícone Waydroid no lançador; --cage abre numa janela isolada
tamagi-waydroid status | windows on|off | install app.apk | stop | reset
```

O Waydroid vem na imagem, desligado. Não há módulo de kernel para instalar: o
kernel do Fedora traz o Binder em Rust embutido.

## Jogos

`gamescope`, MangoHud, GameMode, as regras de controles (`steam-devices`) e os
escalonadores sched_ext já vêm na imagem. O resto é sob demanda:

```bash
ujust setup-gaming       # Steam, Lutris, Heroic, ProtonUp-Qt, Bottles (Flatpak)
ujust scx on             # escalonador scx_lavd (volta ao padrão com: ujust scx off)
```

## Segurança

O padrão é o nível **equilibrado**: sysctls de endurecimento, argumentos de
kernel (`init_on_alloc`, `slab_nomerge`, `randomize_kstack_offset`…), MAC
aleatório por rede Wi-Fi, firewall e SELinux ligados — nada que quebre jogos,
distrobox, Flatpak, Waydroid ou GPU.

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
claude | qwen | antigravity          # agentes de terceiros, já instalados
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
  tiling ligado, teclado ABNT2, pt-BR.
- **Flatpak (primeiro boot):** OnlyOffice, Flatseal, Warehouse, Mission Center,
  Loupe, Papers, DistroShelf, COSMIC Tweaks, Examine e três applets do COSMIC.
- **Omarchy:** omacalc (`Super+Shift+C`) e omawrite (`Super+Shift+W`),
  compilados do código-fonte; seguem o tema do Tamagi.
- **Temas:** `tamagi-theme set <tema>` — tokyo-night, catppuccin, gruvbox,
  nord, everforest, rose-pine, kanagawa, matte-black, ristretto, latte. Trocam
  claro/escuro, papel de parede e a paleta das janelas do Tamagi (não as cores
  de destaque do COSMIC).
- **Papéis de parede:** cinco estilos em cada tema, nas cores dele
  (`tamagi-theme wallpaper <estilo>`), desenhados por código com motivos do
  Japão do século XVII: `mon` (padrão; brasão no estilo dos kamon — um anel,
  *tamaki* 環, com três joias, *tama* 玉 — sobre ondas), `seigaiha` (ondas em
  escamas), `asanoha` (folha de cânhamo), `byobu` (biombo: nuvens, lua e
  monte) e `ondas`.
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
  usr/bin/tamagi-*       bank, boot-report, browser, warsaw, pkg, waydroid, harden, ai, agent, update,
                         webapp, theme, menu, assist, keys, firstrun
  usr/libexec/tamagi/    browser-provision, warsaw-box, omarchy-repo (rodam dentro
                         das caixas), firstboot, popup
live/                    mídia de instalação: Containerfile, módulo do tik, tamagi-install,
                         build-iso.sh
tests/                   image.sh, desktop.sh, agent.sh, browser.sh, iso.sh + vm_check.py
                         (--luks, --en)
disk_config/             disco de VM pelo bootc-image-builder (alternativa, pede sudo)
```

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
- O instalador e o idioma do sistema existem em português e inglês; as
  ferramentas `tamagi-*` e o assistente de primeiro login falam só português.
- Testado em VM (QEMU/KVM), não em máquina física: a instalação automática
  sem e com LUKS (`just test-iso`), o instalador gráfico passo a passo até o
  login no sistema instalado, `tamagi-harden strict` com reinício e
  `tamagi-waydroid setup` (imagens baixadas, serviço e firewall prontos). O
  Android em si não foi aberto: a VM de teste não tem GPU.
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
- Claude Code, Qwen Code e Antigravity só atualizam quando a imagem é
  reconstruída (o build diário do CI cobre isso). O repositório RPM do
  Antigravity não é assinado pelo Google.
- O rollback automático só desfaz; ele não impede que a mesma imagem com
  defeito volte na atualização automática seguinte (`ujust autoupdate off`
  pausa). `ujust rollback` continua valendo para voltar à mão.
- O modo banco sem `--manter-registro` se apresenta ao banco como um aparelho
  novo a cada sessão; alguns bancos pedem para autorizar o aparelho de novo.
- NVIDIA com driver proprietário: a base atual não traz o driver. É preciso
  trocar `BASE_IMAGE` por uma base que o traga; não foi testado.

> **Licenças:** Chrome e Warsaw são proprietários e não fazem parte da
> imagem — são baixados na sua máquina, dos sites dos fabricantes.
