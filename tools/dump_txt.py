#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""把导出的 .bin 帧按字符原样打印出来（保留布局，丢失颜色），用于精确核对排版。

用法: python dump_txt.py preview_html/all_08.bin [--ruler] [--y 0-10]
"""
import sys, struct

def read_bin(path):
    d = open(path, "rb").read()
    assert d[:4] == b"WEMV", path
    w, h = struct.unpack_from("<II", d, 4)
    rows = []
    off = 12
    for y in range(h):
        line = []
        for x in range(w):
            cp = struct.unpack_from("<I", d, off + 6)[0]
            off += 10
            line.append(" " if cp == 0 else chr(cp))
        rows.append(line)
    return w, h, rows

def main():
    path = sys.argv[1]
    ruler = "--ruler" in sys.argv
    y0, y1 = 0, 10**9
    if "--y" in sys.argv:
        v = sys.argv[sys.argv.index("--y") + 1]
        a, _, b = v.partition("-")
        y0 = int(a)
        y1 = int(b) if b else int(a)
    w, h, rows = read_bin(path)
    print(f"== {path}  {w}x{h} ==")
    if ruler:
        print("    " + "".join(str((i // 10) % 10) for i in range(w)))
        print("    " + "".join(str(i % 10) for i in range(w)))
    for y in range(max(0, y0), min(h, y1 + 1)):
        print(f"{y:3d}|" + "".join(rows[y]).rstrip())

main()
