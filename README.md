# Tamagi

**Tamagi 1.0 "Kazunobu M"** — distro Linux atômica e imutável cujo foco é
**IA e agentes de IA**: um lugar seguro e reversível para trabalhar com
agentes de programação (Claude Code, Codex, CLIO, opencode, Qwen Code, Gemini
CLI, OpenClaw) e com modelos locais. Cada agente roda preso à pasta do projeto,
sem ver as suas chaves, e tudo o que ele fizer pode ser desfeito. O host é uma imagem [bootc](https://github.com/bootc-dev/bootc)
sobre o [Universal Blue](https://universal-blue.org/) com o desktop [COSMIC](https://github.com/pop-os/cosmic-epoch)
(escrito em Rust). A imagem é enxuta: navegadores, jogos, máquinas virtuais,
agentes de IA e ferramentas de desenvolvimento são opcionais e entram pela
escolha do usuário — por Flatpak, nativamente numa camada reversível ou em
caixas [distrobox](https://distrobox.it/). A IA local usa o [llama.cpp](https://llama-cpp.com/), compilado na imagem.

O sistema inteiro é um contêiner OCI: `podman build` gera a imagem, a máquina
troca de versão de forma atômica e a anterior fica guardada para rollback.

Veja também: [por que mais uma distro?](#por-que-mais-uma-distro) e os
[agradecimentos](#agradecimentos).

## Um comando: `tamagi`

Tudo o que é do Tamagi está atrás de um comando só, com nomes em inglês em
qualquer idioma do sistema (as mensagens saem em português, inglês, espanhol
ou francês):

```bash
tamagi                 # lista todos os comandos
tamagi update          # atualiza tudo        tamagi doctor     # diagnóstico
tamagi install         # catálogo do que é opcional
tamagi browser install firefox --flatpak     # ou --native, ou --box
tamagi agents install claude codex qwen      # agentes de IA
tamagi devbox create ubuntu                  # caixa de desenvolvimento
tamagi db add postgres                       # banco de dados para desenvolvimento
```

`tamagi <comando>` é o programa `tamagi-<comando>`. Não há mais receitas do
`ujust` repetindo esses comandos: no `ujust` ficou só o que não tem comando
próprio (`setup-lazyvim`, `switch-image` e as receitas do Universal Blue).

As mensagens são escritas em **inglês** (o idioma de origem) e traduzidas para
português, espanhol e francês por catálogos em `/usr/share/tamagi/i18n`; em
qualquer outro idioma do sistema, sai inglês.

## Agentes de IA: presos ao projeto, com volta no tempo

```bash
tamagi agents install claude codex opencode   # ou: tamagi apply ai (o perfil pronto)
cd ~/Projetos/meu-app
tamagi agents run claude                       # o agente, preso a esta pasta
tamagi agents jail                             # um shell na mesma jaula: veja o que ele vê
```

Dentro da jaula (bubblewrap, o mesmo dos Flatpaks) o agente enxerga **a pasta
do projeto e as próprias configurações**, e nada mais da sua pasta pessoal:
sem chaves SSH e GPG, sem tokens, sem perfis de navegador, sem outros
projetos, sem discos removíveis. O sistema fica só para leitura. A rede
continua ligada (o agente fala com a API dele), a não ser com `--offline`.

- **Com compiladores:** havendo a caixa de desenvolvimento (`tamagi devbox`),
  a jaula é montada dentro dela, e o agente tem Node, Python, Rust, git e os
  clientes de banco. Sem a caixa, roda no host (`--host` força isso).
- **Volta no tempo:** antes de o agente começar, o estado do projeto é
  guardado — uma referência no git (`git diff refs/tamagi/before-agent/<hora>`
  mostra tudo o que ele mudou) e um snapshot da pasta pessoal, quando os
  snapshots estão ligados (`tamagi rewind` traz de volta). Com `--db`, os
  bancos do `tamagi db` em execução ganham snapshot também.
- **Login:** faça o login do agente uma vez fora da jaula (abra `claude`,
  `codex`… normalmente); dentro dela não há navegador nem sessão do desktop.
- Sem chaves SSH, o agente não faz `git push`: quem publica é você.

## Perfis prontos

```bash
tamagi profiles        # ai, dev, gamer, office
tamagi apply ai        # agentes de IA + caixa de desenvolvimento (o sugerido no primeiro login)
tamagi apply gamer     # Steam nativo, gamescope com HDR, MangoHud, Heroic
tamagi apply https://exemplo.org/minha-maquina.toml   # um arquivo compartilhado
```

Um perfil é um `tamagi.toml` pronto em `/usr/share/tamagi/profiles`: só
acrescenta o que falta, nunca remove. Um arquivo vindo de um endereço (só
`https://`) é sempre mostrado antes de mudar a máquina, mesmo com `-y`.

## Nesta versão

Além do que está abaixo: agentes presos ao projeto e perfis prontos (acima),
inglês como idioma de origem, Tailscale opcional (`tamagi install tailscale`)
e duas retiradas — o `tamagi-pkg` com o repositório do Omarchy (pacotes de
outras distros agora vão numa caixa: `tamagi devbox create ubuntu|arch`) e a
troca de `sudo`/coreutils por versões em Rust no shell.


**Inovação 1 — o tema é do COSMIC, e os terminais o seguem.** Temas e papéis
de parede se escolhem do jeito normal, nas Configurações do COSMIC: os onze
temas do Tamagi ficam em `/usr/share/cosmic-themes` (com miniatura, junto dos
temas do próprio COSMIC) e os papéis de parede na lista do sistema. O que o
COSMIC não alcança — o terminal (foot), o btop, o Neovim e as janelas do
Tamagi — acompanha sozinho: uma unidade do usuário (`tamagi-theme-sync.path`)
vê o tema mudar e roda `tamagi theme sync`. Vale para qualquer tema do COSMIC,
não só os do Tamagi (as cores de fundo e destaque vão para a paleta do
terminal).

**Inovação 2 — `tamagi try`: experimentar sem instalar.** `tamagi try htop`
abre um shell numa caixa descartável com o pacote; ao sair, tudo some.
`--distro arch|ubuntu|opensuse`, `--here` (a pasta atual em `/work`) e `--gui`
(apps gráficos na sua tela).

**Inovação 3 — bancos de dados com volta no tempo.** `tamagi db snapshot
postgres` guarda os dados como estão; `tamagi db restore postgres` os traz de
volta (antes de uma migração arriscada, por exemplo). E mais: `tamagi db url`,
`tamagi db env` (linhas `export DATABASE_URL=…`) e `tamagi db shell` (psql,
mysql, redis-cli, mongosh, sem instalar nada).

**A mudança — imagem enxuta, tudo opcional num catálogo.** Saíram da imagem o
virt-manager/libvirt/QEMU, o gamescope/MangoHud/GameMode/steam-devices, o mise
e o opencode; nenhum navegador é pressuposto. O assistente de primeiro login
ficou curto (navegador e catálogo) e `tamagi install` oferece o mesmo catálogo
depois: navegadores (Chrome, Firefox, Vivaldi, Chromium — Flatpak, nativo ou
caixa), OnlyOffice, virt-manager nativo, Windows 11 (dockur), Steam e os
pacotes de jogos nativos como no Bazzite, caixas de desenvolvimento (Fedora,
Ubuntu, openSUSE Tumbleweed, Arch) e agentes de IA (Claude Code, Antigravity,
Codex, Qwen Code, opencode, Gemini CLI, OpenClaw e a IA local).

## Três ideias próprias

**1. O sistema num arquivo.** `~/.config/tamagi/tamagi.toml` descreve a
máquina: tema, navegadores, Flatpaks, linguagens (mise, na caixa), agentes de
IA, pacotes dos subsistemas, caixas, camadas, web apps e serviços. As seções e
chaves são em inglês (`[appearance]`, `[browsers]`, `[boxes]`…); arquivos
antigos, com as chaves em português, continuam valendo.

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
just build          # imagem do sistema (~20 min na primeira vez) + rechunk (~7 min)
just test           # ~400 verificações dentro da imagem
just test-desktop   # sobe a sessão COSMIC sem monitor e salva output/desktop.png
just test-browser   # cria de verdade a caixa de navegadores neste computador
just test-agent     # baixa o preset mini (2,8 GB) e testa o agente de IA
just lint
```

O `just build` termina com o **rechunk** (`rechunk.sh`): a imagem é refeita só
com o conteúdo real, em camadas divididas por conteúdo, pelo `rpm-ostree
compose build-chunked-oci`. A base é uma imagem ostree, e o que o build troca
ou remove dela (o kernel, os idiomas) continuaria ocupando espaço nas camadas
de baixo: são ~9,3 GB sem o rechunk e ~6,8 GB com ele (2,9 GB comprimida), e
uma atualização baixa só as camadas que mudaram. `TAMAGI_RECHUNK=0 just build`
pula esse passo durante o desenvolvimento.

## Instalar

O instalador é o [tik](https://github.com/sysrich/tik) com um módulo do Tamagi
que chama o `bootc install`. A ISO é gerada sem root:

```bash
just build-iso      # output/tamagi.iso (5,1 GiB: cabe num pendrive de 8 GB)
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

## Navegadores: Flatpak, nativo ou caixa

Nenhum navegador vem na imagem. Chrome, Firefox, Vivaldi e Chromium podem ser
instalados, cada um, de três maneiras:

```bash
tamagi browser install                     # escolhe o navegador e o método numa lista
tamagi browser install firefox             # Flatpak (o padrão): app do Flathub, isolado
tamagi browser install chromium --native   # RPM numa camada sobre a imagem (tamagi-layer; pede senha)
tamagi browser install chrome --box        # numa caixa Fedora (distrobox), com vídeo por hardware
tamagi browser list                        # o que existe e o que está instalado
tamagi browser default firefox             # navegador padrão do sistema
tamagi browser remove chrome
```

- O botão "Navegador web" da barra abre o navegador padrão; enquanto não há
  nenhum, oferece a instalação. O primeiro navegador instalado vira o padrão e
  toma o lugar desse botão.
- **Nativo:** Chrome e Vivaldi vêm do repositório do fabricante (chave
  conferida); Firefox e Chromium, do Fedora. A camada se desfaz com `tamagi
  browser remove` e é refeita a cada atualização do sistema.
- **Caixa:** enxerga a placa de vídeo, o áudio, a webcam, as chaves de
  segurança USB e a sua pasta pessoal. Os navegadores Chromium abrem com
  decodificação e codificação de vídeo por hardware (VA-API); para mudar, uma
  opção por linha em `~/.config/tamagi/browser-flags.conf`. NVIDIA com driver
  proprietário: a caixa é criada com `--nvidia`. `tamagi browser status`
  mostra VA-API e Vulkan.
- Os web apps (`tamagi webapp`) usam o navegador Chromium que estiver
  instalado (Chrome, Chromium ou Vivaldi).

Medido na máquina de desenvolvimento (Ryzen 7 5700, RX 7600), na caixa: o
`vainfo` lista H.264, HEVC, VP9 e AV1 (decodificação) e H.264, HEVC e AV1
(codificação); o Chrome 158 usou o `VaapiVideoDecoder` para H.264, VP9 e AV1,
com WebGL e WebGPU ligados e o sandbox do Chrome ativo (sem `--no-sandbox`).

## Perfis isolados: pessoal e trabalho

```bash
tamagi-browser profile add trabalho     # cria o perfil e um atalho "Chrome (trabalho)" no menu
tamagi-browser profile add pessoal
tamagi-browser chrome --profile trabalho # abre direto
tamagi-browser profile list | remove <nome>
```

Cada perfil é uma pasta de dados separada do navegador: logins, cookies,
extensões, histórico e senhas não se misturam, e cada um abre como um
aplicativo próprio na barra. Vale para Chrome, Brave, Edge, Vivaldi e Firefox.

**Chave de segurança (FIDO2/passkey):** conecte a chave antes de abrir o
navegador (veja [Limites conhecidos](#limites-conhecidos)). `tamagi-browser
fido` mostra se ela é vista pelo sistema e pela caixa dos navegadores.

**Internet banking:** o Tamagi não traz mais o módulo de segurança dos bancos
(Warsaw). Para banco, use uma máquina virtual: `tamagi vm install` traz o
virt-manager nativo, com KVM, firmware UEFI, TPM virtual e pastas
compartilhadas (veja [Máquinas virtuais](#máquinas-virtuais-nativo-sob-demanda)).

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
antes de qualquer rollback. No `tamagi.toml`: `[layers]` com
`nome = ["pacote", …]`.

App gráfico continua sendo Flatpak; pacote de outra distro e ferramenta de
desenvolvimento vão na caixa (`tamagi devbox`, com o mise lá dentro).

## mise: linguagens e ferramentas, dentro da caixa

O [mise](https://mise.jdx.dev/) continua sendo o gerenciador de versões de
linguagens e ferramentas, mas saiu do host: vive dentro da caixa de
desenvolvimento, onde estão os compiladores e as bibliotecas. O host imutável
fica sem runtimes.

```bash
tamagi devbox lang                    # escolhe: web, bun, deno, python, ruby, go, rust, java, dotnet, zig
tamagi devbox lang node@lts python@3.13   # qualquer ferramenta do mise, na caixa dev
tamagi devbox enter                   # dentro da caixa: mise use, mise run, mise ls
```

- O `devbox-provision` baixa o binário oficial do mise (checksum conferido)
  para dentro da caixa e o ativa no shell dela.
- Cada caixa guarda as suas ferramentas em separado
  (`~/.local/share/mise-<caixa>`): um Node instalado na caixa Arch não é
  executado pelo Ubuntu. A lista do que você quer (`~/.config/mise/config.toml`)
  é uma só.
- **Sobre o erro do `~/.local` com dono root:** o mise era a vítima, não a
  causa. A pasta era criada como root na conversão da pasta pessoal em
  subvolume (snapshots); isso foi corrigido em `home-snapshots` e no primeiro
  boot, e o `tamagi doctor` acusa (com o comando de conserto) se acontecer.

## Máquinas virtuais (nativo, sob demanda)

O virt-manager não vem na imagem. `tamagi vm install` o instala **nativamente**
(RPM do Fedora, não Flatpak) numa camada reversível, com tudo a que tem
direito: QEMU/KVM, firmware UEFI com Secure Boot (edk2-ovmf), TPM virtual
(swtpm, para Windows 11), aceleração 3D no convidado (virgl), SPICE (área de
transferência, som e redirecionamento de USB), pastas compartilhadas
(virtiofs), a rede NAT padrão e os filtros de rede, rede sem root (passt),
nomes das VMs no DNS (libvirt-nss), virt-viewer, virt-install, virsh, virt-top
e as ferramentas de imagem (guestfs-tools: virt-customize, virt-sysprep,
virt-sparsify). É oferecido no primeiro login e em `tamagi install`: instala
quem quiser. Os daemons do libvirt sobem por
socket, só quando usados, e quem é administrador gerencia as VMs sem digitar a
senha.

```bash
tamagi vm             # abre o virt-manager (instala na primeira vez)
tamagi vm install | status | remove     # remover tira a camada; VMs e discos ficam
```

## Atualização automática

Tudo se atualiza sozinho, todo dia:

| O quê | Como |
|---|---|
| Sistema (imagem) | baixada e preparada em segundo plano; entra no próximo boot, com a anterior guardada |
| Flatpaks | timers do sistema e do usuário |
| Subsistemas e caixas distrobox | `tamagi-autoupdate.timer` (não roda na bateria) |

`tamagi-update status` mostra os quatro timers e avisa se há imagem nova
esperando o reinício; `tamagi update auto off` desliga e `on` religa.
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
um tutorial manda rodar: `tamagi-isolate arquivo` (ou, no COSMIC Files, botão
direito → Abrir com → "Abrir isolado"). O programa roda numa jaula
descartável do bubblewrap (o mesmo dos Flatpaks): **sem rede**, com a **pasta
pessoal escondida** (ele vê uma pasta vazia), o resto do sistema **só para
leitura**, sem D-Bus e sem ver os outros processos — mas com tela, som e placa
de vídeo. O arquivo é copiado para dentro (o original não muda) e a jaula é
apagada no fim (`--manter` guarda para inspecionar; `--rede` libera a rede).
Limite: o X11 (Xwayland) fica disponível para os programas que só falam X11,
e um programa X11 pode enxergar outras janelas X11.

## Três recursos que nem o Pop!_OS nem o Fedora atômico têm

### Voltar no tempo

`tamagi voltar` (ou `Super+Espaço` → Voltar no tempo) mostra uma linha do tempo
única: as versões do sistema nesta máquina e as publicadas com data no registro
(`tamagi:45.AAAAMMDD`), intercaladas com os snapshots da pasta pessoal. Escolha
um momento e o que trazer de volta: um arquivo ou pasta, os apps e
configurações daquele dia (o `tamagi.toml` guardado no snapshot, reaplicado
com `tamagi apply`) ou o sistema daquela data (`bootc rollback`/`switch`).
Voltar o sistema não apaga nada: a versão atual fica no menu de boot, e
`tamagi voltar seguir` retoma as atualizações.

### Sessão de convidado descartável

`tamagi-guest on` acrescenta uma conta "Convidado" sem senha na tela de
login, para emprestar o computador. A pasta pessoal dela fica na memória e é
refeita do zero a cada logout e a cada boot. Sem `sudo`; uma regra do polkit
nega ao convidado montar discos, mexer em VMs, instalar no sistema e usar
`pkexec`, mesmo sabendo uma senha. As suas pastas são fechadas para ele
(`0700`). `tamagi-guest off` tira tudo. Vem desligada.

### Modo privacidade de um toque

`Super+Ctrl+P` (ou `tamagi-privacy`) liga e desliga, de uma vez: a câmera
desligada no kernel (as interfaces de vídeo USB saem do driver: nenhum programa
a abre), todos os microfones mudos, Bluetooth bloqueado, o computador invisível
na rede local (sem mDNS), o firewall fechado para conexões de entrada (zona
`drop`, só em execução), o Wi-Fi com um endereço MAC sorteado, o tráfego pelo
nó de saída do Tailscale (se configurado em `~/.config/tamagi/privacidade.conf`),
as notificações em "não perturbe" e a tela apagando em 1 minuto. Uma
notificação fixa avisa que está ligado. Desligar devolve tudo como estava;
reiniciar também. A parte de sistema roda por um ajudante que só o
administrador na sessão ativa pode chamar sem senha.

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
tamagi-windows stop      # desliga e libera a memória
tamagi-windows web  # a tela pelo navegador (acompanhar a instalação)
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

## Caixa de desenvolvimento e Workspaces

O host não tem compiladores nem linguagens: eles vivem na caixa. A caixa
padrão (`dev`) é **Fedora 45**; há também Ubuntu, openSUSE Tumbleweed e Arch.
Toda caixa vem com `gcc`/`g++`/`cmake`/`ninja`, git, SQLite, Rust/Cargo,
Node.js/npm, Python e:

- **bancos de dados:** os clientes `psql`, `mariadb` e `valkey-cli`/`redis-cli`
  já instalados; os bancos do `tamagi db` respondem em `127.0.0.1` de dentro
  da caixa, sem configurar nada;
- **navegador:** links e logins (gh, npm, agentes de IA) abrem no navegador do
  desktop (`BROWSER` aponta para o host); para testes automatizados, `tamagi
  devbox browser` instala o Chromium na caixa e aponta `CHROME_BIN`,
  `PUPPETEER_EXECUTABLE_PATH` e `PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH` para ele;
- **mise** para outras linguagens e versões (`tamagi devbox lang`).

```bash
tamagi devbox                           # menu interativo (padrão: fedora)
tamagi devbox create                    # cria 'dev' (Fedora 45)
tamagi devbox create ubuntu             # também: opensuse (Tumbleweed), arch
tamagi devbox create fedora --extra go,zig,clang,browser
tamagi devbox enter | lang | browser | distros | list | update | remove <nome>
```

### Três inovações arquiteturais

1. **`tamagi-perf` — Motor adaptativo de desempenho (`tuned` + `sched_ext`):**
   coordena perfis de CPU/EPP (`eco`, `balanced`, `performance`, `latency`) com o
   escalonador de CPU em espaço de usuário via BPF (`scx_rusty` no kernel OGC),
   telemetria unificada (`tamagi-perf status` / `--json`) e micro-benchmark
   rápido (`tamagi-perf bench`).
2. **`tamagi-workspace` — Ambientes declarativos por projeto (`tamagi-workspace.toml`):**
   detecta automaticamente o stack do repositório (`Cargo.toml`, `package.json`,
   `pyproject.toml`, `go.mod`, SQLite, `docker-compose.yml`), gera o manifesto do
   projeto e orquestra `tamagi-devbox` + pacotes nativos + bancos locais
   (`tamagi-db`) + tarefas (`tamagi-workspace init | status | up | enter | run <tarefa>`).
3. **`tamagi-doctor` — Diagnóstico autônomo de saúde, deriva e integridade:**
   executa 12 verificações arquiteturais (imagem `bootc`, kernel/Secure Boot,
   Btrfs/NoCoW em VMs, `zram`/writeback, `systemd`, camadas `sysext`, contêineres,
   GPU, KVM, SELinux, menu/`.desktop` e IA local), com `--json` (para o
   agente local), `--fix` (auto-remediação segura no usuário) e `--explain`
   (diagnóstico resumido pela IA local no idioma do usuário).

## Do Omarchy: desenvolvimento, captura e atalhos

```bash
tamagi devbox lang web python      # linguagens pelo mise, dentro da caixa de desenvolvimento
tamagi-font                        # fonte mono de tudo: foot, COSMIC e apps GTK
tamagi-capture text               # Shift+Print: seleciona uma área e copia o texto (OCR, Tesseract)
tamagi-capture record              # Alt+Print: grava a tela (GPU Screen Recorder, portal); de novo, para
tamagi-db                          # escolhe: postgres, mysql, mariadb, redis, valkey, mongo
tamagi-db add postgres             # 127.0.0.1:5432, sem senha, liga com a sessão
tamagi-db url postgres | env       # URL de conexão / linhas export DATABASE_URL=…
tamagi-db shell postgres           # o cliente do próprio banco (psql, mysql, redis-cli, mongosh)
tamagi-db snapshot postgres        # guarda os dados como estão; restore traz de volta
tamagi-db list | stop | start | snapshots | remove <banco>
tamagi-remind 25m "Pausa"        # notificação daqui a 25 minutos
```

Os bancos são contêineres Podman sem root (Quadlet em
`~/.config/containers/systemd/`), escutando só em `127.0.0.1`, com os dados em
volumes `tamagi-db-<banco>`. Sem senha, como no Omarchy: só para
desenvolvimento local.

Atalhos no estilo do Omarchy (`Super+K` lista todos): `Super+Shift+G` lazygit,
`Super+Shift+D` bancos, `Super+Shift+O` OnlyOffice e `Super+Shift+M` Mission
Center (instalados no primeiro uso), `Super+Shift+W` Windows, `Super+Ctrl+E` emojis (Smile, instalado no
primeiro uso), `Super+Ctrl+R` lembrete. Com LazyVim (`ujust setup-lazyvim`),
o `tamagi-theme` também troca o colorscheme do Neovim (os mesmos plugins do
Omarchy; no Ipê, base16 com as cores do tema). Elixir, PHP e o que precisa
compilar ficam numa caixa (`tamagi devbox create arch`). Ficaram de fora
o conta-gotas de cor (o portal do COSMIC ainda não tem) e o que depende do
Hyprland (clipboard universal, gaps, scratchpad).

## Jogos (nativo como no Bazzite, sob demanda)

Na imagem fica só o que precisa estar no kernel: o kernel OGC (o do Bazzite),
os controles Xbox (`xone`/`xpadneo`) e o `scx_rusty`. Steam, `steam-devices`,
`gamescope`, MangoHud, GameMode, drivers 32-bit Mesa/Vulkan, Lutris, Wine,
Winetricks, GOverlay, vkBasalt e Protontricks entram **nativamente no `/usr`**,
com os mesmos pacotes que o Bazzite traz embutidos, numa camada reversível
(`tamagi-layer`) — ou por **Flatpak**:

```bash
tamagi gaming                    # menu: nativo, Flatpak ou baixa latência
tamagi gaming native steam       # Steam + steam-devices + gamescope, MangoHud, GameMode e 32-bit
tamagi gaming native lutris wine overlay protontricks   # somam à mesma camada
tamagi gaming flatpak            # Steam, Lutris, Heroic, ProtonUp-Qt, Bottles (Flatpak)
tamagi gaming latency            # perfil de baixa latência (tuned + scx_rusty)
tamagi gaming status | remove
```

### gamescope com HDR

O HDR nos jogos vem do **gamescope rodando direto no vídeo**, fora do COSMIC,
como no Bazzite e no Steam Deck: na tela de login, escolha a sessão **Tamagi
Gaming** (aparece depois de `tamagi gaming native`). Ela abre o Steam Big
Picture com HDR ligado (quando a tela anuncia HDR no EDID) e sincronia
adaptativa; sair do Steam volta para a tela de login. Não depende do HDR do
COSMIC, que continua esperando o upstream (veja Limites conhecidos).

```bash
tamagi gaming hdr                # estado: gamescope, telas com HDR, opções
tamagi gaming hdr on | off       # HDR da sessão de jogos (padrão: ligado)
tamagi gaming hdr nits 300       # brilho do conteúdo SDR dentro do HDR (padrão: 400)
tamagi gaming hdr itm on         # mostra jogos SDR em HDR (inverse tone mapping)
tamagi gaming hdr vrr on | off   # sincronia adaptativa
tamagi gaming run -- %command%   # opção de inicialização do Steam: o jogo no gamescope
```

Dentro do COSMIC, `tamagi gaming run` também pede HDR ao gamescope, mas como
janela ele só sai em HDR quando o compositor do desktop tiver suporte; até
lá, o conteúdo HDR é convertido para SDR. As opções ficam em
`~/.config/tamagi/gamescope.conf` (também `WIDTH`, `HEIGHT`, `REFRESH` e
`CLIENT`, para abrir outro programa no lugar do Steam).

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
tamagi-snap versions ~/Documentos/contrato.odt
tamagi-snap restore ~/Documentos/contrato.odt      # a versão mais recente diferente da atual
tamagi-snap restore ~/Documentos/contrato.odt 42   # a do snapshot 42
tamagi-snap now "antes de mexer"                # um snapshot agora
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
- **Kernel:** Open Gaming Collective (7.3), assinado pelo
  Universal Blue; `CONFIG_RUST=y`, Binder, sched_ext.
- **Terminal:** ripgrep, fd, bat, eza, zoxide, dust, delta,
  hyperfine, uv e starship. O `sudo` e os utilitários básicos
  (`ls`, `cp`…) são os de sempre: a troca por sudo-rs e uutils foi retirada.

As ferramentas `tamagi-*` são bash e Python.

## IA

```bash
tamagi ai setup          # recomenda um modelo pelo hardware, baixa e liga o servidor
agent "por que meu boot está lento?"   # agente do sistema, 100% local (tamagi agent)
tamagi ai chat | code    # interface web do llama.cpp | opencode no modelo local
tamagi agents            # catálogo de agentes de programação (nenhum vem na imagem)
tamagi agents install claude antigravity codex clio qwen opencode gemini openclaw
```

| Agente | Como é instalado |
|---|---|
| Claude Code (`claude`) | instalador da Anthropic, checksum conferido, em `~/.local/bin` |
| Codex CLI (`codex`), opencode | binário oficial do GitHub, digest SHA-256 conferido, em `~/.local/bin` |
| [CLIO](https://www.syntheticautonomicmind.org/) (`clio`) | Perl puro, GPL-3.0: versão do GitHub com digest conferido; roda na caixa de desenvolvimento (o host só tem um Perl mínimo) e fala com o modelo local do llama.cpp |
| Antigravity | IDE do Google, na caixa de navegadores, com o Chrome que o agente dela usa |
| Qwen Code (`qwen`), Gemini CLI (`gemini`), OpenClaw (`openclaw`) | pacotes npm: vivem na caixa de desenvolvimento (Node, git e compiladores estão lá); um lançador em `~/.local/bin` os abre de qualquer lugar |
| IA local | o agente do Tamagi com um modelo no seu computador (`tamagi ai setup`) |

**llm-scripts** (`tamagi install llm-scripts`): o
[llm-scripts](https://github.com/fewtarius/llm-scripts) (GPL-3.0, do mesmo
autor do CLIO) lê os metadados do modelo e a memória da placa de vídeo e
calcula o perfil de execução — contexto, cache KV, sessões. O Tamagi o aponta
para o llama.cpp que já vem na imagem e para os modelos do `tamagi ai`, sem
compilar nada: `llm doctor`, `llm models`, `llm profile --model <modelo>`,
`llm serve`. O motor `gufo` (APUs AMD) e a versão ROCm precisam de compilador:
ficam para a caixa de desenvolvimento.

- **llama.cpp v1.0.0** compilado na imagem com Vulkan e variantes de CPU
  escolhidas em tempo de execução; serve em `127.0.0.1:9931` (API OpenAI e
  Anthropic) com `--jinja`, `--flash-attn auto`, `--fit on` e suporte a
  `reasoning_content`, carrega o modelo sob demanda e o libera após 15 min parado.
- **`tamagi-agent`** (alias `agent`): agente próprio que conhece o sistema e usa o
  registro declarativo `/usr/share/tamagi/actions.toml` (`tamagi_action`).
  Comandos somente-leitura rodam direto, presos pelo Landlock (sem rede, sem
  gravar, sem ver chaves e tokens); o resto pede confirmação. Veja Segurança.

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
  busca. `Super+Espaço` abre esse menu pelo teclado (`cosmic-ext-classic-menu-applet
  --toggle`); a paleta de comandos do Tamagi fica no `Super+A` e o lançador do
  COSMIC no `Super+Alt+Espaço`.
- **Tela de boot:** o brasão do Tamagi, o anel (環, *tamaki*) com as três
  joias (玉, *tama*), nas cores do tema Ipê; as joias giram enquanto o sistema
  sobe, e a senha do disco aparece no mesmo fundo. Desenhado por código no
  build (`build_files/make-plymouth.py`), como os papéis de parede.
- **Idiomas:** português (Brasil), inglês (EUA), espanhol e francês, no
  sistema, no instalador, no assistente de primeiro login, no menu e nos
  atalhos. `tamagi-lang pick` troca depois de instalado.
- **Fontes:** Noto Sans na interface, Roboto Slab como fonte com serifa
  (documentos), JetBrains Mono no terminal.
- **Apps:** a imagem é enxuta. No primeiro boot entram sozinhos só os
  essenciais e leves (Flatseal, Warehouse, Loupe, Papers e três applets do
  COSMIC). No primeiro login o assistente pergunta o navegador (e como
  instalar) e mostra o catálogo do que é opcional, com nada pré-marcado; o
  mesmo catálogo fica em `tamagi install` (`/usr/share/tamagi/catalog.tsv`
  mais os Flatpaks opcionais de `flatpaks.list`).
- **Temas:** escolhidos nas Configurações do COSMIC (Aparência); os terminais
  seguem sozinhos. Os temas do Tamagi — tokyo-night (o padrão, também do
  COSMIC), catppuccin, gruvbox, nord, everforest, rose-pine, kanagawa,
  matte-black, ristretto, latte e ipe (verde-noite com destaque no amarelo do
  ipê) — estão em `/usr/share/cosmic-themes/tamagi-<tema>.ron`, com miniatura.
  `tamagi theme set <tema>` aplica um deles de uma vez ao COSMIC e ao terminal
  (foot), btop, Neovim (LazyVim) e janelas do Tamagi.
- **Papéis de parede:** escolhidos nas Configurações do COSMIC (Papel de
  parede), onde aparecem os oito estilos de cada tema, desenhados por código
  com motivos do Japão do século XVII: `mon` (padrão; brasão no estilo dos
  kamon — um anel, *tamaki* 環, com três joias, *tama* 玉 — sobre ondas),
  `seigaiha` (ondas em escamas), `asanoha` (folha de cânhamo), `byobu`
  (biombo: nuvens, lua e monte), `shippo` (círculos entrelaçados), `hinode`
  (nascer do sol sobre o mar), `uroko` (escamas triangulares) e `ondas`.
  Usando um papel de parede do Tamagi, trocar de tema mantém o estilo e traz
  as cores novas; qualquer outro papel de parede não é tocado.
- **Redes:** WireGuard, OpenVPN, OpenConnect, L2TP, PPTP, SSTP, IKEv2/IPsec,
  SSH; Tailscale opcional, do repositório oficial (`tamagi install tailscale`); SMB, NFS, SSHFS, MTP, iPhone,
  rclone; mDNS, impressoras e scanners sem driver.

## Atalhos (`Super+K` mostra todos)

| Atalho | Ação |
|---|---|
| `Super+Enter` / `Super+Espaço` | terminal (foot) / menu iniciar (o botão do Tamagi) |
| `Super+A` / `Super+Alt+Espaço` | paleta de comandos (`tamagi-menu`) / lançador de aplicativos COSMIC |
| `Super+Shift+A` | agente de IA |
| `Super+W` / `F` / `M` | fechar / tela cheia / maximizar |
| `Super+1…9`, `+Shift` | ir para o workspace / mandar a janela |
| `Super+Setas`, `+Shift` | foco / mover a janela |
| `Super+T` / `Super+Y` | janela flutuante / ligar-desligar tiling |
| `Super+B` / `E` | navegador / arquivos |

## Kernel, módulos e assinatura

- **Kernel Open Gaming Collective, o padrão:**
  [Open Gaming Collective](https://github.com/OpenGamingCollective/linux)
  (sabor `ogc`): patches de agendamento, controles, portáteis e jogos, sobre a
  versão estável do Linux. Uma imagem só é publicada (`latest`).
- **Módulos do Universal Blue** ([ublue-os/akmods](https://github.com/ublue-os/akmods)),
  prontos e assinados para esse kernel: `xone` e `xpadneo` (controles Xbox por
  cabo, adaptador e Bluetooth) e `v4l2loopback` (câmera virtual).
- **Secure Boot:** o kernel Open Gaming Collective é assinado pelo Universal Blue, não
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
  usr/libexec/tamagi/    browser-provision, devbox-provision (rodam dentro
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

O Tamagi é escrito com a ajuda de agentes de IA, sob a direção do autor: o
**Gemini** (Google, pelo Antigravity) e o **Claude** (Anthropic) são
colaboradores do projeto. Veja [CONTRIBUTORS.md](CONTRIBUTORS.md).

- **Base: [Universal Blue](https://universal-blue.org/).** A imagem de partida
  é a `base-main`: sistema atômico com codecs, firmware, distrobox e `ujust` já
  incluídos, a mesma fundação do Bluefin, do Aurora e do Bazzite. Os pacotes
  vêm do Fedora 45, sem modificação.
- **[Bazzite](https://bazzite.gg/):** ajustes de desempenho (memória, disco,
  rede), gamescope, MangoHud, GameMode e regras de controles.
- **[Omarchy](https://omarchy.org/):** uso por teclado, temas e web apps.
- A caixa dos navegadores é Fedora: o Chrome vem do repositório rpm do Google
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

O Tamagi não redistribui o Windows nem os navegadores e agentes de IA de
terceiros: `tamagi-windows`, `tamagi-browser` e `tamagi-agents` baixam cada
um diretamente do fabricante, na máquina do usuário. O Windows exige licença
válida da Microsoft. Detalhes em `NOTICE`
(também em `/usr/share/doc/tamagi/NOTICE`).

## Limites conhecidos

- Só x86_64 e só UEFI.
- O tik lista discos internos; discos USB ficam de fora como destino (padrão
  do tik). A instalação apaga o disco inteiro: não há dual boot assistido.
- Idiomas: o sistema, o instalador, o assistente de primeiro login, o menu
  (`tamagi-menu`), as ações rápidas, os atalhos e as entradas do lançador
  existem em pt-BR, en-US, es e fr, e as ferramentas de linha de comando
  também (inglês é o idioma de origem). Ficam só em português as mensagens de
  serviços internos que rodam como root no boot (`home-snapshots`,
  `firstboot`, `secureboot-guard`) e os erros do atalho `claude`.
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
  o Tamagi espera entrar no oficial, sem fork. Nos jogos o HDR já funciona
  pela sessão **Tamagi Gaming** (gamescope direto no vídeo, veja
  [gamescope com HDR](#gamescope-com-hdr)); dentro do COSMIC, o `gamescope`
  em janela sai em SDR e o VRR tem falhas conhecidas no compositor.
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
  (`tamagi-update quarantine` / `unquarantine`). `tamagi rewind` (ou `sudo bootc rollback`)
  continua valendo para voltar à mão.
- NVIDIA com driver proprietário: a base atual não traz o driver. É preciso
  trocar `BASE_IMAGE` por uma base que o traga; não foi testado.

> **Licenças:** o Chrome é proprietário e não faz parte da imagem — é baixado
> na sua máquina, do site do fabricante.
