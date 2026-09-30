#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""把 --render-video 导出的 .bin 帧序列与歌曲音频合成为 MP4。

用法:
  python tools/render_video.py <bin目录> <音频.mp3> <输出.mp4> [--cellw 8.4 --cellh 16]

依赖: PIL(Pillow)、ffmpeg（默认找 tools/ffmpeg/ffmpeg.exe，找不到再用 PATH 里的）
说明: 帧由 Rust 端逐帧确定渲染，这里只做"字符格 → 像素"的光栅化，
      与 tools/render_frames.py 使用完全相同的字体与排布，画面一致。
"""
import os
import sys
import glob
import struct
import subprocess

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from render_frames import MONO, CJK, wide  # noqa: E402  复用字体路径与宽字判定

from PIL import Image, ImageDraw, ImageFont  # noqa: E402

FONTS = {}


class Rasterizer:
    """字形遮罩缓存光栅化器。

    dr.text() 每次调用都会重新走 FreeType 渲染，是性能瓶颈；
    这里每个字符只渲染一次 L 模式遮罩，之后逐格用 img.paste(tile, pos, mask)
    合成（C 层 memcpy），比纯 text 快一个数量级。
    定位与 render_frames.py 的 text(anchor='ls'/'ms') 完全一致。
    """

    PAD = 12  # 遮罩画布向下多留的余量，容纳下伸部（g/y/; 等），与原渲染的越界行为一致

    def __init__(self, cellw, cellh):
        self.cellw, self.cellh = cellw, cellh
        fsz = int(cellh * 0.86)
        self.fm = ImageFont.truetype(MONO, fsz)
        self.fc = ImageFont.truetype(CJK, fsz)
        self.asc = self.fm.getmetrics()[0]
        self.cw = int(cellw) + 1
        self.masks = {}   # (cjk, ch) -> L 模式遮罩
        self.tiles = {}   # (色量化值, cjk, ch) -> RGB 实色小图

    def _mask(self, cjk, ch):
        key = (cjk, ch)
        m = self.masks.get(key)
        if m is None:
            h = int(self.cellh) + self.PAD
            w = int(self.cellw) * 2 + 2 if cjk else self.cw
            img = Image.new("L", (w, h), 0)
            d = ImageDraw.Draw(img)
            if cjk:
                d.text((self.cellw, self.asc), ch, font=self.fc, fill=255, anchor="ms")
            else:
                d.text((0, self.asc), ch, font=self.fm, fill=255, anchor="ls")
            self.masks[key] = m = img
        return m

    def _tile(self, color, cjk, ch):
        key = (color, cjk, ch)
        t = self.tiles.get(key)
        if t is None:
            m = self._mask(cjk, ch)
            t = Image.new("RGB", m.size, color)
            self.tiles[key] = t
        return t

    def frame(self, data):
        w, h, cells = data
        W, H = int(w * self.cellw), int(h * self.cellh)
        img = Image.new("RGB", (W, H), (0, 0, 0))
        dr = ImageDraw.Draw(img)
        ch_h = int(self.cellh)
        # 背景先行（与原渲染两段式顺序一致）
        for row in range(h):
            base = int(row * self.cellh)
            for col in range(w):
                _, bg, _ = cells[row * w + col]
                if bg != (0, 0, 0):
                    x = int(col * self.cellw)
                    dr.rectangle([x, base, x + int(self.cellw) + 1, base + ch_h], fill=bg)
        # 前景：按行序 paste，覆盖顺序与原 text 方案相同
        for row in range(h):
            top = int(row * self.cellh)
            r0 = row * w
            for col in range(w):
                fg, _, cp = cells[r0 + col]
                if cp == 0 or cp == 32 or cp == 0xFFFF:
                    continue
                ch = chr(cp)
                cjk = wide(cp)
                # 颜色量化到 5bit/通道，拼贴缓存规模有界，肉眼不可辨
                color = (fg[0] & 0xF8, fg[1] & 0xF8, fg[2] & 0xF8)
                img.paste(self._tile(color, cjk, ch),
                          (int(col * self.cellw), top), self._mask(cjk, ch))
        return img


FONTS = {}



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


def render_img(data, rast):
    return rast.frame(data)


def find_ffmpeg():
    local = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ffmpeg", "ffmpeg.exe")
    if os.path.exists(local):
        return local
    return "ffmpeg"


def main():
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("bindir")
    ap.add_argument("audio")
    ap.add_argument("out")
    ap.add_argument("--cellw", type=float, default=8.4)
    ap.add_argument("--cellh", type=float, default=16.0)
    ap.add_argument("--crf", type=int, default=18)
    ap.add_argument("--preset", default="medium")
    ap.add_argument("--noscale", action="store_true", help="保持原始 1344x768，不放大到 1080p")
    args = ap.parse_args()

    bins = sorted(glob.glob(os.path.join(args.bindir, "*.bin")))
    if not bins:
        sys.exit(f"目录里没有 .bin 帧: {args.bindir}")
    w, h, _ = read_bin(bins[0])
    W, H = int(w * args.cellw), int(h * args.cellh)
    n = len(bins)
    print(f"{n} 帧 · {w}x{h} 格 → {W}x{H} px @60fps")

    ff = find_ffmpeg()
    cmd = [ff, "-y", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", f"{W}x{H}", "-r", "60",
           "-i", "pipe:0", "-i", args.audio,
           "-c:v", "libx264", "-preset", args.preset, "-crf", str(args.crf),
           "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "192k",
           "-movflags", "+faststart"]
    if not args.noscale:
        cmd += ["-vf", "scale=1920:1080:flags=lanczos"]
    cmd += [args.out]

    proc = subprocess.Popen(cmd, stdin=subprocess.PIPE,
                            stderr=subprocess.DEVNULL)
    import time
    t0 = time.time()
    rast = Rasterizer(args.cellw, args.cellh)
    try:
        for i, b in enumerate(bins):
            img = render_img(read_bin(b), rast)
            proc.stdin.write(img.tobytes())
            if i % 300 == 0:
                el = time.time() - t0
                spd = (i + 1) / el if el > 0 else 0.0
                eta = (n - i - 1) / spd if spd > 0 else 0.0
                print(f"\r  {i + 1}/{n}  {spd:.1f} 帧/s  剩余约 {eta / 60:.1f} 分钟",
                      end="", flush=True)
        proc.stdin.close()
        proc.wait()
    except BrokenPipeError:
        sys.exit("ffmpeg 提前退出——请检查输出路径/参数")
    print()
    if proc.returncode != 0:
        sys.exit(f"ffmpeg 失败 code={proc.returncode}（可去掉 stderr=DEVNULL 查看日志）")
    print(f"完成: {args.out}（耗时 {(time.time() - t0) / 60:.1f} 分钟）")


if __name__ == "__main__":
    main()
