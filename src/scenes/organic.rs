//! 有机物、神、性别、恍惚：茄子/番茄/猫 → 存在 → 变形 → 共振。

use crate::bigfont;
use crate::buf::{Frame, Rect};
use crate::fx::{self, Particles};
use crate::fx3d;
use crate::scenes::{Ctx, Scene};
use crate::theme;

/// 把多行字符画按整数倍放大贴到舞台中央。
/// 终端格子是 2:1 的，所以两个方向等倍放大 = 保持原图比例。
fn sprite_pass(
    ctx: &mut Ctx,
    art: &[&str],
    colors: &[crate::buf::Rgb],
    col_mul: f32,
    a: f32,
    x0: i32,
    y0: i32,
    sc: i32,
    ox: i32,
    oy: i32,
) {
    for (i, line) in art.iter().enumerate() {
        let col = colors.get(i).copied().unwrap_or(colors[0]).mul(col_mul * a);
        let row0 = y0 + i as i32 * sc + oy;
        let mut x = x0;
        for ch in line.chars() {
            let cwid = crate::buf::cw(ch);
            if cwid == 0 {
                continue;
            }
            if ch != ' ' {
                for sy in 0..sc {
                    for sx in 0..(cwid * sc) {
                        ctx.put(x + sx + ox, row0 + sy, ch, col);
                    }
                }
            }
            x += cwid * sc;
        }
    }
}

fn sprite(
    ctx: &mut Ctx,
    art: &[&str],
    colors: &[crate::buf::Rgb],
    glow: crate::buf::Rgb,
    a: f32,
    dy: i32,
    sc: i32,
) {
    let w = art.iter().map(|l| crate::buf::sw(l)).max().unwrap_or(0) * sc;
    let x0 = (ctx.w() - w) / 2;
    let y0 = ctx.h() / 2 - (art.len() as i32 * sc) / 2 + dy;
    // 先铺一圈暗色轮廓，让主体从背景里"浮"出来
    for (ox, oy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        sprite_pass(ctx, art, colors, 1.0, a * 0.16, x0, y0, sc, ox, oy);
    }
    let _ = glow;
    sprite_pass(ctx, art, colors, 1.0, a, x0, y0, sc, 0, 0);
}

// ════════════════════════════════════════════════════════════
// 7. Organic —— 74.045 → 82.589
// ════════════════════════════════════════════════════════════
/// 茄子轮廓（半宽 → 世界半径），细颈圆腹
fn eggplant_profile() -> Vec<(f32, f32)> {
    let rows: &[f32] = &[
        1.3, 2.6, 3.8, 4.9, 5.9, 6.8, 7.6, 8.2, 8.6, 8.8, 8.9, 8.9, 8.7, 8.4, 8.0, 7.5, 6.9, 6.2,
        5.4, 4.6, 3.8, 3.0, 2.3, 1.6, 1.0,
    ];
    let n = rows.len();
    (0..n)
        .map(|i| {
            let y = 1.25 - 2.5 * i as f32 / (n - 1) as f32;
            (rows[i] / 9.0, y)
        })
        .collect()
}

/// 番茄轮廓：扁圆
fn tomato_profile() -> Vec<(f32, f32)> {
    (0..22)
        .map(|i| {
            let y = 0.86 - 1.72 * i as f32 / 21.0;
            ((1.0 - (y / 0.95).powi(2)).max(0.0).sqrt(), y)
        })
        .collect()
}

pub struct Organic {
    parts: Particles,
    stage: i32,
    parts_t: f32,
    /// 3D 蔬菜：旋转体茄子 / 番茄
    veg: [fx3d::Cloud; 2],
}

impl Organic {
    pub fn new() -> Self {
        Organic {
            parts: Particles::new(),
            stage: -1,
            parts_t: 0.0,
            veg: [
                fx3d::revolve_cloud(&eggplant_profile(), 0.86, 24),
                fx3d::revolve_cloud(&tomato_profile(), 0.9, 24),
            ],
        }
    }
}

const CAT: &[&str] = &[
    "  ▄▄     ▄▄",
    " ▟██▙   ▟██▙",
    "▟█████████████▙",
    "████▀   ● ●   ▀████",
    "███████████████████",
    "▜██████▀▀▀▀▀██████▛",
    " ▜███▛  ▙█▛  ▜███▛",
    "  ▜▛     ▜▛    ▜▛",
];

/// 对称轮廓形状：rows[i] = 第 i 行的半宽（单位=屏幕格，可为小数）。
/// 以 (cx, y0) 为首行中心向下逐行填充；边缘用半块字符收齐，左右严格对称。
fn profile_shape(ctx: &mut Ctx, cx: i32, y0: i32, rows: &[f32], col: crate::buf::Rgb) {
    let n = rows.len() as f32;
    for (i, &hw) in rows.iter().enumerate() {
        let y = y0 + i as i32;
        if hw <= 0.0 {
            continue;
        }
        // 上亮下暗的轻微立体渐变
        let shade = col.mul(1.04 - 0.28 * (i as f32 / n));
        let full = hw.floor() as i32;
        for x in -full..=full {
            ctx.put(cx + x, y, '█', shade);
        }
        // 边缘半块收边：左侧补右半、右侧补左半（对称）
        if hw - full as f32 >= 0.3 {
            let edge = shade.mul(0.72);
            ctx.put(cx - full - 1, y, '▐', edge);
            ctx.put(cx + full + 1, y, '▌', edge);
        }
    }
}

impl Scene for Organic {
    fn name(&self) -> &'static str {
        "NUTRIENTS / ANTIOXIDANTS / PURR"
    }

    fn draw(&mut self, ctx: &mut Ctx) {
        let t = ctx.lt;
        let w = ctx.w();
        let h = ctx.h();
        let cx = w / 2;
        ctx.clear(theme::VOID);
        // 温室的暖调空气：极稀疏的浮尘，而不是满屏纹理
        let warm = match self.stage {
            0 => theme::PURPLE_DIM,
            1 => theme::RED_DIM,
            _ => theme::AMBER_DIM,
        };
        for y in 0..h {
            for x in (0..w).step_by(3) {
                let n = fx::hash2(x, y, 31 + (t * 2.0) as u32);
                if n > 0.955 {
                    let a = 0.35 + 0.65 * (n - 0.955) / 0.045;
                    let breathe = (y as f32 * 0.28 + t * 0.9).sin() * 0.5 + 0.5;
                    ctx.put(x, y, '·', warm.mix(theme::GREEN_DIM, breathe).mul(a * 0.9));
                }
            }
        }
        // 地面
        ctx.hline(0, w - 1, h - 1, '▁', theme::GREEN_DIM.mul(0.5));
        let st = if t < 3.45 {
            0
        } else if t < 5.9 {
            1
        } else {
            2
        };
        // 阶段进度同样由绝对时间推导，保证拖动进度条后依旧正确
        let st_start = match st {
            0 => 0.0,
            1 => 3.45,
            _ => 5.9,
        };
        self.parts_t = t - st_start;
        if st != self.stage {
            self.stage = st;
            let (x, y) = (w as f32 / 2.0, h as f32 / 2.0);
            let col = match st {
                0 => theme::PURPLE,
                1 => theme::RED,
                _ => theme::AMBER,
            };
            self.parts.burst(x, y, 60, 30.0, 1.4, fx::BLOCK_HEAVY, col, &mut ctx.rng);
        }
        let a = fx::smooth((self.parts_t / 0.6).clamp(0.0, 1.0)) * (0.7 + ctx.bass() * 0.5);
        let bass = ctx.bass();
        self.parts.update(ctx.dt, 6.0, 0.7);
        self.parts.draw(ctx.c);

        match st {
            0 => {
                // 3D 茄子：旋转体曲面缓缓自转，绿蒂压在顶端
                let mut r3 = fx3d::R3::new(w, h);
                r3.cam_z = 3.2;
                r3.focal = h as f32 * 2.0;
                r3.begin();
                let mut tg = fx3d::Target::new(ctx.c, ctx.r.x, ctx.r.y, w, h);
                let xf = fx3d::Xform::scaled(t * 0.7, 0.10 * (t * 0.4).sin(), 0.0, 1.02);
                r3.surface(&mut tg, &self.veg[0].pts, &self.veg[0].nrm, &xf, theme::PURPLE, theme::WHITE, 5.2, 0.95 + bass, 0.95);
                // 投在地面的影子
                ctx.c.ellipse(w / 2, h / 2 + 15, 13.0, 2.0, '░', theme::GREEN_DIM.mul(0.5), theme::VOID);
                // 蒂叶与梗（画在曲面之上）
                let top = h as f32 / 2.0 - 15.0;
                ctx.put(cx, top as i32 - 2, '│', theme::GREEN.mul(0.9));
                profile_shape(ctx, cx, top as i32 - 1, &[1.6], theme::GREEN);
                profile_shape(ctx, cx, top as i32, &[2.6], theme::GREEN.mul(0.92));
                ctx.textc_glow(2, "If I'm an eggplant", theme::WHITE, theme::PURPLE);
                bars(ctx, h - 5, "NUTRIENTS", &[0.9, 0.62, 0.34], theme::PURPLE);
            }
            1 => {
                // 3D 番茄：扁圆旋转体
                let mut r3 = fx3d::R3::new(w, h);
                r3.cam_z = 3.2;
                r3.focal = h as f32 * 2.0;
                r3.begin();
                let mut tg = fx3d::Target::new(ctx.c, ctx.r.x, ctx.r.y, w, h);
                let xf = fx3d::Xform::scaled(-t * 0.8, 0.10 * (t * 0.4).sin(), 0.0, 1.05);
                r3.surface(&mut tg, &self.veg[1].pts, &self.veg[1].nrm, &xf, theme::RED, theme::WHITE, 5.2, 0.95 + bass, 0.95);
                ctx.c.ellipse(w / 2, h / 2 + 12, 14.0, 2.0, '░', theme::GREEN_DIM.mul(0.5), theme::VOID);
                let top = h as f32 / 2.0 - 11.0;
                ctx.put(cx, top as i32 - 2, '│', theme::GREEN.mul(0.9));
                profile_shape(ctx, cx, top as i32 - 1, &[1.4, 2.6], theme::GREEN);
                profile_shape(ctx, cx, top as i32, &[4.2, 3.2], theme::GREEN.mul(0.92));
                ctx.textc_glow(2, "If I'm a tomato", theme::WHITE, theme::RED);
                bars(ctx, h - 5, "ANTIOXIDANTS", &[0.94, 0.71, 0.52, 0.30], theme::AMBER);
            }
            _ => {
                // 猫咪舞台：脚下的透视地板 + 呼噜波纹
                {
                    let mut r3 = fx3d::R3::new(w, h);
                    r3.cam_z = 3.0;
                    r3.focal = h as f32 * 2.0;
                    r3.begin();
                    let mut tg = fx3d::Target::new(ctx.c, ctx.r.x, ctx.r.y, w, h);
                    let fl = fx3d::floor_mesh(9, 0.0);
                    let fxf = fx3d::Xform::scaled(0.0, 1.25, 0.0, 1.4);
                    r3.wire(&mut tg, &fl, &fxf, theme::GREEN_DIM, theme::AMBER_DIM, 6.0, 0.55, 0.75);
                }
                let cols = [theme::AMBER.mul(0.95); 8];
                sprite(ctx, CAT, &cols, theme::AMBER, a, -3, 3);
                // 呼噜声：从猫身上扩散的波纹
                for k in 0..3 {
                    let ph = ((self.parts_t * 0.9 + k as f32 / 3.0) % 1.0).max(0.02);
                    let rr = ph * (h as f32 * 0.42);
                    let col = theme::AMBER.mul((1.0 - ph) * 0.6);
                    ctx.c.ellipse(
                        ctx.r.x + w / 2,
                        ctx.r.y + h / 2,
                        rr * 1.9,
                        rr * 0.9,
                        '·',
                        col,
                        theme::VOID,
                    );
                    if (rr as i32) % 2 == 0 {
                        ctx.textb(w / 2 + (rr * 1.4) as i32, h / 2 - 1, "purr", theme::AMBER.mul((1.0 - ph) * 0.8), theme::VOID);
                    }
                }
                ctx.textc_glow(2, "If I'm a tabby cat", theme::WHITE, theme::AMBER);
                let purr = if ctx.bass() > 0.35 { "♪ purrrrrrr… ♪" } else { "♪ purrr… ♪" };
                ctx.textc(h - 5, purr, theme::AMBER.mul(0.5 + ctx.bass()));
                ctx.textc(h - 3, "ENJOYMENT", theme::AMBER);
            }
        }
    }
}

fn bars(ctx: &mut Ctx, y: i32, label: &str, vals: &[f32], col: crate::buf::Rgb) {
    let bw = 18;
    let x0 = ctx.w() / 2 - (bw + 14) / 2;
    ctx.text(x0, y, label, theme::TEXT_DIM);
    for (i, v) in vals.iter().enumerate() {
        fx::bar(
            ctx.c,
            ctx.r.x + x0 + 14,
            ctx.r.y + y + i as i32 - 1,
            bw,
            *v,
            col,
            theme::VOID,
            theme::PANEL_HI,
        );
    }
}

// ════════════════════════════════════════════════════════════
// 8. Deity —— 82.589 → 89.223
// ════════════════════════════════════════════════════════════
/// 神迹的证据：三层不同轴倾角的 3D 陀螺环围绕全视之眼反向旋转——
/// 平面曼陀罗退为衬底，"神"第一次有了体积。
pub struct Deity {
    rings: [fx3d::Cloud; 3],
}

impl Deity {
    pub fn new() -> Self {
        Deity {
            rings: [
                fx3d::torus_cloud(0.92, 0.030, 72, 8),
                fx3d::torus_cloud(0.66, 0.026, 60, 8),
                fx3d::torus_cloud(0.44, 0.022, 48, 6),
            ],
        }
    }
}

impl Scene for Deity {
    fn name(&self) -> &'static str {
        "PROOF OF EXISTENCE"
    }

    fn draw(&mut self, ctx: &mut Ctx) {
        let t = ctx.lt;
        let w = ctx.w();
        let h = ctx.h();
        let cx = w / 2;
        let cy = h / 2 + 1;
        ctx.clear(theme::VOID);

        // ① 背景光晕（先画，免得盖住后面的文字）
        let a0 = 0.10 + ctx.bass() * 0.18;
        for k in (1..=7).rev() {
            let rr = (k as f32 / 7.0) * (h as f32 * 0.46);
            ctx.c.ellipse(
                cx,
                cy,
                rr * 2.2,
                rr,
                '░',
                theme::AMBER.mul(a0 * (1.0 - k as f32 / 7.0) * 0.30),
                theme::VOID,
            );
        }
        // ② 放射神光：从中心向外的细光线，随低频抽搐
        let ray_n = 18;
        for k in 0..ray_n {
            let a = k as f32 / ray_n as f32 * std::f32::consts::TAU + t * 0.08;
            let len = h as f32 * (0.16 + 0.10 * ((t * 2.3 + k as f32 * 1.7).sin() * 0.5 + 0.5)) * (0.7 + ctx.bass() * 0.6);
            for step in 0..((len / 1.4) as i32) {
                let rr = step as f32 * 1.4 + 3.0;
                let x = cx as f32 + a.cos() * rr * 2.1;
                let y = cy as f32 + a.sin() * rr;
                let fade = 1.0 - step as f32 / ((len / 1.4).max(1.0));
                ctx.put(x as i32, y as i32, '·', theme::AMBER.mul(0.30 * fade * (0.5 + ctx.bass() * 0.8)));
            }
        }

        // ③ 平面曼陀罗衬底（由外向内 5 环，奇偶反向）
        let rings = 5;
        for k in 0..rings {
            let u = k as f32 / rings as f32;
            let rr = (1.0 - u) * (h as f32 * 0.44) + 3.0;
            let n = ((rr * 2.4) as i32).max(10);
            let rot = t * (0.15 + (1.0 - u) * 0.7) * if k % 2 == 0 { 1.0 } else { -1.0 };
            let col = theme::AMBER.mul((0.10 + 0.5 * u) * (0.40 + ctx.bass() * 0.7) * 0.55);
            for i in 0..n {
                let a = i as f32 / n as f32 * std::f32::consts::TAU + rot;
                ctx.put(
                    cx + (a.cos() * rr * 2.1) as i32,
                    cy + (a.sin() * rr) as i32,
                    '·',
                    col,
                );
            }
        }

        // ④ 3D 陀螺环：三层，各自绕不同轴倾角自转 + 整体缓慢进动
        {
            let gain = 0.55 + ctx.bass() * 0.55;
            let mut r3 = fx3d::R3::new(w, h);
            r3.cam_z = 3.2;
            r3.focal = h as f32 * 2.4;
            r3.begin();
            let mut tg = fx3d::Target::new(ctx.c, ctx.r.x, ctx.r.y, w, h);
            let span = 5.6;
            for (k, cloud) in self.rings.iter().enumerate() {
                let dir = if k % 2 == 0 { 1.0 } else { -1.0 };
                // 先绕 z 自转（环面在 xy 平面），再倾斜，最后整体进动
                let xf = fx3d::Xform::new(
                    t * 0.13 + k as f32 * 0.8,
                    (0.85 + k as f32 * 0.55) * if k == 1 { -1.0 } else { 1.0 } + 0.10 * (t * 0.4 + k as f32).sin(),
                    t * (0.55 + k as f32 * 0.22) * dir,
                );
                let hot = match k {
                    0 => theme::AMBER,
                    1 => theme::WHITE,
                    _ => theme::MAGENTA,
                };
                r3.surface(&mut tg, &cloud.pts, &cloud.nrm, &xf, theme::AMBER.mul(0.9), hot, span, gain, 0.9);
            }
        }

        // ⑤ 中央之眼
        draw_eye(ctx, cx, cy, h, ctx.bass());

        // ⑥ 存在（大字）
        if t > 3.0 {
            let rev = fx::reveal_count(t - 3.0, 9.0, 9) as f32;
            bigfont::draw_reveal(
                ctx.c,
                ctx.r.x + cx - bigfont::width("EXISTENCE", 1) / 2,
                ctx.r.y + h - 8,
                "EXISTENCE",
                rev,
                theme::WHITE,
                theme::AMBER_DIM,
                theme::VOID,
                1,
            );
        }
        // ⑦ 标题（最后画，保证不被任何东西压住）
        ctx.textc_glow(2, "you're the proof of my existence", theme::WHITE, theme::AMBER);
    }
}

/// 一只画得清楚的"全视之眼"：杏仁轮廓 + 虹膜 + 会随低频收缩的瞳孔。
fn draw_eye(ctx: &mut Ctx, cx: i32, cy: i32, h: i32, bass: f32) {
    let ew = ((h as f32 * 0.30) as i32).clamp(9, 13); // 半宽（列）
    let eh = ((ew as f32 * 0.46).round() as i32).max(3); // 半高（行）

    // 轮廓：每列按透镜公式算上下边界，用半块字符贴合成弧线
    for i in -ew..=ew {
        let u = i as f32 / ew as f32;
        let hh = (1.0 - u * u).max(0.0).sqrt() * eh as f32;
        let hi = hh.round() as i32;
        let col = theme::WHITE.mul(0.55 + 0.45 * (1.0 - u.abs()));
        ctx.put(cx + i, cy - hi, '▄', col);
        ctx.put(cx + i, cy + hi, '▀', col);
        // 两端收成尖角
        if hi == 0 {
            ctx.put(cx + i, cy, '◆', theme::WHITE);
        }
    }

    // 虹膜：实心椭圆，外侧一圈更亮
    let ir = (eh as f32 * 0.95).max(2.0);
    fill_ellipse(ctx, cx, cy, ir * 1.9, ir, '█', theme::MAGENTA.mul(0.75));
    fill_ellipse(ctx, cx, cy, ir * 1.9 - 1.0, ir - 1.0, '█', theme::MAGENTA.mul(1.0));

    // 瞳孔：低频越大越收缩（"看见"的紧张感）
    let pr = (1.2 + (1.0 - bass) * 2.6).max(1.0);
    fill_ellipse(ctx, cx, cy, pr * 2.0, pr, '█', theme::VOID);

    // 高光
    let gx = cx - (ew / 3).max(1);
    let gy = cy - (eh / 3).max(1);
    ctx.put(gx, gy, '·', theme::WHITE);
    ctx.put(gx + 1, gy, '·', theme::WHITE.mul(0.7));

    // 上下睫线
    ctx.hline(cx - ew - 3, cx - ew - 1, cy, '─', theme::AMBER.mul(0.7));
    ctx.hline(cx + ew + 1, cx + ew + 3, cy, '─', theme::AMBER.mul(0.7));
}

/// 填充椭圆（角色格 2:1，xr 已按屏幕比例给出）
fn fill_ellipse(ctx: &mut Ctx, cx: i32, cy: i32, xr: f32, yr: f32, ch: char, col: crate::buf::Rgb) {
    let xr = xr.max(0.0);
    let yr = yr.max(0.0);
    for yy in -(yr.ceil() as i32)..=(yr.ceil() as i32) {
        for xx in -(xr.ceil() as i32)..=(xr.ceil() as i32) {
            let u = xx as f32 / xr.max(0.001);
            let v = yy as f32 / yr.max(0.001);
            if u * u + v * v <= 1.0 {
                ctx.put(cx + xx, cy + yy, ch, col);
            }
        }
    }
}

// ════════════════════════════════════════════════════════════
// 9. Morph —— 89.223 → 101.474
// ════════════════════════════════════════════════════════════
pub struct Morph {
    from: char,
    to: char,
    phase: f32,
    switched: u8,
}

impl Morph {
    pub fn new() -> Self {
        Morph {
            from: 'F',
            to: 'M',
            phase: 0.0,
            switched: 0,
        }
    }
}

impl Scene for Morph {
    fn name(&self) -> &'static str {
        "SWITCH MY GENDER / ROLE"
    }

    fn draw(&mut self, ctx: &mut Ctx) {
        let t = ctx.lt;
        let w = ctx.w();
        let h = ctx.h();
        ctx.clear(theme::VOID);

        // 两段：F→M（gender），S→M（role）——节拍对齐歌词：
        //   Switch my gender 88.59 │ To F to M 90.20 │ Oh switch my role 95.47 │ To S to M 97.74
        // 场景起点 89.223 → 局部 0.00 / 0.97 / 6.24 / 8.52
        let (from, to, t0) = if t < 6.2 { ('F', 'M', 0.95) } else { ('S', 'M', 8.5) };
        if from != self.from {
            self.from = from;
            self.to = to;
        }
        self.phase = ((t - t0) / 1.8).clamp(0.0, 1.0);
        let u = fx::smooth(self.phase);

        // 3D 立体字：有厚度的字块轻轻摇曳，扫描线扫过的部分变成目标字母
        let (fg_grid, gw, gh) = bigfont::bitmap(&self.from.to_string(), 1);
        let (tg_grid, _, _) = bigfont::bitmap(&self.to.to_string(), 1);
        let bw = gw.max(bigfont::width(&self.to.to_string(), 1));
        let sweep = u * (w as f32 * 1.15) - w as f32 * 0.05;

        let mut r3 = fx3d::R3::new(w, h);
        r3.cam_z = 3.2;
        r3.focal = h as f32 * 2.0;
        r3.begin();
        {
            let mut tg3 = fx3d::Target::new(ctx.c, ctx.r.x, ctx.r.y, w, h);
            // 世界单位：屏幕上字宽约 4.6×bw 列
            let su = 4.6 * 3.2 / (h as f32 * 2.0);
            let world_sweep = (sweep - w as f32 / 2.0) * 3.2 / (h as f32 * 2.0);
            let xf = fx3d::Xform::new(0.32 * (t * 0.8).sin(), 0.15 * (t * 0.53).sin(), 0.0);
            let layers = 6;
            for yy in 0..gh {
                for xx in 0..bw {
                    let wx = (xx as f32 - bw as f32 / 2.0) * su;
                    let in_sweep = wx < world_sweep;
                    let ink = if in_sweep {
                        tg_grid[yy as usize].get(xx as usize).copied().unwrap_or(false)
                    } else {
                        fg_grid[yy as usize].get(xx as usize).copied().unwrap_or(false)
                    };
                    if !ink {
                        continue;
                    }
                    let col = if in_sweep { theme::CYAN } else { theme::MAGENTA };
                    for layer in 0..layers {
                        let z = (layer as f32 / (layers - 1) as f32 - 0.5) * 0.55;
                        let p = fx3d::Vec3::new(wx, (gh as f32 / 2.0 - yy as f32) * su, z);
                        let rp = xf.apply(p);
                        let Some((sx, sy, d)) = r3.project(rp) else { continue };
                        let zshade = 0.45 + 0.55 * ((z + 0.275) / 0.55);
                        let wob = 0.75 + 0.25 * ((sx * 0.3 - t * 4.0).sin() * 0.5 + 0.5);
                        r3.plot(&mut tg3, sx, sy, d, '█', col.mul(zshade * wob), 0.95);
                    }
                }
            }
        }
        // 扫描线本体
        if u < 1.0 {
            let cy0 = h / 2;
            for yy in (cy0 - 8)..=(cy0 + 8) {
                ctx.put(sweep as i32, yy, '▌', theme::WHITE);
            }
        }
        // 粒子尾迹
        if u > 0.02 && u < 0.99 {
            for _ in 0..6 {
                let x = sweep as i32 + ctx.rng.irange(-6, 6);
                let y = h / 2 + ctx.rng.irange(-6, 6);
                let ch = *ctx.rng.pick(fx::SPARK);
                ctx.put(x, y, ch, theme::CYAN.mul(0.8));
            }
        }

        // 两侧标签
        let lbl_l = if t < 6.2 { "F" } else { "S" };
        let lbl_r = "M";
        bigfont::center(ctx.c, ctx.r.x + 12, ctx.r.y + h / 2 - 2, lbl_l, theme::MAGENTA_DIM.mul(0.9), theme::VOID, 1);
        bigfont::center(ctx.c, ctx.r.x + w - 12, ctx.r.y + h / 2 - 2, lbl_r, theme::CYAN_DIM.mul(0.9), theme::VOID, 1);

        // 顶部信息
        let (cap, sub) = if t < 6.2 {
            ("switch my gender", "F → M")
        } else {
            ("switch my role", "S → M")
        };
        ctx.textc_glow(1, cap, theme::WHITE, theme::CYAN);
        ctx.textc(2, sub, theme::AMBER);

        // 底部 AM→PM 时钟
        let cyc = (t * 1.15) % 1.0;
        let lbl = if cyc < 0.5 { "AM" } else { "PM" };
        let bx = (w as f32 * 0.25) as i32;
        let bwid = (w as f32 * 0.5) as i32;
        ctx.textc(h - 4, &format!("from AM to PM   ▸  {}", lbl), theme::TEXT_DIM);
        fx::bar(
            ctx.c,
            ctx.r.x + bx,
            ctx.r.y + h - 3,
            bwid,
            cyc,
            theme::AMBER,
            theme::PURPLE,
            theme::PANEL_HI,
        );

        if t > 10.6 {
            let bl = ((t * 4.0) as i32) % 2 == 0;
            if bl {
                ctx.textc_glow(h - 6, "we can enter  ·  the trance", theme::WHITE, theme::PURPLE);
            }
        }
        let _ = self.switched;
    }
}

// ════════════════════════════════════════════════════════════
// 10. Trance —— 101.474 → 110.900
// ════════════════════════════════════════════════════════════
pub struct Trance {
    parts: Particles,
    /// 超时空星流：恍惚 = 世界在耳边飞速后退
    warp: fx3d::Warp,
    /// 3D 悬浮转环
    rings: [fx3d::Cloud; 3],
}

impl Trance {
    pub fn new() -> Self {
        Trance {
            parts: Particles::new(),
            warp: fx3d::Warp::new(240, 0x7ECE),
            rings: [
                fx3d::torus_cloud(0.92, 0.020, 70, 6),
                fx3d::torus_cloud(0.64, 0.018, 56, 6),
                fx3d::torus_cloud(0.40, 0.016, 44, 6),
            ],
        }
    }
}

impl Scene for Trance {
    fn name(&self) -> &'static str {
        "VIBRATIONS → COMPLETION"
    }

    fn draw(&mut self, ctx: &mut Ctx) {
        let t = ctx.lt;
        let w = ctx.w();
        let h = ctx.h();
        let cx = w / 2;
        let cy = h / 2;
        ctx.clear(theme::VOID);

        // COMPLETION 进度决定星流的"减速入站"
        let comp = ((t - 6.6) / 2.6).clamp(0.0, 1.0);
        let speed = (0.085 + ctx.bass() * 0.13) * (1.0 - comp * 0.75);
        let near = theme::CYAN.mix(theme::GREEN, comp * 0.8).mix(theme::WHITE, ctx.bass() * 0.3);
        let warp_bright = 0.75 + ctx.bass() * 0.55;
        {
            let mut tg = fx3d::Target::new(ctx.c, ctx.r.x, ctx.r.y, w, h);
            self.warp.draw(
                &mut tg,
                cx as f32,
                cy as f32,
                h as f32 * 2.0,
                t,
                speed,
                near,
                theme::PURPLE_DIM,
                warp_bright,
            );
        }

        // 3D 悬浮转环（恍惚中围绕镜头的三只陀螺环）
        {
            let ring_bright = (0.95 + ctx.bass() * 0.7) * (1.0 - comp * 0.35);
            let mut r3 = fx3d::R3::new(w, h);
            r3.cam_z = 3.2;
            r3.focal = h as f32 * 2.4;
            r3.begin();
            let mut tg = fx3d::Target::new(ctx.c, ctx.r.x, ctx.r.y, w, h);
            for (k, cloud) in self.rings.iter().enumerate() {
                let dir = if k % 2 == 0 { 1.0 } else { -1.0 };
                let xf = fx3d::Xform::new(
                    t * 0.15 + k as f32 * 0.8,
                    (0.85 + k as f32 * 0.5) * if k == 1 { -1.0 } else { 1.0 } + 0.12 * (t * 0.4 + k as f32).sin(),
                    t * (0.5 + k as f32 * 0.25) * dir,
                );
                let base = if k == 1 { theme::CYAN } else { theme::PURPLE };
                r3.surface(&mut tg, &cloud.pts, &cloud.nrm, &xf, base, theme::WHITE, 5.6, ring_bright, 0.9);
            }
        }

        // 催眠同心环（2D 衬底）
        for k in 0..10 {
            let ph = ((t * 0.5 + k as f32 / 16.0) % 1.0).max(0.001);
            let rr = ph * (h as f32 * 0.55);
            let a = (1.0 - ph).powf(1.6);
            let col = theme::PURPLE.mix(theme::CYAN, ph);
            let ch = if k % 3 == 0 { '○' } else { '·' };
            ctx.c
                .ellipse(cx, cy, rr * 2.2, rr, ch, col.mul(a * (0.45 + ctx.bass() * 0.8)), theme::VOID);
        }

        // 振动波形（VIBRATIONS）
        let vib = ((t - 4.9) / 1.0).clamp(0.0, 1.0);
        if vib > 0.0 {
            let rows = 9;
            for r in 0..rows {
                let y = cy - rows / 2 + r;
                let amp = (1.0 + ctx.bass() * 5.0) * vib * (1.0 - (r - rows / 2).abs() as f32 / rows as f32);
                for x in (0..w).step_by(1) {
                    let v = ((x as f32 * 0.14 + t * 12.0 + r as f32).sin()
                        * (x as f32 * 0.03 + t * 2.0).cos())
                        * amp;
                    if v.abs() < 0.75 {
                        let col = theme::heat((x as f32 / w as f32).min(1.0)).mul(vib * 0.85);
                        ctx.put(x, y + (v * 0.5) as i32, '·', col);
                    }
                }
            }
        }

        // COMPLETION：闭环进度
        if comp > 0.0 {
            let rr = (h as f32 * 0.30).min(14.0);
            let n = (comp * 160.0) as i32;
            for i in 0..n {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 / 160.0 * std::f32::consts::TAU;
                let px = cx as f32 + a.cos() * rr * 2.1;
                let py = cy as f32 + a.sin() * rr;
                ctx.put(px as i32, py as i32, '●', theme::GREEN.mul(0.85));
            }
            // 环上的小节点
            for i in 0..8 {
                let a = i as f32 / 8.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                if (i as f32 / 8.0) <= comp {
                    ctx.put(
                        (cx as f32 + a.cos() * rr * 2.1) as i32,
                        (cy as f32 + a.sin() * rr) as i32,
                        '◆',
                        theme::WHITE,
                    );
                }
            }
            if comp >= 1.0 {
                let pulse = 0.6 + 0.4 * ctx.bass();
                if ctx.rng.chance(0.3) {
                    self.parts.burst(
                        cx as f32,
                        cy as f32,
                        4,
                        12.0,
                        0.6,
                        fx::SPARK,
                        theme::GREEN,
                        &mut ctx.rng,
                    );
                }
                ctx.textc_glow(cy + 4, "COMPLETION", theme::WHITE.mul(pulse), theme::GREEN);
            }
            self.parts.update(ctx.dt, 2.0, 1.0);
            self.parts.draw(ctx.c);
        }

        let cap = if t < 4.9 {
            "the trance  ·  the trance"
        } else if t < 8.8 {
            "feel your vibrations"
        } else {
            "finally be completion"
        };
        ctx.textc_glow(1, cap, theme::WHITE, theme::PURPLE);

        // 边框脉冲
        let a = 0.25 + 0.55 * ctx.bass();
        ctx.frame(Rect::new(1, 0, w - 2, h), Frame::Single, theme::PURPLE.mul(a));
    }
}
