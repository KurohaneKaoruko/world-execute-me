//! 界面外壳：状态栏、频谱条、进度条、歌词条、帮助浮层。
//! 这些是整部 MV 的"常驻硬件"，场景只负责中间的舞台。

use crate::buf::{cw, sw, Canvas, Rect, Rgb};
use crate::fx::{self, HALF, RAIN};
use crate::lyrics::{Line, Lyrics};
use crate::theme::{self, Kw};
use crate::view::{Link, View};

pub struct Layout {
    pub stage: Rect,
    pub y_spectrum: i32,
    pub y_progress: i32,
    pub y_en: i32,
    pub y_zh: i32,
}

impl Layout {
    pub fn compute(w: i32, h: i32) -> Self {
        Layout {
            stage: Rect::new(0, 1, w, (h - 5).max(3)),
            y_spectrum: h - 4,
            y_progress: h - 3,
            y_en: h - 2,
            y_zh: h - 1,
        }
    }
    pub fn min_size_ok(w: i32, h: i32) -> bool {
        w >= 72 && h >= 22
    }
}

pub fn time_str(t: f32) -> String {
    let t = t.max(0.0);
    let m = (t / 60.0) as i32;
    let s = t - m as f32 * 60.0;
    format!("{m:02}:{s:04.1}")
}

/// 按列宽揭示文本：已揭示的正常显示，未揭示的留空，交界处放乱码。
pub fn reveal_str(text: &str, revealed: i32, rng: &mut fx::Rng, wide_step: bool) -> String {
    let mut o = String::with_capacity(text.len());
    let mut col = 0i32;
    for ch in text.chars() {
        let w = cw(ch);
        if w == 0 {
            continue;
        }
        let step = if wide_step { w } else { w };
        if col + step <= revealed {
            o.push(ch);
        } else if col >= revealed {
            o.push(' ');
        } else if ch == ' ' {
            o.push(' ');
        } else {
            o.push(*rng.pick(RAIN));
        }
        col += step;
    }
    o
}

// ── 状态栏 ──────────────────────────────────────────────────
pub fn draw_header(cv: &mut Canvas, v: &View, ly: &Lyrics, dur: f32) {
    let w = cv.w;
    let bg = theme::PANEL;
    for x in 0..w {
        cv.put(x, 0, ' ', theme::TEXT_DIM, bg);
    }
    let accent = match v.link {
        Link::Linked => theme::CYAN,
        Link::Unstable => theme::AMBER,
        Link::Lost => theme::RED.mul(0.85),
    };
    let puls = 0.74 + 0.26 * v.bass;
    let abg = accent.mul(puls).mix(theme::PANEL, 0.3);
    let mut x = 1;
    let brand = " world.execute(me); ";
    x = cv.text(x, 0, brand, theme::VOID, abg) + 1;
    cv.text(x, 0, &format!("v{}", env!("CARGO_PKG_VERSION")), theme::TEXT_FAINT, bg);
    x += 8;

    let scene = format!(
        "▸ {:02}/{:02}  {}",
        v.scene_idx + 1,
        v.scene_count,
        v.scene_name
    );
    cv.text(x, 0, &scene, theme::CYAN.mul(0.85), bg);
    x += sw(&scene) + 2;

    if let Some(i) = v.lyric_idx {
        let s = format!("LYRIC {:03}/{:03}", i + 1, ly.lines.len());
        cv.text(x, 0, &s, theme::TEXT_DIM, bg);
        x += sw(&s) + 2;
        if let Some(l) = ly.lines.get(i) {
            if l.rep > 1 && x < w / 2 {
                let r = format!("REP×{}", l.rep);
                cv.text(x, 0, &r, theme::AMBER.mul(0.8), bg);
            }
        }
    }

    let mut right: Vec<(String, Rgb)> = Vec::new();
    let state = if v.finished {
        ("■ EOF".to_string(), theme::TEXT_FAINT)
    } else if v.paused {
        ("‖ PAUSED".to_string(), theme::AMBER)
    } else {
        ("▶ PLAY".to_string(), theme::GREEN)
    };
    right.push(state);
    right.push((
        format!("{} / {}", time_str(v.t), time_str(dur)),
        theme::TEXT,
    ));
    let (link_s, link_c) = match v.link {
        Link::Linked => ("● user@localhost".to_string(), theme::GREEN),
        Link::Unstable => ("◐ link unstable".to_string(), theme::AMBER),
        Link::Lost => ("○ NO CARRIER".to_string(), theme::RED.mul(0.9)),
    };
    right.push((link_s, link_c));
    right.push((format!("pid {}", 0x4C4F_5645u32), theme::TEXT_DIM));

    let mut rx = w - 2;
    for (s, c) in right {
        let wdt = sw(&s);
        rx -= wdt;
        if rx < x + 2 {
            break;
        }
        cv.text(rx, 0, &s, c, bg);
        rx -= 2;
    }
}

// ── 频谱条 ──────────────────────────────────────────────────
pub fn draw_spectrum(cv: &mut Canvas, v: &View, y: i32) {
    let w = cv.w;
    for x in 0..w {
        cv.put(x, y, ' ', theme::TEXT_DIM, theme::BG);
    }
    let nb = v.bands.len();
    for i in 0..w {
        let bi = ((i as f32 * nb as f32 / w as f32) as usize).min(nb - 1);
        let val = v.bands[bi].clamp(0.0, 1.0);
        let ch = if val >= 0.995 {
            '█'
        } else if val <= 0.03 {
            '▁'
        } else {
            HALF[((val * 8.0) as usize).min(7)]
        };
        let t = (i as f32 / w as f32 * 0.62 + v.bass * 0.42).min(1.0);
        let col = theme::heat(t).mul(0.35 + 0.75 * val);
        cv.put(i, y, ch, col, theme::BG);
    }
    let title = format!("── {} ──", v.scene_name);
    let tw = sw(&title);
    let x0 = (w - tw) / 2;
    for x in (x0 - 3)..(x0 + tw + 3) {
        if x >= 0 && x < w {
            cv.put(x, y, ' ', theme::TEXT_DIM, theme::BG);
        }
    }
    cv.text(x0, y, &title, theme::CYAN.mul(0.9), theme::BG);
}

// ── 进度条 ──────────────────────────────────────────────────
pub fn draw_progress(cv: &mut Canvas, v: &View, y: i32, dur: f32, marks: &[(f32, &'static str)]) {
    let w = cv.w;
    for x in 0..w {
        cv.put(x, y, ' ', theme::TEXT_DIM, theme::BG);
    }
    let lchip = format!(" {} ", time_str(v.t));
    let rchip = format!(" {} ", time_str(dur));
    let lw = sw(&lchip);
    let rw = sw(&rchip);
    cv.text(1, y, &lchip, theme::VOID, theme::PANEL_HI.mul(0.75));
    cv.text(w - 1 - rw, y, &rchip, theme::TEXT_DIM, theme::PANEL_HI.mul(0.5));

    let x0 = 1 + lw + 1;
    let x1 = w - 2 - rw;
    let bw = (x1 - x0).max(4);
    let ratio = (v.t / dur.max(0.001)).clamp(0.0, 1.0);
    let pos = x0 + (ratio * (bw - 1) as f32) as i32;

    for i in 0..bw {
        let x = x0 + i;
        if x <= pos {
            let t = (i as f32 / bw as f32 * 0.75 + v.bass * 0.3).min(1.0);
            cv.put(x, y, '█', theme::heat(t).mul(0.95), theme::BG);
        } else {
            cv.put(x, y, '░', theme::BLUE_DIM.mul(0.8), theme::BG);
        }
    }
    for (mt, _) in marks {
        let x = x0 + ((mt / dur.max(0.001)).clamp(0.0, 1.0) * (bw - 1) as f32) as i32;
        if x > pos + 1 && x < x1 {
            cv.put(x, y, '┊', theme::BLUE.mul(0.9), theme::BG);
        }
    }
    let ch = if v.paused { '▮' } else { '◆' };
    let pulse_col = theme::WHITE.mix(theme::CYAN, 0.3 + 0.6 * v.hit);
    if pos >= x0 && pos <= x1 {
        cv.put(pos, y, ch, pulse_col, theme::BG);
    }
}

// ── 歌词条 ──────────────────────────────────────────────────
struct Piece {
    x: i32,
    s: String,
    kw: Option<Kw>,
    w: i32,
}

pub fn draw_lyrics(cv: &mut Canvas, v: &View, ly: &Lyrics, ye: i32, yz: i32) {
    let w = cv.w;
    for y in [ye, yz] {
        for x in 0..w {
            let d = ((x - w / 2) as f32 / (w as f32 / 2.0)).abs();
            let c = theme::PANEL.mix(theme::PANEL_HI, 0.35 * (1.0 - d));
            cv.put(x, y, ' ', theme::TEXT_DIM, c);
        }
    }

    let idx = match v.lyric_idx {
        Some(i) => i,
        None => {
            let dots = (v.t * 2.0) as i32 % 4;
            let s = format!("{}▪", "· ".repeat(dots as usize));
            cv.text_center(w / 2, ye, &s, theme::TEXT_FAINT, theme::PANEL);
            cv.text_center(w / 2, yz, "等待信号…", theme::TEXT_FAINT, theme::PANEL);
            return;
        }
    };
    let l: &Line = &ly.lines[idx];
    let el = v.elapsed_in_lyric();
    let flash = fx::pulse(el, 0.02, 0.34);

    // 擦除闪光：以中心向外扩散的亮带
    if flash > 0.01 {
        let rad = (flash * w as f32 * 0.62) as i32;
        let c = w / 2;
        for x in (c - rad)..(c + rad) {
            if x < 0 || x >= w {
                continue;
            }
            let cell = *cv.get(x, ye).unwrap_or(&Default::default());
            let nb = cell.bg.mix(theme::CYAN.mul(0.4), flash * 0.5);
            cv.put(x, ye, ' ', cell.fg, nb);
        }
    }

    // 布局
    let mut pieces: Vec<Piece> = Vec::new();
    let mut x = 0i32;
    for (i, tok) in l.tokens.iter().enumerate() {
        match tok.kw {
            Some(_) => {
                if i > 0 {
                    x += 1;
                }
                let word = tok.s.trim().to_string();
                let wd = sw(&word) + 2;
                pieces.push(Piece {
                    x,
                    s: word,
                    kw: tok.kw,
                    w: wd,
                });
                x += wd;
            }
            None => {
                let wd = sw(&tok.s);
                pieces.push(Piece {
                    x,
                    s: tok.s.clone(),
                    kw: None,
                    w: wd,
                });
                x += wd;
            }
        }
    }
    let total = x;
    let sx = ((w - total) / 2).max(2);

    let rev = fx::reveal_count(el, 54.0, 1_000);
    let mut rng = fx::Rng::new(v.frame * 7919 + 13);
    let mut rng2 = fx::Rng::new(v.frame * 104_729 + 7);

    for p in &pieces {
        let px = sx + p.x;
        match p.kw {
            Some(k) => {
                let (fg, cbg) = k.colors();
                let chip_rev = (rev - p.x).clamp(0, p.w);
                let decoding = chip_rev >= p.w && el < 0.18;
                let bgc = if chip_rev > 0 {
                    cbg.mul(0.85 + 0.4 * v.hit)
                } else {
                    theme::PANEL
                };
                for i in 0..p.w {
                    let cx = px + i;
                    if cx < 0 || cx >= w {
                        continue;
                    }
                    cv.put(cx, ye, ' ', fg, bgc);
                }
                if chip_rev > 0 {
                    let shown = if decoding {
                        fx::scramble(&p.s, 1, &mut rng, false)
                    } else {
                        p.s.clone()
                    };
                    let s = reveal_str(&shown, chip_rev - 1, &mut rng2, false);
                    cv.text(px + 1, ye, &s, fg, bgc);
                    let g = k.accent();
                    for i in 0..(p.w - 1).max(0) {
                        cv.glow(px + 1 + i, ye, g, 0.2);
                    }
                }
            }
            None => {
                let s = reveal_str(&p.s, rev - p.x, &mut rng2, false);
                let col = theme::TEXT.mix(theme::WHITE, 0.35 * flash);
                cv.text(px, ye, &s, col, theme::PANEL);
            }
        }
    }

    let accent = pieces
        .iter()
        .find_map(|p| p.kw)
        .map(|k| k.accent())
        .unwrap_or(theme::CYAN);
    let a = 0.35 + 0.65 * v.bass;
    cv.put(0, ye, '▌', accent.mul(a), theme::PANEL);
    cv.put(w - 1, ye, '▐', accent.mul(a), theme::PANEL);

    // 中文行
    let zrev = fx::reveal_count(el - 0.2, 46.0, 1_000);
    if let Some(zh) = &l.zh {
        let zw = sw(zh);
        let zx = ((w - zw) / 2).max(2);
        let shown = reveal_str(zh, zrev, &mut rng, false);
        let fade = ((el - 0.2) / 0.5).clamp(0.0, 1.0);
        let col = theme::CYAN_DIM.mix(theme::TEXT.mix(theme::CYAN, 0.35), fade);
        cv.text(zx, yz, &shown, col, theme::PANEL);
        let orn = theme::TEXT_FAINT.mul(0.6 + 0.4 * v.bass);
        cv.put((zx - 2).max(0), yz, '╸', orn, theme::PANEL);
        cv.put((zx + zw + 1).min(w - 1), yz, '╺', orn, theme::PANEL);
    } else if l.en.contains("execute") {
        cv.text_center(
            w / 2,
            yz,
            "· · ·  world.execute(me);  · · ·",
            theme::TEXT_FAINT,
            theme::PANEL,
        );
    }
}

// ── 帮助浮层 ────────────────────────────────────────────────
pub fn draw_help(cv: &mut Canvas) {
    let lines = [
        ("SPACE", "暂停 / 继续"),
        ("←  →", "后退 / 前进 5 秒"),
        ("R", "从头重播"),
        ("H", "开关本帮助"),
        ("Q / ESC", "退出"),
        ("", ""),
        ("", "画面、歌词与频谱严格跟随音频位置；"),
        ("", "频谱为启动时离线分析所得（32 段）。"),
        ("", ""),
        ("", "这里是一个程序。它被启动了。"),
    ];
    let w = 58;
    let h = lines.len() as i32 + 4;
    let r = Rect::new((cv.w - w) / 2, (cv.h - h) / 2, w, h);
    cv.fill(r, ' ', theme::TEXT, theme::VOID);
    cv.frame(
        r,
        crate::buf::Frame::Rounded,
        theme::CYAN.mul(0.8),
        theme::VOID,
    );
    cv.text(r.x + 3, r.y, " CONTROLS ", theme::CYAN, theme::VOID);
    for (i, (k, d)) in lines.iter().enumerate() {
        let y = r.y + 2 + i as i32;
        if !k.is_empty() {
            cv.text(r.x + 5, y, k, theme::AMBER, theme::VOID);
        }
        cv.text(r.x + 17, y, d, theme::TEXT_DIM, theme::VOID);
    }
}

/// 启动前的标题卡
pub fn draw_title_card(cv: &mut Canvas, hint: &str, t: f32) {
    let cx = cv.w / 2;
    let cy = cv.h / 2;
    let big = [
        "██╗    ██╗ ██████╗ ██████╗ ██╗     ██████╗ ",
        "██║    ██║██╔═══██╗██╔══██╗██║     ██╔══██╗",
        "██║ █╗ ██║██║   ██║██████╔╝██║     ██║  ██║",
        "██║███╗██║██║   ██║██╔══██╗██║     ██║  ██║",
        "╚███╔███╔╝╚██████╔╝██║  ██║███████╗██████╔╝",
        " ╚══╝╚══╝  ╚═════╝ ╚═╝  ╚═╝╚══════╝╚═════╝ ",
    ];
    let start_y = cy - 7;
    for (i, l) in big.iter().enumerate() {
        let p = (i as f32 / 6.0).clamp(0.0, 1.0);
        let col = theme::CYAN.mix(theme::MAGENTA, p);
        cv.text_center(cx, start_y + i as i32, l, col, theme::BG);
    }
    cv.text_center(cx, start_y + 7, ".execute(me);", theme::MAGENTA, theme::BG);
    cv.text_center(cx, start_y + 9, "Mili — 终端 MV", theme::TEXT_DIM, theme::BG);
    let blink = ((t * 1.6) as i32) % 2 == 0;
    if blink {
        cv.text_center(cx, cy + 6, hint, theme::AMBER, theme::BG);
    }
    cv.text_center(cx, cy + 8, "h 查看操作", theme::TEXT_FAINT, theme::BG);
}

/// 终端太小
pub fn draw_too_small(cv: &mut Canvas) {
    cv.clear();
    let msg = "终端窗口过小，请放大到至少 72 × 22";
    cv.text_center(cv.w / 2, cv.h / 2, msg, theme::AMBER, theme::BG);
    cv.text_center(
        cv.w / 2,
        cv.h / 2 + 2,
        "terminal too small — resize to at least 72×22",
        theme::TEXT_DIM,
        theme::BG,
    );
}
