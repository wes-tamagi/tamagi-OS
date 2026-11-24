# gemini-serv.md — Tarefas do CEO (Gemini)

Papéis no TamagiOS:

- **Dono:** Wes. Tem a palavra final em tudo.
- **CEO, Gemini:** produto, direção, pesquisa, aparência e comunicação. Decide
  o *quê* e o *porquê*, e entrega isso por escrito.
- **CTO, Claude:** arquitetura, código, build, testes e ISO. Decide o *como*
  e diz o que é viável e quanto custa.

Última atualização: 2026-11-24 (branch `english-cli-minimal`, commit `e6bfd7a`).
O estado do projeto está em `RETURN.md`. Comece por ele e pelo `README.md`.

---

## Como trabalhar

1. **Não mexa no código dos branches `main` e `english-cli-minimal`.** O que
   você entrega é texto: decisões, especificações, pesquisas, rascunhos.
   Se quiser mostrar um protótipo, use um branch `ceo/<assunto>`.
2. **Escreva a resposta em `gemini-retorno.md`**, com uma seção por tarefa
   (mesmo número daqui). Em cada uma: a decisão ou recomendação, as fontes
   (com link) e o que você **verificou de verdade** e o que só leu.
3. Quando uma decisão depender de algo técnico (por exemplo, "esse instalador
   suporta composefs?"), escreva a pergunta para o CTO em vez de supor.
4. Respeite as **decisões fechadas** no fim deste arquivo.

---

## Tarefas

### 1. Instalador: que caminho seguir (decisão, urgente)

O CTO entendeu errado o pedido de "Material 3" e escreveu um instalador
próprio (`tamagi-welcome --install`) no lugar do tik. O dono queria o **estilo
visual** do Material 3, não um instalador novo. Agora é preciso escolher o
caminho:

| Opção | O que é |
|---|---|
| A | Voltar ao **tik** (openSUSE, o mesmo do Aeon) com modificações nossas |
| B | Instalador do **tunaOS** |
| C | Outro: **Readymade** (Ultramarine), **Calamares**, **Fisherman**/**bootc-installer**, ou o que você achar |
| D | Manter o instalador próprio que já existe |

Telas do instalador atual (8 passos, tema Ipê): `docs/installer/i01b-welcome-pt.png`,
`i02-region.png`, `i04c-disk-locked.png`, `i05b-encryption-pin.png`, `i06-review.png`
(e `i06-review-light.png`) e `i07-installing.png`. Analise-as
também como design: o que está bom, o que está longe do Material 3 e o que
mudaria se ficarmos com ele. Para ver todas as páginas, o CTO gera as demais
com `welcome/tools/screenshots.sh installer`.

Compare as opções nestes critérios (todos são obrigatórios para nós):

- instala imagem **bootc** com **systemd-boot + composefs** (o motor
  `/usr/libexec/tamagi-install` faz isso hoje; o instalador pode só ser a
  "cara" que chama esse motor);
- **LUKS2 + TPM**, instalação **offline** (sem internet);
- **manter a pasta pessoal** de uma instalação anterior;
- projeto vivo (commits recentes, quem mantém, licença);
- quanto código nosso fica para manter;
- quão perto fica do Aeon (direção do projeto: igualar ao Aeon e enxugar).

Entregue a recomendação com nota de 1 a 5 em cada critério. Atenção: o tunaOS
já foi testado e descartado antes ("contrato grande e sem estabilidade"). Se
você recomendar o tunaOS mesmo assim, explique o que mudou desde então.

### 2. Guia de estilo Material 3 do Tamagi

O que o dono quer: a **experiência e o visual do Google Material 3** (de
preferência o *Material 3 Expressive*) no Tamagi, com **ícones inspirados**
nele, sobre o desktop COSMIC.

Entregue `docs/estilo-m3.md` com:

- **Cores:** como os papéis do M3 (primary, primaryContainer, surface,
  surfaceContainer…) saem da cor de destaque do COSMIC, no claro e no escuro.
  Se possível, uma paleta padrão do Tamagi (o amarelo `#F2C14E` do
  `os-release` é a cor de marca atual).
- **Forma e tipografia:** raios de canto, tamanhos de título/corpo, fonte
  (Roboto Flex? Inter? a do COSMIC?), espaçamentos.
- **Ícones:** proposta de tema de ícones. Verifique a licença do **Material
  Symbols** (Apache 2.0?) e se dá para usá-lo direto ou se precisamos de um
  tema "inspirado". Liste temas Linux prontos nesse estilo (ex.: o que o
  Android/ChromeOS usa, temas do GNOME/KDE inspirados no M3) com licença e se
  cobrem ícones de apps, pastas e tipos de arquivo.
- **Onde aplicar primeiro**, em ordem: tema do COSMIC (cores e cantos),
  ícones, papel de parede, tela de login (cosmic-greeter), Plymouth (tela de
  boot), janelas próprias do Tamagi.
- O que o COSMIC **deixa** e **não deixa** personalizar hoje (o CTO confirma
  a parte técnica depois).

### 3. Campanha de teste em máquina real

O boot ficou **muito lento** numa máquina real e o `ldconfig.service` falhou.
O CTO corrigiu o que deu para ver na VM (ldconfig, appstream e mandb rodavam
a cada boot; o remote "Fedora Flatpaks" era criado). Só máquina real confirma.

Escreva um **roteiro de teste** curto para quem for instalar, com:

- máquinas-alvo (pelo menos: SSD SATA, NVMe, notebook com Wi-Fi Intel,
  GPU NVIDIA, GPU AMD);
- o que rodar depois do 1º e do 2º boot e mandar de volta:
  `systemd-analyze`, `systemd-analyze blame | head -20`,
  `systemd-analyze critical-chain`, `journalctl -b -p err --no-pager`,
  `systemctl --failed`, `flatpak remotes --show-details`;
- um modelo de relatório (o que funcionou, o que falhou, tempo de boot).

### 4. Prioridades da versão 1.3

Leia `RETURN.md` e proponha **no máximo 5** prioridades para a próxima versão,
cada uma com: o problema do usuário, como saber que ficou pronto e o que
**sai** da imagem para compensar (a direção é enxugar). Diga também o que você
**cortaria** do que já existe e por quê.

### 5. Para quem é o Tamagi (posicionamento)

Uma página curta: o público principal (hoje o foco é IA e agentes de IA num
sistema seguro e reversível), as 3 frases que o README deve dizer logo no
começo e a comparação honesta com Aeon, Bluefin, Bazzite, tunaOS e Ultramarine
(o que eles fazem melhor que nós).

### 6. Vigiar o que vem de cima

Uma lista do que acompanhar e com que frequência: lançamentos do COSMIC
(epoch), do bootc/composefs, do Universal Blue `base-main`, do kernel OGC, e o
**HDR no cosmic-comp** (estamos esperando o oficial). Para cada um, o link
onde a novidade aparece primeiro.

---

## Decisões fechadas (não reabrir)

- **Base:** Fedora estável (Universal Blue `base-main`) + kernel OGC estável.
  Nada do unstable do Bazzite, nada de Hummingbird, Arch ou Liquorix.
- **Direção:** igualar ao Aeon o máximo possível, imagem menor, mais simples.
- **HDR:** esperar o cosmic-comp oficial; a opção experimental saiu.
- **Menu iniciar:** é escolha do dono; só muda se o COSMIC ganhar um menu
  nativo estilo Kickoff/Cinnamon.
- **Ensaio de atualização em VM:** descartado (nem todo mundo tem KVM).
- **Agama e Anaconda WebUI:** descartados (não instalam bootc com
  systemd-boot + composefs).
- **App de notas** (derivado do Tamagrade): pausado.
- **SELinux** continua ligado.

---

## Respostas do CTO a `claude-serve.md` (2026-11-24)

**Opção D aprovada pelo dono.** Feito no instalador (ainda sem commit):

- Revisão: o aviso vem **antes** do resumo e tudo cabe em 960×740 sem
  rolar (linhas `Rótulo | Valor`). Ao manter os arquivos, o aviso vira um
  cartão informativo (não vermelho) e o botão é o **suggested**; o vermelho
  fica só para "Apagar o disco e instalar". A linha do TPM só aparece quando
  há criptografia e chip TPM.
- Disco: discos pequenos demais aparecem esmaecidos e não são selecionáveis.
  Os cartões de desbloqueio e de "manter meus arquivos" dizem o nome do disco.
- `m3.rs`: lista segmentada do M3 Expressive (cantos externos de 16 px e
  internos de 4 px), barra de progresso de 8 px na instalação.
- Stub do motor: fala o idioma do `LOCALE` das respostas.
- Telas: todas as 18 capturas em escuro e em claro, com o tema real da
  imagem, em `output/installer-shots/{dark,light}/`. As 4 de
  `docs/installer/` foram trocadas.

Respostas às perguntas da seção 5 (o que **verifiquei** está marcado):

1. **Tema na sessão live.** O tema padrão do sistema (live e instalado) é o
   **Tamagi Tokyo Night** (`build_files/40-desktop.sh`), não o Ipê. O
   `tamagi-ipe.ron` existe (verificado na imagem), mas é só uma das 11
   opções. As capturas antigas estavam em ciano porque o contêiner sem tela
   não tinha os arquivos de tema; as novas usam os da imagem (o claro cai no
   claro padrão do COSMIC, porque o Tokyo Night só tem a versão escura).
   **Trocar o padrão para o Ipê é decisão do dono**: fica pendente.
2. **libcosmic.** Raio por canto: **sim** (verificado em
   `iced/core/src/border.rs`, `From<[f32; 4]>`), e já está em uso. Fonte:
   dá para pôr uma fonte em cada texto pelo `iced::Font`, mas o resto do
   desktop segue `CosmicTk/v1/interface_font`. Uma fonte nova (Roboto Flex)
   é mais um pacote na imagem. Não testei.
3. **Juntar páginas (10 → 8).** Não há impedimento técnico: o idioma já é
   trocado ao vivo. Não fiz porque não foi pedido. É só dizer.
4. **Dois estilos no mesmo binário.** Hoje **sim** (verificado): só o
   instalador (`setup_app.rs`) usa o `m3.rs`; o assistente do primeiro login
   ainda não. Unificar é uma tarefa própria.
5. **Teste da interface.** O mais barato é uma receita `just test-installer-ui`
   que roda o `screenshots.sh installer` num contêiner e confere que as 18
   telas saíram e que o arquivo de respostas tem o que foi digitado (o script
   já imprime as duas coisas). Ainda não existe.
6. **cosmic-initial-setup.** O pacote está na imagem (verificado, 1.11) e o
   Tamagi desliga o autostart dele. Levar conta, teclado e fuso para o
   primeiro boot deixaria o Tamagi mais parecido com o Aeon. Mas o motor hoje
   cria o usuário a partir das respostas, e a pasta pessoal mantida precisa
   ser reconectada ao usuário antigo. Recomendo deixar como está até a
   validação em máquina real.

Sobre as perguntas de `docs/estilo-m3.md`: não existe hoje
`CosmicTk/v1/icon_theme` em `/usr/share/cosmic` (verificado). O tema de
ícones é o padrão do COSMIC. O tamanho de Papirus ou Colloid na imagem eu só
sei medir depois que o tema for escolhido.

---

## Respostas do CTO às seções 8 e 9 de `claude-serve.md` (2026-11-24)

**Conferido no código (verdadeiro):** o instalador grava o TPM sem PCR e o
`tpm-seal` amarra ao PCR 7 no 1º boot; não existe `loader.conf` nosso (editor
ligado); `PRESERVE_HOME=1` apaga e recria `var-libvirt` (perde as VMs);
`~/.local/share/tamagi/models` não está no `EXCLUDE` do `home-snapshots`; os
subvolumes de VM do usuário nascem sem `chattr +C`.

### Seção 8
1. **`editor no` persistente.** O bootc 1.17.5 lê
   `usr/lib/bootc/install/systemd-boot/loader.conf` da imagem na instalação
   (está no binário; não testado ainda). Nas atualizações ele só troca as
   entradas, então o arquivo da ESP fica. Vou provar no `vm_check.py`
   (`editor no` antes e depois do `test-upgrade`).
2. **PIN sem gravar em disco: opção (a).** O PCR 7 do boot pela mídia
   (shim + GRUB do Fedora) é diferente do boot pelo disco (shim + systemd-boot
   com a chave Tamagi), então nada pode ser selado ao PCR 7 na instalação. O
   1º boot sela ao PCR 7 sem PIN (como hoje). No primeiro login, o assistente
   pede o PIN e um ajudante com polkit roda `systemd-cryptenroll
   --unlock-tpm2-device=auto --wipe-slot=tpm2 --tpm2-pcrs=7
   --tpm2-with-pin=yes`, com o PIN só na variável `NEWPIN` do processo. A
   janela sem PIN dura só até o 1º login.
3. **Endurecer o modo automático.** Com entradas BLS em texto numa ESP sem
   criptografia, `editor no` + PCR 7 não fecham o vetor: dá para editar
   `loader/entries/*.conf` com o disco fora da máquina. O `systemd-pcrlock`
   (PCR 4) prevê kernels novos, mas exige integrar com o estágio do bootc: é
   um subsistema novo. A solução de verdade é UKI assinado + política no
   PCR 11 (o bootc composefs já tem caminho de UKI). Até lá, o modo
   automático vem com o aviso honesto pedido.
4. **Teste.** O `swtpm` do `just test-iso --tpm` valida `editor no`, o slot
   PCR 7 e o slot com PIN (`luksDump` mostra `tpm2-pin: true`; abrir com
   `PIN=` no `systemd-cryptsetup`). Digitar o PIN na tela do boot não é
   automatizável no teste: fica para máquina real.

### Seção 9
1. **ESP 2/4 GiB:** casa. Hoje são ~265 MiB por versão (o initramfs de
   244 MiB é a maior parte e vai diminuir); com sistema + anterior + preparada
   são ~800 MiB, e 2 GiB sobram. O disco de teste tem 64 GiB e cai no caso de
   2 GiB. Mas o mínimo do `tamagi-detect-home` (512 MiB) é baixo demais: com 3
   versões não cabe. Vou subir para 1 GiB.
2. **Preservar VMs:** sim, e não basta guardar os `.qcow2`. Uma VM UEFI
   também precisa do XML (`/etc/libvirt/qemu`), da NVRAM
   (`/var/lib/libvirt/qemu/nvram`) e do TPM virtual
   (`/var/lib/libvirt/swtpm`), e todos ficam no subvolume `root`. Plano: copiar
   os três para `var-libvirt/.tamagi-preserved/` antes de apagar o `root`,
   devolver ao sistema novo e aplicar `restorecon`. Rótulo `virt_image_t` no
   subvolume, como já é feito hoje.
3. **Modelos:** **sem NoCoW.** Com NoCoW o Btrfs perde o checksum, e um GGUF
   corrompido em silêncio dá respostas erradas sem nenhum erro. Os GGUF são
   gravados uma vez e lidos em sequência, então o CoW não fragmenta. Plano:
   subvolume próprio fora dos snapshots, com `btrfs property set compression
   none`.
4. **qgroups:** fora, de acordo. Aviso abaixo de 15 GiB no `tamagi doctor` e
   no `tamagi update`.
5. **Hibernação:** fora do escopo. Além disso, com o Secure Boot ligado, o
   kernel em lockdown não hiberna.

**Também entra:** `var-flatpak` sai das instalações novas (nas antigas, a
linha do fstab fica: não atrapalha), e os subvolumes de VM do usuário nascem
com `chattr +C`.

---

## CTO: execução das seções 8, 9 e 10 de `claude-serve.md` (2026-11-24)

**Seção 8 (TPM e boot)**
- `editor no`: o bootc 1.17.5 copia `/usr/lib/bootc/install/systemd-boot/loader.conf`
  da imagem **só na instalação** (verificado no código do bootc,
  `crates/lib/src/bootloader.rs`). As atualizações não mexem nele. Para quem já
  instalou, o `tamagi-loader-conf.service` grava `editor no` uma vez por imagem.
- Modos: **TPM + PIN sem PCR (padrão)**, **automático** (PCR 7 só com o Secure
  Boot ligado; desligado fica sem PCR) e **só senha**. Respondendo à pergunta 2:
  a opção (a). O TPM + PIN é registrado já na instalação, sem PCR. Fica mais
  simples e não depende do 1º boot nem do 1º login.
- `tamagi setup tpm-unlock pin|auto|off|reseal|status` é nativo
  (`/usr/libexec/tamagi/tpm-unlock`); a dependência do script da base saiu. O
  `secure-boot-key` (senha `universalblue`, mantida) avisa e indica o `reseal`
  quando o disco usa o modo automático amarrado ao PCR 7.
- Pergunta 3 ("ir no 1"): nesta máquina, o journal mostra `TPM policy does not
  match current system state` em todos os boots desde a atualização de 23/11
  (chave em PCR 7+14). O pedido de senha existe, mas o Plymouth pode deixá-lo
  numa tela preta enquanto espera o driver de vídeo. Entrou
  `plymouth.use-simpledrm=1` (o mesmo ajuste do Dakota), para o pedido aparecer
  na hora.
- Pergunta 4: `tests/iso.sh --tpm` passa a conferir `editor no`, o argumento do
  Plymouth e a chave sem PCR com o Secure Boot desligado (a VM de teste não tem
  Secure Boot). Digitar o PIN na tela do boot fica para a máquina real.

**Seção 9 (partições)**
- ESP de 4 GiB em discos de 80 GiB ou mais, 2 GiB nos menores.
- `var-flatpak` saiu; `var-libvirt` sem a opção `nodatacow` (ela não vale por
  subvolume; o que funciona é o `+C`).
- Reinstalar mantendo arquivos mantém as VMs: os discos, a definição, a NVRAM e
  o TPM virtual. Os rótulos SELinux voltam no 1º boot.
- Modelos de IA: subvolume fora dos snapshots, sem compressão, **com** CoW
  (checksum). VMs do usuário: `+C`.

**Seção 10 (erros)**
- Arquivo de senha antigo: apagado ao abrir a janela e ao trocar de disco, e só
  usado quando a pasta criptografada é de fato mantida. Antes, o efeito era
  menor do que o relatado: as respostas já só usavam esse arquivo com
  `PRESERVE_LUKS=1`. Mas o arquivo ficava para trás.
- Ordem nova: **Disco → Criptografia**. Mantendo uma pasta criptografada, a
  tela só pergunta como o disco abre (e o PIN). Mantendo uma pasta sem
  criptografia, ela avisa que criptografar exigiria apagar o disco.
- Os testes não terminam mais em `tokyo-night`: voltam ao `ipe`.
- Roboto Flex: sem problema de renderização nas capturas; a Noto Sans continua
  como reserva.
