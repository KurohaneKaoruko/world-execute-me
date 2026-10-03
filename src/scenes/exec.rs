//! 处刑：十二连击 → 六国语言倒计时 → 最后的死刑。

use crate::bigfont;
use crate::buf::{Frame, Rect};
use crate::fx::{self, Particles, Rain};
use crate::scenes::{Ctx, Scene};
use crate::theme;

const HITS: [f32; 13] = [
    147.660, 148.600, 149.520, 150.540, 151.520, 152.280, 153.160, 153.980, 155.200, 156.080,
    157.040, 158.000, 161.584,
];
const COUNTS: [(&str, &str, &str); 6] = [
    ("EIN", "DE", "一"),
    ("DOS", "ES", "二"),
    ("TROIS", "FR", "三"),
    ("NE", "KO", "四"),
    ("FEM", "SV", "五"),
    ("LIU", "ZH", "六"),
];
const CTIMES: [f32; 6] = [158.900, 159.321, 159.657, 160.244, 160.693, 161.124];

fn hit_index(t: f32) -> usize {
    HITS.iter().filter(|x| **x <= t).count()
}

fn count_index(t: f32) -> usize {
    CTIMES.iter().filter(|x| **x <= t).count()
}

// ════════════════════════════════════════════════════════════
// 15. Barrage —— 147.660 → 162.632
// ════════════════════════════════════════════════════════════
pub struct Barrage {
    parts: Particles,
    rain: Option<Rain>,
    last: usize,
    hit_t: f32,
    lines: Vec<(f32, i32)>,
    /// 每次连击在随机位置盖章的 "(字数, 年龄, x, y, 序号)"
    stamps: Vec<(f32, i32, i32, i32)>,
    seq: usize,
}

impl Barrage {
    pub fn new() -> Self {
        Barrage {
            parts: Particles::new(),
            rain: None,
            last: 0,
            hit_t: 99.0,
            lines: Vec::new(),
            stamps: Vec::new(),
            seq: 0,
        }
    }
}

/// 把点阵大字按整数倍放大绘制，做出"砸下来"的压迫感
fn big_word(
    cv: &mut crate::buf::Canvas,
    cx: i32,
    y: i32,
    word: &str,
    fg: crate::buf::Rgb,
    bg: crate::buf::Rgb,
    sc: i32,
) {
    let (grid, gw, gh) = bigfont::bitmap(word, 1);
    let x0 = cx - gw * sc / 2;
    for yy in 0..gh {
        for xx in 0..gw {
            if !grid[yy as usize][xx as usize] {
                continue;
            }
            for sy in 0..sc {
                for sx in 0..sc {
                    cv.put(x0 + xx * sc + sx, y + yy * sc + sy, '█', fg, bg);
                }
            }
        }
    }
}

impl Scene for Barrage {
    fn name(&self) -> &'static str {
        "EXECUTION ×12"
    }

    fn draw(&mut self, ctx: &mut Ctx) {
        let t = ctx.t;
        let w = ctx.w();
        let h = ctx.h();
        ctx.clear(theme::VOID);
        let idx = hit_index(t);
        let log_h = (h - 14).clamp(8, 14);
        if idx != self.last {
            self.last = idx;
            self.hit_t = 0.0;
            // 冲击爆发
            let cx = (w / 2) as f32 + ctx.rng.range(-w as f32 * 0.3, w as f32 * 0.3);
            let cy = (h / 2) as f32 + ctx.rng.range(-h as f32 * 0.25, h as f32 * 0.25);
            self.parts.burst(cx, cy, 90, 46.0, 0.9, fx::BLOCK_HEAVY, theme::RED, &mut ctx.rng);
            self.parts.burst(cx, cy, 40, 26.0, 1.4, fx::SPARK, theme::WHITE, &mut ctx.rng);
            self.lines.push((0.0, idx as i32));
            let sx = ctx.rng.irange(28, (w - 28).max(29));
            let sy = ctx.rng.irange(log_h + 5, (h - 11).max(log_h + 6));
            self.stamps.push((0.0, sx, sy, idx as i32));
            self.seq += 1;
        }
        self.hit_t += ctx.dt;

        // 背景：刷屏的处刑日志（放在一块压暗的底板上，保证读得清）
        if self.rain.is_none() {
            let mut r = fx::Rng::new(0xDEADBEEF);
            self.rain = Some(Rain::new(w, h, &mut r));
        }
        let log_h = (h - 14).clamp(8, 14);
        for y in 1..=log_h {
            for x in 0..w {
                let d = 1.0 - y as f32 / log_h as f32;
                ctx.putb(x, y, ' ', theme::TEXT_DIM, theme::PANEL.mul(0.35 + 0.35 * d));
            }
        }
        for (i, l) in self.lines.iter_mut().enumerate() {
            l.0 += ctx.dt;
            let y = (i as i32 * 2) % log_h.max(2);
            let a = (1.0 - l.0 / 2.5).max(0.0);
            let s = format!(
                "[{:02}] EXECUTION  0x{:08X}  SIGKILL → {:}",
                l.1,
                (l.1 as u32).wrapping_mul(0x9E3779B9),
                "me"
            );
            let age_flash = (1.0 - l.0 / 0.35).clamp(0.0, 1.0);
            ctx.textb(
                2,
                y + 1,
                &s,
                theme::RED.mul(a * 0.55).mix(theme::WHITE, age_flash * 0.6),
                theme::PANEL.mul(0.5),
            );
        }
        if self.lines.len() > 24 {
            self.lines.remove(0);
        }

        self.parts.update(ctx.dt, 10.0, 0.25);
        self.parts.draw(ctx.c);

        // 每次连击的"盖章"：一个大 EXECUTION 出现在随机位置然后淡出，砸落带冲击环
        for s in self.stamps.iter_mut() {
            s.0 += ctx.dt;
        }
        self.stamps.retain(|s| s.0 < 0.55);
        for (age, x, y, n) in self.stamps.iter() {
            let a = (1.0 - age / 0.55).max(0.0);
            let _ = n;
            if *age < 0.45 {
                let q = age / 0.45;
                let rr = q * 15.0;
                ctx.c.ellipse(*x, *y + 3, rr * 2.1, rr, '●', theme::RED.mul((1.0 - q) * 0.6), theme::VOID);
            }
            big_word(
                ctx.c,
                *x,
                *y,
                "EXECUTION",
                theme::WHITE.mul(0.25 + 0.75 * a),
                theme::VOID,
                1,
            );
        }

        // 中央大字（命中瞬间 3× 砸落 → 回落 2×，持续弹跳）
        let pop_sc = if self.hit_t < 0.14 { 3 } else { 2 };
        let wave = (self.hit_t * 22.0).sin() * (1.0 - self.hit_t.min(1.0));
        big_word(
            ctx.c,
            w / 2,
            19 + (wave as i32),
            "EXECUTION",
            theme::WHITE.mix(theme::RED, 0.15 + self.hit_t * 0.45),
            theme::VOID,
            pop_sc,
        );

        // 连击计数
        let chip = format!(" EXECUTION {:02}/12 ", idx.min(12));
        let cw = crate::buf::sw(&chip);
        ctx.textb((w - cw) / 2, log_h + 1, &chip, theme::VOID, theme::RED.mul(0.85));
        // 连击点阵
        for k in 0..12 {
            let x = (w - 24) / 2 + k * 2;
            let on = k < idx as i32;
            ctx.put(
                x,
                log_h + 3,
                if on { '●' } else { '○' },
                if on { theme::RED } else { theme::RED_DIM.mul(0.6) },
            );
        }

        // 六国语言倒计时
        let ci = count_index(t);
        if ci > 0 {
            let y = h - 5;
            let total = ci as i32 * 9;
            let x0 = (w - total) / 2;
            for k in 0..ci {
                let (num, lang, zh) = COUNTS[k];
                let appear = 1.0 - ((t - CTIMES[k]) / 0.35).clamp(0.0, 1.0);
                let col = theme::heat(k as f32 / 6.0).mix(theme::WHITE, appear * 0.5);
                // 槽宽 9：数字左对齐，语言/中文固定在第 6 列——
                // 标签纵向成列，且相邻槽位之间至少隔 1 格（TROIS 最长，
                // 按 sw+1 排会与下一槽的 NE 粘成 "FRNE"）
                ctx.textb(x0 + k as i32 * 9, y, num, col, theme::VOID);
                ctx.textb(x0 + k as i32 * 9 + 6, y, lang, theme::TEXT_FAINT, theme::VOID);
                ctx.textb(x0 + k as i32 * 9 + 6, y + 1, zh, theme::TEXT_FAINT.mul(0.8), theme::VOID);
            }
            let since6 = t - CTIMES[5];
            if (0.0..0.7).contains(&since6) {
                let bl = ((t * 8.0) as i32) % 2 == 0;
                if bl {
                    ctx.flash(0.18, theme::RED);
                }
            }
        }

        // 冲击残响：只在"进招"的瞬间闪一下，不要长时间泛红
        if self.hit_t < 0.28 {
            let f = 1.0 - self.hit_t / 0.28;
            ctx.flash(f * 0.16, theme::RED);
            let amp = f * 4.0;
            for y in ctx.r.y..=ctx.r.bottom() {
                let d = ((self.hit_t * 40.0).sin() * amp * ((y as f32 * 0.8).sin())) as i32;
                if d != 0 {
                    ctx.c.row_shift(y, d);
                }
            }
        }
        // 常驻底噪：只落在中央区域，稀疏而克制
        for y in (log_h + 5..h - 6).step_by(2) {
            if ctx.rng.chance(0.30) {
                let x = ctx.rng.irange(0, w - 12);
                let a = 0.12 + ctx.rng.f() * 0.12;
                ctx.text(x, y, "EXECUTION", theme::RED.mul(a));
            }
        }
        // 红黑描边框（最后画，保证边框始终清晰）
        ctx.frame(Rect::new(0, 0, w, h), Frame::Double, theme::RED.mul(0.45 + ctx.bass() * 0.5));
        let _ = &self.seq;
    }
}

// ════════════════════════════════════════════════════════════
// 16. FinalExec —— 162.632 → 177.246
// ════════════════════════════════════════════════════════════
pub struct FinalExec {
    shots: Vec<Shot>,
    parts: Particles,
    last: usize,
    freeze: f32,
}

struct Shot {
    x: f32,
    y: f32,
    vx: f32,
    age: f32,
    n: usize,
}

impl FinalExec {
    pub fn new() -> Self {
        FinalExec {
            shots: Vec::new(),
            parts: Particles::new(),
            last: 0,
            freeze: 0.0,
        }
    }
}

/// 发射时刻（EXECUTION 那句）
const SHOTS_AT: [f32; 3] = [165.166, 168.911, 172.712];

/// 把字符画按整数倍放大贴到舞台坐标上
fn blit_art(ctx: &mut Ctx, x0: i32, y0: i32, art: &[&str], col: crate::buf::Rgb, sc: i32) {
    for (i, line) in art.iter().enumerate() {
        let mut x = x0;
        for ch in line.chars() {
            let cwid = crate::buf::cw(ch);
            if cwid == 0 {
                continue;
            }
            if ch != ' ' {
                for sy in 0..sc {
                    for sx in 0..(cwid * sc) {
                        ctx.put(x + sx, y0 + i as i32 * sc + sy, ch, col);
                    }
                }
            }
            x += cwid * sc;
        }
    }
}

impl Scene for FinalExec {
    fn name(&self) -> &'static str {
        "I WILL RUN THE EXECUTION"
    }

    fn draw(&mut self, ctx: &mut Ctx) {
        let t = ctx.t;
        let w = ctx.w();
        let h = ctx.h();
        let cx = w / 2;
        let cy = h / 2;
        let bass = ctx.bass();
        ctx.clear(theme::VOID);

        // ── 地平线 + 透视地面（越近越密越亮）──
        let horizon = cy + 2;
        for k in 1..=11i32 {
            let y = horizon + (k * k) / 3;
            if y >= h - 1 {
                break;
            }
            let a = 0.16 + 0.62 * (k as f32 / 11.0);
            ctx.hline(0, w - 1, y, '╌', theme::RED_DIM.mul(a));
        }
        // 从消失点散开的地面分割线
        for k in -9..=9i32 {
            let xe = (cx + k * 20) as f32;
            for yy in horizon..h {
                let dy = (yy - horizon) as f32 / (h - horizon).max(1) as f32;
                let x = cx as f32 + (xe - cx as f32) * dy * 1.7;
                if (0.0..w as f32).contains(&x) {
                    ctx.put(x as i32, yy, '·', theme::RED_DIM.mul(0.30 + dy * 0.35));
                }
            }
        }

        // ── 右侧裂口：一道会呼吸的发光裂缝 ──
        let rift = (w - 26) as f32;
        for k in 0..56 {
            let u = k as f32 / 56.0 - 0.5;
            let wob = ((u * 9.0 + t * 1.6).sin() * 1.6) as i32;
            let x = rift + u * 7.0 + wob as f32;
            let y = cy as f32 + u * (h as f32 * 0.84);
            if y < 0.0 || y >= h as f32 {
                continue;
            }
            let a = 0.5 + bass * 0.5;
            ctx.put(x as i32, y as i32, '▌', theme::MAGENTA.mul(a));
            ctx.put(x as i32 + 1, y as i32, '▐', theme::MAGENTA.mul(a * 0.5));
            ctx.glow(x as i32 - 1, y as i32, theme::PURPLE, 0.6);
            ctx.glow(x as i32 + 2, y as i32, theme::CYAN, 0.35);
        }
        // 裂缝里透出来的光
        for k in 0..14 {
            let y = cy - 20 + k * 3;
            if y < 1 || y >= h - 1 {
                continue;
            }
            let a = 0.10 + 0.26 * ((t * 3.0 + k as f32).sin() * 0.5 + 0.5);
            ctx.c
                .ellipse(rift as i32 + 4, y, 7.0, 1.4, '░', theme::MAGENTA.mul(a), theme::VOID);
        }

        // ── 左下角的迫击炮 ──
        let mx = 9;
        let my = h - 5;
        let ang = -0.62f32;
        for k in 0..20 {
            let x = mx as f32 + k as f32;
            let y = my as f32 + k as f32 * ang.tan() * 0.5;
            ctx.put(x as i32, y as i32, '█', theme::TEXT_DIM);
            ctx.put(x as i32, y as i32 + 1, '▀', theme::TEXT_FAINT);
        }
        ctx.hline(mx - 3, mx + 4, my + 2, '▄', theme::AMBER.mul(0.65));
        ctx.text(mx - 4, my + 3, "▲ MORTAR", theme::TEXT_FAINT);

        // ── 发射 ──
        let idx = SHOTS_AT.iter().filter(|x| **x <= t).count();
        let muzzle = (mx as f32 + 18.0, my as f32 - 8.0);
        if idx != self.last {
            self.last = idx;
            for volley in 0..7 {
                self.shots.push(Shot {
                    x: muzzle.0,
                    y: muzzle.1,
                    vx: 62.0 + volley as f32 * 2.0,
                    age: -(volley as f32 * 0.09),
                    n: ctx.rng.irange(0, 8) as usize,
                });
            }
            self.parts.burst(muzzle.0, muzzle.1, 40, 22.0, 0.8, fx::SPARK, theme::AMBER, &mut ctx.rng);
        }
        // 炮口的硝烟常驻
        if ctx.rng.chance(0.55) {
            self.parts.burst(
                muzzle.0,
                muzzle.1,
                2,
                6.0,
                1.6,
                fx::SPARK,
                theme::AMBER.mul(0.7),
                &mut ctx.rng,
            );
        }
        // 裂口处被击中的火星
        if ctx.rng.chance(0.35 + bass * 0.4) {
            self.parts.burst(
                rift as f32,
                cy as f32 + ctx.rng.range(-14.0, 14.0),
                3,
                14.0,
                1.2,
                fx::SPARK,
                theme::MAGENTA,
                &mut ctx.rng,
            );
        }

        let frozen = t > 174.9;
        if frozen {
            self.freeze += ctx.dt;
        }
        let speed_f = if frozen { 0.0 } else { 1.0 };
        for s in self.shots.iter_mut() {
            s.age += ctx.dt * speed_f;
            if s.age < 0.0 {
                continue;
            }
            s.x += s.vx * ctx.dt * speed_f;
            s.y -= s.vx * 0.42 * ctx.dt * speed_f;
            let a = if frozen {
                0.55 + 0.45 * ((self.freeze * 3.0).sin() * 0.5 + 0.5)
            } else {
                1.0
            };
            let words = ["EXECUTION", "EXECUTION", "EXECUTION", "EXECUT", "EXECU", "EXEC", "EXE", "EX"];
            let s2 = words[s.n.min(words.len() - 1)];
            let col = theme::RED.mul(a).mix(theme::WHITE, 0.25 + ctx.rng.f() * 0.2);
            if s.x > 2.0 && s.x < (w - 2) as f32 && s.y > 0.0 && s.y < (h - 1) as f32 {
                ctx.text(s.x as i32 - 2, s.y as i32, s2, col);
            }
            // 拖尾
            for k in 1..5 {
                let tx = (s.x - s.vx * 0.018 * k as f32) as i32;
                let ty = (s.y + s.vx * 0.0075 * k as f32) as i32;
                ctx.put(tx, ty, '·', theme::AMBER.mul(a * (0.55 - k as f32 * 0.1)));
            }
        }
        self.shots.retain(|s| s.x < (w + 30) as f32 && s.age < 5.0);
        self.parts.update(ctx.dt, 2.0, 0.4);
        self.parts.draw(ctx.c);

        // ── 一只手伸向裂口 ──
        let hand = [
            "      ▄▄▄▄",
            "   ▄██████▙",
            "  ▟█▛ ▜█▛ ▜█▙",
            "▟██▘  ▐▌  ▝██▙",
            "▜██▖  ▐▌  ▗██▛",
            "  ▜█▙ ▟█▙ ▟█▛",
            "   ▜██████▛",
            "     ▜██▛",
        ];
        let reach = if frozen { 1.0 } else { ((t - 173.0) / 2.0).clamp(0.0, 1.0) };
        let hx = (rift - 48.0 - (1.0 - reach) * 34.0) as i32;
        let hy = cy - 8;
        blit_art(ctx, hx, hy, &hand, theme::AMBER.mul(0.25 + reach * 0.7), 2);
        if frozen {
            ctx.textc_glow(hy + 18, "we are trapped ah", theme::WHITE.mul(0.85), theme::MAGENTA);
        }

        // ── 右下角读数 ──
        let hxr = w - 26;
        ctx.text(hxr, h - 6, &format!("SHELLS  {:>3}", self.shots.len()), theme::AMBER.mul(0.85));
        ctx.text(hxr, h - 5, &format!("VOLLEY  {:>3}", idx), theme::RED.mul(0.9));
        ctx.text(hxr, h - 4, &format!("RANGE   {:>3}", ((rift as i32 - mx) / 2).max(0)), theme::TEXT_FAINT);

        // ── 顶部说明 ──
        let cap = if t < 165.1 {
            "if i can give them all the"
        } else if t < 168.9 {
            "then i can be your only"
        } else if t < 174.9 {
            "if i can have you back  ·  i will run the"
        } else {
            "we are trapped ah"
        };
        ctx.textc_glow(1, cap, theme::WHITE, theme::RED);

        if !frozen && self.last > 0 && (t - SHOTS_AT[self.last - 1]) < 0.3 {
            let f = 1.0 - (t - SHOTS_AT[self.last - 1]) / 0.3;
            ctx.flash(f * 0.3, theme::AMBER);
        }
    }
}
