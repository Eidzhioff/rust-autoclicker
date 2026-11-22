#!/usr/bin/env python3
"""Рисует assets/icon.ico: тёмная плашка, курсор-стрелка, зелёные дуги клика."""
from PIL import Image, ImageDraw

S = 256  # базовый размер, PIL сам сделает остальные


def make(size: int) -> Image.Image:
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    s = size
    # плашка
    d.rounded_rectangle([0, 0, s - 1, s - 1], radius=int(s * 0.18), fill=(24, 34, 52, 255))
    # дуги "клика" вокруг острия курсора
    cx, cy = s * 0.38, s * 0.34
    w = max(2, int(s * 0.05))
    for r in (s * 0.17, s * 0.28):
        d.arc([cx - r, cy - r, cx + r, cy + r], start=195, end=330, fill=(86, 190, 110, 255), width=w)
    # курсор-стрелка (классический указатель), остриё в (cx, cy)
    pts = [(0.38, 0.34), (0.38, 0.80), (0.49, 0.69), (0.57, 0.87), (0.65, 0.83), (0.57, 0.66), (0.72, 0.66)]
    poly = [(x * s, y * s) for x, y in pts]
    d.polygon(poly, fill=(246, 248, 252, 255))
    d.line(poly + [poly[0]], fill=(12, 18, 28, 255), width=max(1, int(s * 0.02)), joint="curve")
    return img


base = make(S)
base.save("autoclicker/assets/icon.ico", sizes=[(n, n) for n in (16, 24, 32, 48, 64, 128, 256)])
print("icon.ico written")
