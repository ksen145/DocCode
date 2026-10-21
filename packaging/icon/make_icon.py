#!/usr/bin/env python3
"""Генератор иконки DocCode.

Рисует мастер-иконку 1024x1024 средствами Pillow (без внешних рендереров)
и раскладывает её во все нужные форматы:

    packaging/icon/png/doccode-{16,24,32,48,64,128,256,512}.png — для Linux (hicolor) и AppImage
    packaging/icon/doccode.ico                                — для Windows (exe + установщик)
    packaging/icon/doccode-128.rgba                           — сырые пиксели для иконки окна iced

Три варианта композиции (мастер рисуется в 1024 и уменьшается LANCZOS,
поэтому на мелких размерах всё остаётся чистым):

    glyph  — синий скруглённый квадрат и белый знак </>  (16, 24 px)
    sheet  — тот же квадрат, белый лист и синий </>      (32, 48 px)
    full   — лист, </> и строки «текста»                 (64 px и выше)

Запуск:  python packaging/icon/make_icon.py
"""

from __future__ import annotations

import struct
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

S = 1024  # размер мастер-рендера

# Палитра -------------------------------------------------------------------
GRAD_TOP = (0x25, 0x63, 0xEB)  # синий сверху
GRAD_BOTTOM = (0x11, 0x28, 0x6B)  # тёмно-синий снизу
SHEET_TOP = (255, 255, 255)
SHEET_BOTTOM = (0xE9, 0xF0, 0xFF)
CODE_COLOR = (0x1D, 0x4E, 0xD8)  # синий код на белом листе
BAR_COLOR = (0x8F, 0xA3, 0xC2)  # серо-синие строки текста
SHADOW_COLOR = (0x06, 0x0F, 0x2D, 150)

SQ_RADIUS = 224  # радиус скругления «квадрата»
SHEET = (282, 190, 742, 834)  # лист: x0, y0, x1, y1
SHEET_RADIUS = 48

BAR_START_X = 342
BAR_HEIGHT = 34
BAR_Y = (545, 619, 693, 767)  # центры строк текста
BAR_W = (340, 360, 318, 278)

CODE_CX, CODE_CY = 512, 372  # центр знака </> на листе


def lerp(a, b, t):
    return tuple(int(round(a[i] + (b[i] - a[i]) * t)) for i in range(3))


def vertical_gradient(top, bottom, height=S, width=S):
    """Вертикальный градиент: рисуем в 1px ширину и растягиваем."""
    column = Image.new("RGB", (1, height))
    column.putdata([lerp(top, bottom, y / max(height - 1, 1)) for y in range(height)])
    return column.resize((width, height), Image.BILINEAR)


def rounded_mask(box, radius, size=S):
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle(box, radius, fill=255)
    return mask


def draw_stroke(draw, points, width, color, cap=1):
    """Ломаная с круглыми торцами (joint='curve' скругляет стыки)."""
    draw.line(points, fill=color, width=width, joint="curve")
    r = width // 2
    for x, y in points:
        draw.ellipse([x - r, y - r, x + r, y + r], fill=color)


def code_glyph(draw, cx, cy, stroke, color):
    """Знак </> из трёх ломаных."""
    h = int(stroke * 2.2)  # полу-высота углов скобок
    arm = int(stroke * 2.0)  # длина плеча скобки
    gap = int(stroke * 1.5)  # отступ от центра до плеча

    # <
    left = cx - int(stroke * 4.2)
    draw_stroke(draw, [(left + arm, cy - h), (left, cy), (left + arm, cy + h)], stroke, color)
    # /
    slash = int(stroke * 0.95)
    draw_stroke(
        draw,
        [(cx - slash, cy + int(h * 1.15)), (cx + slash, cy - int(h * 1.15))],
        stroke,
        color,
    )
    # >
    right = cx + int(stroke * 4.2)
    draw_stroke(
        draw, [(right - arm, cy - h), (right, cy), (right - arm, cy + h)], stroke, color
    )


def base_square():
    """Синий скруглённый квадрат с градиентом и лёгким бликом сверху."""
    grad = vertical_gradient(GRAD_TOP, GRAD_BOTTOM).convert("RGBA")

    # Мягкий блик в верхней части — «объём».
    gloss = Image.new("L", (1, S))
    gloss.putdata([int(58 * max(0.0, 1.0 - y / (0.45 * S))) for y in range(S)])
    gloss = gloss.resize((S, S), Image.BILINEAR)
    white = Image.new("RGBA", (S, S), (255, 255, 255, 255))
    white.putalpha(gloss)
    grad = Image.alpha_composite(grad, white)

    grad.putalpha(rounded_mask([0, 0, S - 1, S - 1], SQ_RADIUS))
    return grad


def render(variant):
    """Рисует мастер-иконку 1024x1024."""
    canvas = base_square()
    layer = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    draw = ImageDraw.Draw(layer)

    if variant in ("sheet", "full"):
        # Мягкая тень под листом.
        shadow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
        sd = ImageDraw.Draw(shadow)
        x0, y0, x1, y1 = SHEET
        sd.rounded_rectangle([x0, y0 + 16, x1, y1 + 16], SHEET_RADIUS, fill=SHADOW_COLOR)
        shadow = shadow.filter(ImageFilter.GaussianBlur(26))
        canvas = Image.alpha_composite(canvas, shadow)

        # Сам лист — белый с лёгким градиентом.
        sheet = vertical_gradient(SHEET_TOP, SHEET_BOTTOM).convert("RGBA")
        sheet.putalpha(rounded_mask([x0, y0, x1, y1], SHEET_RADIUS))
        canvas = Image.alpha_composite(canvas, sheet)

        # Код </> синим по белому.
        if variant == "sheet":
            code_glyph(draw, CODE_CX, CODE_CY, 46, CODE_COLOR)
        else:
            code_glyph(draw, CODE_CX, CODE_CY, 36, CODE_COLOR)

    else:
        # glyph: без листа — белый </> прямо на синем (читается на 16-24 px).
        code_glyph(draw, CODE_CX, S // 2, 64, (255, 255, 255, 255))

    if variant == "full":
        for cy, w in zip(BAR_Y, BAR_W):
            draw.rounded_rectangle(
                [BAR_START_X, cy - BAR_HEIGHT // 2, BAR_START_X + w, cy + BAR_HEIGHT // 2],
                BAR_HEIGHT // 2,
                fill=BAR_COLOR,
            )

    return Image.alpha_composite(canvas, layer)


def variant_for(size):
    if size <= 24:
        return "glyph"
    if size <= 48:
        return "sheet"
    return "full"


def write_ico(path, images_by_size):
    """Собирает .ico из PNG-кадров (формат поддерживается с Windows Vista)."""
    entries = []
    blobs = []
    offset = 6 + 16 * len(images_by_size)
    for size, img in sorted(images_by_size.items()):
        import io

        buf = io.BytesIO()
        img.save(buf, format="PNG")
        data = buf.getvalue()
        blobs.append(data)
        entries.append(
            struct.pack(
                "<BBBBHHII",
                0 if size >= 256 else size,
                0 if size >= 256 else size,
                0,  # палитра
                0,  # зарезервировано
                1,  # color planes
                32,  # bpp
                len(data),
                offset,
            )
        )
        offset += len(data)

    with open(path, "wb") as fh:
        fh.write(struct.pack("<HHH", 0, 1, len(entries)))
        for entry in entries:
            fh.write(entry)
        for data in blobs:
            fh.write(data)


def main():
    here = Path(__file__).resolve().parent
    png_dir = here / "png"
    png_dir.mkdir(parents=True, exist_ok=True)

    sizes = (16, 24, 32, 48, 64, 128, 256, 512)
    masters = {v: render(v) for v in ("glyph", "sheet", "full")}

    produced = {}
    for size in sizes:
        variant = variant_for(size)
        img = masters[variant].resize((size, size), Image.LANCZOS)
        out = png_dir / f"doccode-{size}.png"
        img.save(out, format="PNG", optimize=True)
        produced[size] = img
        print(f"  png  {out.name:22} variant={variant}")

    # Мастер для документации / ручных правок.
    masters["full"].save(here / "doccode.png", format="PNG")
    print("  png  doccode.png (1024)")

    # Windows: много-размерный .ico.
    ico_sizes = [16, 24, 32, 48, 64, 128, 256]
    write_ico(here / "doccode.ico", {s: produced[s] for s in ico_sizes})
    print(f"  ico  doccode.ico ({', '.join(str(s) for s in ico_sizes)})")

    # Иконка окна iced: сырые RGBA-пиксели (без зависимостей в рантайме).
    raw = masters["sheet"].resize((128, 128), Image.LANCZOS).convert("RGBA")
    (here / "doccode-128.rgba").write_bytes(raw.tobytes())
    print("  raw  doccode-128.rgba (128x128 RGBA)")


if __name__ == "__main__":
    main()
