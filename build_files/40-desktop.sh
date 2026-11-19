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

# ── Papéis de parede: cinco estilos por tema, gerados das cores do .conf ──
# (o brasão "mon" é o padrão; tamagi-theme wallpaper troca o estilo)
python3 /ctx/build_files/make-wallpapers.py /usr/share/tamagi/themes \
    /usr/share/backgrounds/tamagi /usr/share/tamagi/wallpapers.xml
# O cosmic-bg recebe PNG.
for svg in /usr/share/backgrounds/tamagi/*.svg; do
    rsvg-convert -w 3840 "$svg" -o "${svg%.svg}.png"
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
# o popup usa o alacritty com a classe "tamagi-popup".
rules="$C/com.system76.CosmicSettings.WindowRules/v1/tiling_exception_defaults"
python3 - "$rules" <<'PY'
import sys
path = sys.argv[1]
text = open(path, encoding="utf-8").read().rstrip()
assert text.endswith("]"), "tiling_exception_defaults mudou de formato"
text = text[:-1].rstrip() + '\n\t// Tamagi\n\t(appid: "tamagi-popup", titles: [".*"]),\n\t(appid: "zenity", titles: [".*"]),\n]\n'
open(path, "w", encoding="utf-8").write(text)
PY

# Paleta padrão das janelas do Tamagi (o tamagi-theme gera a do usuário).
tamagi-theme alacritty tokyo-night > /usr/share/tamagi/alacritty.toml

# ── Dock: o navegador é o Chrome da caixa (google-chrome.desktop do Tamagi) ──
cat > "$C/com.system76.CosmicAppList/v1/favorites" <<'RON'
[
    "google-chrome",
    "com.system76.CosmicFiles",
    "com.system76.CosmicTerm",
    "com.system76.CosmicEdit",
    "tamagi-agent",
    "com.system76.CosmicStore",
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
    ("[Super]", "space", "System(Launcher)"),
    ("[Super, Ctrl]", "space", "System(InputSourceSwitch)"),
    ("[Super]", "b", "System(WebBrowser)"),
    ("[Super]", "e", "System(HomeFolder)"),
    ("[Super]", "f", "Fullscreen"),
    ("[Super]", "t", "ToggleWindowFloating"),
    ("[Super]", "a", f'Spawn("{popup} tamagi-assist")'),
    ("[Super, Alt]", "space", f'Spawn("{popup} tamagi-menu")'),
    ("[Super, Shift]", "a", f'Spawn("{popup} tamagi-agent")'),
    ("[Super]", "k", f'Spawn("{popup} tamagi-keys")'),
    ("[Super, Shift]", "t", f'Spawn("{popup} btop")'),
    ("[Super, Shift]", "n", f'Spawn("{popup} --tile nvim")'),
    ("[Super, Shift]", "c", 'Spawn("omacalc")'),
    ("[Super, Shift]", "w", 'Spawn("omawrite")'),
    ("[Super, Shift, Ctrl]", "space", 'Spawn("tamagi-theme next")'),
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
# o alacritty aceita comando; o cosmic-term continua sendo o do Super+Enter.
install -Dm644 /dev/stdin /etc/xdg/xdg-terminals.list <<'LIST'
Alacritty.desktop
com.system76.CosmicTerm.desktop
LIST

gtk-update-icon-cache -f /usr/share/icons/hicolor 2>/dev/null || true

for f in /usr/share/applications/tamagi-*.desktop /usr/share/applications/google-chrome.desktop \
         /etc/xdg/autostart/tamagi-*.desktop; do
    desktop-file-validate "$f"
done
