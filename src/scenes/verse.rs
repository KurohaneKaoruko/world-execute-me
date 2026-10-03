//! 主歌段：几何自述、电与眩晕、刺激与满足、被困模拟。

use crate::bigfont;
use crate::buf::{Frame, Rect};
use crate::fx::{self, Particles};
use crate::fx3d;
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
    pts: Vec<fx3d::Vec3>,
    /// 3D 装饰网格（new() 预计算，draw() 只做旋转投影）
    ico: fx3d::Mesh,
    globe: fx3d::Mesh,
}

impl Geometry {
    pub fn new() -> Self {
        // 斐波那契球面点云
        let n = 420;
        let ga = std::f32::consts::PI * (3.0 - 5.0f32.sqrt());
        let pts = (0..n)
            .map(|i| {
                let y = 1.0 - (i as f32 / (n - 1) as f32) * 2.0;
                let r = (1.0 - y * y).max(0.0).sqrt();
                let th = ga * i as f32;
                fx3d::Vec3::new(th.cos() * r, y, th.sin() * r)
            })
            .collect();
        Geometry {
            mode: String::new(),
            mode_t: 0.0,
            pts,
            ico: fx3d::icosa_mesh(),
            globe: fx3d::latlong_mesh(12, 8, 1.0),
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
    /// 维度阶梯的主舞台：真透视 3D —— 斐波那契球点云（Z-Buffer 深度分层）
    /// + 反向自旋的内核二十面体 + 逐级点亮坐标装饰（点 → 线 → 面 → 体）。
    fn draw_points(&mut self, ctx: &mut Ctx) {
        let w = ctx.w();
        let h = ctx.h();
        let t = self.mode_t;
        let emerge = fx::ease_out((t / 1.1).clamp(0.0, 1.0));
        let dim = ((t / 0.85) as i32).clamp(0, 3);

        let mut r3 = fx3d::R3::new(w, h);
        r3.cam_z = 3.2;
        r3.focal = h as f32 * 1.8;
        r3.begin();
        let span = 5.2;
        let cy = h as f32 / 2.0 - 1.0;
        let breathe = 1.12 * (1.0 + ctx.bass() * 0.05) * emerge;
        let bass = ctx.bass();
        {
            let mut tg = fx3d::Target::new(ctx.c, ctx.r.x, ctx.r.y, w, h);
            // dim ≥ 1：内核正二十面体，反向自旋（先画，让球面点云压在它前面）
            if dim >= 1 {
                let ixf = fx3d::Xform::scaled(-t * 0.9, 0.6, t * 0.35, breathe * 0.52);
                r3.wire(&mut tg, &self.ico, &ixf, theme::AMBER, theme::WHITE, span, 0.95 + bass * 0.3, 0.95);
            }
            // dim ≥ 3：经纬球壳（体 = 有界面的点集）
            if dim >= 3 {
                let gxf = fx3d::Xform::scaled(-t * 0.22, 0.35 * (t * 0.23).sin(), 0.0, breathe * 1.16);
                r3.wire(&mut tg, &self.globe, &gxf, theme::CYAN, theme::WHITE, span, 1.15 + bass * 0.3, 0.9);
            }
            // 主点云最后画：点与点之间的缝隙里透出内核与球壳
            let xf = fx3d::Xform::scaled(t * 0.55, 0.35 * (t * 0.23).sin(), 0.0, breathe);
            r3.cloud(
                &mut tg,
                &self.pts,
                &xf,
                theme::CYAN.mix(theme::WHITE, bass * 0.25),
                theme::BLUE_DIM,
                fx3d::DEPTH_RAMP,
                span,
                0.80 + bass * 0.35,
                0.95,
            );
        }

        // 维度阶梯的 2D 示意层（图纸风格，压在 3D 上）
        let cx = w as f32 / 2.0;
        let rr = r3.focal * 0.5 * breathe / r3.cam_z; // 投影后的赤道半径（行）
        if dim >= 1 {
            ctx.c.hline(
                ctx.r.x + (cx - rr * 2.4) as i32,
                ctx.r.x + (cx + rr * 2.4) as i32,
                ctx.r.y + cy as i32,
                '─',
                theme::TEXT_FAINT.mul(0.8),
                theme::VOID,
            );
        }
        if dim >= 2 {
            ctx.c.ellipse(
                ctx.r.x + cx as i32,
                ctx.r.y + cy as i32,
                rr * 2.05,
                rr,
                '┈',
                theme::CYAN.mul(0.4),
                theme::VOID,
            );
        }
        // 维度阶梯
        let labels = ["dim = 0  ·  point", "dim = 1  ·  line", "dim = 2  ·  plane", "dim = 3  ·  me"];
        ctx.textc_glow(2, labels[dim as usize], theme::WHITE, theme::CYAN);
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
    /// 双球融合用的球面点云（new() 预计算）
    sph: Vec<fx3d::Vec3>,
}

impl Electric {
    pub fn new() -> Self {
        Electric {
            bolts: Vec::new(),
            bolt_t: 0.0,
            sph: fx3d::fib_sphere(300),
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
    /// 3D 示波器：俯视台面上躺着的交流波形，幅度塌陷成直流
    fn ac_dc(&mut self, ctx: &mut Ctx, t: f32) {
        let w = ctx.w();
        let h = ctx.h();
        let bass = ctx.bass();
        let amp = ((1.0 - t / 2.0).max(0.0)) + 0.05;

        let mut r3 = fx3d::R3::new(w, h);
        r3.cam_z = 3.0;
        r3.focal = h as f32 * 2.0;
        r3.begin();
        let span = 6.5;
        {
            let mut tg = fx3d::Target::new(ctx.c, ctx.r.x, ctx.r.y, w, h);
            // 台面向镜头倾倒（绕 x 轴 ~58°）
            let xf = fx3d::Xform::scaled(0.0, 1.02, 0.0, 1.25);
            let floor = fx3d::floor_mesh(13, 0.0);
            r3.wire(&mut tg, &floor, &xf, theme::BLUE_DIM, theme::CYAN_DIM, span, 0.6, 0.8);
            // 波形躺在台面上：AC → DC 的塌陷一目了然
            let n = 110;
            for i in 0..=n {
                let u = i as f32 / n as f32;
                let x = (u - 0.5) * 2.0;
                let ph = x * 9.0 + t * 9.0;
                let mut y = ph.sin() * amp * 0.34;
                if amp < 0.12 {
                    // DC 残留纹波：随低频颤动
                    y = (ph * 2.3).sin() * (0.015 + bass * 0.05);
                }
                let rp = xf.apply(fx3d::Vec3::new(x, y, 0.0));
                let Some((sx, sy, d)) = r3.project(rp) else { continue };
                let dep = r3.depth01(d, span);
                let col = theme::heat((u * 0.7 + 0.2).min(1.0))
                    .mul((1.2 - dep * 0.55) * (0.75 + bass * 0.5));
                let ch = if amp < 0.12 {
                    '='
                } else if i % 4 == 0 {
                    '●'
                } else {
                    '·'
                };
                r3.plot(&mut tg, sx, sy, d, ch, col, 1.0);
            }
        }
        // 台面上的闪电（2D 叠加，bloom 会把它点亮）
        self.bolt_t += ctx.dt;
        if self.bolt_t > 0.09 && t > 0.6 {
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
                ("mode", if amp < 0.12 { "DC" } else { "AC" }),
                ("amp", &format!("{:.3}", amp)),
                ("blind", "vision"),
            ],
        );
    }

    /// 3D 涡旋隧道：碎片从深处被吸向镜头，越近旋转越快、越亮
    fn dizzy(&mut self, ctx: &mut Ctx, t: f32) {
        let w = ctx.w();
        let h = ctx.h();
        let cx = w as f32 / 2.0;
        let cy = h as f32 / 2.0;
        let focal = h as f32 * 2.0;
        let bass = ctx.bass();
        let n = 560;
        for i in 0..n {
            let r0 = 0.25 + fx::hash2(i, 1, 77) * 0.85;
            let phi0 = fx::hash2(i, 2, 77) * fx3d::TAU;
            let speed = 0.22 + fx::hash2(i, 3, 77) * 0.22;
            let q = (t * speed + fx::hash2(i, 4, 77) * 2.0) % 1.0; // 0 深 → 1 近
            let d = 1.9 * (1.0 - q) + 0.10;
            let swirl = phi0 + t * 1.3 + q * (3.0 + r0 * 5.0);
            let sx = cx + focal * 0.34 * r0 * swirl.cos() / d;
            let sy = cy - focal * 0.17 * r0 * swirl.sin() / d;
            if sx < -4.0 || sy < -4.0 || sx > w as f32 + 4.0 || sy > h as f32 + 4.0 {
                continue;
            }
            let heat = (phi0 / fx3d::TAU + swirl * 0.1).fract();
            let col = theme::heat(heat).mul((0.38 + q * q * 1.25) * (0.75 + bass * 0.8));
            let ch = if q > 0.86 {
                '●'
            } else if q > 0.5 {
                '·'
            } else {
                ':'
            };
            ctx.put(sx as i32, sy as i32, ch, col);
        }
        // 同心环
        fx::rings(ctx.c, ctx.r.x + w / 2, ctx.r.y + h / 2, t * 1.4, h as f32 * 0.5, 6, theme::MAGENTA, false);
        // 眩晕文字环绕
        for k in 0..6 {
            let th = t * 1.6 + k as f32 / 6.0 * std::f32::consts::TAU;
            let r = h as f32 * 0.40;
            let x = cx + th.cos() * r * 2.0;
            let y = cy + th.sin() * r * 0.62;
            let a = 0.35 + 0.65 * ((th * 0.5).sin() * 0.5 + 0.5);
            ctx.textb(x as i32 - 4, y as i32, "so dizzy", theme::CYAN.mul(a), theme::VOID);
        }
        ctx.textc_glow(1, "and then blind my vision", theme::WHITE, theme::MAGENTA);
        // 视觉抖动：整屏行偏移
        let shake = (t * 22.0).sin() * (1.0 + bass * 3.0);
        let full = ctx.r;
        for y in full.y..=full.bottom() {
            let d = (shake * ((y as f32 * 0.4).sin())) as i32;
            if d != 0 {
                ctx.c.row_shift(y, d);
            }
        }
    }

    /// 年份超时空：公元 2026 一路倒退进公元前，标签从深处扑向镜头
    fn time_travel(&mut self, ctx: &mut Ctx, t: f32) {
        let w = ctx.w();
        let h = ctx.h();
        let cx = w as f32 / 2.0;
        let cy = h as f32 / 2.0;
        let base = 2026.0f32 - t * 320.0;
        let slots = 18;
        for i in 0..slots {
            let h1 = fx::hash2(i, 3, 913);
            let h2 = fx::hash2(i, 7, 913);
            let q = (t * 0.5 + h1 * 3.0) % 1.0; // 0 远 → 1 近
            let year = (base - q * 130.0 - h2 * 110.0) as i32;
            let label = if year > 0 {
                format!("{year:>4} AD")
            } else {
                format!("{:>4} BC", -year)
            };
            let ux = (h1 - 0.5) * 2.6;
            let uy = (h2 - 0.5) * 1.5;
            let spread = 0.22 + q * q * 2.8;
            let sx = (cx + ux * spread * 30.0) as i32;
            let sy = (cy - uy * spread * 22.0) as i32;
            if sx < 0 || sy < 1 || sx > w - 9 || sy > h - 2 {
                continue;
            }
            let a = 0.20 + q * q * 1.0;
            let col = if year > 0 {
                theme::CYAN.mix(theme::AMBER, q * 0.6)
            } else {
                theme::AMBER
            };
            ctx.text(sx, sy, &label, col.mul(a.min(1.0)));
        }
        // 中央：旋转时钟（时间锚点）
        let cy2 = h / 2;
        let r = (h as f32 * 0.20).min(11.0);
        ctx.c
            .ellipse(w / 2, cy2 + 3, r * 2.0, r, '○', theme::AMBER.mul(0.85), theme::VOID);
        for k in 0..12 {
            let a = k as f32 / 12.0 * std::f32::consts::TAU;
            ctx.put(
                w / 2 + (a.cos() * r * 1.85) as i32,
                cy2 + 3 + (a.sin() * r * 0.92) as i32,
                '·',
                theme::AMBER.mul(0.5),
            );
        }
        let ha = -t * 12.0;
        ctx.c.line(
            w / 2,
            cy2 + 3,
            w / 2 + (ha.cos() * r * 1.6) as i32,
            cy2 + 3 + (ha.sin() * r * 0.8) as i32,
            '─',
            theme::AMBER,
            theme::VOID,
        );
        let ma = -t * 26.0;
        ctx.c.line(
            w / 2,
            cy2 + 3,
            w / 2 + (ma.cos() * r * 1.2) as i32,
            cy2 + 3 + (ma.sin() * r * 0.6) as i32,
            '─',
            theme::CYAN,
            theme::VOID,
        );
        ctx.c.line(
            w / 2 - 20,
            cy2 + 3,
            w / 2 + 20,
            cy2 + 3,
            '─',
            theme::TEXT_FAINT.mul(0.6),
            theme::VOID,
        );
        // 速度线
        for k in 0..26 {
            let y = (k * 7 + (t * 90.0) as i32) % h;
            let a = (k as f32 / 26.0 * 6.28 + t * 3.0).sin() * 0.5 + 0.5;
            ctx.hline(0, 3 + (a * 8.0) as i32, y, '▬', theme::PURPLE.mul(0.25 + a * 0.5));
        }
        ctx.textc_glow(1, "we can travel  ·  A.D → B.C", theme::WHITE, theme::PURPLE);
    }

    /// 双球融合：青/品红两团点云从两侧飞向彼此，合一的瞬间白热爆发
    fn unite(&mut self, ctx: &mut Ctx, t: f32) {
        let w = ctx.w();
        let h = ctx.h();
        let bass = ctx.bass();
        let u = fx::smooth((t / 2.4).clamp(0.0, 1.0));
        let off = (1.0 - u) * 1.45;
        let merged = u >= 0.999;

        let mut r3 = fx3d::R3::new(w, h);
        r3.cam_z = 3.4;
        r3.focal = h as f32 * 2.0;
        r3.begin();
        let span = 5.6;
        {
            let mut tg = fx3d::Target::new(ctx.c, ctx.r.x, ctx.r.y, w, h);
            let xf = fx3d::Xform::scaled(t * 0.5, 0.3, 0.0, 0.62 + u * 0.16 + bass * 0.03);
            let hi = if merged {
                theme::WHITE
            } else {
                theme::CYAN
            };
            let hi2 = if merged { theme::WHITE } else { theme::MAGENTA };
            // 左球（青）
            let a: Vec<fx3d::Vec3> = self.sph.iter().map(|p| fx3d::Vec3::new(p.x - off, p.y, p.z)).collect();
            r3.cloud(&mut tg, &a, &xf, hi, theme::BLUE_DIM, fx3d::DEPTH_RAMP, span, 0.95, 0.95);
            // 右球（品红）
            let b: Vec<fx3d::Vec3> = self.sph.iter().map(|p| fx3d::Vec3::new(p.x + off, p.y, p.z)).collect();
            r3.cloud(&mut tg, &b, &xf, hi2, theme::MAGENTA_DIM, fx3d::DEPTH_RAMP, span, 0.95, 0.95);
        }
        // 合一冲击环
        if t > 2.4 && t < 3.4 {
            let q = (t - 2.4) / 1.0;
            let rr = q * h as f32 * 0.75;
            let a = (1.0 - q) * 0.9;
            ctx.c.ellipse(w / 2, h / 2, rr * 2.1, rr, '●', theme::WHITE.mul(a), theme::VOID);
            ctx.c.ellipse(w / 2, h / 2, rr * 1.6, rr * 0.76, '○', theme::MAGENTA.mul(a * 0.7), theme::VOID);
        }
        // 融合后的白热核心
        if merged {
            let pulse = 0.7 + bass * 0.5;
            let rr = 2.0 + bass * 3.0;
            ctx.c.ellipse(w / 2, h / 2, rr * 2.0, rr, '░', theme::WHITE.mul(pulse * 0.5), theme::VOID);
        }
        ctx.textc_glow(1, "and we can unite  ·  so deeply", theme::WHITE, theme::MAGENTA);
        if u >= 0.999 {
            let txt = "one";
            bigfont::center_glow(
                ctx.c,
                ctx.r.x + w / 2,
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
    /// 3D 反应堆：内核二十面体 + 表面点云
    ico: fx3d::Mesh,
    sph: Vec<fx3d::Vec3>,
}

impl Stimulus {
    pub fn new() -> Self {
        Stimulus {
            parts: Particles::new(),
            last_hit: -1.0,
            ico: fx3d::icosa_mesh(),
            sph: fx3d::fib_sphere(220),
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
        // 每次重音爆发 + 冲击环
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
        let age = t - self.last_hit;
        if age < 0.55 {
            let q = age / 0.55;
            let rr = q * h as f32 * 0.42;
            let a = (1.0 - q) * 0.7;
            ctx.c.ellipse(cx, cy, rr * 2.1, rr, '●', theme::CYAN.mul(a), theme::VOID);
        }
        self.parts.update(ctx.dt, 4.0, 0.5);
        self.parts.draw(ctx.c);

        // 满足度 = 反应堆转速；3D 内核随进度与低频脉动
        let prog = fx::smooth((t / 2.6).clamp(0.0, 1.0));
        let mut r3 = fx3d::R3::new(w, h);
        r3.cam_z = 3.3;
        r3.focal = h as f32 * 2.0;
        r3.begin();
        let span = 5.4;
        let bass = ctx.bass();
        {
            let mut tg = fx3d::Target::new(ctx.c, ctx.r.x, ctx.r.y, w, h);
            let spin = t * (0.5 + prog * 1.6);
            let breathe = 0.92 + bass * 0.10 + prog * 0.14;
            let xf = fx3d::Xform::scaled(spin, 0.42 * (t * 0.4).sin(), spin * 0.6, breathe);
            // 表面点云（发热的反应堆芯）
            r3.cloud(
                &mut tg,
                &self.sph,
                &xf,
                theme::heat((0.3 + prog * 0.6).min(1.0)).mix(theme::WHITE, bass * 0.3),
                theme::BLUE_DIM,
                fx3d::DEPTH_RAMP,
                span,
                0.9,
                0.9,
            );
            // 外壳线框
            let ixf = fx3d::Xform::scaled(-spin * 0.7, 0.3, -spin * 0.4, breathe * 1.22);
            r3.wire(&mut tg, &self.ico, &ixf, theme::CYAN, theme::WHITE, span, 1.0, 0.9);
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
/// 真 3D 牢笼：相机在笼外迎面推进，铁笼缓缓合拢——
/// 笼壁铁栏、地板 / 天花板网格、漂浮数据尘全部由 fx3d 透视渲染，
/// "walls closing" 不再是一块 2D 边框，而是一只逼近的立体囚笼。
pub struct Trapped {
    cage: fx3d::Mesh,
    floor: fx3d::Mesh,
    ceil: fx3d::Mesh,
    motes: Vec<fx3d::Vec3>,
}

impl Trapped {
    pub fn new() -> Self {
        let mut rng = fx::Rng::new(0x7A1E);
        let motes = (0..130)
            .map(|_| {
                fx3d::Vec3::new(
                    rng.range(-1.3, 1.3),
                    rng.range(-1.3, 1.3),
                    rng.range(-1.3, 1.3),
                )
            })
            .collect();
        Trapped {
            cage: fx3d::box_mesh(6),
            floor: fx3d::floor_mesh(9, -1.0),
            ceil: fx3d::floor_mesh(7, 1.0),
            motes,
        }
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

        let shrink = fx::smooth((t / 3.4).clamp(0.0, 1.0));
        let s = 1.0 - shrink * 0.45; // 笼壁：1.0 → 0.55，同时镜头推进，双重压迫
        let span = 6.0;

        // 相机在笼外正前方缓慢推进（2.55 → 2.0）：看着牢笼迎面合拢
        let mut r3 = fx3d::R3::new(w, h);
        r3.cam_z = 2.55 - shrink * 0.55;
        r3.near = 0.10;
        r3.focal = h as f32 * 1.75;
        r3.begin();

        // 低频 + 收缩压力驱动的镜头抖动（绝对时间的确定性函数）
        let tremble = 0.006 + shrink * 0.012 + ctx.bass() * 0.010;
        let jx = ((t * 13.7).sin() + (t * 29.3).sin() * 0.6) * tremble;
        let xf = fx3d::Xform::scaled(t * 0.14, 0.15 * (t * 0.21).sin() + jx, jx * 0.7, s);
        let bass = ctx.bass();
        let cage_col = theme::CYAN.mix(theme::BLUE, 0.3);

        {
            let mut tg = fx3d::Target::new(ctx.c, ctx.r.x, ctx.r.y, w, h);
            // 环绕的数据尘（独立慢速旋转 → 视差层深）
            let mxf = fx3d::Xform::scaled(-t * 0.06, 0.10 * (t * 0.17).sin(), 0.0, 1.55);
            r3.cloud(&mut tg, &self.motes, &mxf, theme::CYAN_DIM, theme::BLUE_DIM, fx3d::DEPTH_RAMP, span, 0.7, 0.8);
            // 地板 + 天花板网格（烘在笼体坐标系里，随笼一起收缩）
            r3.wire(&mut tg, &self.floor, &xf, theme::CYAN_DIM, theme::CYAN, span, 0.75 + bass * 0.35, 0.9);
            r3.wire(&mut tg, &self.ceil, &xf, theme::BLUE_DIM, theme::CYAN_DIM, span, 0.6, 0.8);
            // 铁笼本体：棱上的"栏杆"随收缩向镜头逼近
            r3.wire(&mut tg, &self.cage, &xf, cage_col, theme::WHITE, span, 1.0 + bass * 0.3, 1.0);
        }

        // 背景大字（随低频呼吸）
        let bw = bigfont::width("SIMULATION", 1);
        let breathe = 0.45 + 0.35 * bass;
        bigfont::draw(
            ctx.c,
            ctx.r.x + (w - bw) / 2,
            ctx.r.y + h / 2 - 2,
            "SIMULATION",
            theme::CYAN.mix(theme::BLUE, 0.35).mul(breathe + 0.35),
            theme::VOID,
            1,
        );
        ctx.textc_glow(1, "though we are trapped  ·  in this strange simulation", theme::WHITE, theme::CYAN);
        // 边界告警：脉冲式红闪，不做常驻红洗
        if shrink > 0.75 {
            let bl = ((t * 5.0) as i32) % 2 == 0;
            if bl {
                ctx.textb(3, 3, " WALLS CLOSING ", theme::VOID, theme::RED.mul(0.8));
                ctx.flash(0.10 + 0.06 * bass, theme::RED);
            } else {
                ctx.flash(0.02, theme::RED);
            }
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
