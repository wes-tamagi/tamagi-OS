#!/usr/bin/env python3
"""Generates the Tamagi OS 1.7 ("Kazunobu M") 1080p 60fps Promotional Video
(`docs/tamagi-1.7-promo.mp4`) using Cairo vector motion graphics, real UI
screenshots from `output/installer-shots/dark/`, and a procedural warm ambient
synth soundtrack encoded via ffmpeg (H.264 + AAC)."""

import math
import os
import struct
import subprocess
import tempfile
import wave
import cairo
import gi

gi.require_version("GdkPixbuf", "2.0")
from gi.repository import GdkPixbuf  # noqa: E402

W, H = 1920, 1080
FPS = 60
DURATION = 36.0
TOTAL_FRAMES = int(DURATION * FPS)

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
OUT_MP4 = os.path.join(ROOT, "docs", "tamagi-1.7-promo.mp4")

# Tamagi Ipê Dark Material 3 palette (normalized RGB)
BG_TOP = (0x10 / 255, 0x1A / 255, 0x15 / 255)
BG_BOT = (0x0B / 255, 0x12 / 255, 0x0F / 255)
CARD_BG = (0x1B / 255, 0x28 / 255, 0x22 / 255)
CARD_DARK = (0x11 / 255, 0x1A / 255, 0x16 / 255)
GOLD = (0xF2 / 255, 0xC1 / 255, 0x4E / 255)
TEAL = (0x6F / 255, 0xC3 / 255, 0xB3 / 255)
GREEN = (0x8F / 255, 0xC9 / 255, 0x8A / 255)
CREAM = (0xEA / 255, 0xE5 / 255, 0xD3 / 255)
MUTED = (0xA8 / 255, 0xAF / 255, 0xA2 / 255)


def clamp(x, a=0.0, b=1.0):
    return max(a, min(b, x))


def ease_out_cubic(t):
    t = clamp(t)
    return 1.0 - (1.0 - t) ** 3


def ease_in_out(t):
    t = clamp(t)
    return t * t * (3.0 - 2.0 * t)


def load_png_surface(path, target_w=None):
    if not os.path.isfile(path):
        return None
    pb = GdkPixbuf.Pixbuf.new_from_file(path)
    if target_w:
        target_h = int(pb.get_height() * (target_w / pb.get_width()))
        pb = pb.scale_simple(target_w, target_h, GdkPixbuf.InterpType.BILINEAR)
    w, h = pb.get_width(), pb.get_height()
    stride = pb.get_rowstride()
    ch = pb.get_n_channels()
    pixels = pb.get_pixels()
    surf = cairo.ImageSurface(cairo.FORMAT_ARGB32, w, h)
    dst_stride = surf.get_stride()
    buf = bytearray(h * dst_stride)
    for y in range(h):
        src_row = y * stride
        dst_row = y * dst_stride
        for x in range(w):
            si = src_row + x * ch
            di = dst_row + x * 4
            r, g, b = pixels[si], pixels[si + 1], pixels[si + 2]
            a = pixels[si + 3] if ch == 4 else 255
            if a < 255:
                r = (r * a) // 255
                g = (g * a) // 255
                b = (b * a) // 255
            buf[di] = b
            buf[di + 1] = g
            buf[di + 2] = r
            buf[di + 3] = a
    surf.get_data()[:] = buf
    surf.mark_dirty()
    return surf


def rounded_rect(ctx, x, y, w, h, r):
    ctx.new_sub_path()
    ctx.arc(x + w - r, y + r, r, -math.pi / 2, 0)
    ctx.arc(x + w - r, y + h - r, r, 0, math.pi / 2)
    ctx.arc(x + r, y + h - r, r, math.pi / 2, math.pi)
    ctx.arc(x + r, y + r, r, math.pi, 3 * math.pi / 2)
    ctx.close_path()


def draw_background(ctx, t):
    grad = cairo.LinearGradient(0, 0, W * 0.4, H)
    grad.add_color_stop_rgb(0.0, *BG_TOP)
    grad.add_color_stop_rgb(1.0, *BG_BOT)
    ctx.rectangle(0, 0, W, H)
    ctx.set_source(grad)
    ctx.fill()

    # Subtle animated aurora orbs
    ox1 = W * (0.18 + 0.06 * math.sin(t * 0.45))
    oy1 = H * (0.20 + 0.05 * math.cos(t * 0.35))
    g1 = cairo.RadialGradient(ox1, oy1, 10, ox1, oy1, 620)
    g1.add_color_stop_rgba(0, GOLD[0], GOLD[1], GOLD[2], 0.14)
    g1.add_color_stop_rgba(1, GOLD[0], GOLD[1], GOLD[2], 0.0)
    ctx.set_source(g1)
    ctx.rectangle(0, 0, W, H)
    ctx.fill()

    ox2 = W * (0.82 + 0.05 * math.cos(t * 0.4))
    oy2 = H * (0.76 + 0.06 * math.sin(t * 0.3))
    g2 = cairo.RadialGradient(ox2, oy2, 10, ox2, oy2, 660)
    g2.add_color_stop_rgba(0, TEAL[0], TEAL[1], TEAL[2], 0.13)
    g2.add_color_stop_rgba(1, TEAL[0], TEAL[1], TEAL[2], 0.0)
    ctx.set_source(g2)
    ctx.rectangle(0, 0, W, H)
    ctx.fill()

    # Subtle architectural grid
    ctx.set_source_rgba(1, 1, 1, 0.022)
    ctx.set_line_width(1)
    for gx in range(0, W, 80):
        ctx.move_to(gx, 0)
        ctx.line_to(gx, H)
    for gy in range(0, H, 80):
        ctx.move_to(0, gy)
        ctx.line_to(W, gy)
    ctx.stroke()


def draw_crest(ctx, cx, cy, scale, rot_deg, alpha=1.0):
    ctx.save()
    ctx.translate(cx, cy)
    ctx.scale(scale, scale)

    glow = cairo.RadialGradient(0, 0, 5, 0, 0, 95)
    glow.add_color_stop_rgba(0, TEAL[0], TEAL[1], TEAL[2], 0.30 * alpha)
    glow.add_color_stop_rgba(1, TEAL[0], TEAL[1], TEAL[2], 0.0)
    ctx.set_source(glow)
    ctx.arc(0, 0, 95, 0, 2 * math.pi)
    ctx.fill()

    ctx.set_source_rgba(GOLD[0], GOLD[1], GOLD[2], alpha)
    ctx.set_line_width(11.5)
    ctx.arc(0, 0, 82.5, 0, 2 * math.pi)
    ctx.stroke()

    ctx.set_source_rgba(GOLD[0], GOLD[1], GOLD[2], 0.55 * alpha)
    ctx.set_line_width(2.6)
    ctx.arc(0, 0, 68.0, 0, 2 * math.pi)
    ctx.stroke()

    ctx.save()
    ctx.rotate(math.radians(rot_deg))
    for i in range(3):
        ctx.save()
        ctx.rotate(i * 2 * math.pi / 3)
        ctx.translate(0, -37.0)
        ctx.set_source_rgba(GOLD[0], GOLD[1], GOLD[2], alpha)
        ctx.arc(0, 4, 21.5, 0, 2 * math.pi)
        ctx.fill()
        ctx.move_to(0, -26)
        ctx.curve_to(8, -14, 21.5, -8, 21.5, 4)
        ctx.line_to(-21.5, 4)
        ctx.curve_to(-21.5, -8, -8, -14, 0, -26)
        ctx.fill()
        ctx.restore()
    ctx.restore()

    ctx.set_source_rgba(TEAL[0], TEAL[1], TEAL[2], alpha)
    ctx.arc(0, 0, 9.8, 0, 2 * math.pi)
    ctx.fill()
    ctx.restore()


def draw_text(ctx, text, x, y, size, color, bold=False, mono=False, align="left", alpha=1.0):
    ctx.save()
    family = "JetBrains Mono" if mono else "Roboto Flex"
    weight = cairo.FONT_WEIGHT_BOLD if bold else cairo.FONT_WEIGHT_NORMAL
    ctx.select_font_face(family, cairo.FONT_SLANT_NORMAL, weight)
    ctx.set_font_size(size)
    ext = ctx.text_extents(text)
    dx = x
    if align == "center":
        dx = x - ext.width / 2 - ext.x_bearing
    elif align == "right":
        dx = x - ext.width - ext.x_bearing
    ctx.move_to(dx, y)
    ctx.set_source_rgba(color[0], color[1], color[2], alpha)
    ctx.show_text(text)
    ctx.restore()


def draw_pill(ctx, text, x, y, w, h, color, bg_alpha=0.18, alpha=1.0):
    rounded_rect(ctx, x, y, w, h, h / 2)
    ctx.set_source_rgba(color[0] * 0.25, color[1] * 0.25, color[2] * 0.25, (0.85 + bg_alpha) * alpha)
    ctx.fill_preserve()
    ctx.set_source_rgba(color[0], color[1], color[2], 0.78 * alpha)
    ctx.set_line_width(1.8)
    ctx.stroke()
    draw_text(ctx, text, x + w / 2, y + h * 0.66, h * 0.44, color, bold=True, mono=True, align="center", alpha=alpha)


def draw_top_bar(ctx, section_label, progress, alpha=1.0):
    if alpha <= 0.01:
        return
    draw_crest(ctx, 84, 62, 0.28, progress * 120, alpha=alpha)
    draw_text(ctx, "TAMAGI OS 1.7", 126, 60, 22, GOLD, bold=True, alpha=alpha)
    draw_text(ctx, f"·  {section_label}", 315, 60, 20, MUTED, bold=False, alpha=alpha)
    # Progress bar at bottom
    ctx.set_source_rgba(1, 1, 1, 0.08 * alpha)
    ctx.rectangle(0, H - 6, W, 6)
    ctx.fill()
    ctx.set_source_rgba(GOLD[0], GOLD[1], GOLD[2], 0.9 * alpha)
    ctx.rectangle(0, H - 6, W * clamp(progress), 6)
    ctx.fill()


def scene_1_intro(ctx, st, global_t):
    # 0..6s: Hero reveal
    a = ease_out_cubic(st / 1.0)
    fade_out = 1.0 - ease_in_out((st - 5.2) / 0.8) if st > 5.2 else 1.0
    alpha = a * fade_out

    scale = 1.18 + 0.14 * ease_out_cubic(st / 1.5)
    draw_crest(ctx, W / 2, 315, scale, global_t * 28.0, alpha=alpha)

    y_shift = (1.0 - ease_out_cubic((st - 0.3) / 0.9)) * 35
    a2 = clamp((st - 0.25) / 0.8) * fade_out
    draw_text(ctx, "Tamagi OS 1.7", W / 2, 545 + y_shift, 92, CREAM, bold=True, align="center", alpha=a2)
    draw_text(
        ctx,
        'Codinome "Kazunobu M"  ·  O Desktop Linux Atômico de 3ª Geração',
        W / 2,
        608 + y_shift,
        30,
        GOLD,
        bold=True,
        align="center",
        alpha=a2,
    )
    draw_text(
        ctx,
        "Construído para IA agêntica, engenharia de software e gaming nativo de baixa latência.",
        W / 2,
        662 + y_shift,
        24,
        MUTED,
        align="center",
        alpha=a2,
    )

    badges = [
        ("composefs-rs + fs-verity", GOLD, 330),
        ("bootc 1.17.5+", TEAL, 240),
        ("OpenCode + MCP OS", GREEN, 280),
        ("Steam Nativo + GE-Proton", GOLD, 340),
        ("DxAI · Tamagi-SKL · SKL", TEAL, 330),
    ]
    total_w = sum(b[2] for b in badges) + 18 * (len(badges) - 1)
    bx = (W - total_w) / 2
    for idx, (lbl, col, bw) in enumerate(badges):
        ba = clamp((st - 0.9 - idx * 0.14) / 0.55) * fade_out
        draw_pill(ctx, lbl, bx, 742 + (1 - ba) * 20, bw, 44, col, alpha=ba)
        bx += bw + 18


def scene_2_composefs(ctx, st, global_t):
    # 6..12s: composefs-rs + bootc 1.17.5
    fin = ease_out_cubic(st / 0.6)
    fout = 1.0 - ease_in_out((st - 5.3) / 0.7) if st > 5.3 else 1.0
    alpha = fin * fout
    draw_top_bar(ctx, "NÚCLEO IMUTÁVEL · COMPOSEFS-RS & BOOTC 1.17.5", global_t / DURATION, alpha)

    draw_text(ctx, "Raiz Imutável com Selo Criptográfico fs-verity", 110, 175, 52, CREAM, bold=True, alpha=alpha)
    draw_text(
        ctx,
        "containers/composefs-rs (cfsctl) + bootc 1.17.5 + systemd-boot + Btrfs zstd",
        110,
        222,
        25,
        TEAL,
        mono=True,
        alpha=alpha,
    )

    layers = [
        ("CAMADA 4 · CONTAINERS & VMS", "Distrobox · Podman Dev Containers · Quadlets MCP · QEMU/KVM", TEAL),
        ("CAMADA 3 · IA & ESPAÇO DO USUÁRIO", "OpenCode · Claude · Codex · Gemini · llama.cpp Vulkan · GE-Proton", GREEN),
        ("CAMADA 2 · FLATPAK PORTÁTIL (--USER)", "LibreOffice · OnlyOffice · Okular · COSMIC Apps em /var/home", TEAL),
        ("CAMADA 1 · CAMADAS NATIVAS (SYSEXT)", "Drivers NVIDIA/CUDA · AMD ROCm · Intel oneAPI · LACT (reversível)", GOLD),
        ("CAMADA 0 · NÚCLEO COMPOSEFS + FS-VERITY", "Fedora 45 Bootc · Kernel OGC · COSMIC · Ptyxis + foot leve", GOLD),
    ]
    for i, (title, sub, col) in enumerate(layers):
        delay = 0.25 + (4 - i) * 0.22
        la = ease_out_cubic((st - delay) / 0.65) * fout
        ly = 278 + i * 136 + (1.0 - la) * 28
        rounded_rect(ctx, 110, ly, 1020, 116, 18)
        ctx.set_source_rgba(CARD_BG[0], CARD_BG[1], CARD_BG[2], 0.94 * la)
        ctx.fill_preserve()
        ctx.set_source_rgba(col[0], col[1], col[2], 0.7 * la)
        ctx.set_line_width(2.0)
        ctx.stroke()
        draw_text(ctx, title, 144, ly + 46, 23, col, bold=True, mono=True, alpha=la)
        draw_text(ctx, sub, 144, ly + 84, 20, CREAM, alpha=la)

    # Right inspector card
    ra = ease_out_cubic((st - 0.8) / 0.8) * fout
    rounded_rect(ctx, 1170, 278, 640, 664, 22)
    ctx.set_source_rgba(CARD_DARK[0], CARD_DARK[1], CARD_DARK[2], 0.96 * ra)
    ctx.fill_preserve()
    ctx.set_source_rgba(GOLD[0], GOLD[1], GOLD[2], 0.65 * ra)
    ctx.set_line_width(2.0)
    ctx.stroke()

    draw_text(ctx, "$ tamagi sys cfs status", 1206, 336, 22, GOLD, bold=True, mono=True, alpha=ra)
    cli_lines = [
        ("Engine      : bootc 1.17.5 + cfsctl", CREAM),
        ("Root /usr   : composefs (ro, fs-verity)", GREEN),
        ("Bootloader  : systemd-boot (/efi)", TEAL),
        ("Doctor      : 14/14 checks PASSED", GREEN),
        ("", CREAM),
        ("$ tamagi sys why /usr/bin/bootc", GOLD),
        ("  [composefs + fs-verity]", TEAL),
        ("  bootc-1.17.5.fc45.x86_64", CREAM),
        ("", CREAM),
        ("✓ Rollback atômico automático", GREEN),
        ("✓ Zero deriva em /usr", GREEN),
        ("✓ Ptyxis (1 aba) + foot leve", TEAL),
    ]
    for idx, (ln, col) in enumerate(cli_lines):
        l_alpha = clamp((st - 1.1 - idx * 0.16) / 0.35) * fout
        draw_text(ctx, ln, 1206, 390 + idx * 42, 20, col, mono=True, alpha=l_alpha)


def scene_3_ai_opencode(ctx, st, global_t):
    # 12..18s: OpenCode + Agentic AI + MCP OS
    fin = ease_out_cubic(st / 0.6)
    fout = 1.0 - ease_in_out((st - 5.3) / 0.7) if st > 5.3 else 1.0
    alpha = fin * fout
    draw_top_bar(ctx, "IA AGÊNTICA · OPENCODE, MCP NATIVO & SHADOW WORKSPACES", global_t / DURATION, alpha)

    draw_text(ctx, "OpenCode + IA Local Vulkan + Jaula CoW Instantânea", 110, 175, 50, CREAM, bold=True, alpha=alpha)
    draw_text(
        ctx,
        "anomalyco/opencode verificado por SHA-256 · mcp-tamagi-os (0 MB parado) · Btrfs reflink em milissegundos",
        110,
        222,
        23,
        GREEN,
        alpha=alpha,
    )

    # Terminal window mockup (Ptyxis Ipê Dark — 1 clean tab!)
    rounded_rect(ctx, 110, 268, 1120, 670, 20)
    ctx.set_source_rgba(0x12 / 255, 0x1B / 255, 0x17 / 255, 0.97 * alpha)
    ctx.fill_preserve()
    ctx.set_source_rgba(GREEN[0], GREEN[1], GREEN[2], 0.6 * alpha)
    ctx.set_line_width(2.0)
    ctx.stroke()

    # Headerbar showing single clean tab
    rounded_rect(ctx, 110, 268, 1120, 52, 20)
    ctx.set_source_rgba(0x1C / 255, 0x29 / 255, 0x23 / 255, 0.98 * alpha)
    ctx.fill()
    draw_pill(ctx, "Ptyxis · ~/projeto (1 aba limpa)", 132, 278, 360, 32, GOLD, alpha=alpha)
    draw_text(ctx, "Tema Ipê sincronizado", 1205, 301, 16, MUTED, mono=True, align="right", alpha=alpha)

    term_script = [
        (0.4, "$ tamagi ai mcp add os", GOLD),
        (0.9, ":: MCP 'tamagi-os' sincronizado (OpenCode, Claude, Gemini, Codex)", TEAL),
        (1.5, "$ tamagi ai run --shadow opencode", GOLD),
        (2.0, ":: Shadow CoW criado em 8 ms (cp --reflink=auto)", GREEN),
        (2.4, ":: Egress Guard ativo (--unshare-net · ~/.ssh oculto)", GREEN),
        (2.9, "┌─ OpenCode (anomalyco/opencode · AVX2 · Tema Tamagi Ipê)", CREAM),
        (3.3, "│ [mcp] tamagi_composefs() -> fs-verity OK, 14/14 checks", TEAL),
        (3.8, "│ ✓ Patch aplicado e testado na sombra sem tocar a árvore real", GREEN),
        (4.3, "└─ Use 'tamagi ai shadow diff' antes de promover as mudanças.", MUTED),
    ]
    for delay, line, col in term_script:
        if st >= delay:
            chars = int((st - delay) * 95)
            shown = line[:chars]
            idx = term_script.index((delay, line, col))
            draw_text(ctx, shown, 146, 376 + idx * 58, 21, col, mono=True, alpha=alpha)

    # Right feature cards
    cards = [
        ("OpenCode Sob Demanda", "/usr/bin/opencode baixa com SHA-256, detecta AVX2 e herda o tema do COSMIC.", GOLD),
        ("mcp-tamagi-os (Read-Only)", "Inspeciona bootc, composefs, doctor e camadas sysext via JSON-RPC stdio.", TEAL),
        ("Zero Ícones Quebrados", "Herança Papirus-Dark + Cosmic + Adwaita e 10 ícones vetoriais redesenhados.", GREEN),
    ]
    for i, (ct, cs, col) in enumerate(cards):
        ca = ease_out_cubic((st - 0.6 - i * 0.35) / 0.7) * fout
        cy = 268 + i * 230
        rounded_rect(ctx, 1265, cy, 545, 206, 18)
        ctx.set_source_rgba(CARD_BG[0], CARD_BG[1], CARD_BG[2], 0.95 * ca)
        ctx.fill_preserve()
        ctx.set_source_rgba(col[0], col[1], col[2], 0.65 * ca)
        ctx.set_line_width(1.8)
        ctx.stroke()
        draw_text(ctx, ct, 1295, cy + 52, 25, col, bold=True, alpha=ca)
        words = cs.split()
        l1 = " ".join(words[:6])
        l2 = " ".join(words[6:])
        draw_text(ctx, l1, 1295, cy + 102, 19.5, CREAM, alpha=ca)
        draw_text(ctx, l2, 1295, cy + 134, 19.5, MUTED, alpha=ca)


def scene_4_gaming(ctx, st, global_t):
    # 18..24s: Native Steam + GE-Proton + gamescope + MangoHud
    fin = ease_out_cubic(st / 0.6)
    fout = 1.0 - ease_in_out((st - 5.3) / 0.7) if st > 5.3 else 1.0
    alpha = fin * fout
    draw_top_bar(ctx, "GAMING NATIVO · STEAM, GE-PROTON, GAMESCOPE HDR & MANGOHUD", global_t / DURATION, alpha)

    draw_text(ctx, "Steam Nativo Pronto para GE-Proton, gamescope & MangoHud", 110, 175, 49, CREAM, bold=True, alpha=alpha)
    draw_text(
        ctx,
        "Sem barreiras de sandbox entre o Steam, o compositor HDR gamescope e as camadas Vulkan 32/64-bit",
        110,
        222,
        23,
        GOLD,
        alpha=alpha,
    )

    # Simulated MangoHud Live Telemetry HUD Card
    rounded_rect(ctx, 110, 272, 820, 420, 22)
    ctx.set_source_rgba(CARD_DARK[0], CARD_DARK[1], CARD_DARK[2], 0.96 * alpha)
    ctx.fill_preserve()
    ctx.set_source_rgba(GOLD[0], GOLD[1], GOLD[2], 0.75 * alpha)
    ctx.set_line_width(2.2)
    ctx.stroke()

    draw_pill(ctx, "MANGOHUD · TAMAGI GAMING SESSION (HDR + VRR)", 142, 302, 520, 36, GOLD, alpha=alpha)
    fps = 142 + int(4 * math.sin(global_t * 5.0))
    draw_text(ctx, f"{fps} FPS", 146, 420, 74, GREEN, bold=True, mono=True, alpha=alpha)
    draw_text(ctx, "6.9 ms  ·  Kernel 7.3.5-ogc  ·  sched_ext: scx_rusty", 146, 468, 22, TEAL, mono=True, alpha=alpha)

    # Animated frametime graph
    ctx.save()
    ctx.set_source_rgba(GREEN[0], GREEN[1], GREEN[2], 0.85 * alpha)
    ctx.set_line_width(3.0)
    gx0, gy0, gw = 146, 565, 740
    for px in range(0, gw, 8):
        val = math.sin((px * 0.04) + global_t * 8.0) * 6.0 + math.cos((px * 0.09) - global_t * 4.0) * 3.0
        py = gy0 + val
        if px == 0:
            ctx.move_to(gx0 + px, py)
        else:
            ctx.line_to(gx0 + px, py)
    ctx.stroke()
    ctx.restore()
    draw_text(ctx, "Frametime ultra-estável  ·  Atalho overlay: Right_Shift+F12", 146, 646, 19, MUTED, mono=True, alpha=alpha)

    # Right column: GE-Proton + GPU Drivers at First Boot
    rounded_rect(ctx, 970, 272, 840, 420, 22)
    ctx.set_source_rgba(CARD_BG[0], CARD_BG[1], CARD_BG[2], 0.95 * alpha)
    ctx.fill_preserve()
    ctx.set_source_rgba(TEAL[0], TEAL[1], TEAL[2], 0.65 * alpha)
    ctx.set_line_width(2.0)
    ctx.stroke()

    draw_text(ctx, "Gaming + Drivers de GPU no First Boot", 1006, 332, 30, TEAL, bold=True, alpha=alpha)
    items = [
        ("Steam Nativo + 32-bit PipeWire/Vulkan", "Pré-instalado no Tamagi-DxAI com sessão HDR no login", GOLD),
        ("GE-Proton Verificado (SHA-512)", "tamagi pkg gaming ge-proton install | update", GREEN),
        ("NVIDIA GeForce/RTX & CUDA Compute", "Camadas reversíveis RPM Fusion no First Boot", TEAL),
        ("AMD ROCm & Intel Arc / Core Ultra", "HIP, OpenCL, oneAPI Level Zero e LACT prontos", GREEN),
    ]
    for i, (t1, t2, col) in enumerate(items):
        ia = ease_out_cubic((st - 0.5 - i * 0.25) / 0.6) * fout
        iy = 392 + i * 74
        ctx.save()
        ctx.set_source_rgba(col[0], col[1], col[2], ia)
        ctx.set_line_width(3.0)
        ctx.set_line_cap(cairo.LINE_CAP_ROUND)
        ctx.set_line_join(cairo.LINE_JOIN_ROUND)
        ctx.move_to(1006, iy - 8)
        ctx.line_to(1013, iy - 1)
        ctx.line_to(1024, iy - 14)
        ctx.stroke()
        ctx.restore()
        draw_text(ctx, t1, 1036, iy, 22, col, bold=True, alpha=ia)
        draw_text(ctx, t2, 1036, iy + 28, 18.5, CREAM, alpha=ia)

    # Bottom command banner
    rounded_rect(ctx, 110, 730, 1700, 180, 20)
    ctx.set_source_rgba(CARD_BG[0], CARD_BG[1], CARD_BG[2], 0.92 * alpha)
    ctx.fill_preserve()
    ctx.set_source_rgba(GOLD[0], GOLD[1], GOLD[2], 0.5 * alpha)
    ctx.set_line_width(1.6)
    ctx.stroke()
    draw_text(ctx, "LibreOffice + Apps COSMIC + Drivers GPU logo no Primeiro Login", 148, 790, 28, GOLD, bold=True, alpha=alpha)
    draw_text(
        ctx,
        "Escolha LibreOffice, Okular, GE-Proton, OpenCode, navegador adicional e a pilha exata da sua GPU em segundos.",
        148,
        836,
        22,
        CREAM,
        alpha=alpha,
    )
    draw_text(
        ctx,
        "$ tamagi pkg gaming ge-proton install   ·   $ tamagi pkg gaming run -- %command%",
        148,
        880,
        20,
        TEAL,
        mono=True,
        alpha=alpha,
    )


def scene_5_installer_skl(ctx, st, global_t, shot_welcome, shot_disk):
    # 24..30s: Real UI Screenshots — Secondary Disks (/run/media vs /mnt/...) & SKL Fork
    fin = ease_out_cubic(st / 0.6)
    fout = 1.0 - ease_in_out((st - 5.3) / 0.7) if st > 5.3 else 1.0
    alpha = fin * fout
    draw_top_bar(ctx, "INSTALADOR INTELIGENTE · DISCOS SECUNDÁRIOS (/MNT) & FORK SKL", global_t / DURATION, alpha)

    draw_text(ctx, "UX Material 3, Discos Extras em /mnt/… e Novo Fork SKL", 110, 168, 48, CREAM, bold=True, alpha=alpha)
    draw_text(
        ctx,
        "Ao achar outros HDs/SSDs, o instalador propõe /run/media (sob demanda) ou /mnt/<disco> (nofail, sem apagar dados)",
        110,
        214,
        22,
        TEAL,
        alpha=alpha,
    )

    if shot_welcome:
        sa1 = ease_out_cubic((st - 0.3) / 0.8) * fout
        sx1, sy1 = 110, 255 + (1 - sa1) * 30
        rounded_rect(ctx, sx1 - 6, sy1 - 6, shot_welcome.get_width() + 12, shot_welcome.get_height() + 12, 18)
        ctx.set_source_rgba(GOLD[0], GOLD[1], GOLD[2], 0.7 * sa1)
        ctx.fill()
        ctx.save()
        ctx.translate(sx1, sy1)
        ctx.set_source_surface(shot_welcome, 0, 0)
        ctx.paint_with_alpha(sa1)
        ctx.restore()
        draw_pill(ctx, "Seletor Tamagi-DxAI · SKL · Tamagi-SKL", sx1 + 20, sy1 + shot_welcome.get_height() + 22, 470, 38, GOLD, alpha=sa1)

    if shot_disk:
        sa2 = ease_out_cubic((st - 0.65) / 0.8) * fout
        sx2, sy2 = 995, 255 + (1 - sa2) * 30
        rounded_rect(ctx, sx2 - 6, sy2 - 6, shot_disk.get_width() + 12, shot_disk.get_height() + 12, 18)
        ctx.set_source_rgba(TEAL[0], TEAL[1], TEAL[2], 0.7 * sa2)
        ctx.fill()
        ctx.save()
        ctx.translate(sx2, sy2)
        ctx.set_source_surface(shot_disk, 0, 0)
        ctx.paint_with_alpha(sa2)
        ctx.restore()
        draw_pill(ctx, "Proposta /run/media vs /mnt/<disco> (nofail)", sx2 + 20, sy2 + shot_disk.get_height() + 22, 520, 38, TEAL, alpha=sa2)


def scene_6_outro(ctx, st, global_t):
    # 30..36s: Finale CTA
    a = ease_out_cubic(st / 0.9)
    draw_crest(ctx, W / 2, 290, 1.30, global_t * 30.0, alpha=a)

    draw_text(ctx, "Tamagi OS 1.7", W / 2, 525, 96, GOLD, bold=True, align="center", alpha=a)
    draw_text(
        ctx,
        "Atômico por dentro. Livre, rápido e pronto para criar por fora.",
        W / 2,
        590,
        32,
        CREAM,
        bold=True,
        align="center",
        alpha=a,
    )

    editions = [
        ("Tamagi-DxAI", "IA · Dev · VMs · Steam Nativo", TEAL, 470),
        ("SKL (Fork)", "Escola & Laboratórios · Painel Travado", GREEN, 490),
        ("Tamagi-SKL", "Gestão Escolar · SSH & Cockpit", GOLD, 470),
    ]
    ex = (W - (470 + 490 + 470 + 48)) / 2
    for idx, (ename, esub, col, ew) in enumerate(editions):
        ea = ease_out_cubic((st - 0.5 - idx * 0.2) / 0.65)
        rounded_rect(ctx, ex, 650, ew, 120, 18)
        ctx.set_source_rgba(CARD_BG[0], CARD_BG[1], CARD_BG[2], 0.95 * ea)
        ctx.fill_preserve()
        ctx.set_source_rgba(col[0], col[1], col[2], 0.78 * ea)
        ctx.set_line_width(2.2)
        ctx.stroke()
        draw_text(ctx, ename, ex + ew / 2, 702, 28, col, bold=True, align="center", alpha=ea)
        draw_text(ctx, esub, ex + ew / 2, 742, 18.5, CREAM, mono=True, align="center", alpha=ea)
        ex += ew + 24

    draw_pill(ctx, "github.com/wes-tamagi/tamagi-OS", (W - 560) / 2, 825, 560, 54, GOLD, bg_alpha=0.3, alpha=a)
    draw_text(
        ctx,
        "Open Source (Apache-2.0)  ·  Zero Telemetria  ·  ISO Multi-Edição 6,7 GB",
        W / 2,
        935,
        22,
        MUTED,
        align="center",
        alpha=a,
    )


def generate_ambient_audio(wav_path):
    """Synthesizes a warm, modern 36s stereo ambient electronic soundtrack (D minor9 / Fmaj7 / Cmaj9 / G6)."""
    sr = 44100
    n_samples = int(DURATION * sr)
    chords = [
        # (start_s, end_s, [freqs])
        (0.0, 9.0, [146.83, 220.00, 261.63, 329.63, 440.00]),     # Dm9
        (9.0, 18.0, [174.61, 220.00, 261.63, 329.63, 523.25]),    # Fmaj7
        (18.0, 27.0, [130.81, 196.00, 246.94, 293.66, 392.00]),   # Cmaj9
        (27.0, 36.0, [146.83, 220.00, 293.66, 349.23, 440.00]),   # Dm7 resolve
    ]
    arp_notes = [293.66, 349.23, 440.00, 523.25, 659.25, 523.25, 440.00, 349.23]

    with wave.open(wav_path, "wb") as wf:
        wf.setnchannels(2)
        wf.setsampwidth(2)
        wf.setframerate(sr)
        chunk = bytearray()
        for i in range(n_samples):
            t = i / sr
            # Master fade-in & fade-out
            env = clamp(t / 1.8) * clamp((DURATION - t) / 2.4)

            freqs = chords[-1][2]
            c_local = 0.0
            for cs, ce, cf in chords:
                if cs <= t < ce:
                    freqs = cf
                    c_local = (t - cs) / (ce - cs)
                    break

            chord_swell = math.sin(c_local * math.pi) ** 0.35
            pad_l = 0.0
            pad_r = 0.0
            for k, f in enumerate(freqs):
                detune = 1.0025 if k % 2 == 0 else 0.9975
                s1 = math.sin(2.0 * math.pi * f * t)
                s2 = math.sin(2.0 * math.pi * (f * detune) * t + k * 0.7)
                w = 0.14 if k == 0 else 0.065
                pad_l += w * (0.65 * s1 + 0.35 * s2)
                pad_r += w * (0.35 * s1 + 0.65 * s2)

            # Gentle crystal pulse arpeggio (every 0.25s)
            step_idx = int(t / 0.25)
            step_t = (t % 0.25) / 0.25
            arp_f = arp_notes[step_idx % len(arp_notes)]
            arp_env = math.exp(-step_t * 5.5) * 0.055
            arp_sig = arp_env * math.sin(2.0 * math.pi * arp_f * t)
            pan = 0.5 + 0.35 * math.sin(step_idx * 0.9)

            # Subtle warm sub pulse on scene transitions (every 6s)
            scene_t = t % 6.0
            sub = 0.14 * math.exp(-scene_t * 2.2) * math.sin(2.0 * math.pi * 73.42 * t)

            left = env * (chord_swell * pad_l + arp_sig * pan + sub)
            right = env * (chord_swell * pad_r + arp_sig * (1.0 - pan) + sub)

            li = int(max(-32767, min(32767, left * 26000)))
            ri = int(max(-32767, min(32767, right * 26000)))
            chunk.extend(struct.pack("<hh", li, ri))
            if len(chunk) >= 65536:
                wf.writeframesraw(chunk)
                chunk.clear()
        if chunk:
            wf.writeframesraw(chunk)


def main():
    os.makedirs(os.path.dirname(OUT_MP4), exist_ok=True)
    shot_w_path = os.path.join(ROOT, "output", "installer-shots", "dark", "i01-welcome.png")
    shot_d_path = os.path.join(ROOT, "output", "installer-shots", "dark", "i04b-disk-keep.png")
    shot_welcome = load_png_surface(shot_w_path, target_w=815)
    shot_disk = load_png_surface(shot_d_path, target_w=815)

    with tempfile.TemporaryDirectory() as tmp:
        wav_path = os.path.join(tmp, "audio.wav")
        print("Synthesizing stereo ambient soundtrack...")
        generate_ambient_audio(wav_path)

        cmd = [
            "ffmpeg",
            "-y",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "bgra",
            "-s",
            f"{W}x{H}",
            "-r",
            str(FPS),
            "-i",
            "-",
            "-i",
            wav_path,
            "-c:v",
            "libx264",
            "-preset",
            "medium",
            "-crf",
            "18",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-shortest",
            "-movflags",
            "+faststart",
            OUT_MP4,
        ]
        print(f"Rendering {TOTAL_FRAMES} frames ({DURATION:.0f}s @ {FPS}fps) -> {OUT_MP4}")
        proc = subprocess.Popen(cmd, stdin=subprocess.PIPE, stderr=subprocess.DEVNULL)
        assert proc.stdin is not None

        surf = cairo.ImageSurface(cairo.FORMAT_ARGB32, W, H)
        ctx = cairo.Context(surf)

        for frame in range(TOTAL_FRAMES):
            t = frame / FPS
            draw_background(ctx, t)
            if t < 6.0:
                scene_1_intro(ctx, t, t)
            elif t < 12.0:
                scene_2_composefs(ctx, t - 6.0, t)
            elif t < 18.0:
                scene_3_ai_opencode(ctx, t - 12.0, t)
            elif t < 24.0:
                scene_4_gaming(ctx, t - 18.0, t)
            elif t < 30.0:
                scene_5_installer_skl(ctx, t - 24.0, t, shot_welcome, shot_disk)
            else:
                scene_6_outro(ctx, t - 30.0, t)
            surf.flush()
            proc.stdin.write(surf.get_data())

        proc.stdin.close()
        rc = proc.wait()
        if rc != 0:
            raise SystemExit(f"ffmpeg exited with {rc}")
    print("Done:", OUT_MP4)


if __name__ == "__main__":
    main()
