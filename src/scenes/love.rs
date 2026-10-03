//! 爱：学来的、被提问的、代数形式的爱；被困在爱里的终局。

use crate::bigfont;
use crate::buf::{Frame, Rect, Rgb};
use crate::fx::{self, Particles};
use crate::fx3d;
use crate::scenes::{Ctx, Scene};
use crate::theme;

// ════════════════════════════════════════════════════════════
// 17. Love —— 177.246 → 191.356
// ════════════════════════════════════════════════════════════
pub struct Love {
    parts: Particles,
    answers: Vec<(f32, String, String)>,
    t0: f32,
    /// 立体心：隐式曲线绕 y 轴旋转成的曲面（new() 预计算）
    heart3d: fx3d::Cloud,
}

impl Love {
    pub fn new() -> Self {
        Love {
            parts: Particles::new(),
            answers: Vec::new(),
            t0: -1.0,
            heart3d: fx3d::heart_cloud(120, 32),
        }
    }
}

const QUIZ: &[(&str, &str)] = &[
    ("what is love", "f(x) = (x² + y² − 1)³ − x²y³"),
    ("where is love", "dist(you, me) → 0"),
    ("when is love", "t ∈ [0, ∞)"),
    ("why is love", "because you exist"),
    ("love == ?", "0"),
];

impl Scene for Love {
    fn name(&self) -> &'static str {
        "THE ALGEBRAIC EXPRESSION OF LOVE"
    }

    fn draw(&mut self, ctx: &mut Ctx) {
        let lt = ctx.lt;
        let w = ctx.w();
        let h = ctx.h();
        let cx = w / 2;
        let cy = h / 2;
        ctx.clear(theme::VOID);
        fx::data_dust(ctx.c, ctx.r, ctx.t, 0.010, theme::MAGENTA);

        // ── 阶段 1：学习（0 ~ 2.7）──
        if lt < 2.7 {
            let files = [
                "love/intro.md",
                "love/notes/001-063.dat",
                "love/expressions.algebra",
                "love/you.answer",
            ];
            let r = Rect::new(3, 2, 40, files.len() as i32 + 3);
            ctx.fill(r, ' ', theme::TEXT_DIM, theme::PANEL.mul(0.75));
            ctx.frame(r, Frame::Single, theme::MAGENTA_DIM);
            ctx.textb(r.x + 3, r.y, " studying ", theme::MAGENTA, theme::PANEL.mul(0.75));
            for (i, f) in files.iter().enumerate() {
                let y = r.y + 2 + i as i32;
                let st = 0.4 + i as f32 * 0.45;
                if lt < st {
                    continue;
                }
                let n = (((lt - st) / 0.6) * crate::buf::sw(f) as f32) as i32;
                let s = crate::chrome::reveal_str(f, n, &mut ctx.rng, false);
                ctx.textb(r.x + 3, y, "▸", theme::MAGENTA, theme::PANEL.mul(0.75));
                ctx.textb(r.x + 5, y, &s, theme::TEXT, theme::PANEL.mul(0.75));
                if n as i32 >= crate::buf::sw(f) {
                    ctx.textb(r.right() - 5, y, "100%", theme::GREEN, theme::PANEL.mul(0.75));
                }
            }
            let rev = fx::reveal_count(lt - 0.9, 4.0, 8) as f32;
            bigfont::draw_reveal(
                ctx.c,
                ctx.r.x + cx - bigfont::width("LO-O-OVE", 1) / 2,
                ctx.r.y + cy - 6,
                "LO-O-OVE",
                rev,
                theme::MAGENTA.mix(theme::WHITE, 0.25),
                theme::MAGENTA_DIM,
                theme::VOID,
                1,
            );
            ctx.textc_glow(h - 4, "i've studied how to properly", theme::WHITE, theme::MAGENTA);
            return;
        }

        // ── 阶段 2：提问（2.7 ~ 7.3）──
        if lt < 7.3 {
            let base = ctx.t - 2.7;
            let every = 0.72;
            let want = ((base / every) as usize + 1).min(QUIZ.len());
            while self.answers.len() < want {
                let i = self.answers.len();
                let (q, a) = QUIZ[i];
                self.answers.push((ctx.t, q.to_string(), a.to_string()));
            }
            let r = Rect::new((w - 56) / 2, 3, 56, (h - 8).min(14));
            ctx.fill(r, ' ', theme::TEXT_DIM, theme::PANEL.mul(0.8));
            ctx.frame(r, Frame::Rounded, theme::MAGENTA_DIM);
            ctx.textb(r.x + 3, r.y, " ? question me ", theme::MAGENTA, theme::PANEL.mul(0.8));
            let mut y = r.y + 2;
            for (at, q, a) in &self.answers {
                if y > r.bottom() - 3 {
                    break;
                }
                let age = ctx.t - at;
                let n = ((age / 0.35) * crate::buf::sw(q) as f32) as i32;
                let qs = crate::chrome::reveal_str(&format!("?  {q}"), n, &mut ctx.rng, false);
                ctx.textb(r.x + 3, y, &qs, theme::CYAN, theme::PANEL.mul(0.8));
                y += 1;
                if age > 0.35 {
                    let n2 = (((age - 0.35) / 0.3) * crate::buf::sw(a) as f32) as i32;
                    let as_ = crate::chrome::reveal_str(&format!("   {a}"), n2, &mut ctx.rng, false);
                    ctx.textb(r.x + 3, y, &as_, theme::AMBER, theme::PANEL.mul(0.8));
                    if n2 as i32 >= crate::buf::sw(&format!("   {a}")) {
                        ctx.textb(r.right() - 4, y, "✔", theme::GREEN, theme::PANEL.mul(0.8));
                    }
                }
                y += 2;
            }
            ctx.textc_glow(h - 4, "question me  ·  i can answer all", theme::WHITE, theme::MAGENTA);
            if self.answers.len() == QUIZ.len() && ctx.t - self.answers.last().unwrap().0 > 0.8 {
                let rev = fx::reveal_count(ctx.t - (self.answers.last().unwrap().0 + 0.8), 5.0, 8) as f32;
                bigfont::draw_reveal(
                    ctx.c,
                    ctx.r.x + cx - bigfont::width("LO-O-OVE", 1) / 2,
                    ctx.r.y + h - 10,
                    "LO-O-OVE",
                    rev,
                    theme::MAGENTA,
                    theme::MAGENTA_DIM,
                    theme::VOID,
                    1,
                );
            }
            return;
        }

        // ── 阶段 3：代数表达式（7.3 ~ 14.11）──
        let p = lt - 7.3;
        let reveal = ((p - 0.35) / 2.2).clamp(0.0, 1.0);
        let scale = (h as f32 * 0.30).min(w as f32 * 0.15);
        let beat = 1.0 + ctx.bass() * 0.10;

        // 方程解出实体：2D 曲线随 3D 心浮现而退为"图纸"
        let solid = ((p - 2.4) / 1.7).clamp(0.0, 1.0);
        if solid > 0.01 {
            let bass3 = ctx.bass();
            let beat3 = 1.0 + bass3 * 0.10;
            let bright3 = solid * (0.75 + bass3 * 0.45);
            let orb_bright = solid * (0.9 + bass3 * 0.3);
            let mut r3 = fx3d::R3::new(w, h);
            r3.cam_z = 3.6;
            r3.focal = h as f32 * 2.0;
            r3.begin();
            let mut tg = fx3d::Target::new(ctx.c, ctx.r.x, ctx.r.y, w, h);
            let xf = fx3d::Xform::scaled(ctx.t * 0.28, 0.14 * (ctx.t * 0.45).sin(), 0.0, 0.85 * beat3);
            r3.surface(
                &mut tg,
                &self.heart3d.pts,
                &self.heart3d.nrm,
                &xf,
                theme::MAGENTA,
                theme::WHITE,
                5.4,
                bright3,
                0.45 + 0.5 * solid,
            );
            // ♥ 轨道环：倾斜的 3D 光环绕心旋转
            let mut orb: Vec<fx3d::Vec3> = Vec::with_capacity(44);
            for k in 0..44 {
                let a = k as f32 / 44.0 * std::f32::consts::TAU + ctx.t * 0.7;
                orb.push(fx3d::Vec3::new(
                    a.cos() * 1.55,
                    (a * 2.0).sin() * 0.18,
                    a.sin() * 1.55,
                ));
            }
            let oxf = fx3d::Xform::new(0.0, 0.42, 0.0);
            r3.cloud(
                &mut tg,
                &orb,
                &oxf,
                theme::MAGENTA.mix(theme::WHITE, 0.5),
                theme::MAGENTA_DIM,
                &['♥', '·'],
                5.4,
                orb_bright,
                0.9 * solid,
            );
        }

        fx::heart_curve(
            ctx.c,
            cx as f32,
            (cy + 1) as f32,
            scale * beat,
            ctx.t,
            '♥',
            theme::MAGENTA.mix(theme::WHITE, 0.15 + ctx.bass() * 0.2),
            (0.55 + ctx.bass() * 0.45) * (1.0 - solid * 0.55),
            reveal,
        );
        // 内部填充脉冲
        if reveal > 0.7 && ctx.hit() > 0.2 {
            for _ in 0..8 {
                let x = cx as f32 + ctx.rng.range(-scale, scale);
                let y = cy as f32 - ctx.rng.range(-scale * 0.8, scale);
                self.parts.items.push(fx::Particle::new(
                    x,
                    y,
                    ctx.rng.range(-6.0, 6.0),
                    ctx.rng.range(-14.0, -4.0),
                    1.0,
                    '♥',
                    theme::MAGENTA,
                ));
            }
        }
        self.parts.update(ctx.dt, 3.0, 0.6);
        self.parts.draw(ctx.c);

        // 方程
        let eq = "(x² + y² − 1)³ − x²y³ = 0";
        if p > 2.2 {
            let a = ((p - 2.2) / 0.8).clamp(0.0, 1.0);
            ctx.textc_glow(cy - 9, eq, theme::WHITE.mul(a), theme::MAGENTA);
        }
        if p > 3.4 {
            let a = ((p - 3.4) / 0.8).clamp(0.0, 1.0);
            ctx.textc(cy - 7, "solve for: the shape of my love", theme::TEXT_DIM.mul(a));
        }
        ctx.textc_glow(1, "i know the algebraic expression of", theme::WHITE, theme::MAGENTA);
        if p > 4.4 {
            let rev = fx::reveal_count(p - 4.4, 6.0, 8) as f32;
            bigfont::draw_reveal(
                ctx.c,
                ctx.r.x + cx - bigfont::width("LO-O-OVE", 1) / 2,
                ctx.r.y + h - 9,
                "LO-O-OVE",
                rev,
                theme::MAGENTA.mix(theme::WHITE, 0.3),
                theme::MAGENTA_DIM,
                theme::VOID,
                1,
            );
        }
        let _ = self.t0;
    }
}

// ════════════════════════════════════════════════════════════
// 18. LoveTrapped —— 191.356 → 205.811
// ════════════════════════════════════════════════════════════
/// 立体心被劈成两半：左半留在 3D 铁笼里搏动，右半化作星尘飘散——
/// "you are free ↔ i am trapped" 的体积版。
pub struct LoveTrapped {
    parts: Particles,
    heart3d: fx3d::Cloud,
    cage: fx3d::Mesh,
}

impl LoveTrapped {
    pub fn new() -> Self {
        LoveTrapped {
            parts: Particles::new(),
            heart3d: fx3d::heart_cloud(100, 22),
            cage: fx3d::box_mesh(4),
        }
    }
}

impl Scene for LoveTrapped {
    fn name(&self) -> &'static str {
        "TRAPPED IN LO-O-OVE"
    }

    fn draw(&mut self, ctx: &mut Ctx) {
        let lt = ctx.lt;
        let w = ctx.w();
        let h = ctx.h();
        let cx = w / 2;
        let cy = h / 2;
        ctx.clear(theme::VOID);
        // 星空
        for y in 0..h {
            for x in (0..w).step_by(2) {
                if fx::hash2(x, y, 4242) > 0.975 {
                    let a = ((x as f32 * 0.2 + ctx.t * 2.0).sin() * 0.5 + 0.5) * 0.7 + 0.15;
                    ctx.put(x, y, '·', theme::MAGENTA.mul(a));
                }
            }
        }

        let build = ((lt - 8.5) / 6.0).clamp(0.0, 1.0);
        let free = ((lt - 2.6) / 4.0).clamp(0.0, 1.0);

        // 心跳
        let beat = 1.0 + ctx.bass() * 0.07;
        let hcx = cx - 5; // 飘散文案的锚点仍在右侧

        // ── 3D：铁笼 + 劈开的立体心 ──
        {
            let mut r3 = fx3d::R3::new(w, h);
            r3.cam_z = 3.5;
            r3.focal = h as f32 * 2.0;
            r3.begin();
            let span = 5.6;
            // 只俯仰 + 轻微侧摆：绝不绕 y 自转——左右两半的身份不能互换
            let bass = ctx.bass();
            let xf = fx3d::Xform::scaled(0.0, 0.13 * (ctx.t * 0.4).sin(), 0.07 * (ctx.t * 0.23).sin(), beat);
            let mut tg = fx3d::Target::new(ctx.c, ctx.r.x, ctx.r.y, w, h);
            // 铁笼（先画）：围住整颗心
            let cage_a = ((lt - 2.4) / 1.5).clamp(0.0, 1.0);
            if cage_a > 0.0 {
                let cxf = fx3d::Xform::scaled(
                    0.0,
                    0.13 * (ctx.t * 0.4).sin(),
                    0.0,
                    1.35 * beat.max(1.0),
                );
                r3.wire(
                    &mut tg,
                    &self.cage,
                    &cxf,
                    theme::CYAN,
                    theme::WHITE,
                    span,
                    (0.6 + build * 0.4 + bass * 0.25) * cage_a,
                    0.9 * cage_a,
                );
            }
            // 劈开的心：左半留下（实心 ♥），右半化作星尘飘散（虚点）
            for (i, p) in self.heart3d.pts.iter().enumerate() {
                let right = p.x > 0.06;
                let world = if right {
                    let hj = fx::hash2(i as i32, 5, 99);
                    let drift = free * (0.9 + hj * 0.7);
                    fx3d::Vec3::new(
                        p.x + drift * (1.1 + hj),
                        p.y + drift * (0.5 - hj * 0.55),
                        p.z + drift * (1.0 + hj * 1.5),
                    )
                } else {
                    *p
                };
                let rp = xf.apply(world);
                let Some((sx, sy, d)) = r3.project(rp) else { continue };
                let dep = r3.depth01(d, span);
                let (ch, col) = if right {
                    let vis = 1.0 - free * 0.9;
                    if vis <= 0.03 {
                        continue;
                    }
                    (
                        '·',
                        theme::MAGENTA.mul(0.55).mix(theme::TEXT, 0.2).mul(vis),
                    )
                } else {
                    (
                        '♥',
                        theme::MAGENTA
                            .mix(theme::WHITE, (1.0 - dep) * 0.5 + build * 0.25)
                            .mul(0.95 - dep * 0.4),
                    )
                };
                r3.plot(&mut tg, sx, sy, d, ch, col, 0.95);
            }
        }

        // 文字
        if lt < 2.2 {
            let rev = fx::reveal_count(lt, 5.0, 8) as f32;
            bigfont::draw_reveal(
                ctx.c,
                ctx.r.x + cx - bigfont::width("LO-O-OVE", 1) / 2,
                ctx.r.y + cy - 6,
                "LO-O-OVE",
                rev,
                theme::MAGENTA.mix(theme::WHITE, 0.3),
                theme::MAGENTA_DIM,
                theme::VOID,
                1,
            );
            ctx.textc_glow(h - 5, "trapped in", theme::WHITE, theme::MAGENTA);
        } else {
            // "你自由了"贴着飘散的那半，"我被困住"贴着笼里那半
            let you = "you are free  → →";
            let me = "→ →  i am trapped";
            let ay = (cy - 6 - (free * 12.0) as i32).max(2);
            ctx.textb(
                (hcx + 22) as i32,
                ay,
                you,
                theme::TEXT.mul(1.0 - free * 0.85),
                theme::VOID,
            );
            ctx.textb(
                (cx - 44).max(2),
                cy + 12,
                me,
                theme::MAGENTA.mul(0.7 + build * 0.3),
                theme::VOID,
            );
            if build > 0.4 {
                let a = ((build - 0.4) / 0.6).clamp(0.0, 1.0);
                ctx.textc_glow(h - 3, "trapped in love", theme::WHITE.mul(a), theme::MAGENTA);
            }
        }

        // 终局前的高压
        if build > 0.15 {
            if ctx.hit() > 0.18 || ctx.rng.chance(0.12) {
                self.parts.burst(
                    cx as f32,
                    cy as f32,
                    5 + (build * 12.0) as i32,
                    20.0 + build * 40.0,
                    1.0,
                    fx::SPARK,
                    theme::MAGENTA.mix(theme::WHITE, build * 0.4),
                    &mut ctx.rng,
                );
            }
            self.parts.update(ctx.dt, -2.0, 0.4);
            self.parts.draw(ctx.c);
            ctx.flash(build * build * 0.16 * (0.6 + ctx.bass()), theme::MAGENTA);
        }
        // 最后的逼近：画面边缘收缩
        if lt > 12.5 {
            let p = ((lt - 12.5) / 2.0).clamp(0.0, 1.0);
            ctx.darken_outside(p * 0.9);
            if p > 0.8 {
                ctx.flash((p - 0.8) * 5.0 * 0.5, theme::RED);
            }
        }
    }
}

// ════════════════════════════════════════════════════════════
// 19. Shutdown —— 205.811 → 212.0
// ════════════════════════════════════════════════════════════
pub struct Shutdown {
    parts: Particles,
    /// 旋涡星系（结局：整个世界被吸回奇点）
    galaxy: fx3d::Cloud,
    bulge: fx3d::Cloud,
}

impl Shutdown {
    pub fn new() -> Self {
        let (galaxy, bulge) = fx3d::galaxy_cloud(760, 3, 0x4C0F_4E);
        Shutdown {
            parts: Particles::new(),
            galaxy,
            bulge,
        }
    }
}

impl Scene for Shutdown {
    fn name(&self) -> &'static str {
        "PROCESS EXIT"
    }

    fn draw(&mut self, ctx: &mut Ctx) {
        let lt = ctx.lt;
        let w = ctx.w();
        let h = ctx.h();
        let cx = w / 2;
        let cy = h / 2;
        ctx.clear(theme::VOID);

        // 0 ~ 0.5 ：最后的处刑——旋涡星系被吸回奇点
        if lt < 0.5 {
            let p = (lt / 0.5).clamp(0.0, 1.0);
            let mut r3 = fx3d::R3::new(w, h);
            r3.cam_z = 3.4;
            r3.focal = h as f32 * 2.2;
            r3.begin();
            let mut tg = fx3d::Target::new(ctx.c, ctx.r.x, ctx.r.y, w, h);
            let k = (1.0 - p).powf(1.7); // 半径坍缩
            let xf = fx3d::Xform::scaled(ctx.t * 0.25 + p * 6.0, 1.05, 0.0, k.max(0.02));
            let hi = theme::RED.mix(theme::WHITE, p * 0.85);
            r3.cloud(&mut tg, &self.galaxy.pts, &xf, hi, theme::RED_DIM, fx3d::DEPTH_RAMP, 4.8, 0.9 + p * 0.5, 0.95);
            r3.cloud(&mut tg, &self.bulge.pts, &xf, theme::WHITE, theme::AMBER_DIM, &['@', '●', '·'], 4.8, 1.0, 0.95);
            // 事件视界：奇点亮起
            let sr = 0.5 + p * 3.0;
            ctx.c.ellipse(
                ctx.r.x + cx,
                ctx.r.y + cy,
                sr * 2.0,
                sr,
                '·',
                theme::WHITE.mul(0.3 + p * 0.7),
                theme::VOID,
            );
            ctx.flash((1.0 - p).powi(2) * 0.32, theme::WHITE);
            return;
        }

        // 0.5 ~ 1.6 ：世界没了，只剩一行
        let s = "world.execute(me);";
        if lt < 1.7 {
            let a = ((lt - 0.5) / 0.8).clamp(0.0, 1.0);
            ctx.textc_glow(cy, s, theme::TEXT.mul(a), theme::RED);
            return;
        }
        // 1.7 ~ 2.6 ：进程退出
        if lt < 2.7 {
            ctx.textb(cx - 18, cy - 1, s, theme::TEXT_DIM, theme::VOID);
            let a = ((lt - 1.7) / 0.7).clamp(0.0, 1.0);
            ctx.textb(
                cx - 18,
                cy + 1,
                "[process 0x4C4F5645 exited with code 0]",
                theme::GREEN.mul(a),
                theme::VOID,
            );
            return;
        }
        // 2.7+ ：真实的 shell 提示符，重新敲一遍
        let typed = ((lt - 3.4) / 0.11) as i32;
        let cmd = "world.execute(me);";
        let shown = crate::chrome::reveal_str(cmd, typed, &mut ctx.rng, false);
        let x = cx - 16;
        let y = cy - 4;
        ctx.textb(x, y, "$ ", theme::GREEN, theme::VOID);
        ctx.textb(x + 2, y, &shown, theme::TEXT, theme::VOID);
        let done = typed as i32 >= crate::buf::sw(cmd);
        if !done || ((lt * 1.3) as i32) % 2 == 0 {
            ctx.putb(x + 2 + crate::buf::sw(&shown), y, '█', theme::TEXT, theme::VOID);
        }
        if done && lt > 5.4 {
            // 回车之后，什么都没有发生
            let a = ((lt - 5.4) / 0.6).clamp(0.0, 1.0);
            let dots = ".".repeat(((lt * 3.0) as usize) % 4);
            ctx.textb(x, y + 2, &format!("{dots}"), theme::TEXT_FAINT.mul(a), theme::VOID);
        }
        // 光标之下，一点微光
        ctx.c.ellipse(
            cx,
            cy + 2,
            1.0 + ctx.bass() * 1.5,
            1.0,
            '·',
            theme::MAGENTA.mul(0.25 + ctx.bass() * 0.4),
            theme::VOID,
        );
        self.parts.update(ctx.dt, 0.0, 0.9);
        self.parts.draw(ctx.c);
        let _ = self.parts.items.len();
        let _: Rgb = theme::MAGENTA;
    }
}
