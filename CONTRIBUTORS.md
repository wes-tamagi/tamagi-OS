# Quem faz o Tamagi

**Autor e mantenedor:** wes-tamagi — direção do projeto, decisões de produto e o
menu iniciar (`cosmic-ext-classic-menu`).

## Colaboradores de IA

O Tamagi é escrito com a ajuda de agentes de IA, sob a direção do autor. Eles
são colaboradores do projeto e ficam registrados aqui e nas mensagens de
commit.

- **Gemini (Google), pelo Antigravity** — colaborador. Implementou a caixa de
  desenvolvimento multi-distro, os jogos nativos, a primeira tradução completa
  e os ajustes de desempenho; revisa o código do projeto e propõe ideias. A
  revisão do branch `english-cli-minimal` encontrou, entre outros, a falha da
  jaula dos agentes dentro da caixa de desenvolvimento e as pastas que
  faltavam na camada do virt-manager — corrigidas a partir dela.
- **Claude (Anthropic), pelo Claude Code** — colaborador. Comando único
  `tamagi`, imagem enxuta e catálogo, agentes presos ao projeto, temas que
  seguem o COSMIC, rechunk da imagem e a suíte de testes.

## Como o trabalho é registrado

- Commits escritos com um agente levam a linha `Co-Authored-By:` do agente.
- Achados de revisão que viram correção levam `Reviewed-by:` no commit.
- Toda mudança passa pela suíte (`just test`) antes de ir para o `main`; o que
  não foi testado é dito na mensagem do commit.
