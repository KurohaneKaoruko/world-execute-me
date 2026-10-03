//! 特效库：随机数、粒子、故障、字符雨、辉光、扫描线、心形曲线……
//! 场景只负责"编排"，具体表现全部在这里。

// 特效工具箱：效果按需取用，保留完整调色板
#![allow(dead_code)]

use crate::buf::{cw, Canvas, Rect, Rgb, SKIP, sw};
use crate::theme;

// ── 随机数（xorshift*，足够快且可复现）─────────────────────
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    #[inline]
    pub fn f(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u32 << 24) as f32
    }
    #[inline]
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }
    #[inline]
    pub fn irange(&mut self, a: i32, b: i32) -> i32 {
        if b <= a {
            return a;
        }
        a + (self.next_u64() % ((b - a) as u64)) as i32
    }
    #[inline]
    pub fn chance(&mut self, p: f32) -> bool {
        self.f() < p
    }
    pub fn pick<'a, T>(&mut self, v: &'a [T]) -> &'a T {
        &v[(self.next_u64() % v.len() as u64) as usize]
    }
}

/// 稳定的哈希噪声（同坐标同帧值相同，用于静态颗粒）
#[inline]
pub fn hash2(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B9)
        ^ (y as u32).wrapping_mul(0x85EB_CA6B)
        ^ seed.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545_F491);
    h ^= h >> 13;
    (h & 0xFFFF) as f32 / 65535.0
}

#[inline]
pub fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
pub fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

#[inline]
pub fn ease_in(t: f32) -> f32 {
    t.clamp(0.0, 1.0).powi(3)
}

/// 0→1→0 的脉冲
#[inline]
pub fn pulse(t: f32, attack: f32, release: f32) -> f32 {
    if t < 0.0 {
        0.0
    } else if t < attack {
        t / attack
    } else if t < attack + release {
        1.0 - (t - attack) / release
    } else {
        0.0
    }
}

// ── 字符集 ──────────────────────────────────────────────────
pub const RAIN: &[char] = &[
    '0', '1', '7', '3', '5', 'x', 'y', 'z', 'f', '=', '+', '-', '*', '/', '(', ')', ';', '{', '}',
    '[', ']', '<', '>', '|', '&', '!', '?', '#', '$', '%', '@', '∑', '∫', '√', 'π', '∞', '≈', '≠',
    'α', 'β', 'γ', 'δ', 'λ', 'μ', 'σ', 'ω', 'の', 'あ', 'カ', 'タ', 'ナ', 'ハ', 'マ', 'ヤ', 'ラ',
];
pub const GARBAGE: &[char] = &[
    '▓', '▒', '░', '█', '▄', '▀', '│', '─', '┼', '╳', '※', '#', '%', '&', '*', '~', '^', '¤', '§',
    '¥', '€', '¡', '¿',
];
pub const HALF: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
pub const BLOCKS: &[char] = &['░', '▒', '▓', '█'];
pub const SPARK: &[char] = &['·', '˙', '∴', '∵', '·', '⁘', '⁙'];
pub const BLOCK_HEAVY: &[char] = &['█', '▓', '▒', '░', '◆', '●', '■'];
pub const DIGITS: &[char] = &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
/// 字符碎片（"erase all the pointless fragments" 用）
pub const SHRED: &[char] = &['░', '▒', '‧', '·', ':', ';', '.'];


/// 扫描线 / CRT 行暗化
pub fn scanlines(cv: &mut Canvas, t: f32, strength: f32) {
    let phase = ((t * 18.0) as i32).rem_euclid(2);
    for y in 0..cv.h {
        let odd = (y + phase).rem_euclid(2) == 1;
        let mut f = if odd { 1.0 - strength } else { 1.0 };
        // 一条缓慢下滚的亮带
        let band = ((t * 26.0) as i32).rem_euclid(cv.h + 24) - 12;
        if (y - band).abs() <= 1 {
            f = (f + 0.10).min(1.35);
        }
        if f != 1.0 {
            let row = y * cv.w;
            for x in 0..cv.w {
                let c = &mut cv.cells[(row + x) as usize];
                c.fg = c.fg.mul(f);
                c.bg = c.bg.mul(f);
            }
        }
    }
}

/// 四角渐晕：让画面聚焦中心
pub fn vignette(cv: &mut Canvas, strength: f32) {
    let cx = cv.w as f32 / 2.0;
    let cy = cv.h as f32 / 2.0;
    let maxd = (cx * cx + cy * cy).sqrt();
    for y in 0..cv.h {
        for x in 0..cv.w {
            let dx = x as f32 - cx;
            let dy = (y as f32 - cy) * 1.9;
            let d = (dx * dx + dy * dy).sqrt() / maxd;
            let f = 1.0 - strength * (d * d).clamp(0.0, 1.0);
            let c = &mut cv.cells[(y * cv.w + x) as usize];
            c.fg = c.fg.mul(f);
            c.bg = c.bg.mul(f);
        }
    }
}

/// 辉光（bloom）：把高亮单元向四邻"溢出"。
/// 先收集再写入，避免同帧内互相反馈；空格子染一点背景色，形成柔和光晕。
pub fn bloom(cv: &mut Canvas, threshold: f32, gain: f32) {
    let mut bright: Vec<(usize, Rgb, f32)> = Vec::new();
    for (i, c) in cv.cells.iter().enumerate() {
        if c.ch == SKIP || c.ch == ' ' {
            continue;
        }
        let lum = c.fg.lum();
        if lum > threshold {
            bright.push((i, c.fg, ((lum - threshold) / (1.0 - threshold).max(0.01)).clamp(0.0, 1.0)));
        }
    }
    let w = cv.w;
    let h = cv.h;
    for (i, col, k) in bright {
        let x = (i as i32) % w;
        let y = (i as i32) / w;
        let a = k * gain;
        let spill = col.mul(a * 0.55);
        for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                continue;
            }
            let j = (ny * w + nx) as usize;
            let c = &mut cv.cells[j];
            if c.ch != SKIP && c.ch != ' ' {
                c.fg = c.fg.mix(col, a * 0.30);
            }
            c.bg = c.bg.add(spill.mul(0.16));
        }
    }
}

/// 故障：随机行位移 + 字符腐蚀 + 色散
pub fn glitch(cv: &mut Canvas, rng: &mut Rng, amount: f32, t: f32) {
    if amount <= 0.001 {
        return;
    }
    let rows = ((cv.h as f32 * amount * 0.35) as i32).clamp(0, 18);
    for _ in 0..rows {
        let y = rng.irange(0, cv.h);
        let dx = rng.irange(1, 3 + (12.0 * amount) as i32) * if rng.chance(0.5) { 1 } else { -1 };
        cv.row_shift(y, dx);
        // 色散：把该行再叠一层品红/青
        let tint = if rng.chance(0.5) { theme::MAGENTA } else { theme::CYAN };
        let row = y * cv.w;
        for x in 0..cv.w {
            let c = cv.cells[(row + x) as usize];
            if c.ch != ' ' && c.ch != SKIP {
                cv.cells[(row + x) as usize].fg = c.fg.mix(tint, 0.35 * amount);
            }
        }
    }
    // 随机腐蚀
    let n = (cv.w as f32 * cv.h as f32 * amount * 0.004) as i32;
    for _ in 0..n {
        let x = rng.irange(0, cv.w);
        let y = rng.irange(0, cv.h);
        let g = *rng.pick(GARBAGE);
        let f = theme::heat(rng.f()).mul(0.8);
        cv.put(x, y, g, f, theme::BG);
    }
    let _ = t;
}

/// 字符雨（矩阵雨）。state 由调用方持有以保持列状态连续。
pub struct Rain {
    cols: Vec<RainCol>,
    w: i32,
    h: i32,
}

struct RainCol {
    y: f32,
    speed: f32,
    len: i32,
    ch: Vec<char>,
}

impl Rain {
    pub fn new(w: i32, h: i32, rng: &mut Rng) -> Self {
        let mut cols = Vec::new();
        for _ in 0..w {
            cols.push(RainCol {
                y: rng.range(-40.0, h as f32),
                speed: rng.range(6.0, 26.0),
                len: rng.irange(6, 26),
                ch: (0..40).map(|_| *rng.pick(RAIN)).collect(),
            });
        }
        Rain { cols, w, h }
    }

    pub fn draw(&mut self, cv: &mut Canvas, rect: Rect, dt: f32, head: Rgb, tail: Rgb, alpha: f32) {
        if rect.w != self.w || rect.h != self.h {
            return;
        }
        for (i, c) in self.cols.iter_mut().enumerate() {
            c.y += c.speed * dt;
            if c.y - c.len as f32 > self.h as f32 {
                c.y = -c.len as f32;
                c.len = 6 + (i as i32 * 7 % 20);
            }
            if fast_rand(i as u64 ^ (c.y as u64)) < 0.06 {
                let len = c.ch.len();
                let slot = (c.y.max(0.0) as usize) % len;
                let pick = (fast_rand(c.y as u64 * 31 + i as u64) * RAIN.len() as f32) as usize % RAIN.len();
                c.ch[slot] = RAIN[pick];
            }
            let head_y = c.y as i32;
            for k in 0..c.len {
                let y = head_y - k;
                if y < rect.y || y > rect.bottom() {
                    continue;
                }
                let f = if k == 0 {
                    1.0
                } else {
                    (1.0 - k as f32 / c.len as f32).powf(1.6)
                };
                let col = head.mix(tail, 1.0 - f);
                let x = rect.x + i as i32;
                if let Some(cell) = cv.get(x, y) {
                    if cell.ch != ' ' && cell.ch != SKIP && alpha < 0.5 {
                        continue;
                    }
                }
                let ch = c.ch[((y.max(0) + k) as usize) % c.ch.len()];
                let a = (f * alpha).clamp(0.0, 1.0);
                let existing = cv.get(x, y).copied().unwrap_or_default();
                let fg = existing.fg.mix(col, a);
                cv.put(x, y, ch, fg, existing.bg);
            }
        }
    }
}

#[inline]
fn fast_rand(seed: u64) -> f32 {
    let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 29;
    (x & 0xFFFF) as f32 / 65535.0
}

// ── 粒子 ────────────────────────────────────────────────────
#[derive(Clone, Copy)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub max_life: f32,
    pub ch: char,
    pub col: Rgb,
}

impl Particle {
    pub fn new(x: f32, y: f32, vx: f32, vy: f32, life: f32, ch: char, col: Rgb) -> Self {
        Particle {
            x,
            y,
            vx,
            vy,
            life,
            max_life: life,
            ch,
            col,
        }
    }
}

pub struct Particles {
    pub items: Vec<Particle>,
}

impl Particles {
    pub fn new() -> Self {
        Particles { items: Vec::new() }
    }
    pub fn update(&mut self, dt: f32, gravity: f32, drag: f32) {
        for p in self.items.iter_mut() {
            p.life -= dt;
            p.vy += gravity * dt;
            p.vx *= 1.0 - drag * dt;
            p.vy *= 1.0 - drag * dt;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
        }
        self.items.retain(|p| p.life > 0.0);
    }
    pub fn draw(&self, cv: &mut Canvas) {
        for p in &self.items {
            let a = (p.life / p.max_life).clamp(0.0, 1.0);
            let col = p.col.mul(0.25 + 0.75 * a);
            let x = p.x.round() as i32;
            let y = p.y.round() as i32;
            let ex = cv.get(x, y).copied().unwrap_or_default();
            if ex.ch != ' ' && ex.ch != SKIP {
                cv.put(x, y, p.ch, ex.fg.mix(col, a), ex.bg);
            } else {
                cv.put(x, y, p.ch, col, ex.bg);
            }
        }
    }
    pub fn burst(&mut self, x: f32, y: f32, n: i32, speed: f32, life: f32, chs: &[char], col: Rgb, rng: &mut Rng) {
        for _ in 0..n {
            let a = rng.range(0.0, std::f32::consts::TAU);
            let s = rng.range(speed * 0.3, speed);
            let ch = *rng.pick(chs);
            self.items.push(Particle::new(
                x,
                y,
                a.cos() * s * 2.0,
                a.sin() * s,
                rng.range(life * 0.4, life),
                ch,
                col,
            ));
        }
    }
}

// ── 文本揭示 ────────────────────────────────────────────────
/// 打字机：返回本帧应显示的字符数
#[inline]
pub fn reveal_count(t: f32, cps: f32, total: i32) -> i32 {
    ((t.max(0.0) * cps) as i32).min(total)
}

/// 数据解码：未揭示的部分用乱码占位
pub fn scramble(text: &str, revealed: i32, rng: &mut Rng, keep_spaces: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    for ch in text.chars() {
        if i < revealed || (keep_spaces && ch == ' ') {
            out.push(ch);
        } else {
            out.push(*rng.pick(RAIN));
        }
        i += cw(ch);
    }
    out
}

/// 二进制雪花噪声落在矩形上（用于"数据流"背景）
pub fn data_dust(cv: &mut Canvas, rect: Rect, t: f32, density: f32, col: Rgb) {
    let n = (rect.w as f32 * rect.h as f32 * density) as i32;
    for k in 0..n {
        let h = hash2(k, (t * 6.0) as i32, 7);
        let x = rect.x + (h * rect.w as f32) as i32;
        let y = rect.y + (hash2(k * 3 + 1, (t * 6.0) as i32, 11) * rect.h as f32) as i32;
        let ch = if hash2(k, 0, 3) > 0.5 { '0' } else { '1' };
        let a = 0.25 + 0.75 * hash2(k, (t * 12.0) as i32, 5);
        if let Some(e) = cv.get(x, y) {
            if e.ch == ' ' {
                cv.put(x, y, ch, col.mul(a * 0.55), e.bg);
            }
        }
    }
}

// ── 数学可视化 ──────────────────────────────────────────────
/// 隐式心形曲线 (x²+y²−1)³ − x²y³ = 0 的字符光栅化
pub fn heart_curve(cv: &mut Canvas, cx: f32, cy: f32, scale: f32, t: f32, ch: char, col: Rgb, alpha: f32, reveal: f32) {
    let lim = 1.32f32;
    let step_x = 1.0 / scale.max(4.0);
    let mut x = -lim;
    while x <= lim {
        // 只绘制 reveal 进度内的部分
        if (x + lim) / (2.0 * lim) <= reveal + 0.02 {
            let mut y = -1.35;
            while y <= 1.35 {
                let a = x * x + y * y - 1.0;
                let v = a * a * a - x * x * y * y * y;
                if v.abs() < 0.0022 {
                    let px = (cx + x * scale).round() as i32;
                    let py = (cy - y * scale * 0.52).round() as i32;
                    let wob = ((x * 7.0 + t * 3.0).sin() * 0.5 + 0.5) * 0.25 + 0.75;
                    if let Some(e) = cv.get(px, py) {
                        if e.ch == ' ' || e.ch == SKIP {
                            cv.put(px, py, ch, col.mul(alpha * wob), e.bg);
                        } else {
                            cv.put(px, py, ch, e.fg.mix(col, 0.8), e.bg);
                        }
                    }
                    // 描边加粗
                    if let Some(e) = cv.get(px, py + 1) {
                        if (e.ch == ' ') && v.abs() < 0.0009 {
                            cv.put(px, py + 1, ch, col.mul(alpha * 0.5), e.bg);
                        }
                    }
                }
                y += step_x * 1.9;
            }
        }
        x += step_x;
    }
}

/// 正弦波 + 切线（TANGENTS 那句）
pub fn sine_with_tangent(cv: &mut Canvas, rect: Rect, t: f32, phase: f32, amp: f32, col: Rgb, tcol: Rgb, tx: f32) {
    let w = rect.w as f32;
    let h = rect.h as f32;
    let cy = rect.y as f32 + h / 2.0;
    for i in 0..rect.w {
        let u = i as f32 / w;
        let y = cy - (u * 8.0 + phase).sin() * amp;
        let yi = y.round() as i32;
        if yi >= rect.y && yi <= rect.bottom() {
            cv.put(rect.x + i, yi, '·', col, theme::BG);
            let _ = t;
        }
    }
    // 切线：在 x=tx 处
    let ux = (tx * w) as i32;
    if ux >= 0 && ux < rect.w {
        let u = tx;
        let y0 = cy - (u * 8.0 + phase).sin() * amp;
        let slope = -(u * 8.0 + phase).cos() * amp * 8.0 / w;
        for i in -60..60 {
            let x = ux + i;
            if x < 0 || x >= rect.w {
                continue;
            }
            let y = y0 + slope * i as f32;
            let yi = y.round() as i32;
            if yi >= rect.y && yi <= rect.bottom() {
                cv.put(rect.x + x, yi, '─', tcol, theme::BG);
            }
        }
        let py = y0.round() as i32;
        cv.put(rect.x + ux, py, '×', theme::WHITE, theme::BG);
        // 切点标记
        let _ = cw('x');
    }
}

/// 抛物线点云：趋近无穷的可视化
pub fn asymptote(cv: &mut Canvas, rect: Rect, t: f32, col: Rgb) {
    let w = rect.w as f32;
    let h = rect.h as f32;
    let cy = rect.y as f32 + h * 0.62;
    for i in 0..rect.w {
        let x = (i as f32 / w - 0.5) * 8.0;
        let y = 1.0 / (1.0 + (-x).exp());
        let yi = (cy - y * h * 0.5).round() as i32;
        if yi >= rect.y && yi <= rect.bottom() {
            let a = smooth((t - i as f32 / w * 1.6).clamp(0.0, 1.0));
            let _ = a;
            cv.put(rect.x + i, yi, '·', col.mul(0.35 + 0.65 * y), theme::BG);
        }
    }
    // 渐近线
    let ay = (cy - h * 0.5).round() as i32;
    cv.hline(rect.x, rect.right(), ay, '┈', theme::AMBER_DIM, theme::BG);
}

/// 把一条"进度"画成条形（用 block 字符，支持小数）
pub fn bar(cv: &mut Canvas, x: i32, y: i32, w: i32, ratio: f32, fg: Rgb, bg: Rgb, track: Rgb) {
    let r = ratio.clamp(0.0, 1.0);
    let full = r * w as f32;
    for i in 0..w {
        let f = (full - i as f32).clamp(0.0, 1.0);
        let ch = if f >= 0.999 {
            '█'
        } else if f <= 0.001 {
            '░'
        } else {
            HALF[((f * 8.0) as usize).min(7)]
        };
        let col = if f <= 0.001 {
            track.mix(theme::TEXT_FAINT, 0.35)
        } else {
            fg.mix(bg, 1.0 - f)
        };
        cv.put(x + i, y, ch, col, theme::BG);
    }
}

/// 频谱柱：bands 为 [0,1] 的数组
pub fn spectrum(cv: &mut Canvas, x: i32, y: i32, w: i32, h: i32, bands: &[f32], up: bool) {
    let step = (bands.len() as f32 / w as f32).max(1e-6);
    for i in 0..w {
        let bi = ((i as f32 * step) as usize).min(bands.len() - 1);
        let v = bands[bi].clamp(0.0, 1.0);
        let cells = v * h as f32;
        let n = cells as i32;
        for k in 0..h {
            let (yy, frac) = if up {
                (y + h - 1 - k, cells - k as f32)
            } else {
                (y + k, cells - k as f32)
            };
            if k < n {
                let t = (i as f32 / w as f32 * 0.7 + v * 0.45).min(1.0);
                cv.put(x + i, yy, '█', theme::heat(t), theme::BG);
            } else if k == n && frac > 0.12 {
                let idx = ((frac.clamp(0.0, 1.0) * 7.0) as usize).min(7);
                let t = (i as f32 / w as f32 * 0.7 + v * 0.45).min(1.0);
                cv.put(x + i, yy, HALF[idx], theme::heat(t), theme::BG);
            }
        }
    }
}

/// 环形（雷达/催眠）波纹
pub fn rings(cv: &mut Canvas, cx: i32, cy: i32, t: f32, base: f32, n: i32, col: Rgb, sq: bool) {
    for k in 0..n {
        let ph = (t * 0.55 + k as f32 / n as f32).fract();
        let r = base * ph;
        let a = (1.0 - ph) * 0.9;
        let ch = if sq { '□' } else { '·' };
        cv.ellipse(cx, cy, r * 2.2, r, ch, col.mul(a), theme::BG);
    }
}

/// 在矩形里居中画一块"提示条"（终端原生观感）
pub fn toast(cv: &mut Canvas, rect: Rect, text: &str, fg: Rgb, bg: Rgb) {
    let w = sw(text) + 4;
    let x = rect.cx() - w / 2;
    let y = rect.bottom() - 2;
    let r = Rect::new(x, y, w, 1);
    cv.fill(r, ' ', fg, bg);
    cv.text(x + 2, y, text, fg, bg);
}

/// 画一个"文件列表"行（元效果：文件系统）
pub fn fs_row(cv: &mut Canvas, x: i32, y: i32, name: &str, size: &str, state: FsState, t: f32) {
    let (icon, col) = match state {
        FsState::Live => ('▸', theme::TEXT),
        FsState::Fading => ('▸', theme::TEXT_DIM),
        FsState::Gone => ('✕', theme::RED_DIM),
        FsState::Ghost => ('░', theme::TEXT_FAINT),
    };
    cv.put(x, y, icon, col, theme::BG);
    let shown = match state {
        FsState::Gone => format!("{}{}", "\u{0336}".repeat(1), name),
        _ => name.to_string(),
    };
    let wob = if matches!(state, FsState::Gone) {
        (t * 20.0).sin() * 0.15
    } else {
        0.0
    };
    let _ = wob;
    cv.text(x + 2, y, &shown, col, theme::BG);
    cv.text_right(x + 46, y, size, col.mul(0.7), theme::BG);
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FsState {
    Live,
    Fading,
    Gone,
    Ghost,
}

/// 十六进制转储风格的字节流
pub fn hexdump(cv: &mut Canvas, x: i32, y: i32, w: i32, seed: u32, t: f32, col: Rgb) {
    let mut cx = x;
    let mut k = 0;
    while cx < x + w {
        let a = hash2(k, (t * 4.0) as i32, seed);
        let byte = (a * 255.0) as u8;
        let s = format!("{:02X}", byte);
        let c = if byte == 0 { col.mul(0.25) } else { col };
        cv.text(cx, y, &s, c, theme::BG);
        cx += 3;
        k += 1;
    }
    let _ = cw('0');
}

/// 数字滚动（用于时钟 / 计数）
pub fn numeral(cv: &mut Canvas, cx: i32, y: i32, val: f32, digits: i32, col: Rgb, glow: Rgb) {
    let s = format!("{:0width$}", val.abs().round() as i64, width = digits as usize);
    cv.text_glow(cx, y, &s, col, glow, theme::BG);
}
