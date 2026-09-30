#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""把 world.execute(me); 导出的 .bin 帧渲染成 PNG，用于逐帧视觉校验。

用法:
  python render_frames.py out.png f_00.bin f_01.bin ...        # 纵向拼图
  python render_frames.py out.png --cellw 8 --cellh 15 *.bin
"""
import sys, struct, os
from PIL import Image, ImageDraw, ImageFont

MONO = "C:/Windows/Fonts/CascadiaMono.ttf"
MONO2 = "C:/Windows/Fonts/consola.ttf"
CJK = "C:/Windows/Fonts/msyh.ttc"


def read_bin(path):
    d = open(path, "rb").read()
    if d[:4] != b"WEMV":
        raise ValueError("bad magic " + path)
    w, h = struct.unpack_from("<II", d, 4)
    cells = []
    off = 12
    for _ in range(w * h):
        r, g, b, r2, g2, b2 = d[off:off + 6]
        ch = struct.unpack_from("<I", d, off + 6)[0]
        cells.append(((r, g, b), (r2, g2, b2), ch))
        off += 10
    return w, h, cells


def wide(cp):
    return (0x1100 <= cp <= 0x115F or 0x2E80 <= cp <= 0xA4CF or 0xAC00 <= cp <= 0xD7A3
            or 0xF900 <= cp <= 0xFAFF or 0xFE30 <= cp <= 0xFE6F or 0xFF00 <= cp <= 0xFF60
            or 0xFFE0 <= cp <= 0xFFE6 or 0x1F300 <= cp <= 0x1FAFF)


def render(paths, out, cellw=8.4, cellh=16.0, caption=True):
    frames = [read_bin(p) for p in paths]
    w, h = frames[0][0], frames[0][1]
    fsz = int(cellh * 0.86)
    fm = ImageFont.truetype(MONO, fsz)
    fc = ImageFont.truetype(CJK, fsz)
    asc, desc = fm.getmetrics()
    cap_h = 20 if caption else 0
    W = int(w * cellw)
    H = int(h * cellh)
    img = Image.new("RGB", (W, cap_h * len(frames) + H * len(frames)), (0, 0, 0))
    dr = ImageDraw.Draw(img)
    try:
        fcap = ImageFont.truetype(MONO, 13)
    except Exception:
        fcap = fm
    y0 = 0
    for (fw, fh, cells), p in zip(frames, paths):
        if caption:
            dr.rectangle([0, y0, W, y0 + cap_h], fill=(8, 18, 12))
            dr.text((8, y0 + 3), os.path.basename(p), font=fcap, fill=(120, 255, 170))
            y0 += cap_h
        # 背景
        for row in range(fh):
            for col in range(fw):
                _, bg, _ = cells[row * fw + col]
                if bg != (0, 0, 0):
                    x = int(col * cellw)
                    dr.rectangle([x, y0 + int(row * cellh), x + int(cellw) + 1,
                                  y0 + int((row + 1) * cellh)], fill=bg)
        # 前景
        for row in range(fh):
            base = y0 + int(row * cellh) + asc
            for col in range(fw):
                fg, bg, cp = cells[row * fw + col]
                if cp == 0 or cp == 32:
                    continue
                if cp == 0xFFFF:
                    continue
                ch = chr(cp)
                x = col * cellw
                if wide(cp):
                    dr.text((x + cellw, base), ch, font=fc, fill=fg, anchor="ms")
                else:
                    dr.text((x, base), ch, font=fm, fill=fg, anchor="ls")
        y0 += H
    img.save(out)
    print("saved", out, img.size)


if __name__ == "__main__":
    args = sys.argv[1:]
    out = args[0]
    cellw, cellh = 8.4, 16.0
    paths = []
    i = 1
    while i < len(args):
        a = args[i]
        if a == "--cellw":
            i += 1
            cellw = float(args[i])
        elif a == "--cellh":
            i += 1
            cellh = float(args[i])
        elif a == "--nocap":
            caption = False
            globals()["__nocap"] = True
        else:
            paths.append(a)
        i += 1
    render(paths, out, cellw, cellh)
