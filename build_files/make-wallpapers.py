#!/usr/bin/python3
"""Gera os papéis de parede SVG de cada tema a partir das cores do .conf:
o estilo "ondas" e sete estilos do Japão do século XVII (veja STYLES).

Uso: make-wallpapers.py <dir-temas> <dir-saida> <xml-gnome>
Arte própria do Tamagi: faixas onduladas em degradê, sem imagens de terceiros.
"""
import math
import pathlib
import re
import sys

W, H = 3840, 2160


def read_theme(path):
    text = path.read_text(encoding="utf-8")
    name = re.search(r'^NAME="(.*)"', text, re.M).group(1)
    wall = re.search(r"^WALL=\((.*)\)", text, re.M).group(1).replace('"', "").split()
    if len(wall) != 5 or not all(re.fullmatch(r"#[0-9a-fA-F]{6}", c) for c in wall):
        raise SystemExit(f"{path}: WALL precisa de 5 cores #rrggbb")
    return name, wall


def wave(y0, amp, period, phase):
    """Caminho fechado: onda senoidal em y0 descendo até a base."""
    pts = []
    for x in range(0, W + 1, 96):
        y = y0 + amp * math.sin(2 * math.pi * x / period + phase) \
            + amp * 0.35 * math.sin(2 * math.pi * x / (period * 0.37) + phase * 2.1)
        pts.append(f"{x},{y:.0f}")
    return "M0,%d L%s L%d,%d Z" % (H, " L".join(pts), W, H)


def svg(colors):
    deep, base, c1, c2, c3 = colors
    layers = [
        (c3, 0.16, wave(H * 0.50, 150, 3100, 0.4)),
        (c2, 0.20, wave(H * 0.62, 130, 2500, 2.1)),
        (c1, 0.26, wave(H * 0.74, 110, 2900, 4.0)),
        (deep, 0.55, wave(H * 0.86, 80, 2100, 1.2)),
    ]
    body = "\n".join(
        f'  <path d="{d}" fill="{c}" fill-opacity="{o}"/>' for c, o, d in layers)
    return f'''<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">
  <defs>
    <linearGradient id="sky" x1="0" y1="0" x2="0.35" y2="1">
      <stop offset="0" stop-color="{base}"/>
      <stop offset="1" stop-color="{deep}"/>
    </linearGradient>
    <radialGradient id="glow" cx="0.74" cy="0.30" r="0.55">
      <stop offset="0" stop-color="{c1}" stop-opacity="0.34"/>
      <stop offset="0.45" stop-color="{c2}" stop-opacity="0.10"/>
      <stop offset="1" stop-color="{base}" stop-opacity="0"/>
    </radialGradient>
  </defs>
  <rect width="{W}" height="{H}" fill="url(#sky)"/>
  <rect width="{W}" height="{H}" fill="url(#glow)"/>
{body}
</svg>
'''


# ── Estilos do Japão do século XVII (início do período Edo) ─────────────────
# Arte própria, desenhada por geometria: nenhum arquivo de terceiros.
#   mon       brasão no estilo dos kamon: um anel (環, "tamaki") com três
#             joias (玉, "tama", a hōju em forma de gota) e ondas seigaiha
#   seigaiha  青海波, "ondas do mar azul": escamas de arcos concêntricos
#   asanoha   麻の葉, folha de cânhamo: estrelas de seis pontas em malha
#   byobu     屏風, biombo: faixas de nuvem (suyari-gasumi), lua e monte

def head(colors):
    deep, base, c1, c2, c3 = colors
    return f'''<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{W}" height="{H}" viewBox="0 0 {W} {H}">
  <defs>
    <linearGradient id="sky" x1="0" y1="0" x2="0.35" y2="1">
      <stop offset="0" stop-color="{base}"/>
      <stop offset="1" stop-color="{deep}"/>
    </linearGradient>
    <radialGradient id="glow" cx="0.72" cy="0.42" r="0.6">
      <stop offset="0" stop-color="{c1}" stop-opacity="0.30"/>
      <stop offset="0.5" stop-color="{c2}" stop-opacity="0.08"/>
      <stop offset="1" stop-color="{base}" stop-opacity="0"/>
    </radialGradient>
    <linearGradient id="fade" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#fff" stop-opacity="0"/>
      <stop offset="1" stop-color="#fff" stop-opacity="1"/>
    </linearGradient>
    <linearGradient id="fadeside" x1="0" y1="0" x2="1" y2="0.4">
      <stop offset="0" stop-color="#fff" stop-opacity="1"/>
      <stop offset="0.75" stop-color="#fff" stop-opacity="0.15"/>
    </linearGradient>
    <mask id="mfade"><rect width="{W}" height="{H}" fill="url(#fade)"/></mask>
    <mask id="mside"><rect width="{W}" height="{H}" fill="url(#fadeside)"/></mask>
  </defs>
  <rect width="{W}" height="{H}" fill="url(#sky)"/>
  <rect width="{W}" height="{H}" fill="url(#glow)"/>
'''


def seigaiha_group(colors, r, y0, y1, opacity):
    """Escamas: cada uma são arcos concêntricos; as fileiras de baixo cobrem as de cima."""
    deep, base, c1, c2, c3 = colors
    rings = "".join(
        f'<circle r="{r * k:.1f}" fill="{deep}" stroke="{c}" stroke-width="{r * 0.055:.1f}"/>'
        for k, c in ((1.0, c1), (0.76, c2), (0.52, c1), (0.28, c3)))
    out = [f'  <defs><g id="scale">{rings}</g></defs>', f'  <g opacity="{opacity}">']
    row = 0
    y = y0
    while y < y1 + r:
        x = -r if row % 2 else 0
        while x < W + r:
            out.append(f'<use xlink:href="#scale" x="{x:.0f}" y="{y:.0f}"/>')
            x += 2 * r
        y += r / 2
        row += 1
    out.append("  </g>")
    return "\n".join(out)


def jewel(cx, cy, size, angle):
    """Hōju (joia): gota com a ponta para fora, girada em torno do centro do brasão."""
    s = size
    d = (f"M0,{-1.25 * s:.1f} C{0.22 * s:.1f},{-0.8 * s:.1f} {s:.1f},{-0.55 * s:.1f} {s:.1f},{0.1 * s:.1f} "
         f"A{s:.1f},{s:.1f} 0 1 1 {-s:.1f},{0.1 * s:.1f} "
         f"C{-s:.1f},{-0.55 * s:.1f} {-0.22 * s:.1f},{-0.8 * s:.1f} 0,{-1.25 * s:.1f} Z")
    return f'<path d="{d}" transform="rotate({angle} {cx} {cy}) translate({cx} {cy - 1.55 * s:.1f})"/>'


def crest(colors, cx, cy, radius, opacity=1.0):
    deep, base, c1, c2, c3 = colors
    ring = radius * 0.13
    js = radius * 0.27
    jewels = "".join(jewel(cx, cy, js, a) for a in (0, 120, 240))
    return f'''  <g opacity="{opacity}">
    <circle cx="{cx}" cy="{cy}" r="{radius}" fill="{deep}" fill-opacity="0.55"/>
    <circle cx="{cx}" cy="{cy}" r="{radius - ring / 2:.1f}" fill="none" stroke="{c1}" stroke-width="{ring:.1f}"/>
    <circle cx="{cx}" cy="{cy}" r="{radius - ring * 1.75:.1f}" fill="none" stroke="{c1}" stroke-width="{ring * 0.22:.1f}"/>
    <g fill="{c1}">{jewels}</g>
    <circle cx="{cx}" cy="{cy}" r="{js * 0.42:.1f}" fill="{c3}"/>
  </g>'''


def svg_mon(colors):
    return (head(colors)
            + f'  <g mask="url(#mfade)">\n{seigaiha_group(colors, 150, H * 0.52, H, 0.55)}\n  </g>\n'
            + crest(colors, int(W * 0.72), int(H * 0.42), 430) + "\n</svg>\n")


def svg_seigaiha(colors):
    return (head(colors)
            + f'  <g mask="url(#mside)">\n{seigaiha_group(colors, 170, 0, H, 0.7)}\n  </g>\n'
            + crest(colors, int(W * 0.82), int(H * 0.26), 190, 0.9) + "\n</svg>\n")


def svg_asanoha(colors):
    deep, base, c1, c2, c3 = colors
    a = 320.0
    h = a * math.sqrt(3) / 2

    def point(i, j):
        return (i * a + (j % 2) * a / 2, j * h)

    segs = []
    for j in range(-1, int(H / h) + 2):
        for i in range(-1, int(W / a) + 3):
            first = point(i, j)
            for tri in ((first, point(i + 1, j), point(i + (j % 2), j + 1)),
                        (first, point(i - 1 + (j % 2), j + 1), point(i + (j % 2), j + 1))):
                gx = sum(p[0] for p in tri) / 3
                gy = sum(p[1] for p in tri) / 3
                for k in range(3):
                    x1, y1 = tri[k]
                    x2, y2 = tri[(k + 1) % 3]
                    segs.append(f"M{x1:.0f},{y1:.0f}L{x2:.0f},{y2:.0f}")
                    segs.append(f"M{x1:.0f},{y1:.0f}L{gx:.0f},{gy:.0f}")
    lines = f'<path d="{"".join(segs)}" fill="none" stroke="{c1}" stroke-width="5" stroke-linecap="round"/>'
    return (head(colors) + f'  <g mask="url(#mside)" opacity="0.55">{lines}</g>\n'
            + crest(colors, int(W * 0.80), int(H * 0.70), 240, 0.95) + "\n</svg>\n")


def svg_byobu(colors):
    deep, base, c1, c2, c3 = colors

    def cloud(x, y, w, hh, color, op):
        return f'<rect x="{x}" y="{y}" width="{w}" height="{hh}" rx="{hh / 2}" fill="{color}" fill-opacity="{op}"/>'

    bands = [
        cloud(-200, 250, 1500, 110, c3, 0.30), cloud(300, 330, 1700, 90, c3, 0.22),
        cloud(2300, 180, 1900, 120, c2, 0.26), cloud(2700, 275, 1500, 80, c2, 0.20),
        cloud(-300, 1120, 2100, 130, c2, 0.24), cloud(600, 1220, 1500, 90, c3, 0.20),
        cloud(1900, 1380, 2300, 140, c3, 0.26), cloud(2500, 1490, 1600, 90, c1, 0.18),
        cloud(-200, 1840, 2600, 150, c1, 0.22), cloud(1500, 1950, 2700, 130, c2, 0.24),
    ]
    mx, my = W * 0.36, H * 0.86          # monte com o cume nevado
    mount = (f'<path d="M{mx - 1500:.0f},{H} C{mx - 700:.0f},{my - 250:.0f} {mx - 330:.0f},{my - 760:.0f} {mx - 150:.0f},{my - 900:.0f} '
             f'L{mx + 150:.0f},{my - 900:.0f} C{mx + 330:.0f},{my - 760:.0f} {mx + 700:.0f},{my - 250:.0f} {mx + 1500:.0f},{H} Z" fill="{c2}" fill-opacity="0.30"/>'
             f'<path d="M{mx - 150:.0f},{my - 900:.0f} L{mx + 150:.0f},{my - 900:.0f} C{mx + 230:.0f},{my - 840:.0f} {mx + 300:.0f},{my - 760:.0f} {mx + 340:.0f},{my - 690:.0f} '
             f'L{mx + 210:.0f},{my - 760:.0f} L{mx + 110:.0f},{my - 680:.0f} L{mx:.0f},{my - 770:.0f} L{mx - 120:.0f},{my - 680:.0f} L{mx - 220:.0f},{my - 760:.0f} L{mx - 340:.0f},{my - 690:.0f} '
             f'C{mx - 300:.0f},{my - 760:.0f} {mx - 230:.0f},{my - 840:.0f} {mx - 150:.0f},{my - 900:.0f} Z" fill="{c3}" fill-opacity="0.75"/>')
    moon = f'<circle cx="{W * 0.72:.0f}" cy="{H * 0.34:.0f}" r="300" fill="{c1}" fill-opacity="0.85"/>'
    return (head(colors) + f"  {moon}\n  {mount}\n  " + "\n  ".join(bands) + "\n"
            + crest(colors, int(W * 0.72), int(H * 0.34), 150, 0.55) + "\n</svg>\n")


def svg_shippo(colors):
    """Shippō (sete tesouros): círculos que se cruzam formando pétalas."""
    deep, base, c1, c2, c3 = colors
    r = 190
    circles = []
    row = 0
    y = 0
    while y < H + r:
        x = r if row % 2 else 0
        while x < W + r:
            circles.append(f'<circle cx="{x}" cy="{y}" r="{r}"/>')
            x += 2 * r
        y += r
        row += 1
    dots = "".join(f'<circle cx="{x}" cy="{y}" r="{r * 0.09:.0f}"/>'
                   for y in range(0, H + r, 2 * r) for x in range(0, W + r, 2 * r))
    return (head(colors)
            + f'  <g mask="url(#mside)" opacity="0.6">'
              f'<g fill="none" stroke="{c1}" stroke-width="6">{"".join(circles)}</g>'
              f'<g fill="{c3}">{dots}</g></g>\n'
            + crest(colors, int(W * 0.78), int(H * 0.66), 260, 0.95) + "\n</svg>\n")


def svg_hinode(colors):
    """Hinode (nascer do sol): raios saindo do horizonte, sobre ondas."""
    deep, base, c1, c2, c3 = colors
    cx, cy = W * 0.5, H * 0.78
    rays = []
    n = 28
    for i in range(n):
        if i % 2:
            continue
        a0 = math.pi + math.pi * i / n
        a1 = math.pi + math.pi * (i + 1) / n
        rr = W * 0.9
        rays.append(f'<path d="M{cx:.0f},{cy:.0f} L{cx + rr * math.cos(a0):.0f},{cy + rr * math.sin(a0):.0f} '
                    f'L{cx + rr * math.cos(a1):.0f},{cy + rr * math.sin(a1):.0f} Z"/>')
    sun = (f'<circle cx="{cx:.0f}" cy="{cy:.0f}" r="520" fill="{c1}" fill-opacity="0.9"/>'
           f'<circle cx="{cx:.0f}" cy="{cy:.0f}" r="600" fill="none" stroke="{c3}" stroke-width="10" stroke-opacity="0.6"/>')
    sea = "".join(f'<path d="{wave(H * y, amp, per, ph)}" fill="{c}" fill-opacity="{o}"/>'
                  for c, o, y, amp, per, ph in ((c2, 0.55, 0.76, 60, 2300, 0.6), (deep, 0.75, 0.84, 50, 1900, 2.4),
                                                (deep, 0.95, 0.92, 40, 2600, 4.1)))
    return (head(colors) + f'  <g fill="{c2}" fill-opacity="0.16">{"".join(rays)}</g>\n  {sun}\n  {sea}\n'
            + crest(colors, int(cx), int(cy - 150), 170, 0.6) + "\n</svg>\n")


def svg_uroko(colors):
    """Uroko (escamas): triângulos alternados, como em quimonos de teatro."""
    deep, base, c1, c2, c3 = colors
    a = 260.0
    h = a * math.sqrt(3) / 2
    tris = {c1: [], c2: [], c3: []}
    order = (c1, c2, c1, c3)
    for j in range(int(H / h) + 2):
        for i in range(-1, int(W / a) + 2):
            x = i * a + (j % 2) * a / 2
            y = j * h
            tris[order[(i + 2 * j) % 4]].append(f"M{x:.0f},{y + h:.0f}L{x + a / 2:.0f},{y:.0f}L{x + a:.0f},{y + h:.0f}Z")
    body = "".join(f'<path d="{"".join(d)}" fill="{c}" fill-opacity="0.34"/>' for c, d in tris.items())
    return (head(colors) + f'  <g mask="url(#mfade)">{body}</g>\n'
            + crest(colors, int(W * 0.5), int(H * 0.36), 330) + "\n</svg>\n")


STYLES = {"mon": svg_mon, "seigaiha": svg_seigaiha, "asanoha": svg_asanoha, "byobu": svg_byobu,
          "shippo": svg_shippo, "hinode": svg_hinode, "uroko": svg_uroko}


def main():
    themes, out, xml = (pathlib.Path(p) for p in sys.argv[1:4])
    out.mkdir(parents=True, exist_ok=True)
    entries = []
    for conf in sorted(themes.glob("*.conf")):
        name, wall = read_theme(conf)
        target = out / f"{conf.stem}.svg"
        target.write_text(svg(wall), encoding="utf-8")       # estilo "ondas"
        for style, draw in STYLES.items():
            (out / f"{conf.stem}-{style}.svg").write_text(draw(wall), encoding="utf-8")
        entries.append(f'''  <wallpaper deleted="false">
    <name>Tamagi {name}</name>
    <filename>{target}</filename>
    <filename-dark>{target}</filename-dark>
    <options>zoom</options>
    <shade_type>solid</shade_type>
    <pcolor>{wall[1]}</pcolor>
  </wallpaper>''')
    xml.parent.mkdir(parents=True, exist_ok=True)
    xml.write_text('<?xml version="1.0"?>\n<!DOCTYPE wallpapers SYSTEM "gnome-wp-list.dtd">\n<wallpapers>\n'
                   + "\n".join(entries) + "\n</wallpapers>\n", encoding="utf-8")


if __name__ == "__main__":
    main()
