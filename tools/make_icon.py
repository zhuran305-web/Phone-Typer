# -*- coding: utf-8 -*-
"""生成 Phone-Typer 应用图标（PIL，Linear Aesthetic 风格）。
输出：web/icons/icon-192.png、icon-256.png、icon-512.png、icon.ico
"""
import os

from PIL import Image, ImageDraw, ImageFilter

INDIGO = (99, 102, 241)
PURPLE = (168, 85, 247)
BG = (10, 10, 10)
SCREEN = (3, 3, 3)
WHITE = (237, 237, 239)


def lerp(c1, c2, t):
    return tuple(int(c1[i] + (c2[i] - c1[i]) * t) for i in range(3))


def v_gradient(draw, box, c1, c2):
    x0, y0, x1, y1 = [int(v) for v in box]
    for y in range(y0, y1):
        t = (y - y0) / max(y1 - y0 - 1, 1)
        draw.line([(x0, y), (x1 - 1, y)], fill=lerp(c1, c2, t))


def make_icon(size: int = 512) -> Image.Image:
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    pad = size * 0.04
    draw.rounded_rectangle([pad, pad, size - pad, size - pad], radius=size * 0.235, fill=BG + (255,))

    bloom = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    bd = ImageDraw.Draw(bloom)
    cx = size * 0.36
    cy = size * 0.5
    for r in range(int(size * 0.30), 0, -2):
        a = int(60 * (1 - r / (size * 0.30)) ** 2)
        bd.ellipse([cx - r, cy - r, cx + r, cy + r], fill=INDIGO + (a,))
    img = Image.alpha_composite(img, bloom)
    draw = ImageDraw.Draw(img)

    ph = size * 0.52
    pw = size * 0.27
    px = size * 0.25
    py = (size - ph) / 2
    phone = [px, py, px + pw, py + ph]
    grad = Image.new("RGBA", (int(pw), int(ph)), (0, 0, 0, 0))
    gd = ImageDraw.Draw(grad)
    v_gradient(gd, [0, 0, pw, ph], INDIGO, PURPLE)
    mask = Image.new("L", (int(pw), int(ph)), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, pw - 1, ph - 1], radius=pw * 0.22, fill=255)
    img.paste(grad, (int(px), int(py)), mask)
    draw = ImageDraw.Draw(img)

    sw = pw * 0.74
    sh = ph * 0.40
    sx = px + (pw - sw) / 2
    sy = py + ph * 0.13
    draw.rounded_rectangle([sx, sy, sx + sw, sy + sh], radius=sw * 0.14, fill=SCREEN + (255,))
    cur_x = sx + sw * 0.5
    draw.line([(cur_x, sy + sh * 0.25), (cur_x, sy + sh * 0.75)], fill=WHITE + (255,), width=max(2, int(size * 0.012)))

    lx = px + pw + size * 0.07
    ly0 = py + ph * 0.18
    gap = size * 0.085
    lh = size * 0.028
    for i, frac in enumerate((1.0, 0.72, 0.45)):
        lw = size * 0.26 * frac
        c = lerp(INDIGO, PURPLE, i / 2)
        draw.rounded_rectangle([lx, ly0 + i * gap, lx + lw, ly0 + i * gap + lh], radius=lh / 2, fill=c + (255,))

    return img


if __name__ == "__main__":
    out_dir = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "web", "icons")
    os.makedirs(out_dir, exist_ok=True)
    base = make_icon(512)
    for s in (192, 256, 512):
        p = os.path.join(out_dir, f"icon-{s}.png")
        base.resize((s, s), Image.LANCZOS).save(p)
        print("生成", p)
    ico = os.path.join(out_dir, "icon.ico")
    base.resize((256, 256), Image.LANCZOS).save(ico, format="ICO", sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    print("生成", ico)
