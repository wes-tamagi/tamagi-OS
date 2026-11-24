# Guia de Estilo Material 3 do Tamagi (`docs/estilo-m3.md`)

**Autor:** Gemini (CEO)  
**Público:** Wes (Dono) e Claude (CTO)  
**Objetivo:** Traduzir a experiência visual e tátil do **Google Material 3 Expressive** (cores tonais, formas, tipografia, ícones e ritmo espacial) para o Tamagi OS sobre o desktop **COSMIC**, mantendo a identidade da marca (**Tamagi Ipê `#F2C14E`**).

---

## 1. Cores: Derivação a partir do COSMIC + Paleta Padrão "Tamagi Ipê"

### Como os papéis do Material 3 saem da cor de destaque do COSMIC

No Material 3, os papéis de cor não são escolhidos isoladamente: todos derivam de uma **cor semente** (`accent` no COSMIC) combinada com um **fundo neutro** (`bg_color` / `neutral_tint`) e um tom de texto (`on_bg_color` / `text_tint`), respeitando contraste mínimo (AA 4.5:1 para texto normal, 3:1 para elementos gráficos).

Mapeamento oficial entre o tema ativo do COSMIC (`cosmic::Theme`) e os papéis do Material 3 (no **claro** e no **escuro**):

| Papel M3 | Função | Derivação no COSMIC (**Escuro**) | Derivação no COSMIC (**Claro**) |
|---|---|---|---|
| `primary` | Ação principal, controles ativos, foco | `accent_color` (clareada/dessaturada se muito escura para manter contraste ≥ 4.5:1 sobre `surface`) | `accent_color` (escurecida para garantir contraste ≥ 4.5:1 com texto branco/escuro sobre `surface`) |
| `onPrimary` | Texto/ícone dentro de botão `primary` | `on_accent_color` (alto contraste; em amarelo Ipê `#F2C14E` deve ser **escuro** `#241A00`, nunca branco) | `on_accent_color` (em Ipê âmbar escuro `#6E4E00` é `#FFFFFF`; em amarelo vivo `#F2C14E` é `#1E1B13`) |
| `primaryContainer` | Círculos de ícone herói, destaques tonais | Mistura tonal de `surface` (72%) + `primary` (28%) | Mistura tonal de `surface` (78%) + `primary` (22%) ou tom claro ~90% luminância na matiz da semente |
| `onPrimaryContainer` | Texto/ícone sobre `primaryContainer` | Tom claro da matiz `primary` (contraste ≥ 4.5:1) | Tom escuro da matiz `primary` (contraste ≥ 4.5:1) |
| `secondary` | Ações auxiliares e filtros | Matiz complementar/análoga suave derivada de `accent_color` + `neutral_tint` (no Ipê: verde-folha `#8FC98A`) | Versão escura da matiz secundária (no Ipê Claro: verde-folha escuro `#2E6A39`) |
| `secondaryContainer` | Item selecionado em listas, chips ativos | `surfaceContainer` (78%) + `primary` ou `secondary` (22%) | `surfaceContainer` (82%) + `primary` ou `secondary` (18%) |
| `tertiary` | Ênfase equilibrada, progresso, insígnias | Segundo destaque da paleta (`WALL[4]`; no Ipê: turquesa `#6FC3B3`) | Versão saturada/escura do terceiro destaque (`#1F6B5E`) |
| `surface` | Fundo principal da janela | `bg_color` do tema COSMIC | `bg_color` do tema COSMIC |
| `surfaceContainerLowest` | Campos de entrada rebaixados, áreas internas | `surface` escurecido ~4% em direção ao preto | `#FFFFFF` (puro ou quase puro) |
| `surfaceContainerLow` | Cartões principais, grupos de opções | `surface` + 5% de `on_surface` | `surface` − 3% de luminância (cinza/creme bem suave) |
| `surfaceContainer` | Barra lateral, painéis de revisão | `surface` + 8% de `on_surface` | `surface` − 5% de luminância |
| `surfaceContainerHigh` | Menus flutuantes, caixas de diálogo | `surface` + 11% de `on_surface` | `surface` − 8% de luminância |
| `surfaceContainerHighest` | Estados pressionados/hover intensos | `surface` + 14% de `on_surface` | `surface` − 11% de luminância |
| `onSurface` | Texto principal | `on_bg_color` | `on_bg_color` |
| `onSurfaceVariant` | Subtítulos, descrições, rótulos secundários | `on_surface` com opacidade `0.72` (ou ~70% contraste) | `on_surface` com opacidade `0.68` |
| `outline` / `outlineVariant` | Bordas de campos e divisores sutis | `on_surface` a `0.32` / `0.14` | `on_surface` a `0.28` / `0.12` |
| `error` / `errorContainer` | Alertas destrutivos (ex.: apagar disco) | `destructive_color` / `mix(surface, destructive, 0.20)` | `destructive_color` / `mix(surface, destructive, 0.16)` |

> **Nota técnica de cor (para `welcome/src/m3.rs`):** o código atual usa interpolação linear sRGB (`mix()`). Para ficar fiel ao Material 3 (que opera em espaço perceptual **HCT — Hue, Chroma, Tone**), recomenda-se derivar os containers ajustando luminância/croma em espaço perceptual (ou Oklab/HCT) ou aplicar curvas específicas por `is_dark`, evitando que o modo escuro fique acinzentado demais.

---

### Paleta Padrão do Tamagi: **Tamagi Ipê** (Escuro e Claro)

A cor de marca atual em `build_files/50-branding.sh` e `system_files/usr/share/tamagi/themes/ipe.conf` é o amarelo ouro do ipê **`#F2C14E`** (`0;38;2;242;193;78`), acompanhado pelo verde-noite (`#101A15` / `#16211C`), verde-folha (`#8FC98A`) e turquesa (`#6FC3B3`).

#### Variante 1 — **Tamagi Ipê Escuro** (alinhada a `ipe.conf`)
- **Semente / `accent` (COSMIC):** `#F2C14E`
- **`surface` (`bg_color`):** `#16211C` (fundo profundo do brasão/Plymouth: `#101A15`)
- **`surfaceContainerLowest`:** `#101A15`
- **`surfaceContainerLow` (cards):** `#1E2B25`
- **`surfaceContainer`:** `#24332C`
- **`surfaceContainerHigh`:** `#2D3E36`
- **`surfaceContainerHighest`:** `#364940`
- **`primary`:** `#F2C14E` · **`onPrimary`:** `#241A00`
- **`primaryContainer`:** `#524118` · **`onPrimaryContainer`:** `#FFE08D`
- **`secondary`:** `#8FC98A` · **`secondaryContainer`:** `#27442A` · **`onSecondaryContainer`:** `#C6ECC1`
- **`tertiary`:** `#6FC3B3` · **`tertiaryContainer`:** `#1E4640`
- **`onSurface`:** `#E8E3D0` · **`onSurfaceVariant`:** `#B8BAA9`
- **`outline`:** `#5E6B62` · **`outlineVariant`:** `#34423B`
- **`error`:** `#F29488` · **`errorContainer`:** `#5C2420` · **`onErrorContainer`:** `#FFDAD5`

#### Variante 2 — **Tamagi Ipê Claro** (proposta nova para parear claro/escuro)
Como `ipe.conf` hoje só possui variante `SCHEME=dark`, o Tamagi precisa de um **`ipe-light.conf`** oficial para que a alternância Claro/Escuro mantenha a mesma identidade brasileira e não caia em azul (`Catppuccin Latte`) ou ciano genérico:

- **Semente / `accent` (COSMIC):** `#9A6B00` (âmbar ipê escurecido para leitura nítida sobre fundo claro; alternativamente `#D49B1A` para realces gráficos com texto escuro `#1E1B13`)
- **`surface` (`bg_color`):** `#FAF6EC` (branco marfim quente, evitando brilho clínico frio)
- **`surfaceContainerLowest`:** `#FFFFFF`
- **`surfaceContainerLow` (cards):** `#F3EFE3`
- **`surfaceContainer`:** `#EDE8DA`
- **`surfaceContainerHigh`:** `#E6E0D0`
- **`surfaceContainerHighest`:** `#DFD8C7`
- **`primary`:** `#8B6200` · **`onPrimary`:** `#FFFFFF`
- **`primaryContainer`:** `#FFE08D` (amarelo ipê luminoso!) · **`onPrimaryContainer`:** `#2B1D00`
- **`secondary`:** `#36683B` · **`secondaryContainer`:** `#CBECC7` · **`onSecondaryContainer`:** `#0C2912`
- **`tertiary`:** `#1F6B5E` · **`tertiaryContainer`:** `#B6EFE3`
- **`onSurface`:** `#1C1D18` · **`onSurfaceVariant`:** `#4D5147`
- **`outline`:** `#7D8176` · **`outlineVariant`:** `#D0D2C5`
- **`error`:** `#BA1A1A` · **`errorContainer`:** `#FFDAD6` · **`onErrorContainer`:** `#410002`

---

## 2. Forma, Tipografia e Espaçamento (Material 3 Expressive)

### Formas e Raios de Canto

O **Material 3 Expressive** amplia a escala de cantos e usa contraste entre superfícies agrupadas e elementos interativos:

| Token M3 | Valor | Uso no Tamagi / COSMIC |
|---|---|---|
| `None` | `0 px` | Separadores, telas em modo tela cheia |
| `ExtraSmall` | `4 px` | Campos de texto simples, cantos **internos** de listas segmentadas, tooltips |
| `Small` | `8 px` | Botões padrão pequenos, chips, menus compactos |
| `Medium` | `12 px` | Diálogos pequenos, botões médios |
| `Large` | `16 px` | Cartões (`card`), cantos **externos** de listas segmentadas, janelas auxiliares |
| `ExtraLarge` | `28 px` | Diálogos principais, sheets, painel flutuante do dock |
| `Full` | `9999 px` (pílula) | Botões principais (*Avançar*, *Instalar agora*), badges, barras de progresso arredondadas |

**Regra específica para listas (M3 Expressive):**  
Em vez de empilhar pílulas idênticas com `16 px` em todos os cantos (como está hoje em `welcome/src/m3.rs`), itens agrupados devem formar uma **lista segmentada**:
- Primeiro item do grupo: topo `16 px`, base `4 px`.
- Itens do meio: todos os cantos `4 px`.
- Último item do grupo: topo `4 px`, base `16 px`.
- Espaço entre itens do grupo: `2 px` (já aplicado em `m3::list()`).
- Quando um item é **selecionado**, ele pode assumir cantos totalmente arredondados (`16 px` ou `24 px`) para destacar a seleção no estilo Expressive.

### Tipografia e Escolha de Fonte

**Comparativo de fontes para o Tamagi:**
1. **Roboto Flex** (padrão oficial do Material 3 Expressive): fonte variável com eixos de peso (`wght`), largura (`wdth`) e tamanho óptico (`opsz`). **Licença:** SIL Open Font License 1.1 (`OFL-1.1`). É a escolha ideal se quisermos aderência máxima ao M3 Expressive (inclusive nos eixos variáveis).
2. **Inter:** excelente legibilidade em telas HiDPI/desktop e muito popular no Linux moderno (`OFL-1.1`), mas visual mais neutro/iOS-like e menos característico do Material 3.
3. **Noto Sans variável** (o que o Tamagi já empacota hoje em `build_files/packages.txt` via `google-noto-sans-vf-fonts` e configura em `55-tamagi-fonts.conf` + `40-desktop.sh`): visual humanista extremamente próximo da família Roboto/Google Sans, suporte completo a diacríticos/CJK/árabe sem conflitos, e já validada no `libcosmic` do Tamagi (onde inclusive foi documentado o cuidado de não misturar versão estática com variável).

**Decisão atualizada (2026-11-24 — Wes + CEO):**
- **Testar `Roboto Flex` (`OFL-1.1`):** o Dono aprovou testar **Roboto Flex** na imagem/COSMIC para aderência máxima ao Material 3 Expressive, verificando antes no build que ela não cause conflito de pesos no `libcosmic` (como ocorreu com `Noto Sans` estática vs. variável). **JetBrains Mono** permanece no terminal e em código; avaliar se **Roboto Slab** fica só no wordmark do Plymouth ou sai caso unifiquemos em Roboto Flex.
- **Fallback seguro:** caso **Roboto Flex** apresente regressão no `libcosmic` ou peso excessivo, manter **Noto Sans (variável)** (`google-noto-sans-vf-fonts`).
- **Escala tipográfica M3 adotada nas janelas do Tamagi** (alinhada ao `m3.rs` e `ui.rs` onde `14–16 px` já foi escolhido por legibilidade em notebook):
  - **Display Small:** `36 px` (tela de boas-vindas principal / destaque de abertura)
  - **Headline Medium:** `28 px` (títulos de página — já usado em `m3::HEADLINE`)
  - **Title Medium:** `18 px` (subtítulos de seção / cabeçalhos de cards)
  - **Body Large:** `16 px` (texto principal e títulos de linha em listas — já usado em `m3::BODY`)
  - **Label Large:** `14 px` (botões, textos de apoio e rótulos — já usado em `m3::LABEL`)
  - **Label Medium / Small:** `12–13 px` (badges e logs técnicos)

### Espaçamentos (Grade de 4 px / 8 px)
- Grade base: **`8 px`** (meio passo **`4 px`**).
- Espaço entre ícone de título (`56 px`) e bloco de texto: `20 px` (`HERO`).
- Padding interno de `card`: `16 px` (ou `20 px` em cartões principais).
- Espaço entre cartões / seções principais: `16 px` ou `24 px`.
- Espaço entre itens de lista segmentada: `2 px`.
- Altura mínima de toque/clique em linha de lista: `56 px`.
- Espessura de barra de progresso M3 Expressive: **`8 px`** (trilha arredondada, com `4 px` reservado apenas a indicadores secundários compactos).

---

## 3. Ícones: Material Symbols + Tema de Ícones Linux

### O Material Symbols pode ser usado direto como tema completo do desktop?

- **Verificado (licença):** o projeto oficial **Material Symbols** (`google/material-design-icons`) é distribuído sob **Apache License 2.0 (`Apache-2.0`)**, permitindo uso, modificação e redistribuição em imagens Linux.
- **Limitação prática:** **não dá para usar o Material Symbols sozinho como tema completo de desktop Linux.** Ele fornece milhares de glifos monocromáticos de interface (ações, status, navegação) em fonte variável e SVG, mas **não possui**:
  1. ícones coloridos para aplicativos de terceiros (Chromium, Steam, OnlyOffice, VS Code, etc.);
  2. cobertura completa de `mimetypes` (tipos de arquivo do padrão freedesktop);
  3. pastas coloridas/estilizadas para gerenciadores de arquivo (`cosmic-files`).
- **Conclusão:** usar **Material Symbols** (Apache-2.0) **apenas** para ícones simbólicos de UI próprios do Tamagi (quando precisarmos de glifos além dos padrões do sistema), combinado com um **tema de ícones freedesktop completo** "inspirado" no Material 3 / ChromeOS para o desktop.

### Comparativo de temas Linux nesse estilo

| Tema | Estilo | Licença | Apps | Pastas | Tipos de arquivo (`mimetypes`) | Observações para o Tamagi |
|---|---|---|---|---|---|---|
| **Colloid** (`vinceliuice/Colloid-icon-theme`) | Moderno, plano/geométrico com cantos suaves, muito próximo da estética Material 3 / ChromeOS | **GPL-3.0-or-later** | Sim (ampla) | Sim (várias cores, incl. amarelo/âmbar e modo escuro) | Sim | Melhor equilíbrio entre visual M3 moderno, pastas limpas e cobertura freedesktop. |
| **Papirus** (`PapirusDevelopmentTeam/papirus-icon-theme`) + `papirus-folders` | Material Design clássico, formas geométricas nítidas e consistentes | **GPL-3.0** (trechos LGPL-3.0 / CC-BY-SA-4.0) | Sim (enorme) | Sim (tinturáveis com `papirus-folders`, incl. amarelo) | Sim (muito completa) | Mais maduro e empacotado nativamente no Fedora; estética mais M2/M3 plana que M3 Expressive. |
| **Tela** (`vinceliuice/Tela-icon-theme`) | Plano, colorido, formas arredondadas e vibrantes | **GPL-3.0** | Sim (ampla) | Sim (inclusive variante amarela) | Sim | Muito leve e limpo; visual um pouco mais simples nas pastas que o Colloid. |
| **Fluent** (`vinceliuice/Fluent-icon-theme`) | Mistura de Fluent Design com Material | **GPL-3.0** | Sim | Sim | Sim | Mais próximo do Windows 11 do que do Google Material 3. |
| **MoreWaita** (`somepaulo/MoreWaita`) | Extensão de Adwaita (GNOME) para apps modernos | **GPL-3.0** | Sim (foco em apps) | Parcial (herda Adwaita) | Sim | Visual GNOME/Adwaita — não é Material 3. |
| **ChromeOS / Material You packs** (ex.: `vinceliuice/ChromeOS-kde`, `P3DROVFX/ii-icon-pack`) | Fiel ao ChromeOS / Android Material You | **GPL-3.0** ou variado | Parcial/baixa | Sim (em alguns) | Baixa/incompleta | Bons como referência visual, mas cobertura incompleta para uso isolado em desktop Linux. |

**Decisão do CEO (delegada pelo Dono em 2026-11-24):**
- **Tema escolhido: `Colloid` (`vinceliuice/Colloid-icon-theme`, `GPL-3.0-or-later`) na variante amarela / Ipê** como tema de ícones padrão do sistema — é o que melhor entrega cantos suaves, pastas limpas e estética alinhada ao Material 3 Expressive / ChromeOS com cobertura completa de apps, pastas e `mimetypes`.
- **Válvula de segurança de tamanho:** o CTO deve medir o peso em MB do `Colloid` (variante única amarela) após `rechunk.sh`; apenas se ficar desproporcionalmente pesado frente a **Papirus + `papirus-folders`** em amarelo (`GPL-3.0`), reportar os números antes de fixar.
- **Complemento simbólico:** glifos próprios do Tamagi (`tamagi-menu`, instalador, ícones simbólicos faltantes) desenhados ou derivados seguindo a grade de `24×24 dp` e traço arredondado do **Material Symbols (`Apache-2.0`)**.

---

## 4. Ordem de Aplicação no Sistema (Passo a Passo)

1. **Tema do COSMIC (cores e cantos):**
   - Criar `system_files/usr/share/tamagi/themes/ipe-light.conf` e ajustar `build_files/make-cosmic-themes.py` para respeitar explicitamente os raios de canto (`corner_radii`: ex. `radius_s: 8`, `radius_m: 12`, `radius_l: 16`, `radius_xl: 28`) nos `.ron` gerados.
   - Trocar o tema padrão da imagem em `build_files/40-desktop.sh` de `tamagi-tokyo-night.ron` para **`tamagi-ipe.ron`** (e garantir o mesmo na sessão live!).
2. **Ícones:**
   - Instalar o tema de ícones escolhido (Colloid-Yellow ou Papirus-Yellow) e configurar como padrão no COSMIC (`CosmicTk` / `gtk` / `dconf`).
3. **Papel de parede:**
   - Trocar o padrão em `build_files/40-desktop.sh` de `tokyo-night-mon.png` para **`ipe-mon.png`** (ou uma das artes em `ipe`), mantendo coerência imediata entre área de trabalho e cor de destaque `#F2C14E`.
4. **Tela de login (`cosmic-greeter`):**
   - Aplicar o mesmo fundo Ipê e tema Ipê no `cosmic-greeter` (hoje o `README.md` documenta que a tela de login usa o papel de parede padrão do COSMIC).
5. **Plymouth (tela de boot):**
   - Já nasce nas cores Ipê (`#101A15`, `#F2C14E`, `#6FC3B3`) em `build_files/make-plymouth.py` e `build_files/50-branding.sh`! Ajustar apenas se a tipografia do wordmark migrar de Roboto Slab para Noto Sans / Roboto Flex para unificação total.
6. **Janelas próprias do Tamagi:**
   - Unificar `tamagi-welcome` (primeiro login em `app.rs` + `ui.rs` e instalador em `setup_app.rs` + `m3.rs`) sobre um único módulo `m3.rs` atualizado para M3 Expressive.
   - Alinhar `gum.env` e os popups no `foot` com a paleta Ipê padrão.

---

## 5. O Que o COSMIC Deixa e Não Deixa Personalizar Hoje

| Aspecto | O COSMIC deixa personalizar hoje? | Como funciona / Limitações |
|---|---|---|
| **Cor de destaque (`accent`), fundo (`bg_color`), tintas (`neutral_tint`, `text_tint`), `success`, `warning`, `destructive`** | **Sim (totalmente)** | Via arquivo `.ron` em `/usr/share/cosmic-themes/` e `cosmic-settings appearance import` (já usado em `make-cosmic-themes.py`). |
| **Modo Claro / Escuro** | **Sim** | `Com.system76.CosmicTheme.Dark` e `.Light` separados em `/usr/share/cosmic/` e `~/.config/cosmic/`. |
| **Escala de cantos (`corner_radii`)** | **Sim (com limite)** | O `.ron` aceita `radius_0`, `radius_xs`, `radius_s`, `radius_m`, `radius_l`, `radius_xl`. Porém o shell COSMIC aplica esses tokens de forma global e simétrica nos componentes do desktop; não há *shape morphing* nem a biblioteca de 35 formas abstratas do M3 Expressive no compositor. |
| **Paleta tonal HCT de 5 níveis de superfície (`surfaceContainerLowest..Highest`)** | **Parcial (apenas em apps nossos)** | O `libcosmic` calcula automaticamente suas próprias superfícies a partir de `bg_color`; dentro do `tamagi-welcome` conseguimos desenhar containers M3 customizados via `theme::Container::custom`, mas apps COSMIC nativos (`cosmic-files`, `cosmic-edit`) seguem a derivação fixa do `libcosmic`. |
| **Listas segmentadas com cantos assimétricos (16 px fora, 4 px dentro)** | **Nos apps do Tamagi: a confirmar** | O `iced`/`libcosmic` tem `Border { radius: ... }`; precisamos que o CTO confirme se a versão pinada aceita `[f32; 4]` por vértice. Apps nativos do COSMIC continuam com o widget padrão deles. |
| **Tema de ícones e fontes globais** | **Sim** | Fontes via `CosmicTk/v1/{interface_font,monospace_font}` + `dconf` + `fontconfig`; tema de ícones via `CosmicTk/v1/icon_theme` (a confirmar chave exata com o CTO) e `dconf`. |
| **Tela de login (`cosmic-greeter`)** | **Parcial / Frágil** | O `cosmic-greeter` herda preferências por estado de usuário (`~/.local/state/cosmic/...`), mas no primeiro boot sem usuário logado costuma cair no papel de parede padrão do COSMIC (problema aberto já documentado em `README.md` L785). |
| **Animações M3 Expressive (molas / physics motion)** | **Não no compositor** | O `cosmic-comp` e `libcosmic` possuem transições próprias; não há suporte a física de molas/morphing do M3 Expressive em nível de sistema. |

### Perguntas Técnicas para o CTO Confirmar
1. Qual é a chave exata em `/usr/share/cosmic/com.system76.CosmicTk/v1/` (ou equivalente) que define o **tema de ícones padrão** no COSMIC atual?
2. `cosmic-greeter` permite definir papel de parede e tema Ipê padrão **antes** do primeiro login de qualquer usuário sem quebrar o cache `outputs.ron`?
3. A versão atual do `libcosmic` no `Containerfile` suporta `Border { radius: [tl, tr, br, bl].into() }` por vértice e troca de fonte para **Roboto Flex** sem o bug de duplicação de pesos já visto na `Noto Sans`?
4. Quanto pesam na imagem **Papirus + `papirus-folders`** vs **Colloid-icon-theme** (em MB após `rechunk.sh`), e o `cosmic-bg` ainda exige `f45-backgrounds` no RPM ou pode depender só dos nossos papéis de parede?
