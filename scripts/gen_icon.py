#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
ScreenTune 占位图标生成器（纯 Python 标准库，无第三方依赖）

生成 assets/icon.png（256×256）与 assets/icon.ico（PNG 内嵌 ICO）。
图案：Fluent 深色渐变背景 + 三条「R/G/B 亮度条」意象，
与托盘图标 / 窗口图标（运行时绘制）保持一致。

用法：python3 scripts/gen_icon.py
"""

import os
import struct
import zlib

SIZE = 256


def make_pixels() -> bytes:
    """生成 256×256 RGBA 像素（渐变背景 + 三色条）"""
    px = bytearray()
    for y in range(SIZE):
        for x in range(SIZE):
            fx = x / (SIZE - 1)
            fy = y / (SIZE - 1)
            t = min(1.0, fx * 0.7 + fy * 0.3)
            r = int(31 + 40 * t)
            g = int(31 + 48 * t)
            b = int(43 + 66 * t)

            in_bar_x = 32 <= fx * SIZE < 224
            in_bar_y = 64 <= fy * SIZE < 192
            if in_bar_x and in_bar_y:
                bar = int((fx * SIZE - 32) // 64)
                level = 0.3 + 0.55 * fy
                if bar == 0:
                    r, g, b = (
                        int(0.9 * level * 255),
                        int(0.55 * level * 255),
                        int(0.35 * level * 255),
                    )
                elif bar == 1:
                    r, g, b = (
                        int(0.6 * level * 255),
                        int(0.75 * level * 255),
                        int(0.6 * level * 255),
                    )
                else:
                    r, g, b = (
                        int(0.4 * level * 255),
                        int(0.55 * level * 255),
                        int(0.95 * level * 255),
                    )
            px += bytes((r, g, b, 255))
    return bytes(px)


def png_bytes(rgba: bytes, w: int, h: int) -> bytes:
    """RGBA 像素 → PNG 文件字节"""

    def chunk(tag: bytes, data: bytes) -> bytes:
        body = tag + data
        return (
            struct.pack(">I", len(data))
            + body
            + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)
        )

    ihdr = struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)
    raw = b"".join(b"\x00" + rgba[y * w * 4 : (y + 1) * w * 4] for y in range(h))
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def ico_bytes(png: bytes, w: int, h: int) -> bytes:
    """PNG → ICO 容器（Windows 支持 PNG 压缩条目）"""
    header = struct.pack("<HHH", 0, 1, 1)
    entry = struct.pack("<BBBBHHII", w % 256, h % 256, 0, 0, 1, 32, len(png), 22)
    return header + entry + png


def main() -> None:
    root = os.path.join(os.path.dirname(__file__), "..", "assets")
    os.makedirs(root, exist_ok=True)
    rgba = make_pixels()
    png = png_bytes(rgba, SIZE, SIZE)
    with open(os.path.join(root, "icon.png"), "wb") as f:
        f.write(png)
    with open(os.path.join(root, "icon.ico"), "wb") as f:
        f.write(ico_bytes(png, SIZE, SIZE))
    print(f"已生成 {os.path.abspath(root)}/icon.png 与 icon.ico")


if __name__ == "__main__":
    main()
