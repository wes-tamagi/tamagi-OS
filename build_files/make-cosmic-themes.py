#!/usr/bin/python3
"""Gera um tema do COSMIC (.ron) para cada tema do Tamagi.

    make-cosmic-themes.py <pasta dos .conf> <pasta de saída> [modelo-escuro.ron modelo-claro.ron]

O tema do COSMIC é o arquivo que "Aparência > Importar" (ou
`cosmic-settings appearance import`) lê. A paleta-base, os espaçamentos e os
cantos vêm de um tema que o próprio COSMIC distribui (o modelo); do tema do
Tamagi entram o fundo (BG), o destaque (WALL[2]), o tom do texto (FG) e as
cores de sucesso, aviso e erro (verde, amarelo e vermelho da PALETTE).

Os cantos seguem a escala do Material 3 Expressive (docs/estilo-m3.md):
4 px (extra pequeno), 8 px (pequeno), 12 px (médio), 28 px (extra grande,
diálogos) e pílula nos botões (o radius_xl do COSMIC)."""
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
TEMPLATES = {"dark": "/usr/share/cosmic-themes/nebula-dark.ron", "light": "/usr/share/cosmic-themes/comet-light.ron"}
FALLBACK_TEMPLATES = {"dark": str(HERE / "templates/nebula-dark.ron"), "light": str(HERE / "templates/comet-light.ron")}


def conf(path):
    text = path.read_text(encoding="utf-8")
    one = lambda key: re.search(rf'^{key}="?([^"\n]+)"?$', text, re.M).group(1)
    many = lambda key: re.findall(r"#[0-9a-fA-F]{6}", re.search(rf"^{key}=\((.*)\)$", text, re.M).group(1))
    return {"name": one("NAME"), "scheme": one("SCHEME"), "bg": one("BG"), "fg": one("FG"),
            "palette": many("PALETTE"), "wall": many("WALL")}


def rgb(color, alpha=False):
    r, g, b = (int(color[i:i + 2], 16) / 255 for i in (1, 3, 5))
    body = f"\n        red: {r:.7f},\n        green: {g:.7f},\n        blue: {b:.7f},\n"
    if alpha:
        body += "        alpha: 1.0,\n"
    return f"Some(({body}    ))"


def replace(text, key, value):
    """Troca o valor de uma chave de primeiro nível (None ou Some((...)))."""
    pattern = re.compile(rf"^    {key}: (?:None|Some\(\(.*?^    \)\)),$", re.M | re.S)
    assert len(pattern.findall(text)) == 1, f"o modelo mudou de formato: {key}"
    return pattern.sub(f"    {key}: {value},", text)


# radius_0, xs, s, m, l, xl do COSMIC <- None, ExtraSmall, Small, Medium,
# ExtraLarge e Full do Material 3.
M3_RADII = {"radius_0": 0, "radius_xs": 4, "radius_s": 8, "radius_m": 12, "radius_l": 28, "radius_xl": 160}


def corner_radii(text):
    for key, value in M3_RADII.items():
        pattern = re.compile(rf"^        {key}: \([^)]*\),$", re.M)
        assert len(pattern.findall(text)) == 1, f"o modelo mudou de formato: {key}"
        v = f"{value:.1f}"
        text = pattern.sub(f"        {key}: ({v}, {v}, {v}, {v}),", text)
    return text


def main():
    themes, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    templates = dict(TEMPLATES)
    if len(sys.argv) > 4:
        templates = {"dark": sys.argv[3], "light": sys.argv[4]}
    else:
        for scheme, p in list(templates.items()):
            if not pathlib.Path(p).is_file():
                templates[scheme] = FALLBACK_TEMPLATES[scheme]
    out.mkdir(parents=True, exist_ok=True)
    count = 0
    for path in sorted(themes.glob("*.conf")):
        t = conf(path)
        text = pathlib.Path(templates[t["scheme"]]).read_text(encoding="utf-8")
        text = replace(text, "bg_color", rgb(t["bg"], alpha=True))
        text = replace(text, "accent", rgb(t["wall"][2]))
        text = replace(text, "neutral_tint", rgb(t["palette"][8]))
        text = replace(text, "text_tint", rgb(t["fg"]))
        text = replace(text, "success", rgb(t["palette"][2]))
        text = replace(text, "warning", rgb(t["palette"][3]))
        text = replace(text, "destructive", rgb(t["palette"][1]))
        text = corner_radii(text)
        (out / f"tamagi-{path.stem}.ron").write_text(text, encoding="utf-8")
        count += 1
    print(f"temas do COSMIC gerados: {count} em {out}")


if __name__ == "__main__":
    main()
