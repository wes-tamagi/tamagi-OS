#!/usr/bin/python3
"""Illustrated wallpapers of Tamagi
(system_files/usr/share/backgrounds/tamagi/art; COSMIC lists subfolders). Run on a development machine;
the results are committed, the image build only copies them.

    tools/art-wallpapers.py split <sheet.jpeg> <name1> <name2> <name3> <name4>
        cuts a 2x2 contact sheet into four 16:9 wallpapers (2560x1440)
    tools/art-wallpapers.py crests
        draws the crest wallpapers that have no sheet (same language as the
        "mon" style of build_files/make-wallpapers.py: crest over seigaiha
        waves, the name and one kanji). Needs rsvg-convert and a CJK font.
"""
import importlib.util
import pathlib
import subprocess
import sys
import tempfile

from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parents[1]
OUT = ROOT / "system_files/usr/share/backgrounds/tamagi/art"
SIZE = (2560, 1440)
GUTTER = 8          # the sheets have a dark line between the pictures

# name: (deep, base, crest/letters, second, centre jewel), kanji
CRESTS = {
    "jin":    (("#170d29", "#3b2163", "#dccdf5", "#8a63c9", "#f2c14e"), "仁"),   # benevolence — wisteria
    "makoto": (("#051b1a", "#0e4744", "#e6cb80", "#2f8f88", "#f6eed6"), "誠"),   # sincerity — pine and gold
}


def save(im, name):
    OUT.mkdir(parents=True, exist_ok=True)
    path = OUT / f"{name}.jpg"
    im.convert("RGB").save(path, "JPEG", quality=90, optimize=True, progressive=True)
    print(path.relative_to(ROOT), im.size, f"{path.stat().st_size // 1024} KiB")


def split(sheet, names):
    im = Image.open(sheet)
    w, h = im.size[0] // 2, im.size[1] // 2
    for i, name in enumerate(names):
        x0, y0 = (i % 2) * w + GUTTER, (i // 2) * h + GUTTER
        cell = im.crop((x0, y0, x0 + w - 2 * GUTTER, y0 + h - 2 * GUTTER))
        cw, ch = cell.size
        want = round(ch * SIZE[0] / SIZE[1])                  # centre crop to 16:9
        if want <= cw:
            cell = cell.crop(((cw - want) // 2, 0, (cw - want) // 2 + want, ch))
        else:
            want = round(cw * SIZE[1] / SIZE[0])
            cell = cell.crop((0, (ch - want) // 2, cw, (ch - want) // 2 + want))
        save(cell.resize(SIZE, Image.LANCZOS), name)


def crests():
    spec = importlib.util.spec_from_file_location("walls", ROOT / "build_files/make-wallpapers.py")
    walls = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(walls)
    W, H = walls.W, walls.H
    for name, (colors, kanji) in CRESTS.items():
        letters = colors[2]
        svg = (walls.head(colors)
               + f'  <g mask="url(#mfade)">\n{walls.seigaiha_group(colors, 150, H * 0.40, H, 0.60)}\n  </g>\n'
               + walls.crest(colors, W // 2, int(H * 0.34), int(H * 0.19)) + "\n"
               + f'  <text x="{W // 2}" y="{int(H * 0.665)}" text-anchor="middle" font-family="Noto Sans, sans-serif" '
                 f'font-weight="700" font-size="{int(H * 0.085)}" letter-spacing="{int(H * 0.012)}" fill="{letters}">TAMAGI</text>\n'
               + f'  <text x="{W // 2}" y="{int(H * 0.84)}" text-anchor="middle" font-family="Noto Serif CJK JP, Noto Sans CJK JP, serif" '
                 f'font-size="{int(H * 0.13)}" fill="{letters}">{kanji}</text>\n</svg>\n')
        with tempfile.TemporaryDirectory() as tmp:
            src, png = pathlib.Path(tmp) / "w.svg", pathlib.Path(tmp) / "w.png"
            src.write_text(svg, encoding="utf-8")
            subprocess.run(["rsvg-convert", "-w", str(SIZE[0]), "-h", str(SIZE[1]), str(src), "-o", str(png)], check=True)
            save(Image.open(png), name)


if __name__ == "__main__":
    if len(sys.argv) == 7 and sys.argv[1] == "split":
        split(sys.argv[2], sys.argv[3:7])
    elif sys.argv[1:] == ["crests"]:
        crests()
    else:
        sys.exit(__doc__)
