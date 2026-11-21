#!/usr/bin/python3
"""Gera o tema do Plymouth (tela de boot) do Tamagi.

O símbolo é o brasão do Tamagi, o mesmo do menu iniciar e dos papéis de parede
"mon": um anel (環, tamaki) com três joias em forma de gota (玉, tama, a hōju).
Tama + ki: o nome do sistema desenhado. Cores do tema Ipê (o tema próprio do
Tamagi): o amarelo do ipê sobre o verde da noite.

Durante o boot as três joias giram devagar dentro do anel e a pedra do centro
pulsa; embaixo, "Tamagi" em Roboto Slab (a fonte com serifa do sistema).
Desenho por geometria, sem arquivo de terceiros: saem SVGs, o rsvg-convert
transforma em PNG (o Plymouth não lê SVG).

  make-plymouth.py <pasta-do-tema>
"""
import math
import pathlib
import subprocess
import sys

# Tema Ipê (system_files/usr/share/tamagi/themes/ipe.conf)
DEEP, BASE, GOLD, GREEN, TEAL, TEXT = "#101a15", "#16211c", "#f2c14e", "#8fc98a", "#6fc3b3", "#e8e3d0"
SIZE = 192          # lado do brasão animado, em pixels (o Plymouth escala em telas HiDPI)
FRAMES = 60         # 120° de giro (as três joias se repetem a cada 120°): volta sem emenda


def jewel(cx, cy, s, angle):
    """Hōju: gota com a ponta para fora, girada em torno do centro (como em make-wallpapers.py)."""
    d = (f"M0,{-1.25 * s:.2f} C{0.22 * s:.2f},{-0.8 * s:.2f} {s:.2f},{-0.55 * s:.2f} {s:.2f},{0.1 * s:.2f} "
         f"A{s:.2f},{s:.2f} 0 1 1 {-s:.2f},{0.1 * s:.2f} "
         f"C{-s:.2f},{-0.55 * s:.2f} {-0.22 * s:.2f},{-0.8 * s:.2f} 0,{-1.25 * s:.2f} Z")
    return f'<path d="{d}" transform="rotate({angle:.2f} {cx} {cy}) translate({cx} {cy - 1.55 * s:.2f})"/>'


def crest(frame):
    c = SIZE / 2
    radius = SIZE * 0.46
    ring = radius * 0.13
    js = radius * 0.27
    turn = 120.0 * frame / FRAMES
    # A pedra do centro "respira": um ciclo por volta das joias.
    pulse = 0.5 - 0.5 * math.cos(2 * math.pi * frame / FRAMES)
    glow = 0.18 + 0.32 * pulse
    gem = js * (0.40 + 0.06 * pulse)
    jewels = "".join(jewel(c, c, js, turn + a) for a in (0, 120, 240))
    return f'''<svg xmlns="http://www.w3.org/2000/svg" width="{SIZE}" height="{SIZE}" viewBox="0 0 {SIZE} {SIZE}">
  <defs>
    <radialGradient id="g"><stop offset="0" stop-color="{TEAL}" stop-opacity="{glow:.3f}"/>
      <stop offset="1" stop-color="{TEAL}" stop-opacity="0"/></radialGradient>
  </defs>
  <circle cx="{c}" cy="{c}" r="{radius * 0.55:.2f}" fill="url(#g)"/>
  <circle cx="{c}" cy="{c}" r="{radius - ring / 2:.2f}" fill="none" stroke="{GOLD}" stroke-width="{ring:.2f}"/>
  <circle cx="{c}" cy="{c}" r="{radius - ring * 1.75:.2f}" fill="none" stroke="{GOLD}" stroke-opacity="0.55" stroke-width="{ring * 0.22:.2f}"/>
  <g fill="{GOLD}">{jewels}</g>
  <circle cx="{c}" cy="{c}" r="{gem:.2f}" fill="{TEAL}"/>
</svg>
'''


def wordmark():
    w, h = 420, 96
    return f'''<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
  <text x="{w / 2}" y="58" text-anchor="middle" font-family="Roboto Slab" font-weight="300"
        font-size="44" letter-spacing="10" fill="{TEXT}">Tamagi</text>
  <rect x="{w / 2 - 28}" y="76" width="56" height="3" rx="1.5" fill="{GOLD}" fill-opacity="0.85"/>
</svg>
'''


def render(svg_text, png):
    svg = png.with_suffix(".svg")
    svg.write_text(svg_text, encoding="utf-8")
    subprocess.run(["rsvg-convert", str(svg), "-o", str(png)], check=True)
    svg.unlink()


def main(out):
    out = pathlib.Path(out)
    out.mkdir(parents=True, exist_ok=True)
    for i in range(FRAMES):
        render(crest(i), out / f"throbber-{i + 1:04d}.png")
    render(crest(0), out / "logo.png")          # quadro parado (senha do disco, fim do boot)
    render(wordmark(), out / "watermark.png")
    print(f"tema do Plymouth: {FRAMES} quadros do brasão + marca em {out}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    main(sys.argv[1])
