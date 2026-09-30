//! 主歌段：几何自述、电与眩晕、刺激与满足、被困模拟。

use crate::bigfont;
use crate::buf::{Frame, Rect};
use crate::fx::{self, Particles};
use crate::scenes::{Ctx, Scene};
use crate::theme;

/// 左上角的"公式卡片"，是本段所有画面的信息锚点
fn formula_card(ctx: &mut Ctx, title: &str, lines: &[(&str, &str)]) {
    let w = 30i32;
    let h = lines.len() as i32 + 3;
    let r = Rect::new(2, 1, w, h);
    ctx.fill(r, ' ', theme::TEXT_DIM, theme::PANEL.mul(0.75));
    ctx.frame(r, Frame::Single, theme::CYAN_DIM);
    ctx.textb(r.x + 2, r.y, &format!(" {title} "), theme::CYAN, theme::PANEL.mul(0.75));
    for (i, (k, v)) in lines.iter().enumerate() {
        let y = r.y + 1 + i as i32;
        ctx.textb(r.x + 2, y, k, theme::TEXT_FAINT, theme::PANEL.mul(0.75));
        ctx.textb(r.x + 12, y, v, theme::AMBER, theme::PANEL.mul(0.75));
    }
}

// ════════════════════════════════════════════════════════════
// 3. Geometry —— 29.709 → 44.452
// ════════════════════════════════════════════════════════════
pub struct Geometry {
    mode: String,
    mode_t: f32,
    pts: Vec<(f32, f32, f32)>,
}

impl Geometry {
    pub fn new() -> Self {
        // 斐波那契球面点云
        let n = 420;
        let mut pts = Vec::new();
        let ga = std::f32::consts::PI * (3.0 - 5.0f32.sqrt());
        for i in 0..n {
            let y = 1.0 - (i as f32 / (n - 1) as f32) * 2.0;
            let r = (1.0 - y * y).max(0.0).sqrt();
            let th = ga * i as f32;
            pts.push((th.cos() * r, y, th.sin() * r));
        }
        Geometry {
            mode: String::new(),
            mode_t: 0.0,
            pts,
        }
    }
}

impl Scene for Geometry {
    fn name(&self) -> &'static str {
        "GEOMETRY OF ME"
    }

    fn draw(&mut self, ctx: &mut Ctx) {
        // 当前子场景 = "最近一条带关键词的歌词"。完全由绝对时间推导，
        // 所以拖动进度条之后状态也一定正确（不能靠 dt 累加）。
        let sect_start = ctx.t - ctx.lt;
        let mut mode = String::new();
        let mut mstart = sect_start;
        for l in ctx.ly.lines.iter() {
            if l.t < sect_start {
                continue;
            }
            if l.t > ctx.t {
                break;
            }
            if let Some(tk) = l.tokens.iter().find(|t| t.kw.is_some()) {
                mode = tk.s.trim().to_string();
                mstart = l.t;
            }
        }
        self.mode = mode;
        self.mode_t = (ctx.t - mstart).max(0.0);

        let w = ctx.w();
        let h = ctx.h();
        ctx.clear(theme::VOID);

        // 统一的坐标网格（点阵图纸）
        let gc = theme::BLUE.mul(0.34);
        for x in (0..w).step_by(6) {
            ctx.vline(x, 0, h - 1, '·', gc);
        }
        for y in (0..h).step_by(3) {
            ctx.hline(0, w - 1, y, '·', gc);
        }

        // 底部坐标轴：先画，保证标题文字压在它上面
        let ax = h - 2;
        ctx.hline(2, w - 3, ax, '─', theme::TEXT_FAINT);
        ctx.put(w - 3, ax, '▶', theme::TEXT_FAINT);
        let lx = w / 2;
        ctx.vline(lx, 4, h - 4, '│', theme::TEXT_FAINT.mul(0.8));
        ctx.put(lx, 3, '▲', theme::TEXT_FAINT);
        for i in 0..7 {
            let x = 2 + (w - 6) * i / 6;
            ctx.put(x, ax, '┬', theme::TEXT_FAINT);
        }

        match self.mode.as_str() {
            "CIRCUMFERENCE" => self.draw_circle(ctx),
            "TANGENTS" => self.draw_sine(ctx),
            "LIMITATIONS" => self.draw_limit(ctx),
            _ => self.draw_points(ctx),
        }
    }
}

impl Geometry {
    fn draw_points(&mut self, ctx: &mut Ctx) {
        let w = ctx.w() as f32;
        let h = ctx.h() as f32;
        let cx = w / 2.0;
        let cy = h / 2.0 - 1.0;
        let ry = (h as f32 * 0.30).min(w as f32 * 0.19);
        let rx = ry * 1.9;
        let t = self.mode_t;
        let emerge = fx::ease_out((t / 1.1).clamp(0.0, 1.0));
        let ang = t * 0.55;
        let (sa, ca) = ang.sin_cos();
        let mut zs: Vec<(i32, i32, f32)> = Vec::with_capacity(self.pts.len());
        for (x, y, z) in &self.pts {
            let xr = x * ca - z * sa;
            let zr = x * sa + z * ca;
            let px = cx + xr * rx * emerge;
            let py = cy - y * ry * emerge;
            zs.push((px.round() as i32, py.round() as i32, zr));
        }
        zs.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
        for (i, (px, py, z)) in zs.iter().enumerate() {
            let depth = (z + 1.0) / 2.0;
            let ch = if depth > 0.82 {
                '@'
            } else if depth > 0.55 {
                '●'
            } else if depth > 0.3 {
                '·'
            } else {
                '.'
            };
            let col = theme::CYAN.mix(theme::BLUE_DIM, 1.0 - depth)
                .mul(0.35 + depth * 0.85)
                .mix(theme::MAGENTA, ctx.bass() * 0.25);
            ctx.c.put(
                ctx.r.x + px,
                ctx.r.y + py,
                ch,
                col,
                theme::VOID,
            );
            let _ = i;
        }
        // 维度阶梯
        let steps = t / 0.85;
        let dim = (steps as i32).clamp(0, 3);
        let labels = ["dim = 0  ·  point", "dim = 1  ·  line", "dim = 2  ·  plane", "dim = 3  ·  me"];
        ctx.textc_glow(2, labels[dim as usize], theme::WHITE, theme::CYAN);
        if dim >= 1 {
            for k in 0..14 {
                let u = k as f32 / 13.0;
                let y = cy + (u - 0.5) * ry * 1.5;
                ctx.put((cx - rx * 0.62) as i32, y as i32, '│', theme::CYAN.mul(0.5));
                ctx.put((cx + rx * 0.62) as i32, y as i32, '│', theme::CYAN.mul(0.5));
            }
        }
        if dim >= 2 {
            for k in -8..=8 {
                let y = cy + k as f32 * (ry * 0.11);
                ctx.hline(
                    (cx - rx * 0.68) as i32,
                    (cx + rx * 0.68) as i32,
                    y as i32,
                    '┈',
                    theme::CYAN.mul(0.35),
                );
            }
        }
        formula_card(
            ctx,
            "DIMENSION",
            &[
                ("me", "set of points"),
                ("give", "my dimension"),
                ("dim(me)", &format!("{dim}")),
            ],
        );
    }

    fn draw_circle(&mut self, ctx: &mut Ctx) {
        let w = ctx.w() as f32;
        let h = ctx.h() as f32;
        let cx = w / 2.0;
        let cy = h / 2.0 - 2.0;
        // 横向按 2:1 拉长，所以"展开成直线"的长度必须真的塞得进舞台宽度
        let rad = (h * 0.30).min(w * 0.145).min(11.0);
        let t = self.mode_t;
        let sweep = ((t - 0.20) / 0.90).clamp(0.0, 1.0);
        let unwrap = ((t - 1.50) / 1.40).clamp(0.0, 1.0);
        let u = fx::smooth(unwrap);
        let line_len = std::f32::consts::TAU * rad * 2.0;
        let line_x0 = cx - line_len / 2.0;
        let gy = cy + rad * 1.55;

        // 半径
        let rcol = theme::AMBER.mul(0.75);
        ctx.c
            .line(cx as i32, cy as i32, (cx + rad * 2.0) as i32, cy as i32, '─', rcol, theme::VOID);
        ctx.put((cx + rad) as i32, cy as i32 - 1, 'r', theme::AMBER);
        ctx.put((cx + rad * 2.0) as i32, cy as i32, '●', theme::AMBER);

        let n = 260;
        for i in 0..=n {
            let th = i as f32 / n as f32 * std::f32::consts::TAU;
            if th / std::f32::consts::TAU > sweep && sweep < 1.0 {
                break;
            }
            // 圆上位置
            let cxp = cx + th.cos() * rad * 2.0;
            let cyp = cy + th.sin() * rad;
            // 直线上位置
            let lxp = line_x0 + (th / std::f32::consts::TAU) * line_len;
            let lyp = gy;
            let px = cxp + (lxp - cxp) * u;
            let py = cyp + (lyp - cyp) * u;
            // 曲线必须与背景点阵区分开：用实心点，展开完成后换成等号线
            let ch = if u > 0.82 { '=' } else { '●' };
            let col = theme::CYAN.mix(theme::MAGENTA, th / std::f32::consts::TAU * 0.7);
            ctx.c
                .put(ctx.r.x + px as i32, ctx.r.y + py as i32, ch, col, theme::VOID);
            if (i % 30) == 0 {
                ctx.c.put(
                    ctx.r.x + cxp as i32,
                    ctx.r.y + cyp as i32,
                    '◉',
                    theme::WHITE,
                    theme::VOID,
                );
            }
        }
        // 刻度尺
        if u > 0.5 {
            let a = (u - 0.5) * 2.0;
            for k in 0..=10 {
                let x = line_x0 + k as f32 / 10.0 * line_len;
                ctx.put(x as i32, (gy + 1.0) as i32, '┬', theme::TEXT_FAINT.mul(a));
            }
            ctx.textc_glow(
                (gy + 2.0) as i32,
                "C = 2πr ≈ 6.28318531",
                theme::WHITE.mul(a),
                theme::MAGENTA,
            );
            // 两端标记：起点/终点
            ctx.put(line_x0 as i32, gy as i32, '╶', theme::AMBER.mul(a));
            ctx.put((line_x0 + line_len) as i32, gy as i32, '╴', theme::AMBER.mul(a));
        }
        ctx.textc_glow(2, "unwrap the circumference", theme::WHITE, theme::CYAN);
        formula_card(
            ctx,
            "CIRCUMFERENCE",
            &[
                ("me", "circle"),
                ("r", "1"),
                ("C", "2πr = 6.2832"),
            ],
        );
    }

    fn draw_sine(&mut self, ctx: &mut Ctx) {
        let w = ctx.w();
        let h = ctx.h();
        let rect = Rect::new(4, 4, w - 8, h - 9);
        let t = self.mode_t;
        let amp = (h as f32 * 0.16).min(11.0);
        let phase = -t * 1.7;
        let tx = (t * 0.16) % 1.0;
        fx::sine_with_tangent(
            ctx.c,
            Rect::new(ctx.r.x + rect.x, ctx.r.y + rect.y, rect.w, rect.h),
            t,
            phase,
            amp,
            theme::CYAN,
            theme::AMBER.mul(0.9),
            tx,
        );
        // 切点数值
        let x = (tx - 0.5) * 8.0;
        let f = (x + phase).sin();
        let d = (x + phase).cos();
        ctx.textc_glow(1, "sit on all my tangents", theme::WHITE, theme::AMBER);
        formula_card(
            ctx,
            "TANGENTS",
            &[
                ("f(x)", "sin x"),
                ("f'(x)", "cos x"),
                ("x", &format!("{x:+.2}")),
                ("f'(x)", &format!("{d:+.4}")),
            ],
        );
        let _ = f;
        // 切点扫描线
        let px = rect.x + (tx * rect.w as f32) as i32;
        ctx.vline(px, rect.y, rect.bottom(), '┊', theme::AMBER.mul(0.35));
    }

    fn draw_limit(&mut self, ctx: &mut Ctx) {
        let w = ctx.w();
        let h = ctx.h();
        let rect = Rect::new(4, 5, w - 8, h - 10);
        fx::asymptote(
            ctx.c,
            Rect::new(ctx.r.x + rect.x, ctx.r.y + rect.y, rect.w, rect.h),
            self.mode_t,
            theme::CYAN,
        );
        let t = self.mode_t;
        let x = (10f32).powf((t * 1.2).min(3.2));
        let f = 1.0 / (1.0 + (-x).exp());
        // 采样柱
        let bars_y = h - 6;
        for k in 0..40 {
            let v = 1.0 / (1.0 + (-((k as f32 / 39.0 * 20.0 - 10.0))).exp());
            let ch = if v > 0.97 { '█' } else if v > 0.6 { '▓' } else if v > 0.3 { '▒' } else { '░' };
            ctx.put(3 + k, bars_y, ch, theme::CYAN.mul(0.3 + v * 0.6));
        }
        ctx.textc_glow(1, "approach infinity", theme::WHITE, theme::PURPLE);
        formula_card(
            ctx,
            "LIMITATIONS",
            &[
                ("lim f(x)", "x→∞"),
                ("x", &format!("{x:.1}")),
                ("f(x)", &format!("{f:.9}")),
            ],
        );
        let txt = "you can be my limitations";
        ctx.textc(h - 3, txt, theme::TEXT_DIM);
    }
}


// ════════════════════════════════════════════════════════════
// 4. Electric —— 44.452 → 59.223
// ════════════════════════════════════════════════════════════
pub struct Electric {
    bolts: Vec<Vec<(i32, i32)>>,
    bolt_t: f32,
}

impl Electric {
    pub fn new() -> Self {
        Electric {
            bolts: Vec::new(),
            bolt_t: 0.0,
        }
    }
}

impl Scene for Electric {
    fn name(&self) -> &'static str {
        "AC → DC / DIZZY / TIME"
    }

    fn draw(&mut self, ctx: &mut Ctx) {
        let t = ctx.lt; // 局部时间
        let w = ctx.w();
        let h = ctx.h();
        ctx.clear(theme::VOID);

        if t < 3.2 {
            self.ac_dc(ctx, t);
        } else if t < 7.0 {
            self.dizzy(ctx, t - 3.2);
        } else if t < 10.6 {
            self.time_travel(ctx, t - 7.0);
        } else {
            self.unite(ctx, t - 10.6);
        }
        let _ = (w, h);
    }
}

impl Electric {
    fn ac_dc(&mut self, ctx: &mut Ctx, t: f32) {
        let w = ctx.w();
        let h = ctx.h();
        let cy = h / 2;
        for x in (0..w).step_by(4) {
            ctx.vline(x, 2, h - 3, '·', theme::BLUE_DIM.mul(0.4));
        }
        for y in (2..h - 2).step_by(2) {
            ctx.hline(0, w - 1, y, '·', theme::BLUE_DIM.mul(0.4));
        }
        ctx.hline(0, w - 1, cy, '─', theme::TEXT_FAINT.mul(0.7));

        // AC 幅度逐渐塌陷为 DC
        let amp = ((1.0 - t / 2.0).max(0.0)) * (h as f32 * 0.20) + 0.6;
        let mut prev: Option<(i32, i32)> = None;
        for x in 0..w {
            let ph = x as f32 / 12.0 + t * 9.0;
            let y = cy as f32 - (ph.sin() * amp) - ctx.bass() * (if amp < 2.0 { 0.6 } else { 4.0 }) * (ph * 2.0).sin();
            let p = (x, y.round() as i32);
            let col = theme::heat((x as f32 / w as f32 * 0.6 + 0.2).min(1.0));
            if let Some(pp) = prev {
                if (p.1 - pp.1).abs() > 1 {
                    ctx.c.put(ctx.r.x + p.0, ctx.r.y + p.1, '│', col, theme::VOID);
                }
            }
            ctx.put(p.0, p.1, if amp < 2.0 { '=' } else { '·' }, col);
            prev = Some(p);
        }
        // 闪电
        self.bolt_t += ctx.dt;
        if self.bolt_t > 0.09 && amp < h as f32 * 0.16 {
            self.bolt_t = 0.0;
            let mut b = Vec::new();
            let x0 = ctx.rng.irange(2, w - 2);
            let mut x = x0;
            let mut y = 2;
            while y < h - 2 {
                b.push((x, y));
                x += ctx.rng.irange(-2, 3);
                y += ctx.rng.irange(1, 3);
                x = x.clamp(1, w - 2);
            }
            self.bolts.push(b);
        }
        self.bolts.retain(|b| b.len() > 2);
        for b in &self.bolts {
            for (x, y) in b {
                ctx.put(*x, *y, '⚡', theme::WHITE.mix(theme::CYAN, 0.4));
            }
        }
        if self.bolts.len() > 6 {
            self.bolts.remove(0);
        }

        ctx.textc_glow(2, "Switch my current  ·  AC → DC", theme::WHITE, theme::CYAN);
        formula_card(
            ctx,
            "CURRENT",
            &[
                ("mode", if amp < 2.0 { "DC" } else { "AC" }),
                ("amp", &format!("{:.3}", amp)),
                ("blind", "vision"),
            ],
        );
    }

    fn dizzy(&mut self, ctx: &mut Ctx, t: f32) {
        let w = ctx.w();
        let h = ctx.h();
        let cx = w / 2;
        let cy = h / 2;
        // 螺旋
        let n = 900;
        for i in 0..n {
            let u = i as f32 / n as f32;
            let th = u * 26.0 - t * 3.2;
            let r = u * (h as f32 * 0.46);
            let x = cx as f32 + th.cos() * r * 2.1;
            let y = cy as f32 + th.sin() * r;
            let a = 1.0 - u;
            let col = theme::heat(u).mul(a * (0.5 + ctx.bass() * 0.9));
            ctx.c.put(ctx.r.x + x as i32, ctx.r.y + y as i32, '·', col, theme::VOID);
        }
        // 同心环
        fx::rings(ctx.c, ctx.r.x + cx, ctx.r.y + cy, t * 1.4, h as f32 * 0.5, 7, theme::MAGENTA, false);
        for k in 0..5 {
            let rr = (k as f32 + 1.0) / 5.0;
            let col = theme::MAGENTA.mul(0.25 + 0.5 * ((t * 2.0 + rr * 6.0).sin() * 0.5 + 0.5));
            ctx.c
                .ellipse(cx, cy, h as f32 * 0.46 * rr * 2.1, h as f32 * 0.46 * rr, '○', col, theme::VOID);
        }
        // 眩晕文字环绕
        for k in 0..6 {
            let th = t * 1.6 + k as f32 / 6.0 * std::f32::consts::TAU;
            let r = h as f32 * 0.40;
            let x = cx as f32 + th.cos() * r * 2.0;
            let y = cy as f32 + th.sin() * r * 0.62;
            let a = 0.35 + 0.65 * ((th * 0.5).sin() * 0.5 + 0.5);
            ctx.textb(x as i32 - 4, y as i32, "so dizzy", theme::CYAN.mul(a), theme::VOID);
        }
        ctx.textc_glow(1, "and then blind my vision", theme::WHITE, theme::MAGENTA);
        // 视觉抖动：整屏行偏移
        let shake = (t * 22.0).sin() * (1.0 + ctx.bass() * 3.0);
        let full = ctx.r;
        for y in full.y..=full.bottom() {
            let d = (shake * ((y as f32 * 0.4).sin())) as i32;
            if d != 0 {
                ctx.c.row_shift(y, d);
            }
        }
    }

    fn time_travel(&mut self, ctx: &mut Ctx, t: f32) {
        let w = ctx.w();
        let h = ctx.h();
        let cy = h / 2;
        // 年份反向流逝
        let speed = 420.0 + t * 260.0;
        let base = -(t * speed);
        for row in 0..3 {
            let col = theme::CYAN.mul(0.9 - row as f32 * 0.25);
            let y = cy - 4 + row as i32 * 3;
            ctx.hline(0, w - 1, y, '━', theme::BLUE_DIM.mul(0.5));
            for k in 0..(w / 10 + 2) {
                let yr = base + (k as f32 * 10.0 * (row as f32 + 1.0) * 12.0);
                let x = ((k as f32 * 10.0 - base * 0.02) % (w as f32)).rem_euclid(w as f32);
                if x < 0.0 || x > (w - 8) as f32 {
                    continue;
                }
                let val = if yr >= 0.0 {
                    format!("{:>5.0} AD", yr.min(99999.0))
                } else {
                    format!("{:>5.0} BC", -yr)
                };
                ctx.text(x as i32, y - 1, &val, col.mul(0.4 + 0.6 * (1.0 - x / w as f32)));
            }
        }
        // 中央：旋转时钟
        let cx = w / 2;
        let r = (h as f32 * 0.20).min(11.0);
        ctx.c
            .ellipse(cx, cy + 3, r * 2.0, r, '○', theme::AMBER.mul(0.8), theme::VOID);
        for k in 0..12 {
            let a = k as f32 / 12.0 * std::f32::consts::TAU;
            ctx.put(
                cx + (a.cos() * r * 1.85) as i32,
                cy + 3 + (a.sin() * r * 0.92) as i32,
                '·',
                theme::AMBER.mul(0.5),
            );
        }
        let ha = -t * 12.0;
        ctx.c.line(
            cx,
            cy + 3,
            cx + (ha.cos() * r * 1.6) as i32,
            cy + 3 + (ha.sin() * r * 0.8) as i32,
            '─',
            theme::AMBER,
            theme::VOID,
        );
        let ma = -t * 26.0;
        ctx.c.line(
            cx,
            cy + 3,
            cx + (ma.cos() * r * 1.2) as i32,
            cy + 3 + (ma.sin() * r * 0.6) as i32,
            '─',
            theme::CYAN,
            theme::VOID,
        );
        ctx.c
            .line(cx - 20, cy + 3, cx + 20, cy + 3, '─', theme::TEXT_FAINT.mul(0.6), theme::VOID);

        // 速度线
        for k in 0..26 {
            let y = (k * 7 + (t * 90.0) as i32) % h;
            let a = (k as f32 / 26.0 * 6.28 + t * 3.0).sin() * 0.5 + 0.5;
            ctx.hline(0, 3 + (a * 8.0) as i32, y, '▬', theme::PURPLE.mul(0.25 + a * 0.5));
        }
        ctx.textc_glow(1, "we can travel  ·  A.D → B.C", theme::WHITE, theme::PURPLE);
    }

    fn unite(&mut self, ctx: &mut Ctx, t: f32) {
        let w = ctx.w();
        let h = ctx.h();
        let cx = w / 2;
        let cy = h / 2;
        let u = fx::smooth((t / 2.4).clamp(0.0, 1.0));
        let dx = (1.0 - u) * (h as f32 * 0.30);
        let rr = (h as f32 * 0.16).min(16.0);
        for (sgn, col) in [(-1.0f32, theme::CYAN), (1.0, theme::MAGENTA)] {
            let ex = cx as f32 + sgn * dx * 2.1;
            for i in 0..300 {
                let a = i as f32 / 300.0 * std::f32::consts::TAU;
                let wob = 1.0 + 0.06 * (a * 7.0 + t * 4.0).sin();
                let px = ex + a.cos() * rr * 2.0 * wob;
                let py = cy as f32 + a.sin() * rr * wob;
                ctx.c
                    .put(ctx.r.x + px as i32, ctx.r.y + py as i32, '○', col.mul(0.35 + u * 0.6), theme::VOID);
            }
        }
        // 交叠区
        if u > 0.05 {
            for y in -((rr as i32) * 2)..=((rr as i32) * 2) {
                for x in -((rr as i32) * 3)..=((rr as i32) * 3) {
                    let px = cx + x;
                    let py = cy + y;
                    let d1 = (((x as f32) / 2.1 + dx).powi(2) + (y as f32).powi(2)).sqrt();
                    let d2 = (((x as f32) / 2.1 - dx).powi(2) + (y as f32).powi(2)).sqrt();
                    if d1 < rr && d2 < rr {
                        let a = u * (0.5 + 0.5 * (t * 5.0 + x as f32 * 0.3).sin());
                        ctx.put(px, py, '▒', theme::WHITE.mul(a * 0.75));
                    }
                }
            }
        }
        ctx.textc_glow(1, "and we can unite  ·  so deeply", theme::WHITE, theme::MAGENTA);
        if u >= 0.999 {
            let txt = "one";
            bigfont::center_glow(
                ctx.c,
                ctx.r.x + cx,
                ctx.r.y + h - 9,
                txt,
                theme::WHITE,
                theme::MAGENTA,
                theme::VOID,
                1,
                0.5,
            );
        }
    }
}

// ════════════════════════════════════════════════════════════
// 5. Stimulus —— 59.223 → 64.045
// ════════════════════════════════════════════════════════════
pub struct Stimulus {
    parts: Particles,
    last_hit: f32,
}

impl Stimulus {
    pub fn new() -> Self {
        Stimulus {
            parts: Particles::new(),
            last_hit: -1.0,
        }
    }
}

impl Scene for Stimulus {
    fn name(&self) -> &'static str {
        "STIMULATIONS / SATISFACTION"
    }

    fn draw(&mut self, ctx: &mut Ctx) {
        let t = ctx.lt;
        let w = ctx.w();
        let h = ctx.h();
        let cx = w / 2;
        let cy = h / 2;
        ctx.clear(theme::VOID);
        for y in (0..h).step_by(2) {
            for x in (0..w).step_by(3) {
                ctx.put(x, y, '·', theme::BLUE_DIM.mul(0.35));
            }
        }
        // 每次重音爆发
        if t - self.last_hit > 0.32 && ctx.hit() > 0.25 {
            self.last_hit = t;
            self.parts.burst(
                cx as f32,
                cy as f32,
                26,
                22.0,
                1.1,
                fx::SPARK,
                theme::heat(ctx.rng.f()),
                &mut ctx.rng,
            );
        }
        self.parts.update(ctx.dt, 4.0, 0.5);
        self.parts.draw(ctx.c);

        // 满足度仪表
        let prog = fx::smooth((t / 2.6).clamp(0.0, 1.0));
        let rr = (h as f32 * 0.26).min(13.0);
        ctx.c
            .ellipse(cx, cy, rr * 2.1, rr, '○', theme::CYAN_DIM.mul(0.8), theme::VOID);
        let n = (prog * 120.0) as i32;
        for i in 0..n {
            let a = -std::f32::consts::FRAC_PI_2 + i as f32 / 120.0 * std::f32::consts::TAU;
            let px = cx as f32 + a.cos() * rr * 2.1;
            let py = cy as f32 + a.sin() * rr;
            ctx.c.put(
                ctx.r.x + px as i32,
                ctx.r.y + py as i32,
                '●',
                theme::heat(prog).mul(0.9),
                theme::VOID,
            );
        }
        let pct = (prog * 100.0) as i32;
        let s = format!("{pct}%");
        bigfont::center_glow(
            ctx.c,
            ctx.r.x + cx,
            ctx.r.y + cy - 2,
            &s,
            theme::WHITE,
            theme::heat(prog),
            theme::VOID,
            1,
            0.5,
        );
        ctx.textc(cy + 4, "SATISFACTION", theme::CYAN.mul(0.8));
        ctx.textc_glow(1, "give you all the stimulations", theme::WHITE, theme::CYAN);

        // 下方：执行进度
        let by = h - 4;
        let bw = (w as f32 * 0.7) as i32;
        let p2 = fx::smooth(((t - 2.8) / 1.2).clamp(0.0, 1.0));
        fx::bar(
            ctx.c,
            ctx.r.x + cx - bw / 2,
            ctx.r.y + by,
            bw,
            p2,
            theme::RED,
            theme::AMBER,
            theme::PANEL_HI,
        );
        ctx.textb(cx - bw / 2, by - 1, "RUN THE EXECUTION", theme::RED.mul(0.9), theme::VOID);
        if p2 >= 1.0 {
            let bl = ((t * 6.0) as i32) % 2 == 0;
            ctx.flash(if bl { 0.16 } else { 0.05 }, theme::RED);
        }
    }
}

// ════════════════════════════════════════════════════════════
// 6. Trapped —— 64.045 → 74.045
// ════════════════════════════════════════════════════════════
pub struct Trapped;

impl Trapped {
    pub fn new() -> Self {
        Trapped
    }
}

impl Scene for Trapped {
    fn name(&self) -> &'static str {
        "TRAPPED IN SIMULATION"
    }

    fn draw(&mut self, ctx: &mut Ctx) {
        let t = ctx.lt;
        let w = ctx.w();
        let h = ctx.h();
        ctx.clear(theme::VOID);
        // 先铺透视网格，再把大字压在上面
        for k in 1..16 {
            let u = k as f32 / 16.0;
            let y = (h as f32 / 2.0) + (u * u) * (h as f32 / 2.0);
            if y >= h as f32 {
                break;
            }
            ctx.hline(0, w - 1, y as i32, '─', theme::CYAN_DIM.mul(0.45 + u * 0.5));
        }
        for k in -14..=14 {
            let x = w / 2 + k * 6;
            ctx.c.line(
                ctx.r.x + w / 2,
                ctx.r.y + h / 2,
                ctx.r.x + x,
                ctx.r.y + h - 1,
                '·',
                theme::CYAN_DIM.mul(0.5),
                theme::VOID,
            );
        }
        // 背景大字（随低频呼吸）
        let bw = bigfont::width("SIMULATION", 1);
        let breathe = 0.45 + 0.35 * ctx.bass();
        bigfont::draw(
            ctx.c,
            ctx.r.x + (w - bw) / 2,
            ctx.r.y + h / 2 - 2,
            "SIMULATION",
            theme::CYAN.mix(theme::BLUE, 0.35).mul(breathe + 0.35),
            theme::VOID,
            1,
        );
        // 收缩的牢笼
        let shrink = fx::smooth((t / 3.4).clamp(0.0, 1.0));
        let inset = (shrink * (h as f32 * 0.62)) as i32;
        let cw = (w - inset * 2).max(20);
        let chh = (h - inset).max(6);
        let cell = Rect::new((w - cw) / 2, (h - chh) / 2, cw, chh);
        for yy in cell.y..=cell.bottom() {
            for xx in cell.x..=cell.right() {
                let bar = ((xx + (t * 3.0) as i32) % 7) == 0;
                if bar {
                    ctx.put(xx, yy, '│', theme::CYAN.mul(0.10 + ctx.bass() * 0.12));
                }
            }
        }
        ctx.frame(cell, Frame::Heavy, theme::CYAN.mul(0.55 + ctx.bass() * 0.45));
        // 四角螺栓
        for (x, y) in [
            (cell.x, cell.y),
            (cell.right(), cell.y),
            (cell.x, cell.bottom()),
            (cell.right(), cell.bottom()),
        ] {
            ctx.put(x, y, '◤', theme::WHITE.mul(0.7));
        }
        ctx.textc_glow(1, "though we are trapped  ·  in this strange simulation", theme::WHITE, theme::CYAN);
        // 边界告警
        if shrink > 0.75 {
            let bl = ((t * 5.0) as i32) % 2 == 0;
            if bl {
                ctx.textb(cell.x + 2, cell.y + 1, " WALLS CLOSING ", theme::VOID, theme::RED.mul(0.8));
            }
            let a = 0.05 + 0.10 * ctx.bass();
            ctx.flash(a, theme::RED);
        }
        // 抖动
        let sh = (t * 14.0).sin() * (shrink * 2.0 + ctx.bass() * 1.5);
        for y in ctx.r.y..=ctx.r.bottom() {
            let d = (sh * ((y as f32 * 0.7).sin())) as i32;
            if d != 0 {
                ctx.c.row_shift(y, d);
            }
        }
    }
}
