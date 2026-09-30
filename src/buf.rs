//! 字符画布：Cell / Canvas / 基本绘图图元。
//!
//! 整个 MV 每帧在 Canvas 上作画，再由 term 模块按差分刷到终端。
//! 所有坐标都是屏幕绝对坐标（含中文宽字符处理）。

// 绘图工具箱：图元按需取用，保留完整 API 面
#![allow(dead_code)]

use crate::theme;

// ── 颜色 ────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb { r, g, b }
    }
    /// 线性插值
    pub fn mix(self, o: Rgb, t: f32) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        Rgb::new(
            (self.r as f32 + (o.r as f32 - self.r as f32) * t) as u8,
            (self.g as f32 + (o.g as f32 - self.g as f32) * t) as u8,
            (self.b as f32 + (o.b as f32 - self.b as f32) * t) as u8,
        )
    }
    /// 亮度缩放
    pub fn mul(self, f: f32) -> Rgb {
        let f = f.max(0.0);
        Rgb::new(
            (self.r as f32 * f).min(255.0) as u8,
            (self.g as f32 * f).min(255.0) as u8,
            (self.b as f32 * f).min(255.0) as u8,
        )
    }
    /// 加色（用于辉光叠加）
    pub fn add(self, o: Rgb) -> Rgb {
        Rgb::new(
            self.r.saturating_add(o.r),
            self.g.saturating_add(o.g),
            self.b.saturating_add(o.b),
        )
    }
    /// 灰度值
    pub fn lum(self) -> f32 {
        (0.2126 * self.r as f32 + 0.7152 * self.g as f32 + 0.0722 * self.b as f32) / 255.0
    }
    /// 去饱和（孤独段落的褪色）
    pub fn desat(self, f: f32) -> Rgb {
        let l = self.lum() * 255.0;
        self.mix(Rgb::new(l as u8, l as u8, (l * 1.06).min(255.0) as u8), f)
    }
    pub const fn c256(r: u8, g: u8, b: u8) -> Rgb {
        Rgb { r, g, b }
    }
}

// ── 单元格 ──────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Cell {
    pub ch: char,
    pub fg: Rgb,
    pub bg: Rgb,
}

/// 宽字符右半边占位（不输出任何东西）。
pub const SKIP: char = '\u{0}';
/// 空白但保留背景色。
pub const BLANK: char = ' ';

impl Default for Cell {
    fn default() -> Self {
        Cell {
            ch: BLANK,
            fg: theme::TEXT_DIM,
            bg: theme::BG,
        }
    }
}

// ── 字符宽度 ────────────────────────────────────────────────
/// East Asian Width：返回 0 / 1 / 2 个终端单元格宽。
pub fn cw(c: char) -> i32 {
    let u = c as u32;
    if u == 0 {
        return 0;
    }
    if (0x0300..=0x036F).contains(&u)
        || (0x200B..=0x200F).contains(&u)
        || u == 0xFEFF
        || (0xFE00..=0xFE0F).contains(&u)
    {
        return 0;
    }
    let wide = (0x1100..=0x115F).contains(&u)
        || (0x2E80..=0x303E).contains(&u)
        || (0x3041..=0x33FF).contains(&u)
        || (0x3400..=0x4DBF).contains(&u)
        || (0x4E00..=0x9FFF).contains(&u)
        || (0xA000..=0xA4CF).contains(&u)
        || (0xAC00..=0xD7A3).contains(&u)
        || (0xF900..=0xFAFF).contains(&u)
        || (0xFE30..=0xFE6F).contains(&u)
        || (0xFF00..=0xFF60).contains(&u)
        || (0xFFE0..=0xFFE6).contains(&u)
        || (0x1F300..=0x1FAFF).contains(&u)
        || (0x20000..=0x3FFFD).contains(&u);
    if wide { 2 } else { 1 }
}

/// 字符串显示宽度
pub fn sw(s: &str) -> i32 {
    s.chars().map(cw).sum()
}

// ── 矩形 ────────────────────────────────────────────────────
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect { x, y, w, h }
    }
    pub fn right(&self) -> i32 {
        self.x + self.w - 1
    }
    pub fn bottom(&self) -> i32 {
        self.y + self.h - 1
    }
    pub fn cx(&self) -> i32 {
        self.x + self.w / 2
    }
    pub fn cy(&self) -> i32 {
        self.y + self.h / 2
    }
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x <= self.right() && y >= self.y && y <= self.bottom()
    }
    pub fn inset(&self, n: i32) -> Rect {
        Rect::new(self.x + n, self.y + n, (self.w - 2 * n).max(0), (self.h - 2 * n).max(0))
    }
}

/// 边框风格
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Frame {
    Single,
    Double,
    Rounded,
    Heavy,
    Dashed,
    None,
}

impl Frame {
    fn glyphs(self) -> (char, char, char, char, char, char) {
        // (tl, tr, bl, br, h, v)
        match self {
            Frame::Single => ('┌', '┐', '└', '┘', '─', '│'),
            Frame::Double => ('╔', '╗', '╚', '╝', '═', '║'),
            Frame::Rounded => ('╭', '╮', '╰', '╯', '─', '│'),
            Frame::Heavy => ('┏', '┓', '┗', '┛', '━', '┃'),
            Frame::Dashed => ('┌', '┐', '└', '┘', '┄', '┆'),
            Frame::None => (' ', ' ', ' ', ' ', ' ', ' '),
        }
    }
}

// ── 画布 ────────────────────────────────────────────────────
#[derive(Clone)]
pub struct Canvas {
    pub w: i32,
    pub h: i32,
    pub cells: Vec<Cell>,
}

impl Canvas {
    pub fn new(w: i32, h: i32) -> Self {
        let w = w.max(1);
        let h = h.max(1);
        Canvas {
            w,
            h,
            cells: vec![Cell::default(); (w * h) as usize],
        }
    }

    #[inline]
    pub fn idx(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            None
        } else {
            Some((y * self.w + x) as usize)
        }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Option<&Cell> {
        self.idx(x, y).map(|i| &self.cells[i])
    }

    pub fn clear(&mut self) {
        for c in self.cells.iter_mut() {
            *c = Cell {
                ch: BLANK,
                fg: theme::TEXT_FAINT,
                bg: theme::BG,
            };
        }
    }

    /// 填满整块矩形
    pub fn fill(&mut self, r: Rect, ch: char, fg: Rgb, bg: Rgb) {
        for y in r.y..=r.bottom() {
            for x in r.x..=r.right() {
                self.put(x, y, ch, fg, bg);
            }
        }
    }

    /// 只改背景色，保留字符
    pub fn tint(&mut self, r: Rect, bg: Rgb, alpha: f32) {
        for y in r.y..=r.bottom() {
            for x in r.x..=r.right() {
                if let Some(i) = self.idx(x, y) {
                    let old = self.cells[i].bg;
                    self.cells[i].bg = old.mix(bg, alpha);
                }
            }
        }
    }

    /// 全屏亮度缩放（做暗场 / 褪色）
    pub fn shade(&mut self, f: f32) {
        for c in self.cells.iter_mut() {
            c.fg = c.fg.mul(f);
            c.bg = c.bg.mul(f);
        }
    }

    pub fn shade_rect(&mut self, r: Rect, f: f32) {
        for y in r.y..=r.bottom() {
            for x in r.x..=r.right() {
                if let Some(i) = self.idx(x, y) {
                    self.cells[i].fg = self.cells[i].fg.mul(f);
                    self.cells[i].bg = self.cells[i].bg.mul(f);
                }
            }
        }
    }

    /// 整屏去饱和（用于"孤独"段落的色彩抽离）
    pub fn desaturate(&mut self, f: f32) {
        for c in self.cells.iter_mut() {
            c.fg = c.fg.desat(f);
            c.bg = c.bg.desat(f);
        }
    }

    #[inline]
    pub fn put(&mut self, x: i32, y: i32, ch: char, fg: Rgb, bg: Rgb) {
        if ch == '\n' || ch == '\r' {
            return;
        }
        let w = cw(ch);
        if w == 0 {
            return;
        }
        if w == 2 {
            // 末列放不下宽字符，退化成空格
            if x + 1 >= self.w {
                if let Some(i) = self.idx(x, y) {
                    self.cells[i] = Cell { ch: BLANK, fg, bg };
                }
                return;
            }
            if let Some(i) = self.idx(x, y) {
                self.cells[i] = Cell { ch, fg, bg };
            }
            if let Some(i) = self.idx(x + 1, y) {
                self.cells[i] = Cell { ch: SKIP, fg, bg };
            }
            return;
        }
        if let Some(i) = self.idx(x, y) {
            self.cells[i] = Cell { ch, fg, bg };
        }
    }

    /// 覆盖式 put（只改字符与前景，不动背景）
    #[inline]
    pub fn put_ov(&mut self, x: i32, y: i32, ch: char, fg: Rgb) {
        if let Some(i) = self.idx(x, y) {
            let bg = self.cells[i].bg;
            self.cells[i] = Cell { ch, fg, bg };
        }
    }

    /// 若该位置已有内容，把前景色向 fg 混一点（辉光/雾）
    pub fn glow(&mut self, x: i32, y: i32, fg: Rgb, a: f32) {
        if let Some(i) = self.idx(x, y) {
            let c = self.cells[i];
            if c.ch != BLANK && c.ch != SKIP {
                self.cells[i].fg = c.fg.mix(fg, a.clamp(0.0, 1.0));
                self.cells[i].bg = c.bg.add(fg.mul(a * 0.10));
            }
        }
    }

    pub fn hline(&mut self, x0: i32, x1: i32, y: i32, ch: char, fg: Rgb, bg: Rgb) {
        let (a, b) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        for x in a..=b {
            self.put(x, y, ch, fg, bg);
        }
    }

    pub fn vline(&mut self, x: i32, y0: i32, y1: i32, ch: char, fg: Rgb, bg: Rgb) {
        let (a, b) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
        for y in a..=b {
            self.put(x, y, ch, fg, bg);
        }
    }

    /// 画文本，返回结束后的 x 坐标
    pub fn text(&mut self, x: i32, y: i32, s: &str, fg: Rgb, bg: Rgb) -> i32 {
        let mut cx = x;
        for ch in s.chars() {
            let w = cw(ch);
            if w == 0 {
                continue;
            }
            self.put(cx, y, ch, fg, bg);
            cx += w;
        }
        cx
    }

    pub fn text_center(&mut self, cx: i32, y: i32, s: &str, fg: Rgb, bg: Rgb) {
        let w = sw(s);
        self.text(cx - w / 2, y, s, fg, bg);
    }

    pub fn text_right(&mut self, rx: i32, y: i32, s: &str, fg: Rgb, bg: Rgb) {
        let w = sw(s);
        self.text(rx - w, y, s, fg, bg);
    }

    /// 带辉光的文本：先画两侧暗淡版本，再画本体
    pub fn text_glow(&mut self, cx: i32, y: i32, s: &str, fg: Rgb, g: Rgb, bg: Rgb) {
        let w = sw(s);
        let x = cx - w / 2;
        for dx in [-1, 1] {
            let mut px = x + dx;
            for ch in s.chars() {
                let cwid = cw(ch);
                if cwid == 0 {
                    continue;
                }
                self.glow(px, y, g, 0.45);
                if cwid == 2 {
                    self.glow(px + 1, y, g, 0.45);
                }
                px += cwid;
            }
        }
        self.text(x, y, s, fg, bg);
    }

    pub fn frame(&mut self, r: Rect, style: Frame, fg: Rgb, bg: Rgb) {
        if style == Frame::None || r.w < 2 || r.h < 2 {
            return;
        }
        let (tl, tr, bl, br, h, v) = style.glyphs();
        self.hline(r.x + 1, r.right() - 1, r.y, h, fg, bg);
        self.hline(r.x + 1, r.right() - 1, r.bottom(), h, fg, bg);
        self.vline(r.x, r.y + 1, r.bottom() - 1, v, fg, bg);
        self.vline(r.right(), r.y + 1, r.bottom() - 1, v, fg, bg);
        self.put(r.x, r.y, tl, fg, bg);
        self.put(r.right(), r.y, tr, fg, bg);
        self.put(r.x, r.bottom(), bl, fg, bg);
        self.put(r.right(), r.bottom(), br, fg, bg);
    }

    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, ch: char, fg: Rgb, bg: Rgb) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let (mut x, mut y) = (x0, y0);
        let mut guard = 0;
        loop {
            self.put(x, y, ch, fg, bg);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
            guard += 1;
            if guard > 20000 {
                break;
            }
        }
    }

    /// 中点圆（只用点，不填）
    pub fn circle(&mut self, cx: i32, cy: i32, rad: f32, ch: char, fg: Rgb, bg: Rgb) {
        let n = ((rad * 6.5) as i32).clamp(24, 720);
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            // 字符格子非正方，x 方向按 0.5 压缩
            let x = cx + (a.cos() * rad * 2.0).round() as i32;
            let y = cy + (a.sin() * rad).round() as i32;
            self.put(x, y, ch, fg, bg);
        }
    }

    /// 椭圆（xr / yr 分别给出）
    pub fn ellipse(&mut self, cx: i32, cy: i32, xr: f32, yr: f32, ch: char, fg: Rgb, bg: Rgb) {
        let n = (((xr + yr) * 6.0) as i32).clamp(32, 1400);
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            let x = cx + (a.cos() * xr).round() as i32;
            let y = cy + (a.sin() * yr).round() as i32;
            self.put(x, y, ch, fg, bg);
        }
    }

    /// 整屏内容平移（震屏）
    pub fn shift(&mut self, dx: i32, dy: i32) {
        if dx == 0 && dy == 0 {
            return;
        }
        let src = self.cells.clone();
        self.clear();
        for y in 0..self.h {
            for x in 0..self.w {
                let i = (y * self.w + x) as usize;
                let c = src[i];
                if c.ch == SKIP {
                    continue;
                }
                let nx = x + dx;
                let ny = y + dy;
                if let Some(j) = self.idx(nx, ny) {
                    self.cells[j] = c;
                }
            }
        }
    }

    /// 横向波纹位移：按行做不同的水平偏移（glitch 用）
    pub fn row_shift(&mut self, y: i32, dx: i32) {
        if dx == 0 || y < 0 || y >= self.h {
            return;
        }
        let row: Vec<Cell> = (0..self.w).map(|x| self.cells[(y * self.w + x) as usize]).collect();
        for x in 0..self.w {
            let sx = x - dx;
            if sx >= 0 && sx < self.w {
                self.cells[(y * self.w + x) as usize] = row[sx as usize];
            }
        }
    }

    /// 把一个矩形区域内的字符复制平移（做"滚动字幕"/"列表上升"）
    pub fn blit_scroll(&mut self, r: Rect, dy: i32) {
        if dy == 0 {
            return;
        }
        let mut buf: Vec<Cell> = Vec::with_capacity((r.w * r.h) as usize);
        for y in 0..r.h {
            for x in 0..r.w {
                buf.push(*self.get(r.x + x, r.y + y).unwrap_or(&Cell::default()));
            }
        }
        for y in 0..r.h {
            for x in 0..r.w {
                let sy = y - dy;
                if sy >= 0 && sy < r.h {
                    let c = buf[(sy * r.w + x) as usize];
                    if c.ch == SKIP {
                        continue;
                    }
                    self.put_ov(r.x + x, r.y + y, c.ch, c.fg);
                    if let Some(i) = self.idx(r.x + x, r.y + y) {
                        self.cells[i].bg = c.bg;
                    }
                }
            }
        }
    }
}
