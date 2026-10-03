#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""由 wentai-icon-fg-1024.png 生成各密度启动图标与自适应图标前景。

用法:
    python tools/gen_icons.py

产物输出到 app/src/main/res/mipmap-{mdpi,hdpi,xhdpi,xxhdpi,xxxhdpi}/：
    ic_launcher.png            传统启动图标（含背景）
    ic_launcher_round.png      圆形启动图标
    ic_launcher_foreground.png 自适应图标前景（108dp 画布，内容居中留安全边距）
"""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "wentai-icon-fg-1024.png"
RES_DIR = ROOT / "app" / "src" / "main" / "res"

DENSITIES = {
    "mdpi": 1.0,
    "hdpi": 1.5,
    "xhdpi": 2.0,
    "xxhdpi": 3.0,
    "xxxhdpi": 4.0,
}

LEGACY_BASE_DP = 48
ADAPTIVE_CANVAS_DP = 108
FOREGROUND_SAFE_RATIO = 72 / 108  # 自适应图标前景内容安全区占比
FOREGROUND_FILL_RATIO = 0.82      # 传统图标中前景相对画布的占比

BACKGROUND_RGBA = (10, 10, 12, 255)  # #0A0A0C 品牌深色


def load_source():
    try:
        from PIL import Image
    except ImportError:
        print("[ERROR] 需要 Pillow 才能生成图标，请执行: pip install Pillow", file=sys.stderr)
        sys.exit(2)

    if not SOURCE.is_file():
        print(f"[ERROR] 未找到图标素材：{SOURCE}", file=sys.stderr)
        print("请确认素材位于 wentai/wentai-icon-fg-1024.png", file=sys.stderr)
        sys.exit(1)

    try:
        return Image.open(SOURCE).convert("RGBA")
    except Exception as exc:  # noqa: BLE001
        print(f"[ERROR] 图标素材无法读取：{exc}", file=sys.stderr)
        sys.exit(1)


def generate(img) -> None:
    from PIL import Image, ImageDraw

    for name, scale in DENSITIES.items():
        out_dir = RES_DIR / f"mipmap-{name}"
        out_dir.mkdir(parents=True, exist_ok=True)

        legacy_size = int(LEGACY_BASE_DP * scale)

        # 1) 传统启动图标：品牌底色 + 前景
        canvas = Image.new("RGBA", (legacy_size, legacy_size), BACKGROUND_RGBA)
        fg_size = int(legacy_size * FOREGROUND_FILL_RATIO)
        fg = img.resize((fg_size, fg_size), Image.LANCZOS)
        canvas.alpha_composite(fg, ((legacy_size - fg_size) // 2, (legacy_size - fg_size) // 2))
        canvas.save(out_dir / "ic_launcher.png")

        # 2) 圆形启动图标
        mask = Image.new("L", (legacy_size, legacy_size), 0)
        ImageDraw.Draw(mask).ellipse((0, 0, legacy_size - 1, legacy_size - 1), fill=255)
        round_img = canvas.copy()
        round_img.putalpha(mask)
        round_img.save(out_dir / "ic_launcher_round.png")

        # 3) 自适应图标前景：108dp 透明画布，内容居中留安全边距
        adaptive_size = int(ADAPTIVE_CANVAS_DP * scale)
        foreground = Image.new("RGBA", (adaptive_size, adaptive_size), (0, 0, 0, 0))
        safe_size = int(adaptive_size * FOREGROUND_SAFE_RATIO)
        scaled = img.resize((safe_size, safe_size), Image.LANCZOS)
        foreground.alpha_composite(
            scaled, ((adaptive_size - safe_size) // 2, (adaptive_size - safe_size) // 2)
        )
        foreground.save(out_dir / "ic_launcher_foreground.png")

        print(f"  生成 mipmap-{name}: 图标 {legacy_size}px / 前景 {adaptive_size}px")


def main() -> None:
    print(f"图标素材: {SOURCE}")
    img = load_source()
    generate(img)
    print("图标生成完成。")


if __name__ == "__main__":
    main()
