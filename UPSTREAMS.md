# De onde o Tamagi vem

O Tamagi se apoia em outros projetos. Os que entram na imagem a partir do
código-fonte, os que recebem ajustes nossos e os que servem de base têm um
**fork na conta do projeto** — para fixar a versão que usamos, guardar os
nossos ajustes e mandar contribuições de volta. Os demais são só
**referenciados**.

## Forks (github.com/wes-tamagi)

| Fork | Origem | Licença | Para que serve no Tamagi |
|---|---|---|---|
| [cosmic-term](https://github.com/wes-tamagi/cosmic-term) | pop-os/cosmic-term | GPL-3.0 | O terminal que queremos ter: partir do cosmic-term e trazer o que o Ptyxis faz com contêineres e a leveza do foot (veja "Terminal" abaixo) |
| [cosmic-edit](https://github.com/wes-tamagi/cosmic-edit) | pop-os/cosmic-edit | GPL-3.0 | Base de um editor do Tamagi em Tauri + Rust (veja "Editor" abaixo) |
| [cosmic-ext-classic-menu](https://github.com/wes-tamagi/cosmic-ext-classic-menu) | championpeak87/cosmic-ext-classic-menu | GPL-3.0 | O menu iniciar, compilado na imagem |
| [tik](https://github.com/wes-tamagi/tik) | sysrich/tik | MIT | O instalador (com o módulo do Tamagi em `live/tik`) |
| [llama.cpp](https://github.com/wes-tamagi/llama.cpp) | ggml-org/llama.cpp | MIT | A IA local, compilada na imagem com Vulkan |
| [distrobox](https://github.com/wes-tamagi/distrobox) | 89luca89/distrobox | GPL-3.0 | As caixas; a imagem corrige o `distrobox-create` para o podman 6 |
| [CLIO](https://github.com/wes-tamagi/CLIO) | SyntheticAutonomicMind/CLIO | GPL-3.0 | Agente de programação do catálogo |
| [llm-scripts](https://github.com/wes-tamagi/llm-scripts) | fewtarius/llm-scripts | GPL-3.0 | Perfil de execução de modelos locais |
| [lazydocker](https://github.com/wes-tamagi/lazydocker) | jesseduffield/lazydocker | MIT | Visão de contêineres no terminal (opcional) |
| [main](https://github.com/wes-tamagi/main) | ublue-os/main | Apache-2.0 | A imagem base (`base-main`) |
| [akmods](https://github.com/wes-tamagi/akmods) | ublue-os/akmods | Apache-2.0 | Kernel OGC e módulos assinados |
| [bluefin](https://github.com/wes-tamagi/bluefin) | ublue-os/bluefin | Apache-2.0 | Referência do ambiente de desenvolvimento (VS Code + Dev Containers) |
| [Tamagi](https://github.com/wes-tamagi/Tamagi) | ublue-os/bazzite | Apache-2.0 | Referência de jogos: gamescope da Valve, sessão de jogos, HDR |

Um fork não muda de onde a imagem baixa as coisas: hoje o build continua
usando as origens, nas versões fixadas no `Containerfile`. O fork passa a ser
a fonte quando tiver um ajuste nosso.

## Só referenciados

- **COSMIC** ([pop-os/cosmic-epoch](https://github.com/pop-os/cosmic-epoch) e os
  seus componentes: cosmic-comp, libcosmic, cosmic-settings…) — usado como o
  Fedora o empacota, sem fork. A intenção é **contribuir no próprio projeto**:
  HDR no cosmic-comp (issue
  [#1384](https://github.com/pop-os/cosmic-comp/issues/1384)) e melhorias no
  libcosmic. Nada de fork paralelo do compositor.
- **Ptyxis** ([gitlab.gnome.org/chergert/ptyxis](https://gitlab.gnome.org/chergert/ptyxis)),
  GPL-3.0 — o terminal do sistema, do pacote do Fedora. Fica no GitLab do
  GNOME, não no GitHub.
- **foot** ([codeberg.org/dnkl/foot](https://codeberg.org/dnkl/foot)), MIT — o
  terminal das janelas do Tamagi, do pacote do Fedora. Fica no Codeberg.
- **Docker CE**, **VS Code**, **Tailscale**, os navegadores e os agentes de IA
  de terceiros: opcionais, baixados do fabricante na máquina do usuário.
- **openSUSE Aeon/MicroOS** (a manutenção do btrfs e a ideia do instalador),
  **Omarchy** (uso por teclado, temas), **Fuduntu** (energia automática, ainda
  por fazer).

## Terminal: o plano

1. **Agora:** o Ptyxis é o terminal do sistema (`Super+Enter`). Ele conhece as
   caixas distrobox e os contêineres do podman — o menu dele abre uma aba já
   dentro da caixa — e recebe a paleta do tema (`tamagi theme`). O foot
   continua nas janelas do Tamagi (primeiro login, atalhos, agente), por abrir
   na hora e aceitar comando, título e identificador pela linha de comando.
2. **Depois, no fork do cosmic-term:** levar para um terminal nativo do COSMIC
   o que hoje nos faz usar os outros dois — do Ptyxis, o menu de contêineres e
   os perfis por caixa; do foot, o início rápido, o modo servidor/cliente e as
   opções de linha de comando (comando, título, identificador da janela).
   Quando o fork cobrir isso, ele substitui os dois.

## Editor: o plano

O cosmic-edit já é escrito em Rust; a interface dele é feita com o libcosmic
(iced). Um editor em **Tauri** troca essa interface por uma em HTML/CSS/JS
dentro de uma webview (WebKitGTK), com o núcleo em Rust. Do fork aproveita-se
o que não é interface: abertura e gravação de arquivos, projetos, busca,
integração com git, configuração e temas de sintaxe. A interface é escrita de
novo. Nenhum código foi escrito no fork ainda; o cosmic-edit do Fedora
continua sendo o editor da imagem.
