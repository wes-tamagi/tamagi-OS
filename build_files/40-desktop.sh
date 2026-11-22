#!/usr/bin/bash
# Desktop COSMIC: tela de login, papéis de parede e padrões do Tamagi.
# O COSMIC guarda cada ajuste num arquivo em /usr/share/cosmic/<componente>/v1/,
# então os padrões do Tamagi são só arquivos na imagem; o que o usuário mudar
# vai para ~/.config/cosmic e tem prioridade.

C=/usr/share/cosmic

# Tela de login do COSMIC e boot gráfico.
systemctl enable cosmic-greeter.service
systemctl set-default graphical.target

# O assistente inicial do COSMIC repete o que o instalador já perguntou
# (teclado, fuso — e não avança sem escolher uma cidade). No primeiro login
# quem aparece é o assistente do Tamagi; o do COSMIC continua no lançador.
rm -f /etc/xdg/autostart/com.system76.CosmicInitialSetup.desktop

# ── Papéis de parede: aparecem em Configurações › Papel de parede ──
# Oito estilos por tema, desenhados a partir das cores do .conf. Só os PNG
# vão para /usr/share/backgrounds/tamagi, a pasta que o COSMIC lista: o
# usuário escolhe o papel de parede do jeito normal, nas Configurações.
WALL_SVG=/tmp/tamagi-wallpapers
python3 /ctx/build_files/make-wallpapers.py /usr/share/tamagi/themes "$WALL_SVG" /tmp/tamagi-wallpapers.xml
mkdir -p /usr/share/backgrounds/tamagi
for svg in "$WALL_SVG"/*.svg; do
    python3 -c 'import sys, xml.dom.minidom as m; m.parse(sys.argv[1])' "$svg"
    rsvg-convert -w 3840 "$svg" -o "/usr/share/backgrounds/tamagi/$(basename "${svg%.svg}").png"
done
cat > "$C/com.system76.CosmicBackground/v1/all" <<'RON'
(
    output: "all",
    source: Path("/usr/share/backgrounds/tamagi/tokyo-night-mon.png"),
    filter_by_theme: false,
    rotation_frequency: 3600,
    filter_method: Lanczos,
    scaling_mode: Zoom,
    sampling_method: Alphanumeric,
)
RON

# ── Temas: um tema do COSMIC para cada tema do Tamagi ──
# Ficam em /usr/share/cosmic-themes, junto dos temas do próprio COSMIC, cada
# um com a sua miniatura: é dali que o COSMIC os oferece (escolha de tema) e
# que Configurações › Aparência › Importar os lê. O `tamagi theme sync` leva
# as cores do tema escolhido no COSMIC para os apps de terminal.
python3 /ctx/build_files/make-cosmic-themes.py /usr/share/tamagi/themes /usr/share/cosmic-themes
n_conf=$(find /usr/share/tamagi/themes -name '*.conf' | wc -l)
[[ $(find /usr/share/cosmic-themes -name 'tamagi-*.ron' | wc -l) -eq $n_conf ]] \
    || { echo "faltou gerar tema do COSMIC para algum tema do Tamagi" >&2; exit 1; }
for conf in /usr/share/tamagi/themes/*.conf; do
    name=$(basename "$conf" .conf)
    rsvg-convert -w 768 -h 432 "$WALL_SVG/$name-mon.svg" -o "/usr/share/cosmic-themes/tamagi-$name.png"
done
# O tema padrão da imagem é o Tokyo Night do Tamagi, também para o COSMIC
# (janelas, barra e apps): o import escreve o tema completo numa pasta
# temporária, que vira o padrão do sistema. O que o usuário escolher depois
# fica em ~/.config/cosmic e tem prioridade.
THEME_TMP=$(mktemp -d)
if XDG_CONFIG_HOME="$THEME_TMP" HOME="$THEME_TMP" timeout 60 \
        cosmic-settings appearance import /usr/share/cosmic-themes/tamagi-tokyo-night.ron >/dev/null 2>&1 \
        && compgen -G "$THEME_TMP/cosmic/com.system76.CosmicTheme.Dark.Builder/v*/bg_color" >/dev/null; then
    for d in "$THEME_TMP"/cosmic/com.system76.CosmicTheme.*; do
        [[ -n $(find "$d" -type f -print -quit) ]] || continue
        mkdir -p "$C/$(basename "$d")"
        cp -a "$d/." "$C/$(basename "$d")/"
    done
    echo "tema padrão do COSMIC: Tamagi Tokyo Night"
else
    echo "aviso: não deu para gravar o tema padrão do COSMIC (o cosmic-settings mudou?); fica o tema do COSMIC" >&2
fi

# ── Janelas ──
# Tiling automático ligado em todos os workspaces (Super+Y alterna).
echo true > "$C/com.system76.CosmicComp/v1/autotile"
echo Global > "$C/com.system76.CosmicComp/v1/autotile_behavior"
# Teclado ABNT2 por padrão (o instalador grava outro, se você escolher).
cat > "$C/com.system76.CosmicComp/v1/xkb_config" <<'RON'
(
    rules: "",
    model: "",
    layout: "br",
    variant: "",
    options: None,
    repeat_delay: 600,
    repeat_rate: 25,
)
RON
# Janelas utilitárias do Tamagi (menu, ações rápidas, agente) abrem flutuando:
# o popup usa o foot com o app id "org.tamagi.Menu" (o da paleta: a barra
# mostra o ícone do Tamagi em vez de um ícone genérico).
rules="$C/com.system76.CosmicSettings.WindowRules/v1/tiling_exception_defaults"
python3 - "$rules" <<'PY'
import sys
path = sys.argv[1]
text = open(path, encoding="utf-8").read().rstrip()
assert text.endswith("]"), "tiling_exception_defaults mudou de formato"
text = text[:-1].rstrip() + '\n\t// Tamagi\n\t(appid: "tamagi-popup", titles: [".*"]),\n\t(appid: "org.tamagi.Menu", titles: [".*"]),\n\t(appid: "zenity", titles: [".*"]),\n]\n'
open(path, "w", encoding="utf-8").write(text)
PY

# Paleta padrão das janelas do Tamagi (o tamagi-theme gera a do usuário).
tamagi-theme foot tokyo-night > /usr/share/tamagi/foot.ini
foot --config=/usr/share/tamagi/foot.ini --check-config >/dev/null || { echo "foot.ini das janelas do Tamagi é inválido" >&2; exit 1; }

# ── Terminal padrão: foot (o cosmic-term continua instalado, no lançador) ──
# Configuração do sistema com a fonte e as cores do tema padrão; o
# tamagi-theme grava a do usuário em ~/.config/foot/foot.ini.
tamagi-theme foot tokyo-night > /etc/xdg/foot/foot.ini
foot --check-config >/dev/null || { echo "foot.ini gerado pelo tamagi-theme é inválido" >&2; exit 1; }
# O atalho System(Terminal) do COSMIC (Super+Enter) roda o que estiver aqui.
sed -i 's|^\(\s*Terminal: \)"cosmic-term",|\1"/usr/libexec/tamagi/term",|' "$C/com.system76.CosmicSettings.Shortcuts/v1/system_actions"
grep -q 'Terminal: "/usr/libexec/tamagi/term"' "$C/com.system76.CosmicSettings.Shortcuts/v1/system_actions" \
    || { echo "system_actions do COSMIC mudou de formato: o terminal padrão não virou foot" >&2; exit 1; }

# ── Arquivos (COSMIC Files): grade, pastas primeiro ──
# A lista continua a um clique, no botão de visualização (ou Ctrl+1/Ctrl+2).
F="$C/com.system76.CosmicFiles/v1"
mkdir -p "$F"
grep -q "view:" "$F/tab" 2>/dev/null || echo "aviso: o COSMIC Files mudou o formato de 'tab'; confira os padrões" >&2
rm -f "$F/tab"
cat > "$F/tab" <<'RON'
(
    folders_first: true,
    icon_sizes: (
        list: 100,
        grid: 100,
    ),
    show_hidden: false,
    single_click: false,
    view: Grid,
)
RON

# ── Fontes: Noto Sans na interface do COSMIC (libcosmic) e nos apps GTK ──
# Roboto Slab é a fonte com serifa do sistema (fontconfig, 55-tamagi-fonts.conf).
install -d "$C/com.system76.CosmicTk/v1"
rm -f "$C/com.system76.CosmicTk/v1/interface_font" "$C/com.system76.CosmicTk/v1/monospace_font"
echo '(family: "Noto Sans", weight: Normal, stretch: Normal, style: Normal)' > "$C/com.system76.CosmicTk/v1/interface_font"
echo '(family: "JetBrains Mono", weight: Normal, stretch: Normal, style: Normal)' > "$C/com.system76.CosmicTk/v1/monospace_font"
# cosmic-term e cosmic-edit guardam a fonte na configuração do próprio app.
for app in CosmicTerm CosmicEdit; do
    install -d "$C/com.system76.$app/v1"
    rm -f "$C/com.system76.$app/v1/font_name"
    echo '"JetBrains Mono"' > "$C/com.system76.$app/v1/font_name"
done
# O libcosmic vai mudar a fonte de lugar (de CosmicTk para o tema): quando a
# versão empacotada deixar de ler esta chave, o build avisa em vez de a
# interface voltar para a fonte padrão em silêncio.
grep -aq interface_font /usr/bin/cosmic-settings || { echo "o libcosmic desta versão não lê mais CosmicTk/interface_font: ajuste 40-desktop.sh" >&2; exit 1; }
install -Dm644 /dev/stdin /etc/dconf/db/local.d/10-tamagi-fonts <<'DCONF'
[org/gnome/desktop/interface]
font-name='Noto Sans 11'
document-font-name='Roboto Slab 11'
monospace-font-name='JetBrains Mono 11'
DCONF
# (o perfil padrão do Fedora já lê o banco "local"; só cria se não existir)
[[ -e /etc/dconf/profile/user ]] || printf 'user-db:user\nsystem-db:local\n' | install -Dm644 /dev/stdin /etc/dconf/profile/user
dconf update
# Uma família, uma cópia: Noto Sans estática e variável juntas fazem o texto
# sumir ou trocar de peso em apps do COSMIC.
rpm -q google-noto-sans-fonts >/dev/null 2>&1 && { echo "google-noto-sans-fonts (estática) duplicaria a Noto Sans variável" >&2; exit 1; }
fc-cache -f >/dev/null
[[ $(fc-match -f '%{family}' sans-serif) == "Noto Sans" ]] || { echo "sans-serif não é Noto Sans" >&2; exit 1; }
[[ $(fc-match -f '%{family}' serif) == "Roboto Slab" ]] || { echo "serif não é Roboto Slab" >&2; exit 1; }

# ── Barra: uma só, embaixo, com os apps no centro ──
# Junta o layout "Centered" do AnduinOS (barra de tarefas embaixo, apps no
# meio, bandeja e relógio à direita) com os applets e o acabamento do COSMIC
# (barra flutuante, cantos arredondados, um pouco translúcida). O layout
# original do COSMIC (painel em cima + dock) e o clássico continuam a um
# comando: tamagi-panel.
tamagi-panel defaults "$C"
# O Dock do COSMIC tem de continuar como veio do pacote (o estilo "cosmic" volta a ele).
dock_changed=$(rpm -V cosmic-panel 2>/dev/null | grep 'CosmicPanel\.Dock/' | grep -c '^..5' || true)
[[ $dock_changed -eq 0 ]] || { echo "tamagi-panel alterou os padrões do Dock (hardlink)" >&2; exit 1; }

# ── Menu iniciar ancorado ao botão (cosmic-ext-classic-menu) ──
# Botão com o brasão do Tamagi (o anel com as três joias), sem texto. Layout
# "Retro": usuário no topo, lista de apps à esquerda, lugares (Pasta pessoal,
# Documentos, Configurações…) à direita, busca e botões de energia embaixo.
# Os outros layouts ficam nas configurações do próprio menu.
command -v cosmic-ext-classic-menu-applet >/dev/null || { echo "applet do menu iniciar não instalado" >&2; exit 1; }
[[ -f /usr/share/applications/com.championpeak87.cosmic-ext-classic-menu.desktop ]] \
    || { echo "o applet do menu mudou de identificador: ajuste tamagi-panel" >&2; exit 1; }
MENU_ICON=/usr/share/tamagi/menu/tamagi-start-symbolic.svg
[[ -f $MENU_ICON ]] || { echo "falta o ícone do menu: $MENU_ICON" >&2; exit 1; }
rsvg-convert "$MENU_ICON" -o /tmp/menu-icon-check.png && [[ -s /tmp/menu-icon-check.png ]] && rm -f /tmp/menu-icon-check.png
M="$C/com.championpeak87.cosmic-ext-classic-menu/v1"
mkdir -p "$M"
echo Retro > "$M/menu_layout"
echo Places > "$M/side_panel"
echo Bottom > "$M/search_field_position"
echo Favorites > "$M/featured_applications"
echo IconOnly > "$M/applet_button_style"
echo "\"$MENU_ICON\"" > "$M/button_icon"
# Menu focado (1.1.0): abre já com o teclado (digitar busca, setas navegam),
# também pelo atalho. Ele abre no canto da tela do lado do botão, e sem o
# desfoque do popup. Sem isto, só se digita depois de clicar na busca.
echo true > "$M/focused_menu"
grep -aq focused_menu /usr/bin/cosmic-ext-classic-menu-applet \
    || { echo "o applet do menu não conhece focused_menu (versão antiga?)" >&2; exit 1; }
cat > "$M/favorite_applications" <<'RON'
[
    "tamagi-browser",
    "com.system76.CosmicFiles",
    "org.gnome.Ptyxis",
    "tamagi-agent",
    "org.tamagi.Menu",
    "com.system76.CosmicSettings",
]
RON

# ── Relógio do painel em 24 horas (o primeiro uso troca para AM/PM em inglês) ──
mkdir -p "$C/com.system76.CosmicAppletTime/v1"
echo true > "$C/com.system76.CosmicAppletTime/v1/military_time"

# Poucos favoritos: a imagem é enxuta, e o foco é IA (o agente local fica à
# mão). "tamagi-browser" é o navegador
# genérico: abre o navegador padrão e, enquanto não há nenhum, deixa escolher
# um; o primeiro instalado toma o lugar dele na barra (tamagi-browser).
cat > "$C/com.system76.CosmicAppList/v1/favorites" <<'RON'
[
    "tamagi-browser",
    "com.system76.CosmicFiles",
    "org.gnome.Ptyxis",
    "tamagi-agent",
    "com.system76.CosmicEdit",
    "com.system76.CosmicSettings",
]
RON

# ── Atalhos do Tamagi por cima dos padrões do COSMIC ──
# Cada atalho novo substitui o que já usava a mesma combinação de teclas.
python3 - "$C/com.system76.CosmicSettings.Shortcuts/v1/defaults" <<'PY'
import re
import sys

path = sys.argv[1]
popup = "/usr/libexec/tamagi/popup"
ours = [
    ("[Super]", "Return", "System(Terminal)"),
    ("[Super]", "w", "Close"),
    # Super+Espaço: o menu iniciar da barra (o botão do Tamagi).
    ("[Super]", "space", 'Spawn("/usr/libexec/tamagi/start-menu")'),
    ("[Super, Ctrl]", "space", "System(InputSourceSwitch)"),
    ("[Super]", "b", "System(WebBrowser)"),
    ("[Super]", "e", "System(HomeFolder)"),
    ("[Super]", "f", "Fullscreen"),
    ("[Super]", "t", "ToggleWindowFloating"),
    # Super+A: a paleta de comandos (ações do Tamagi, instalar, configurar).
    ("[Super]", "a", 'Spawn("tamagi-menu")'),
    ("[Super, Alt]", "space", "System(Launcher)"),
    ("[Super, Shift]", "a", f'Spawn("{popup} tamagi-agent")'),
    ("[Super]", "k", f'Spawn("{popup} tamagi-keys")'),
    ("[Super, Shift]", "t", f'Spawn("{popup} btop")'),
    ("[Super, Shift]", "n", f'Spawn("{popup} --tile nvim")'),
    # Do Omarchy: captura, apps com Super+Shift+letra, emoji e lembrete.
    ("[Shift]", "Print", 'Spawn("tamagi-capture text")'),
    ("[Alt]", "Print", 'Spawn("tamagi-capture record")'),
    ("[Super, Shift]", "g", f'Spawn("{popup} --tile lazygit")'),
    ("[Super, Shift]", "d", f'Spawn("{popup} tamagi-db")'),
    ("[Super, Shift]", "o", 'Spawn("/usr/libexec/tamagi/open-or-install org.onlyoffice.desktopeditors")'),
    ("[Super, Shift]", "m", 'Spawn("/usr/libexec/tamagi/open-or-install io.missioncenter.MissionCenter")'),
    ("[Super, Shift]", "w", 'Spawn("tamagi-windows open")'),
    ("[Super, Ctrl]", "e", 'Spawn("/usr/libexec/tamagi/emoji")'),
    ("[Super, Ctrl]", "r", f'Spawn("{popup} tamagi-remind")'),
    ("[Super, Ctrl]", "p", 'Spawn("tamagi-privacy")'),
]

def norm(mods):
    return ",".join(sorted(m.strip() for m in mods.strip("[]").split(",") if m.strip()))

taken = {(norm(m), k) for m, k, _ in ours}
entry = re.compile(r'^\s*\(modifiers:\s*(\[[^\]]*\])(?:,\s*key:\s*"([^"]+)")?\):')
kept = []
for line in open(path, encoding="utf-8").read().splitlines():
    m = entry.match(line)
    if m and (norm(m.group(1)), m.group(2)) in taken:
        continue
    kept.append(line)

end = max(i for i, line in enumerate(kept) if line.strip() == "}")
new = ["", "    // Tamagi"] + [f'    (modifiers: {m}, key: "{k}"): {a},' for m, k, a in ours]
kept[end:end] = new
open(path, "w", encoding="utf-8").write("\n".join(kept) + "\n")
print(f"atalhos do COSMIC: {len(ours)} do Tamagi aplicados")
PY

# Terminal para apps de terminal (.desktop com Terminal=true) e `xdg-terminal-exec`:
# o foot primeiro (é o padrão), depois o cosmic-term.
install -Dm644 /dev/stdin /etc/xdg/xdg-terminals.list <<'LIST'
org.gnome.Ptyxis.desktop
foot.desktop
com.system76.CosmicTerm.desktop
LIST
# O foot instala também o modo servidor (foot-server/footclient); no lançador
# basta o foot.
for f in foot-server footclient; do
    [[ -f /usr/share/applications/$f.desktop ]] && \
        desktop-file-edit --set-key=NoDisplay --set-value=true /usr/share/applications/$f.desktop
done

gtk-update-icon-cache -f /usr/share/icons/hicolor 2>/dev/null || true

for f in /usr/share/applications/tamagi-*.desktop /usr/share/applications/org.tamagi.Menu.desktop \
         /usr/share/tamagi/desktop/*.desktop \
         /etc/xdg/autostart/tamagi-*.desktop; do
    desktop-file-validate "$f"
done
