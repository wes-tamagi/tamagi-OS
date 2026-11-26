#!/usr/bin/bash
# Desktop COSMIC: tela de login, papéis de parede enxutos e padrões por edição
# (Tamagi-DxAI ou Tamagi-SKL).

C=/usr/share/cosmic
EDITION=${TAMAGI_EDITION:-dxai}

systemctl enable cosmic-greeter.service
systemctl set-default graphical.target

rm -f /etc/xdg/autostart/com.system76.CosmicInitialSetup.desktop

# ── Papéis de parede enxutos (dieta v1.5) ──
# Em vez de 40 PNGs 4K (8 estilos x 5 temas), renderizamos em 2560px apenas:
#   - ipe e ipe-light: mon, ondas e seigaiha (6 arquivos)
#   - gruvbox, nord, tokyo-night: mon (3 arquivos, para troca de tema manter papel)
# Total: 9 PNGs leves (economia >75% em /usr/share/backgrounds/tamagi).
WALL_SVG=/tmp/tamagi-wallpapers
python3 /ctx/build_files/make-wallpapers.py /usr/share/tamagi/themes "$WALL_SVG" /tmp/tamagi-wallpapers.xml
mkdir -p /usr/share/backgrounds/tamagi
render_wall() {
    local svg="$WALL_SVG/$1.svg"
    [[ -f $svg ]] || { echo "SVG ausente: $svg" >&2; exit 1; }
    python3 -c 'import sys, xml.dom.minidom as m; m.parse(sys.argv[1])' "$svg"
    rsvg-convert -w 2560 "$svg" -o "/usr/share/backgrounds/tamagi/$1.png"
}
for w in ipe ipe-mon ipe-seigaiha ipe-light ipe-light-mon ipe-light-seigaiha gruvbox-mon nord-mon tokyo-night-mon; do
    render_wall "$w"
done
# Em Tamagi-SKL, remove os papéis de parede ilustrados extras para enxugar ainda mais.
# Em Tamagi-DxAI, importa os papéis de parede espaciais sem marca Pop!_OS/COSMIC
# (NASA/ESA) do pacote cosmic-wallpapers e remove quaisquer variantes com logo.
if [[ $EDITION == skl ]]; then
    rm -rf /usr/share/backgrounds/tamagi/art /usr/share/backgrounds/cosmic
else
    if [[ -d /usr/share/backgrounds/cosmic ]]; then
        find /usr/share/backgrounds/cosmic -maxdepth 2 -type f \
            \( -iname '*logo*' -o -iname '*pop*' -o -iname '*system76*' -o -iname '*cosmic*' \) \
            -delete 2>/dev/null || true
        while IFS= read -r wp; do
            [[ -f $wp ]] || continue
            ln -sf "$wp" "/usr/share/backgrounds/tamagi/$(basename "$wp")"
        done < <(find /usr/share/backgrounds/cosmic -maxdepth 2 -type f \( -iname '*.jpg' -o -iname '*.jpeg' -o -iname '*.png' \) | sort)
    fi
fi

cat > "$C/com.system76.CosmicBackground/v1/all" <<'RON'
(
    output: "all",
    source: Path("/usr/share/backgrounds/tamagi/ipe-mon.png"),
    filter_by_theme: false,
    rotation_frequency: 3600,
    filter_method: Lanczos,
    scaling_mode: Zoom,
    sampling_method: Alphanumeric,
)
RON

# ── Temas do COSMIC (.ron + miniaturas) ──
python3 /ctx/build_files/make-cosmic-themes.py /usr/share/tamagi/themes /usr/share/cosmic-themes
n_conf=$(find /usr/share/tamagi/themes -name '*.conf' | wc -l)
[[ $(find /usr/share/cosmic-themes -name 'tamagi-*.ron' | wc -l) -eq $n_conf ]] \
    || { echo "faltou gerar tema do COSMIC para algum tema do Tamagi" >&2; exit 1; }
for conf in /usr/share/tamagi/themes/*.conf; do
    name=$(basename "$conf" .conf)
    rsvg-convert -w 768 -h 432 "$WALL_SVG/$name-mon.svg" -o "/usr/share/cosmic-themes/tamagi-$name.png"
done

THEME_TMP=$(mktemp -d)
for theme in ipe-light ipe; do
    XDG_CONFIG_HOME="$THEME_TMP" HOME="$THEME_TMP" timeout 60 \
        cosmic-settings appearance import "/usr/share/cosmic-themes/tamagi-$theme.ron" >/dev/null 2>&1 \
        || { echo "cosmic-settings não importou tamagi-$theme.ron" >&2; exit 1; }
done
compgen -G "$THEME_TMP/cosmic/com.system76.CosmicTheme.Dark.Builder/v*/bg_color" >/dev/null \
    && compgen -G "$THEME_TMP/cosmic/com.system76.CosmicTheme.Light.Builder/v*/bg_color" >/dev/null \
    || { echo "o import do tema não gravou os modos claro e escuro" >&2; exit 1; }
for d in "$THEME_TMP"/cosmic/com.system76.CosmicTheme.*; do
    [[ -n $(find "$d" -type f -print -quit) ]] || continue
    mkdir -p "$C/$(basename "$d")"
    cp -a "$d/." "$C/$(basename "$d")/"
done
echo "tema padrão do COSMIC: Tamagi Ipê (escuro) e Ipê Claro"

# ── Janelas & Teclado ──
if [[ $EDITION == skl ]]; then
    echo false > "$C/com.system76.CosmicComp/v1/autotile"
else
    echo true > "$C/com.system76.CosmicComp/v1/autotile"
    echo Global > "$C/com.system76.CosmicComp/v1/autotile_behavior"
fi

case "${TAMAGI_DEFAULT_LANG:-en}" in
    en) XKB_DEFAULT=us ;;
    pt) XKB_DEFAULT=br ;;
    *)  echo "TAMAGI_DEFAULT_LANG deve ser en ou pt" >&2; exit 1 ;;
esac
echo "${TAMAGI_DEFAULT_LANG:-en}" > /usr/share/tamagi/default-lang
if [[ ${TAMAGI_DEFAULT_LANG:-en} == pt ]]; then
    echo 'LANG=pt_BR.UTF-8' > /etc/locale.conf
    echo 'KEYMAP=br-abnt2' > /etc/vconsole.conf
else
    echo 'LANG=en_US.UTF-8' > /etc/locale.conf
    echo 'KEYMAP=us' > /etc/vconsole.conf
fi
cat > "$C/com.system76.CosmicComp/v1/xkb_config" <<RON
(
    rules: "",
    model: "",
    layout: "${XKB_DEFAULT}",
    variant: "",
    options: None,
    repeat_delay: 600,
    repeat_rate: 25,
)
RON

rules="$C/com.system76.CosmicSettings.WindowRules/v1/tiling_exception_defaults"
python3 - "$rules" <<'PY'
import sys
path = sys.argv[1]
text = open(path, encoding="utf-8").read().rstrip()
assert text.endswith("]"), "tiling_exception_defaults mudou de formato"
text = text[:-1].rstrip() + '\n\t// Tamagi\n\t(appid: "tamagi-popup", titles: [".*"]),\n\t(appid: "org.tamagi.Menu", titles: [".*"]),\n\t(appid: "org.tamagi.Welcome", titles: [".*"]),\n\t(appid: "zenity", titles: [".*"]),\n]\n'
open(path, "w", encoding="utf-8").write(text)
PY

sed -i 's|^\(\s*Terminal: \)"cosmic-term",|\1"/usr/libexec/tamagi/term",|' "$C/com.system76.CosmicSettings.Shortcuts/v1/system_actions"
grep -q 'Terminal: "/usr/libexec/tamagi/term"' "$C/com.system76.CosmicSettings.Shortcuts/v1/system_actions" \
    || { echo "system_actions do COSMIC mudou de formato" >&2; exit 1; }

# ── Arquivos (COSMIC Files): grade, pastas primeiro ──
F="$C/com.system76.CosmicFiles/v1"
mkdir -p "$F"
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

# ── Fontes e Ícones Vetoriais (Papirus SVG + Roboto Flex / Inter + JetBrains Mono) ──
install -d "$C/com.system76.CosmicTk/v1"
rm -f "$C/com.system76.CosmicTk/v1/interface_font" "$C/com.system76.CosmicTk/v1/monospace_font" "$C/com.system76.CosmicTk/v1/icon_theme"
echo '(family: "Roboto Flex", weight: Normal, stretch: Normal, style: Normal)' > "$C/com.system76.CosmicTk/v1/interface_font"
echo '(family: "JetBrains Mono", weight: Normal, stretch: Normal, style: Normal)' > "$C/com.system76.CosmicTk/v1/monospace_font"
echo '"Papirus-Dark"' > "$C/com.system76.CosmicTk/v1/icon_theme"
for app in CosmicTerm CosmicEdit; do
    install -d "$C/com.system76.$app/v1"
    rm -f "$C/com.system76.$app/v1/font_name"
    echo '"JetBrains Mono"' > "$C/com.system76.$app/v1/font_name"
done
grep -aq interface_font /usr/bin/cosmic-settings || { echo "o libcosmic desta versão não lê mais CosmicTk/interface_font" >&2; exit 1; }
install -Dm644 /dev/stdin /etc/dconf/db/local.d/10-tamagi-fonts <<'DCONF'
[org/gnome/desktop/interface]
font-name='Roboto Flex 11'
document-font-name='Roboto Flex 11'
monospace-font-name='JetBrains Mono 11'
icon-theme='Papirus-Dark'
DCONF
[[ -e /etc/dconf/profile/user ]] || printf 'user-db:user\nsystem-db:local\n' | install -Dm644 /dev/stdin /etc/dconf/profile/user
dconf update
rpm -q google-noto-sans-fonts >/dev/null 2>&1 && { echo "google-noto-sans-fonts (estática) duplicaria a Noto Sans variável" >&2; exit 1; }
fc-cache -f >/dev/null
[[ $(fc-match -f '%{family}' sans-serif) == "Roboto Flex" ]] || { echo "sans-serif não é Roboto Flex" >&2; exit 1; }
[[ $(fc-list 'Roboto Flex' family | sort -u) == "Roboto Flex" ]] || { echo "Roboto Flex ausente ou duplicada" >&2; exit 1; }
[[ $(fc-match -f '%{family}' 'sans-serif:lang=ja') != "Roboto Flex" ]] || { echo "sem fonte de reserva para japonês" >&2; exit 1; }

# ── Barra e Menu Iniciar ──
if [[ $EDITION == skl ]]; then
    tamagi-panel defaults-skl "$C"
else
    tamagi-panel defaults "$C"
fi
dock_changed=$(rpm -V cosmic-panel 2>/dev/null | grep 'CosmicPanel\.Dock/' | grep -c '^..5' || true)
[[ $dock_changed -eq 0 ]] || { echo "tamagi-panel alterou os padrões do Dock (hardlink)" >&2; exit 1; }

command -v cosmic-ext-classic-menu-applet >/dev/null || { echo "applet do menu iniciar não instalado" >&2; exit 1; }
[[ -f /usr/share/applications/com.championpeak87.cosmic-ext-classic-menu.desktop ]] \
    || { echo "o applet do menu mudou de identificador" >&2; exit 1; }
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
echo true > "$M/focused_menu"
grep -aq focused_menu /usr/bin/cosmic-ext-classic-menu-applet \
    || { echo "o applet do menu não conhece focused_menu" >&2; exit 1; }

mkdir -p "$C/com.system76.CosmicAppletTime/v1"
echo true > "$C/com.system76.CosmicAppletTime/v1/military_time"

# ── WebApps de sistema e favoritos por edição ──
if [[ $EDITION == skl ]]; then
    # Tamagi-SKL: atalhos WebApp (Google Docs, Planilhas, Apresentações e Gemini)
    # no menu e fixados no painel travado + abertura automática do Chromium.
    cat > /usr/share/applications/tamagi-skl-docs.desktop <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Google Docs
Comment=Google Docs (WebApp)
Exec=/usr/bin/chromium-browser --app=https://docs.google.com --class=tamagi-skl-docs
Icon=x-office-document
Terminal=false
Categories=Office;WordProcessor;
StartupWMClass=tamagi-skl-docs
SingleMainWindow=true
DESKTOP
    cat > /usr/share/applications/tamagi-skl-sheets.desktop <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Google Sheets
Name[pt_BR]=Google Planilhas
Name[pt]=Google Planilhas
Comment=Google Sheets (WebApp)
Exec=/usr/bin/chromium-browser --app=https://docs.google.com/spreadsheets --class=tamagi-skl-sheets
Icon=x-office-spreadsheet
Terminal=false
Categories=Office;Spreadsheet;
StartupWMClass=tamagi-skl-sheets
SingleMainWindow=true
DESKTOP
    cat > /usr/share/applications/tamagi-skl-slides.desktop <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Google Slides
Name[pt_BR]=Google Apresentações
Name[pt]=Google Apresentações
Comment=Google Slides (WebApp)
Exec=/usr/bin/chromium-browser --app=https://docs.google.com/presentation --class=tamagi-skl-slides
Icon=x-office-presentation
Terminal=false
Categories=Office;Presentation;
StartupWMClass=tamagi-skl-slides
SingleMainWindow=true
DESKTOP
    cat > /usr/share/applications/tamagi-skl-gemini.desktop <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Gemini
Comment=Google Gemini AI (WebApp)
Exec=/usr/bin/chromium-browser --app=https://gemini.google.com --class=tamagi-skl-gemini
Icon=applications-science
Terminal=false
Categories=Network;Education;
StartupWMClass=tamagi-skl-gemini
SingleMainWindow=true
DESKTOP
    cat > /etc/xdg/autostart/tamagi-skl-browser.desktop <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Tamagi-SKL Browser
Comment=Start Chromium on student login
Exec=/usr/libexec/tamagi/skl-browser-launch
X-GNOME-Autostart-enabled=true
NoDisplay=true
DESKTOP

    # Política padrão gerenciada do Chromium para Tamagi-SKL (a URL inicial é
    # preenchida pelo instalador ou via `sudo tamagi-skl homepage <url>`):
    # - nunca salva senhas (PasswordManagerEnabled: false)
    # - destrói cookies/login ao fechar (ClearBrowsingDataOnExitList)
    # - mantém histórico por exatamente 15 dias = 360 horas (BrowsingDataLifetime)
    install -Dm644 /dev/stdin /etc/chromium/policies/managed/tamagi-skl.json <<'JSON'
{
  "BrowserSignin": 0,
  "IncognitoModeAvailability": 1,
  "DeveloperToolsAvailability": 2,
  "URLBlocklist": [
    "chrome://extensions",
    "chrome://settings/extensions"
  ],
  "PasswordManagerEnabled": false,
  "AutofillAddressEnabled": false,
  "AutofillCreditCardEnabled": false,
  "HomepageIsNewTabPage": false,
  "HomepageLocation": "https://classroom.google.com",
  "RestoreOnStartup": 4,
  "RestoreOnStartupURLs": [
    "https://classroom.google.com"
  ],
  "ClearBrowsingDataOnExitList": [
    "cookies_and_other_site_data",
    "password_signin",
    "autofill",
    "hosted_app_data"
  ],
  "BrowsingDataLifetime": [
    {
      "data_types": [
        "browsing_history",
        "download_history"
      ],
      "time_to_live_in_hours": 360
    }
  ],
  "ManagedBookmarks": [
    { "toplevel_name": "Escola" },
    { "name": "Google Docs", "url": "https://docs.google.com" },
    { "name": "Google Planilhas", "url": "https://docs.google.com/spreadsheets" },
    { "name": "Google Apresentações", "url": "https://docs.google.com/presentation" },
    { "name": "Gemini", "url": "https://gemini.google.com" }
  ]
}
JSON

    cat > "$M/favorite_applications" <<'RON'
[
    "tamagi-browser",
    "org.onlyoffice.desktopeditors",
    "org.turbowarp.TurboWarp",
    "org.gnome.Papers",
    "tamagi-skl-docs",
    "tamagi-skl-sheets",
    "tamagi-skl-slides",
    "tamagi-skl-gemini",
    "com.system76.CosmicFiles",
]
RON
    cat > "$C/com.system76.CosmicAppList/v1/favorites" <<'RON'
[
    "tamagi-browser",
    "org.onlyoffice.desktopeditors",
    "org.turbowarp.TurboWarp",
    "org.gnome.Papers",
    "com.system76.CosmicFiles",
]
RON
else
    # Tamagi-DxAI: WebApp Lark Suite pré-configurada (100% compatível com GHCR
    # público; o instalador nativo sob demanda também está em `tamagi install lark`).
    cat > /usr/share/applications/tamagi-lark.desktop <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Lark Suite
Comment=Lark Suite workspace (WebApp; native RPM via tamagi install lark)
Exec=/usr/bin/tamagi-browser app --app=https://www.larksuite.com --class=tamagi-lark
Icon=internet-chat
Terminal=false
Categories=Office;Network;InstantMessaging;
StartupWMClass=tamagi-lark
SingleMainWindow=true
DESKTOP
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
fi

# ── Atalhos do Tamagi por cima dos padrões do COSMIC ──
python3 - "$C/com.system76.CosmicSettings.Shortcuts/v1/defaults" <<'PY'
import re
import sys

path = sys.argv[1]
popup = "/usr/libexec/tamagi/popup"
ours = [
    ("[Super]", "Return", "System(Terminal)"),
    ("[Super]", "w", "Close"),
    ("[Super]", "space", 'Spawn("/usr/libexec/tamagi/start-menu")'),
    ("[Super, Ctrl]", "space", "System(InputSourceSwitch)"),
    ("[Super]", "b", "System(WebBrowser)"),
    ("[Super]", "e", "System(HomeFolder)"),
    ("[Super]", "f", "Fullscreen"),
    ("[Super]", "t", "ToggleWindowFloating"),
    ("[Super]", "a", 'Spawn("tamagi-menu")'),
    ("[Super, Alt]", "space", "System(Launcher)"),
    ("[Super, Shift]", "a", f'Spawn("{popup} tamagi-agent")'),
    ("[Super]", "k", f'Spawn("{popup} tamagi-keys")'),
    ("[Super, Shift]", "t", f'Spawn("{popup} btm")'),
    ("[Super, Shift]", "n", f'Spawn("{popup} --tile /usr/libexec/tamagi/dev-tool nvim")'),
    ("[Shift]", "Print", 'Spawn("tamagi-capture text")'),
    ("[Alt]", "Print", 'Spawn("tamagi-capture record")'),
    ("[Super, Shift]", "g", f'Spawn("{popup} --tile /usr/libexec/tamagi/dev-tool lazygit")'),
    ("[Super, Shift]", "d", f'Spawn("{popup} tamagi-db")'),
    ("[Super, Shift]", "o", 'Spawn("/usr/libexec/tamagi/open-or-install org.onlyoffice.desktopeditors")'),
    ("[Super, Shift]", "m", 'Spawn("/usr/libexec/tamagi/open-or-install io.missioncenter.MissionCenter")'),
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

install -Dm644 /dev/stdin /etc/xdg/xdg-terminals.list <<'LIST'
org.gnome.Ptyxis.desktop
com.system76.CosmicTerm.desktop
LIST

if [[ -f /etc/chromium/chromium.conf ]] && ! grep -q '/etc/chromium/tamagi.conf' /etc/chromium/chromium.conf; then
    printf '\n# Tamagi OS defaults (Wayland/PipeWire, hardware decode, edition-aware OAuth2)\n[[ -f /etc/chromium/tamagi.conf ]] && . /etc/chromium/tamagi.conf\n' >> /etc/chromium/chromium.conf
fi
bash -n /etc/chromium/chromium.conf /etc/chromium/tamagi.conf

gtk-update-icon-cache -f /usr/share/icons/hicolor 2>/dev/null || true

for f in /usr/share/applications/tamagi-*.desktop /usr/share/applications/org.tamagi.Menu.desktop \
         /usr/share/tamagi/desktop/*.desktop \
         /etc/xdg/autostart/tamagi-*.desktop; do
    [[ -f $f ]] || continue
    desktop-file-validate "$f"
done
