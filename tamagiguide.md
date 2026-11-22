# Guia do Tamagi: instalar, migrar e atualizar

Este guia cobre três coisas: como chegar ao Tamagi (instalação nova ou
migração do Bluefin/Aurora), o que ele oferece e como mantê-lo atualizado.

## 1. Instalação nova ou migração?

| Situação | Recomendação |
|---|---|
| Já uso o Tamagi | **Atualize** (`tamagi update`). Nunca reinstale para atualizar. |
| Uso Bluefin ou Aurora e quero o Tamagi como sistema principal | **Instalação nova** pela ISO, com cópia da pasta pessoal. |
| Uso Bluefin ou Aurora e quero experimentar sem formatar | **Migração** com `bootc switch` (seção 3), sabendo dos limites. |

Por que instalação nova é a recomendação para quem vem de outro sistema:

- **Outro ambiente gráfico.** O Bluefin usa GNOME e o Aurora usa KDE; o Tamagi
  usa COSMIC. A migração troca a imagem do sistema, mas a pasta `/etc` e a sua
  pasta pessoal ficam como estavam, com configurações do ambiente antigo.
- **O disco fica como o instalador antigo deixou.** Os snapshots da pasta
  pessoal (`tamagi rewind`), a compressão e as opções de montagem do Tamagi
  são preparados pelo instalador do Tamagi. Numa máquina migrada eles podem
  não existir.
- **A migração do Bluefin/Aurora para o Tamagi ainda não foi testada pelo
  projeto.** A instalação pela ISO e a atualização de Tamagi para Tamagi são
  testadas em máquina virtual a cada rodada.

## 2. Instalação nova (recomendada)

1. **Copie os seus arquivos** para um disco externo ou outro computador. A
   instalação apaga o disco escolhido por inteiro.
2. Anote o que quer reinstalar: `flatpak list --app --columns=application`
   lista os seus aplicativos Flatpak.
3. Grave a ISO do Tamagi num pen-drive e inicie por ele.
4. Com **Secure Boot** ligado, a primeira inicialização pede o registro da
   chave do Universal Blue (a mesma do Bluefin e do Aurora): senha
   `universalblue`. Quem já usa Bluefin/Aurora nessa máquina normalmente já a
   tem registrada.
5. Siga o instalador: idioma (inglês é o padrão; português do Brasil vem logo
   depois), teclado, fuso horário, conta, criptografia do disco e o disco.
6. No primeiro login, o assistente pergunta o perfil (IA, desenvolvimento,
   jogos, escritório), o navegador e os opcionais. Nada disso é obrigatório;
   tudo pode ser instalado depois com `tamagi install`.
7. Copie os seus arquivos de volta e reinstale os Flatpaks.

## 3. Migração do Bluefin ou do Aurora, sem formatar

O Bluefin, o Aurora e o Tamagi são da mesma família (imagens bootc do
Universal Blue), então dá para apontar o sistema instalado para a imagem do
Tamagi. **Este caminho não foi testado pelo projeto; faça a cópia dos arquivos
antes.**

1. **Cópia dos arquivos** da pasta pessoal.
2. **Remova pacotes adicionados por cima da imagem**, se houver
   (`rpm-ostree status` mostra em `LayeredPackages`):

   ```bash
   rpm-ostree reset
   ```

3. **Troque de imagem:**

   ```bash
   sudo bootc switch ghcr.io/wes-tamagi/tamagi:latest
   ```

4. **Antes de reiniciar, ative a tela de login do COSMIC.** A escolha da tela
   de login fica em `/etc`, que a migração não troca; sem este passo o sistema
   pode subir sem tela de login.

   ```bash
   sudo systemctl disable gdm.service sddm.service 2>/dev/null
   sudo systemctl enable --force cosmic-greeter.service
   ```

   Se o `enable` reclamar que o serviço não existe (ele só existe na imagem
   nova), reinicie e rode o comando num terminal de texto (`Ctrl+Alt+F3`),
   seguido de `sudo systemctl start cosmic-greeter.service`.

5. **Reinicie.** No login, escolha a sessão COSMIC.
6. **Se algo der errado**, volte ao sistema anterior: escolha a entrada
   anterior no menu de boot, ou rode `sudo bootc rollback` e reinicie. A
   imagem antiga fica guardada até a próxima atualização.

O que muda e o que fica depois da migração:

| Item | O que acontece |
|---|---|
| Pasta pessoal e arquivos | Ficam. |
| Aplicativos Flatpak | Ficam. |
| Caixas distrobox e contêineres | Ficam. |
| Homebrew (`brew`) | O Tamagi não traz o Homebrew. Os arquivos em `/home/linuxbrew` ficam no disco, mas o comando sai do caminho. Use as caixas de desenvolvimento (`tamagi devbox`). |
| GNOME ou KDE | Saem com a imagem antiga. As configurações deles ficam na pasta pessoal, sem uso. |
| Extensões do GNOME, widgets do KDE | Não funcionam no COSMIC. |
| Comandos `ujust` do Bluefin/Aurora | Não existem no Tamagi: tudo é `tamagi` (`ujust update` vira `tamagi update`; chave do Secure Boot, TPM, firmware e limpeza ficam em `tamagi setup`). |
| Snapshots da pasta pessoal (`tamagi rewind`) | Dependem de como o disco foi instalado; podem não estar disponíveis. `tamagi doctor` mostra. |
| Assistente de primeiro login | Rode `tamagi firstrun` se ele não abrir sozinho. |

Para desistir de vez depois de alguns dias, aponte de volta:
`sudo bootc switch ghcr.io/ublue-os/bluefin:stable` (ou
`ghcr.io/ublue-os/aurora:stable`) e refaça o passo da tela de login com
`gdm.service` ou `sddm.service`.

## 4. Principais recursos

- **Sistema atômico:** o sistema é uma imagem só de leitura. Cada atualização
  troca a imagem inteira e a anterior fica guardada para voltar.
- **Um comando:** `tamagi` reúne tudo (`tamagi` sozinho lista os comandos).
- **Quase nada vem instalado:** navegador, jogos, máquinas virtuais, Docker,
  VS Code, Tailscale e agentes de IA são opcionais (`tamagi install`).
- **Perfis:** `tamagi profiles` aplica um conjunto pronto (IA,
  desenvolvimento, jogos, escritório).
- **O sistema num arquivo:** `tamagi init` grava o estado atual em
  `tamagi.toml`; `tamagi apply` reproduz esse estado noutra máquina.
- **IA local e agentes:** `tamagi ai` roda modelos na própria máquina;
  `tamagi agents` instala agentes (Claude Code, Codex, Antigravity, CLIO e
  outros). O agente roda preso à pasta do projeto, com um ponto de
  salvamento do git antes de começar.
- **Caixas de desenvolvimento:** `tamagi devbox` cria caixas Fedora, Ubuntu,
  openSUSE ou Arch, com acesso à GPU; as linguagens ficam dentro da caixa.
  `tamagi db` sobe bancos de dados; `tamagi try` abre uma caixa descartável.
- **Contêineres:** Podman é o padrão. Docker e lazydocker são opcionais.
- **Máquinas virtuais:** `tamagi vm` instala o virt-manager nativo;
  `tamagi vm access full` libera pen-drives e discos externos para as VMs.
  `tamagi windows` sobe um Windows 11 em contêiner.
- **Jogos:** `tamagi gaming` instala Steam e ferramentas de forma nativa, com
  uma sessão de jogos em gamescope e HDR nos monitores que o anunciam.
- **Terminal:** Ptyxis (`Super+Enter`), que abre abas direto dentro das caixas.
- **Desfazer:** `tamagi rewind` volta a imagem do sistema ou a pasta pessoal a
  um ponto anterior.
- **Idiomas:** inglês (padrão), português do Brasil, espanhol e francês
  (`tamagi lang`).
- **Atalhos:** `Super+Espaço` abre o menu iniciar, `Super+A` a paleta de
  comandos, `Super+K` a lista de atalhos.

## 5. Como atualizar corretamente

**O comando do dia a dia:**

```bash
tamagi update
```

Ele atualiza, nesta ordem: a imagem do sistema, as camadas nativas opcionais,
os Flatpaks, as caixas distrobox e os agentes de IA.

O que saber:

- **A imagem nova só entra no próximo boot.** O comando baixa e prepara;
  o sistema em uso não muda até reiniciar. Não há pressa para reiniciar.
- **Ver o que muda antes de reiniciar:** `tamagi update changes`.
- **Só o sistema:** `tamagi update --system` (equivale a `sudo bootc upgrade`).
- **Não use `dnf` nem `rpm-ostree install` no sistema.** Programas vão por
  Flatpak, pelas caixas ou pelo catálogo (`tamagi install`). Pacotes por cima
  da imagem tornam as atualizações lentas e sujeitas a falhar.
- **Não desligue o computador durante o `tamagi update`.** Se acontecer, nada
  quebra: a imagem em uso não é tocada; basta rodar de novo.

**Se uma atualização der problema:**

1. No menu de boot, escolha a entrada anterior; ou rode `tamagi rewind`
   (ou `sudo bootc rollback`) e reinicie.
2. Se o sistema novo falhar na verificação de saúde ao iniciar, ele volta
   sozinho para a imagem anterior e marca a imagem com defeito em quarentena,
   para não tentar de novo.
3. `tamagi update quarantine` mostra as imagens bloqueadas;
   `tamagi update unquarantine` libera, quando o defeito tiver sido corrigido.

## 6. Atualizações automáticas

```bash
tamagi update status      # o que está ligado e quando roda de novo
tamagi update auto on     # liga tudo
tamagi update auto off    # desliga tudo
```

Elas já vêm **ligadas** numa instalação nova. São quatro frentes:

| O quê | Como funciona |
|---|---|
| Imagem do sistema | Baixada e preparada em segundo plano; **aplicada só no próximo boot**. O computador nunca reinicia sozinho. |
| Flatpaks do sistema | Atualizados em segundo plano. |
| Flatpaks do usuário | Atualizados em segundo plano. |
| Caixas distrobox | Atualizadas em segundo plano. |

Recomendações:

- **Deixe ligado** em máquina de uso comum: como cada atualização do sistema
  é uma imagem inteira e a anterior fica guardada, o risco é baixo.
- **Reinicie de vez em quando** (uma vez por semana basta) para a imagem nova
  entrar.
- **Desligue** antes de uma apresentação, de uma viagem sem internet boa ou em
  máquina que precisa ficar congelada numa versão; ligue de novo depois.
- Antes de reiniciar numa hora importante, `tamagi update changes` diz o que
  vai mudar.

## 7. Conferir a imagem

As imagens publicadas são assinadas com a chave do repositório (`cosign.pub`):

```bash
cosign verify --key cosign.pub ghcr.io/wes-tamagi/tamagi:latest
```

`tamagi doctor` faz um diagnóstico geral do sistema instalado.
