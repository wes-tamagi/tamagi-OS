# Tamagi

**Tamagi 1.0 "Kazunobu M"** — distro Linux atômica e imutável para uso diário e
desenvolvimento no Brasil. O host é uma imagem [bootc](https://github.com/bootc-dev/bootc)
sobre o [Universal Blue](https://universal-blue.org/) com o desktop [COSMIC](https://github.com/pop-os/cosmic-epoch)
(escrito em Rust). Navegadores e pacotes de outras distros vivem em
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

**2. Mexer no sistema sem mexer na imagem.** `tamagi-layer add <nome>
<pacote>` põe pacotes por cima da imagem numa camada somente-leitura
(`systemd-sysext`) que sai com um comando. A imagem continua idêntica à
publicada, e o rollback continua valendo.

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
just test           # 300 verificações dentro da imagem
just test-desktop   # sobe a sessão COSMIC sem monitor e salva output/desktop.png
just test-browser   # cria de verdade a caixa de navegadores neste computador
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

**Instalação atômica:** antes de apagar o disco o instalador confere o
tamanho dele e a imagem embutida inteira (SHA-256 de cada camada); mídia com
defeito é recusada com o disco intacto. Depois, a partição EFI é criada com o
atributo GPT que manda a firmware ignorá-la (`NoBlockIOProtocol`): enquanto a
cópia acontece, e também se faltar energia ou o instalador for interrompido, o
computador não inicia por aquele disco. O atributo só sai no último passo, com
tudo gravado e conferido: essa única escrita na tabela de partições é o que
transforma o disco num Tamagi instalado. Se a instalação falhar por erro, as
partições criadas ainda são limpas. O resultado é um sistema atômico
(deployment bootc/ostree): cada atualização entra como uma imagem inteira ao
lado da atual, e a anterior fica guardada para rollback.

**Disco criptografado sem digitar senha:** se o computador tem chip TPM 2.0,
o instalador oferece guardar a chave do disco nele (`systemd-cryptenroll`).
No primeiro boot o `tamagi-tpm-seal.service` amarra essa chave ao PCR 7
(estado do Secure Boot). O computador liga direto e o disco continua ilegível fora daquela
máquina; a senha criada na instalação fica de reserva (troca de placa-mãe,
atualização de firmware). `just test-iso --tpm` confere isso com um TPM
virtual.

No primeiro login abre o assistente do Tamagi (tema, barra, tiling, navegador, caixa
de desenvolvimento, IA, Android). O assistente inicial do COSMIC não
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
Em quem já roda Bluefin, Aurora, Bazzite ou outro sistema da família Universal
Blue: `sudo bootc switch <imagem>`.

## Navegadores (caixa Fedora)

O host não traz navegador. O Chrome vive numa caixa distrobox Fedora (o mesmo sistema do host), criada
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

## Perfis isolados: pessoal e trabalho

```bash
tamagi-browser profile add trabalho     # cria o perfil e um atalho "Chrome (trabalho)" no menu
tamagi-browser profile add pessoal
tamagi-browser chrome --perfil trabalho # abre direto
tamagi-browser profile list | remove <nome>
```

Cada perfil é uma pasta de dados separada do navegador: logins, cookies,
extensões, histórico e senhas não se misturam, e cada um abre como um
aplicativo próprio na barra. Vale para Chrome, Brave, Edge, Vivaldi e Firefox.

**Chave de segurança (FIDO2/passkey):** conecte a chave antes de abrir o
navegador (veja [Limites conhecidos](#limites-conhecidos)). `tamagi-browser
fido` mostra se ela é vista pelo sistema e pela caixa dos navegadores.

**Internet banking:** o Tamagi não traz mais o módulo de segurança dos bancos
(Warsaw). Para banco, use uma máquina virtual: o virt-manager vem nativo na
imagem, com KVM, firmware UEFI, TPM virtual e pastas compartilhadas
(veja [Máquinas virtuais](#máquinas-virtuais)).

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

## Camadas: pacotes do Fedora por cima da imagem

Para o que precisa estar no sistema de verdade (um driver, um serviço, uma
ferramenta de baixo nível), sem refazer a imagem:

```bash
tamagi-layer add ferramentas strace ltrace   # cria a camada e liga (pede senha)
tamagi-layer list                            # camadas, pacotes e estado
tamagi-layer remove ferramentas              # o sistema volta a ser a imagem
tamagi-layer off | on                        # desliga / religa todas
tamagi-layer temp                            # /usr gravável até reiniciar
```

Cada camada é um arquivo erofs somente-leitura em `/var/lib/extensions`, que o
[systemd-sysext](https://www.freedesktop.org/software/systemd/man/latest/systemd-sysext.html)
sobrepõe ao `/usr`. A imagem não é alterada. A construção não pede senha: o
dnf resolve as dependências contra os pacotes da imagem e a camada leva só o
que ainda não existe nela. `--copr dono/projeto` acrescenta um COPR.

Limites, de propósito: os scripts de instalação dos pacotes não rodam (pacote
que cria usuário ou mexe no boot pede imagem nova); uma camada que atualizaria
um pacote da imagem é recusada (`--allow-upgrade` força); depois de uma
atualização do sistema a camada fica "desatualizada" e o `tamagi-update` a
refaz; se um boot falha na verificação de saúde, as camadas são desligadas
antes de qualquer rollback. No `tamagi.toml`: `[camadas]` com
`nome = ["pacote", …]`.

App gráfico continua sendo Flatpak; pacote de outra distro, `tamagi-pkg`;
ferramenta de desenvolvimento, mise.

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

### Pop!_OS em VM, com imagem compactada própria

```bash
tamagi-popos instalar          # baixa a ISO oficial (SHA256 da System76) e abre a VM no virt-manager
tamagi-popos compactar         # depois de instalar: limpa (machine-id, chaves SSH, logs) e compacta num modelo
tamagi-popos nova trabalho     # VM nova a partir do modelo, em segundos (clone ligado, começa com poucos MB)
tamagi-popos lista | abrir <nome> | remover <nome>
tamagi-popos exportar ~/popos.qcow2   # leva o modelo para outra máquina (importar do outro lado)
```

Roda sem root, na sessão do usuário do libvirt (no virt-manager: "QEMU/KVM
sessão do usuário", que o comando adiciona), com UEFI, CPU do host, disco
virtio que só cresce com o uso, rede pelo passt e vídeo virtio com 3D (o
COSMIC do Pop!_OS fica fluido). A instalação em si é a do Pop!_OS, feita uma
vez à mão; a imagem compactada nunca altera o disco da VM original, e o
`tamagi-popos` recusa trocar o modelo enquanto houver VMs criadas a partir
dele. Discos em `~/.local/share/libvirt/images`, fora dos snapshots da pasta
pessoal.

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

## Dois recursos do Tamagi

### O que muda nesta atualização

Quando uma atualização fica pronta, a notificação já diz o que muda (ex.:
"Kernel: 7.3.4 → 7.3.6 · Desktop COSMIC: 1.11 → 1.12 · 214 pacotes
atualizados") em vez de só "reinicie". `tamagi-update novidades` mostra os
destaques (kernel, COSMIC, Mesa, systemd, firmware, rede, áudio…), os pacotes
que entram e saem e, com a IA local ligada (`tamagi-ai`), um resumo de três
frases no seu idioma, escrito só a partir da lista real de pacotes. Vem do
`rpm-ostree db diff` entre a imagem em uso e a que está pronta.

### Abrir isolado

Para o script baixado da internet, o AppImage de origem duvidosa, o binário que
um tutorial manda rodar: `tamagi-isolar arquivo` (ou, no COSMIC Files, botão
direito → Abrir com → "Abrir isolado"). O programa roda numa jaula
descartável do bubblewrap (o mesmo dos Flatpaks): **sem rede**, com a **pasta
pessoal escondida** (ele vê uma pasta vazia), o resto do sistema **só para
leitura**, sem D-Bus e sem ver os outros processos — mas com tela, som e placa
de vídeo. O arquivo é copiado para dentro (o original não muda) e a jaula é
apagada no fim (`--manter` guarda para inspecionar; `--rede` libera a rede).
Limite: o X11 (Xwayland) fica disponível para os programas que só falam X11,
e um programa X11 pode enxergar outras janelas X11.

## Programas do Windows

Dois caminhos, conforme o programa:

- **Bottles (Wine), o padrão:** oferecido no primeiro login, e dois cliques num
  `.exe` ou `.msi` abrem nele. Não precisa de Windows nem de licença, não
  reserva memória e usa a GPU de verdade (DXVK/VKD3D). Bom para programas e
  jogos Win32; Office 365 e Adobe costumam não funcionar.
- **Windows 11 de verdade, sob demanda:** `tamagi-windows` (ou "Windows 11 de
  verdade" no `Super+Espaço`). Roda o [dockur/windows](https://github.com/dockur/windows)
  (QEMU/KVM num contêiner Podman sem root, como unidade Quadlet que só liga
  quando você abre) e mostra a área de trabalho pelo FreeRDP (Flatpak). É para
  Office 365, Adobe, programas de banco/certificado e aparelhos USB com driver
  só para Windows.

```bash
tamagi-windows            # abre (instala na primeira vez, depois do aviso de licença)
tamagi-windows parar      # desliga e libera a memória
tamagi-windows navegador  # a tela pelo navegador (acompanhar a instalação)
tamagi-windows status | remover
```

O Windows não vem na imagem: é baixado da Microsoft na sua máquina (~6 GB,
10–30 min para instalar sozinho) e precisa de licença válida (sem chave,
funciona sem ativar). Ligado, reserva um quarto da memória (4 a 8 GB); exige
12 GB de RAM e 40 GB livres. A conta do Windows recebe uma senha aleatória
guardada em `~/.config/tamagi/windows.env`; a pasta `~/Windows` aparece lá
dentro. O disco do Windows fica num subvolume à parte, fora dos snapshots da
pasta pessoal. Sem aceleração de GPU dentro do Windows, e anti-cheat de kernel
não roda em nenhum dos dois caminhos.

Ficaram de fora por enquanto: o **WinBoat** (336 issues abertas, pouca
atividade, sem USB nem GPU no Podman) e o **WinPodX** (beta, ainda sem Fedora
45, e embute um patch de sessões RDP simultâneas que conflita com a licença
do Windows).

## Do Omarchy: desenvolvimento, captura e atalhos

```bash
tamagi-dev                         # escolhe: web (Node+pnpm), bun, deno, python, ruby, go, rust, java, dotnet, zig
tamagi-dev add web python          # via mise, na pasta pessoal; só linguagens com binário pronto
tamagi-font                        # fonte mono de tudo: foot, alacritty, COSMIC e apps GTK
tamagi-capture texto               # Shift+Print: seleciona uma área e copia o texto (OCR, Tesseract)
tamagi-capture gravar              # Alt+Print: grava a tela (GPU Screen Recorder, portal); de novo, para
tamagi-db                          # escolhe: postgres, mysql, mariadb, redis, valkey, mongo
tamagi-db add postgres             # 127.0.0.1:5432, sem senha, liga com a sessão
tamagi-db list | stop | start | remove <banco>
tamagi-lembrete 25m "Pausa"        # notificação daqui a 25 minutos
```

Os bancos são contêineres Podman sem root (Quadlet em
`~/.config/containers/systemd/`), escutando só em `127.0.0.1`, com os dados em
volumes `tamagi-db-<banco>`. Sem senha, como no Omarchy: só para
desenvolvimento local.

Atalhos no estilo do Omarchy (`Super+K` lista todos): `Super+Shift+G` lazygit,
`Super+Shift+D` bancos, `Super+Shift+O` OnlyOffice, `Super+Shift+M` Mission
Center, `Super+Shift+W` Windows, `Super+Ctrl+E` emojis (Smile, instalado no
primeiro uso), `Super+Ctrl+R` lembrete. Com LazyVim (`ujust setup-lazyvim`),
o `tamagi-theme` também troca o colorscheme do Neovim (os mesmos plugins do
Omarchy; no Ipê, base16 com as cores do tema). Elixir, PHP e o que precisa
compilar ficam na caixa Arch (`tamagi-pkg install --arch …`). Ficaram de fora
o conta-gotas de cor (o portal do COSMIC ainda não tem) e o que depende do
Hyprland (clipboard universal, gaps, scratchpad).

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

### Histórico da pasta pessoal (ideia do HAMMER2, do DragonFly BSD)

O sistema já volta sozinho (bootc + greenboot); os seus arquivos também. O
snapper tira um snapshot btrfs da pasta pessoal a cada hora e guarda 24 por
hora, 7 diários e 4 semanais. `~/.cache` e os contêineres ficam de fora.

```bash
tamagi-snap                      # estado e snapshots
tamagi-snap versoes ~/Documentos/contrato.odt
tamagi-snap voltar ~/Documentos/contrato.odt      # a versão mais recente diferente da atual
tamagi-snap voltar ~/Documentos/contrato.odt 42   # a do snapshot 42
tamagi-snap agora "antes de mexer"                # um snapshot agora
```

A versão atual nunca é sobrescrita: fica ao lado como `<arquivo>.antes-<data>`.
Os snapshots também abrem no COSMIC Files em `~/.snapshots/<N>/snapshot`
(arquivos ocultos visíveis). Instalações novas já vêm com isso ligado: a pasta
pessoal nasce como subvolume btrfs. Numa instalação anterior, `tamagi-snap
ativar` agenda a conversão para o próximo boot, antes do login. A cópia é com
reflink, então é rápida e não ocupa o dobro, e a pasta original fica em
`/var/home/.<você>.antes-snapshots` até `tamagi-snap limpar-copia`.

### Agente preso pelo Landlock (ideia do pledge/unveil, do OpenBSD)

O que o `tamagi-agent` roda sem um humano olhar fica preso pelo Landlock do
kernel (`/usr/libexec/tamagi/sandbox`), sem root nem contêiner:

| Comando | Como roda |
|---|---|
| só leitura (roda sem perguntar) e `read_file` | lê o sistema, **menos os segredos**; não grava (só numa pasta temporária própria), não abre conexão TCP, não manda sinal para fora |
| aprovado sozinho pelo `-y` | grava nas subpastas da pasta pessoal, em `/tmp` e em `/run/user`; os segredos ficam só leitura (não se apagam, movem nem trocam) |
| confirmado por você | roda normal: você leu o comando |

Segredos: `~/.ssh`, `~/.gnupg`, `~/.password-store`, tokens (`~/.config/gh`,
`~/.claude`, `~/.aws`, `~/.kube`…), chaveiros e perfis de navegador; mais o
que você listar em `~/.config/tamagi/sandbox-segredos`. Desligar:
`TAMAGI_AGENT_SANDBOX=0`. Limites: no modo `-y` não se cria nem apaga
arquivo direto na raiz da pasta pessoal (nas subpastas, sim), e programas que
dependem de setuid (`sudo`, podman e distrobox sem root) não funcionam presos;
eles sempre pedem confirmação.

## Rust

- **Desktop:** COSMIC 1.11 inteiro (compositor, painel, terminal, arquivos,
  editor, loja, player, tela de login).
- **Kernel:** o do Bazzite (Open Gaming Collective, 7.3), assinado pelo
  Universal Blue; `CONFIG_RUST=y`, Binder, sched_ext.
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
claude                   # Claude Code: baixado da Anthropic no primeiro uso (também no menu e na dock)
tamagi-antigravity       # Antigravity (IDE do Google), instalado na caixa de navegadores no primeiro uso
```

- **llama.cpp v1.0.0** compilado na imagem com Vulkan e variantes de CPU
  escolhidas em tempo de execução; serve em `127.0.0.1:9931` (API OpenAI e
  Anthropic) com `--jinja`, `--flash-attn auto`, `--fit on` e suporte a
  `reasoning_content`, carrega o modelo sob demanda e o libera após 15 min parado.
- **`tamagi-agent`** (alias `ia`): agente próprio que conhece o sistema e usa o
  registro declarativo `/usr/share/tamagi/actions.toml` (`tamagi_action`).
  Comandos somente-leitura rodam direto, presos pelo Landlock (sem rede, sem
  gravar, sem ver chaves e tokens); o resto pede confirmação. Veja Segurança.
- **mise** já ativado no shell: `mise use -g rust@latest node@lts python@3.13`.

| Preset | Perfil | Modelo | Tamanho | Para |
|---|---|---|---|---|
| `mini` | `general` | Gemma 4 E2B | 2,8 GB | qualquer máquina |
| `leve` | `general` | Gemma 4 E4B | 4,6 GB | 8–16 GB de RAM |
| `mistral` | `reasoning` | Ministral 3 8B Reasoning | 5,2 GB | raciocínio rápido (16 GB RAM) |
| `agente` | `agent` | Ornith 1.5 9B | 5,8 GB | agente local e tool calling (16 GB RAM / 8 GB VRAM) |
| `medio` | `general` | Gemma 4 12B | 7,2 GB | 16 GB+ de RAM ou GPU de 8–12 GB |
| `devstral` | `code` | Devstral Small 2 24B | 14,5 GB | engenharia de software multi-arquivo (Mistral) |
| `qwen38` | `general` | Qwen 3.8 27B | 16,5 GB | geração mais recente da Qwen, raciocínio híbrido |
| `codigo` | `code` | Qwen3 Coder 30B (MoE) | 18,0 GB | programação rápida, 32 GB+ de RAM |
| `ornith` | `agent` | Ornith 1.5 35B (MoE) | 20,5 GB | agente autônomo de topo (48 GB RAM / 24 GB VRAM) |
| `grande` | `general` | Qwen 3.6 35B (MoE) | 20,5 GB | 48 GB+ de RAM ou GPU de 24 GB |
| `workstation` | `workstation` | Mistral Small 4 119B | 68,0 GB | 96 GB+ de RAM / multi-GPU |

## O que vem instalado

- **Desktop:** COSMIC, com terminal, arquivos, editor, loja e player próprios;
  tiling ligado, teclado ABNT2. O COSMIC Files abre em grade, pastas primeiro.
- **Terminal:** o [foot](https://codeberg.org/dnkl/foot) é o padrão
  (`Super+Enter`, apps de terminal, dock), com a fonte e as cores do tema; o
  cosmic-term continua instalado, no lançador. O `tamagi-theme` grava
  `~/.config/foot/foot.ini`; para personalizar, apague a primeira linha
  ("Gerado por tamagi-theme") e o arquivo deixa de ser reescrito.
- **Barra:** uma só, embaixo, com os apps no centro, a bandeja e o relógio à
  direita — o layout "Centered" do AnduinOS feito com os applets do COSMIC
  (flutuante, cantos arredondados, levemente translúcida).
  `tamagi-panel set centro|classico|cosmic` troca: `classico` põe os apps à
  esquerda; `cosmic` volta ao painel em cima + dock.
- **Menu iniciar:** o primeiro botão da barra abre um menu junto ao próprio
  botão, com o brasão do Tamagi como ícone: lista de apps, lugares (pasta
  pessoal, documentos, configurações), busca e os botões de energia
  ([cosmic-ext-classic-menu](https://github.com/championpeak87/cosmic-ext-classic-menu),
  GPL-3.0, layout "Retro"; os outros layouts ficam nas configurações do menu).
  A versão 1.1.0 é compilada na imagem (o COPR parou na 1.0.0) com o "menu
  focado" ligado: abriu, já dá para digitar e usar as setas, sem clicar na
  busca. Ele abre no canto da tela do lado do botão. O lançador do COSMIC fica no
  `Super+Alt+Espaço`.
- **Tela de boot:** o brasão do Tamagi, o anel (環, *tamaki*) com as três
  joias (玉, *tama*), nas cores do tema Ipê; as joias giram enquanto o sistema
  sobe, e a senha do disco aparece no mesmo fundo. Desenhado por código no
  build (`build_files/make-plymouth.py`), como os papéis de parede.
- **Idiomas:** português (Brasil), inglês (EUA), espanhol e francês, no
  sistema, no instalador, no assistente de primeiro login, no menu e nos
  atalhos. `tamagi-lang pick` troca depois de instalado.
- **Fontes:** Noto Sans na interface, Roboto Slab como fonte com serifa
  (documentos), JetBrains Mono no terminal.
- **Apps (inspirado no primeiro login do openSUSE Aeon):** no primeiro boot
  entram sozinhos só os essenciais e leves (Flatseal, Warehouse, Loupe, Papers
  e três applets do COSMIC). No primeiro login, uma decisão só: **Instalar o
  recomendado** (Chrome, Mission Center, Pika Backup) com um Enter, ou
  **Escolher app por app** num catálogo de ~30 apps por categoria
  (navegadores, escritório, comunicação, criação, jogos, desenvolvimento e
  sistema), com o tamanho de cada um e preferência pelos verificados pelo
  próprio fabricante no Flathub. Diferente do Aeon, os apps baixam **em segundo
  plano** (unidade do systemd do usuário, esperando a rede): o assistente segue
  com as outras perguntas e uma notificação avisa quando termina
  (`/usr/libexec/tamagi/apps-setup status`). Escolhendo só o Firefox ou o
  Brave, ele vira o navegador padrão. O catálogo fica em
  `/usr/share/tamagi/flatpaks.list`; o que ficar de fora continua no
  `Super+Espaço` (Instalar).
- **Temas:** `tamagi-theme set <tema>` — tokyo-night, catppuccin, gruvbox,
  nord, everforest, rose-pine, kanagawa, matte-black, ristretto, latte e ipe
  (o tema próprio do Tamagi: verde-noite com destaque no amarelo do ipê).
  Trocam as cores do COSMIC (fundo, destaque e texto), o claro/escuro, o papel
  de parede, o terminal (foot), o btop, o Neovim (LazyVim) e a paleta das
  janelas do Tamagi. Cada tema vira um arquivo em
  `/usr/share/cosmic-themes/tamagi-<tema>.ron`, que também aparece em
  Configurações > Aparência > Importar.
- **Papéis de parede:** oito estilos em cada tema, nas cores dele
  (`tamagi-theme wallpaper <estilo>`), desenhados por código com motivos do
  Japão do século XVII: `mon` (padrão; brasão no estilo dos kamon — um anel,
  *tamaki* 環, com três joias, *tama* 玉 — sobre ondas), `seigaiha` (ondas em
  escamas), `asanoha` (folha de cânhamo), `byobu` (biombo: nuvens, lua e
  monte), `shippo` (círculos entrelaçados), `hinode` (nascer do sol sobre o
  mar), `uroko` (escamas triangulares) e `ondas`.
- **Redes:** WireGuard, OpenVPN, OpenConnect, L2TP, PPTP, SSTP, IKEv2/IPsec,
  SSH, Tailscale do repositório oficial (`ujust setup-tailscale`); SMB, NFS, SSHFS, MTP, iPhone,
  rclone; mDNS, impressoras e scanners sem driver.

## Atalhos (`Super+K` mostra todos)

| Atalho | Ação |
|---|---|
| `Super+Enter` / `Super+Espaço` | terminal (foot) / paleta de comandos (`tamagi-menu`) |
| `Super+A` / `Super+Alt+Espaço` | ações rápidas / lançador de aplicativos COSMIC |
| `Super+Shift+A` | agente de IA |
| `Super+W` / `F` / `M` | fechar / tela cheia / maximizar |
| `Super+1…9`, `+Shift` | ir para o workspace / mandar a janela |
| `Super+Setas`, `+Shift` | foco / mover a janela |
| `Super+T` / `Super+Y` | janela flutuante / ligar-desligar tiling |
| `Super+B` / `E` | navegador / arquivos |

## Kernel, módulos e assinatura

- **O kernel do Bazzite, o padrão:** o do
  [Open Gaming Collective](https://github.com/OpenGamingCollective/linux)
  (sabor `ogc`), o mesmo do Bazzite: patches de agendamento, controles,
  portáteis e jogos, sobre a versão estável do Linux. Uma imagem só é
  publicada (`latest`).
- **Módulos do Universal Blue** ([ublue-os/akmods](https://github.com/ublue-os/akmods)),
  prontos e assinados para esse kernel: `xone` e `xpadneo` (controles Xbox por
  cabo, adaptador e Bluetooth) e `v4l2loopback` (câmera virtual).
- **Secure Boot:** o kernel do Bazzite é assinado pelo Universal Blue, não
  pelo Fedora. Com o Secure Boot ligado, registre a chave uma vez:
  `ujust enroll-secure-boot-key` (senha `universalblue`), reinicie e confirme
  "Enroll MOK" (conferir: `mokutil --test-key /etc/pki/akmods/certs/akmods-ublue.der`).
  Com o Secure Boot desligado não há nada a fazer. Proteções:
  - numa instalação existente, o `tamagi-secureboot-guard` confere cada
    atualização preparada e a **cancela** se ela não fosse iniciar (Secure
    Boot ligado, chave ausente), avisando o que fazer — o firmware recusaria
    o kernel antes de o greenboot poder agir;
  - na ISO, a chave vai na partição EFI: se o shim recusar o kernel, o
    MokManager abre; "Enroll key from disk" → `TAMAGI_EFI` →
    `universalblue.der` registra a chave e a ISO inicia.
- **Outros sabores, para build local:** `KERNEL_FLAVOR=main just build` (o
  kernel da base, assinado pelo Fedora, sem exigir chave) e
  `KERNEL_FLAVOR=ark just build` (o kernel-ark, do Fedora Rawhide, assinado
  pelo Fedora; sem os módulos do Universal Blue). O build do `ark` recusa um
  kernel com depuração pesada ou sem a assinatura do Fedora.
- **Imagem assinada (cosign):** o CI segue o
  [ublue-os/image-template](https://github.com/ublue-os/image-template) e
  assina cada imagem publicada com a chave do repositório. Conferir:
  `cosign verify --key cosign.pub ghcr.io/wes-tamagi/tamagi:latest`. A chave
  privada fica só no segredo `SIGNING_SECRET` do GitHub.

## Estrutura do repositório

```
Containerfile            estágios: llama.cpp → apps Omarchy → contexto → imagem
build_files/             scripts numerados do build; packages.txt
system_files/            árvore copiada para a raiz da imagem
  usr/bin/tamagi         o sistema num arquivo (init, diff, apply)
  usr/bin/tamagi-*       layer, boot-report, browser, pkg, harden, ai, agent, update,
                         webapp, theme, menu, assist, keys, firstrun
  usr/libexec/tamagi/    browser-provision, omarchy-repo (rodam dentro
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
roda, mas numa máquina virtual, longe do sistema e dos seus arquivos.

Fazer mais uma distro quase nunca é o caminho mais barato; seria mais fácil
se adaptar a uma que já existe. O Tamagi existe por causa de uma ideia: um
sistema seguro e previsível não deveria obrigar ninguém a abrir mão da sua
língua, do seu banco e das suas ferramentas. Nenhuma das distros de que ele
descende foi feita para esse caso. Alguém precisava juntar as peças.

## Agradecimentos

O projeto agradece a todos os que tornaram esta distro possível.

- **Base: [Universal Blue](https://universal-blue.org/).** A imagem de partida
  é a `base-main`: sistema atômico com codecs, firmware, distrobox e `ujust` já
  incluídos, a mesma fundação do Bluefin, do Aurora e do Bazzite. Os pacotes
  vêm do Fedora 45, sem modificação.
- **[Bazzite](https://bazzite.gg/):** ajustes de desempenho (memória, disco,
  rede), gamescope, MangoHud, GameMode, regras de controles e o kernel (o do
  Open Gaming Collective).
- **[Omarchy](https://omarchy.org/):** uso por teclado, temas, web apps
  e o repositório de pacotes, numa caixa Arch.
- **[Ubuntu](https://ubuntu.com/) 26.04 LTS** na caixa `deb` do `tamagi-pkg`.
  A caixa dos navegadores é Fedora: o Chrome vem do repositório rpm do Google
  e o driver de vídeo completo (H.264/HEVC) do RPM Fusion.
- **[COSMIC](https://github.com/pop-os/cosmic-epoch)** (System76): o desktop.
- **Instalador:** [tik](https://github.com/sysrich/tik), do openSUSE Aeon.
- **[GrapheneOS](https://grapheneos.org/):** o hardened_malloc (empacotado
  pelo [secureblue](https://secureblue.dev/)) e o modelo de segurança.
- **Outros:** bootc, distrobox, llama.cpp, mise e Rust for Linux.

Nenhum desses projetos endossa o Tamagi.

## Licença

O código deste repositório é BSD de 3 cláusulas (veja `LICENSE`). A imagem
construída reúne software de terceiros, cada um com a sua licença (Fedora,
COSMIC, llama.cpp, tik e outros). O Chrome é proprietário e não está aqui nem
na imagem: é baixado na máquina de quem usa, do site do fabricante.

### Marcas e licenças

O Tamagi é um projeto independente, baseado no Fedora e no Universal Blue, e
não é afiliado nem endossado pelo Projeto Fedora, Red Hat, System76,
Canonical, Microsoft ou Basecamp. Fedora, Pop!_OS, COSMIC, Ubuntu, Windows e
Omarchy são marcas de seus donos e aparecem aqui só para descrever
compatibilidade ou inspiração. A imagem não traz os logos do Fedora (troca
`fedora-logos` por `generic-logos`; o os-release usa o nome, o logo e os
endereços do Tamagi).

O Tamagi não redistribui Pop!_OS nem Windows: `tamagi-popos` e
`tamagi-windows` baixam as imagens oficiais diretamente dos servidores da
System76 e da Microsoft, na máquina do usuário. As imagens criadas com
`tamagi-popos compactar` são para uso próprio; quem redistribuí-las responde
pelas licenças (GPL e outras) e pelas políticas de marca da System76 e da
Canonical. O Windows exige licença válida da Microsoft. Detalhes em `NOTICE`
(também em `/usr/share/doc/tamagi/NOTICE`).

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
- A caixa de navegadores foi testada na máquina de desenvolvimento
  (`just test-browser`), com GPU AMD. Intel e NVIDIA seguem o mesmo caminho,
  mas não foram medidos.
- Chave de segurança FIDO2 no navegador da caixa: funciona se a chave já
  estiver conectada quando o navegador abre. Conectada depois, o navegador não
  a vê até ser fechado e aberto de novo. Causa medida: o aviso de dispositivo
  conectado chega à caixa como vindo do usuário 65534, que faz parte do mapa
  de usuários do contêiner, e a libudev o descarta. Deixar só esse usuário fora
  do mapa resolve (5 de 5 eventos aceitos), mas exige mapas diferentes para
  usuários e grupos, que o distrobox não permite configurar; tirar também o
  grupo quebra o `sudo` na caixa. `tamagi-browser fido` faz o diagnóstico.
- A tela de login usa o papel de parede padrão do COSMIC, não o do Tamagi.
- HDR ainda não existe no COSMIC (issue
  [pop-os/cosmic-comp#1384](https://github.com/pop-os/cosmic-comp/issues/1384)):
  o Tamagi espera entrar no oficial. VRR e `gamescope` embutido têm falhas
  conhecidas no compositor.
- Claude Code e Antigravity são proprietários e não vêm na imagem pública:
  o `claude` baixa o Claude Code da Anthropic no primeiro uso (canal estável,
  checksum conferido) para `~/.local/bin`, de onde ele se atualiza sozinho; o
  `tamagi-antigravity` instala o Antigravity na caixa de navegadores, ao lado
  do Chrome que o agente dele usa (o repositório do Google ainda não tem
  assinatura de pacote: a integridade depende do HTTPS).
- A imagem traz locales e traduções só de inglês, português, espanhol e
  francês. Android (Waydroid), Qwen Code, omacalc/omawrite e os
  escalonadores scx saíram da imagem para reduzir manutenção e tamanho.
- O rollback automático grava o digest defeituoso em `/var/lib/tamagi/quarantine.list`,
  impedindo que a atualização automática reinstale a mesma imagem quebrada
  (`tamagi-update quarantine` / `unquarantine`). `ujust rollback` continua valendo
  para voltar à mão.
- NVIDIA com driver proprietário: a base atual não traz o driver. É preciso
  trocar `BASE_IMAGE` por uma base que o traga; não foi testado.

> **Licenças:** o Chrome é proprietário e não faz parte da imagem — é baixado
> na sua máquina, do site do fabricante.
