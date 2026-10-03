//! 开场：CRT 通电 → 自检日志 → 对象构建 → 世界启动 → 标题。

use crate::bigfont;
use crate::buf::{Frame, Rect};
use crate::fx::{self, Rain};
use crate::fx3d;
use crate::scenes::{Ctx, Scene};
use crate::theme;

// ════════════════════════════════════════════════════════════
// 1. Boot —— 0.000 → 16.000
// ════════════════════════════════════════════════════════════
pub struct Boot {
    /// 右栏的 3D 内核（随自检进度旋转的二十面体）
    ico: fx3d::Mesh,
    /// SIMULATION 阶段成形的经纬球
    globe: fx3d::Mesh,
}

impl Boot {
    pub fn new() -> Self {
        Boot {
            ico: fx3d::icosa_mesh(),
            globe: fx3d::latlong_mesh(12, 8, 1.0),
        }
    }
}

impl Scene for Boot {
    fn name(&self) -> &'static str {
        "BOOT / SELF-TEST"
    }

    fn draw(&mut self, ctx: &mut Ctx) {
        let t = ctx.lt;
        let w = ctx.w();
        let h = ctx.h();
        ctx.clear(theme::VOID);

        // ── 阶段 0：CRT 通电（0~0.75s）──
        if t < 0.75 {
            let p = (t / 0.75).clamp(0.0, 1.0);
            let cy = h / 2;
            let half = (fx::ease_out(p) * (h as f32 / 2.0)) as i32;
            for d in -half..=half {
                let y = cy + d;
                if y < 0 || y >= h {
                    continue;
                }
                let fade = 1.0 - (d.abs() as f32 / (half.max(1) as f32 + 1.0));
                let col = theme::CYAN.mix(theme::WHITE, fade * 0.6);
                ctx.hline(0, w - 1, y, if d == 0 { '█' } else { '▁' }, col.mul(0.3 + fade));
            }
            // 噪声
            for y in 0..h {
                for x in 0..w {
                    if fx::hash2(x, y, (t * 90.0) as u32) > 0.986 {
                        let g = fx::GARBAGE
                            [((fx::hash2(x, y, 3) * 22.0) as usize).min(21)];
                        ctx.put(x, y, g, theme::TEXT_FAINT);
                    }
                }
            }
            if p > 0.72 {
                ctx.flash((p - 0.72) / 0.28, theme::WHITE);
            }
            return;
        }

        // ── 常驻：扫描噪声 + 六角网格 ──
        let gcol = theme::CYAN_DIM.mul(0.32);
        for y in (2..h).step_by(2) {
            for x in (0..w).step_by(4) {
                ctx.put(x, y, '·', gcol);
            }
        }

        // ── 阶段 1：自检日志（0.75~11.2）──
        if t < 11.2 {
            let pw = (w as f32 * 0.64) as i32;
            let ph = h - 3;
            let panel = Rect::new(2, 1, pw, ph);
            ctx.fill(panel, ' ', theme::TEXT_DIM, theme::PANEL.mul(0.85));
            ctx.frame(panel, Frame::Rounded, theme::CYAN_DIM);
            ctx.textb(panel.x + 3, panel.y, " /var/log/boot.log ", theme::CYAN, theme::PANEL.mul(0.85));

            // 逐条打印歌词，把它们当作自检项
            let mut rows: Vec<(f32, String, String, bool)> = Vec::new();
            for l in &ctx.ly.lines {
                if l.t > t || l.t > 14.5 {
                    break;
                }
                let kw = l
                    .tokens
                    .iter()
                    .find(|k| k.kw.is_some())
                    .map(|k| k.s.trim().to_string());
                let (status, warn) = match kw.as_deref() {
                    Some(k) => (k.to_string(), false),
                    None => ("ok".to_string(), false),
                };
                rows.push((l.t, l.en.clone(), status, warn));
            }
            let cap = (panel.h - 3) as usize;
            let off = rows.len().saturating_sub(cap);
            for (i, (lt, msg, status, _)) in rows.iter().enumerate().skip(off) {
                let y = panel.y + 2 + (i - off) as i32;
                let age = t - lt;
                let fresh = age < 0.45;
                let base = if fresh { theme::WHITE } else { theme::TEXT };
                let m = format!("[{:05.2}]", lt);
                ctx.textb(panel.x + 2, y, &m, theme::TEXT_FAINT, theme::PANEL.mul(0.85));
                ctx.putb(panel.x + 9, y, '│', theme::CYAN_DIM, theme::PANEL.mul(0.85));
                let maxw = panel.w - 22;
                let mut text = msg.clone();
                if crate::buf::sw(&text) > maxw - crate::buf::sw(status) {
                    while crate::buf::sw(&text) > maxw - crate::buf::sw(status) - 1 && !text.is_empty() {
                        text.pop();
                    }
                    text.push('…');
                }
                let shown = if fresh {
                    let n = ((age / 0.45) * 1.4 * crate::buf::sw(&text) as f32) as i32;
                    crate::chrome::reveal_str(&text, n, &mut ctx.rng, false)
                } else {
                    text
                };
                let col = if fresh { theme::WHITE } else { base };
                ctx.textb(panel.x + 11, y, &shown, col, theme::PANEL.mul(0.85));
                // 点线 + 状态
                if age > 0.28 {
                    let sx = panel.right() - 4 - crate::buf::sw(status) as i32;
                    let tx = panel.x + 11 + crate::buf::sw(&shown) + 1;
                    for x in tx..sx {
                        ctx.putb(x, y, '·', theme::TEXT_FAINT.mul(0.7), theme::PANEL.mul(0.85));
                    }
                    let scol = if status == "ok" {
                        theme::GREEN
                    } else {
                        theme::AMBER
                    };
                    let s = format!("[{status}]");
                    ctx.textb(sx - 2, y, &s, scol, theme::PANEL.mul(0.85));
                }
            }

            // 右侧：对象构建 + 资源表
            let rx = panel.right() + 2;
            let rw = w - rx - 2;
            if rw > 20 {
                let rp = Rect::new(rx, 1, rw, ph);
                ctx.fill(rp, ' ', theme::TEXT_DIM, theme::PANEL.mul(0.6));
                ctx.frame(rp, Frame::Single, theme::BLUE_DIM);
                ctx.textb(rp.x + 2, rp.y, " world.execute(me) ", theme::BLUE, theme::PANEL.mul(0.6));

                let steps = [
                    ("power-line", 0.9),
                    ("protection", 1.6),
                    ("object.pieces", 3.9),
                    ("data.parameters", 7.4),
                    ("initialization", 8.0),
                    ("simulation", 11.1),
                    ("identity", 13.5),
                ];
                for (i, (nm, st)) in steps.iter().enumerate() {
                    let y = rp.y + 2 + i as i32 * 2;
                    if y > rp.bottom() - 4 {
                        break;
                    }
                    let prog = ((t - st) / 2.2).clamp(0.0, 1.0);
                    let col = if prog >= 1.0 { theme::GREEN } else { theme::AMBER };
                    ctx.textb(rp.x + 2, y, nm, col, theme::PANEL.mul(0.6));
                    let bw = rp.w - 4;
                    fx::bar(
                        ctx.c,
                        rp.x + 2,
                        y + 1,
                        bw,
                        prog,
                        theme::CYAN,
                        theme::BLUE,
                        theme::PANEL_HI,
                    );
                }
                // 核心：随低频脉动的 3D 内核（自检 = 把多面体拼起来）
                let cy = rp.bottom() - 5;
                let cx = rp.cx();
                let core_bass = ctx.bass();
                {
                    let mut r3 = fx3d::R3::new(w, h);
                    r3.cx = cx as f32;
                    r3.cy = cy as f32;
                    r3.cam_z = 3.0;
                    r3.focal = 21.0;
                    r3.begin();
                    let mut tg = fx3d::Target::new(ctx.c, ctx.r.x, ctx.r.y, w, h);
                    let xf = fx3d::Xform::scaled(t * 0.6, 0.45, t * 0.3, 1.0 + core_bass * 0.18);
                    r3.wire(&mut tg, &self.ico, &xf, theme::CYAN, theme::WHITE, 4.4, 0.85 + core_bass * 0.3, 0.95);
                }
                ctx.textb(rp.x + 2, rp.bottom() - 1, "CORE 0x4C4F5645", theme::TEXT_FAINT, theme::PANEL.mul(0.6));
            }

            // 底部：十六进制流
            let y = h - 2;
            ctx.text(2, y, "0x00:", theme::TEXT_FAINT);
            fx::hexdump(ctx.c, ctx.r.x + 8, ctx.r.y + y, w - 12, 9, t, theme::CYAN_DIM);
            return;
        }

        // ── 阶段 2：SIMULATION（11.2~16）──
        let p = ((t - 11.2) / 4.8).clamp(0.0, 1.0);
        // 世界成形：经纬球从波纹里长出来
        {
            let mut r3 = fx3d::R3::new(w, h);
            r3.cam_z = 3.0;
            r3.focal = h as f32 * 2.0;
            r3.begin();
            let mut tg = fx3d::Target::new(ctx.c, ctx.r.x, ctx.r.y, w, h);
            let xf = fx3d::Xform::scaled(t * 0.3, 0.3, 0.0, 0.5 + p * 0.75);
            r3.wire(&mut tg, &self.globe, &xf, theme::CYAN, theme::WHITE, 4.6, 0.75 + p * 0.55, 0.95 * p);
        }
        for y in (1..h).step_by(1) {
            for x in (0..w).step_by(2) {
                let d = (((x as f32 - w as f32 / 2.0).powi(2) + ((y as f32 - h as f32 / 2.0) * 2.2).powi(2))
                    .sqrt())
                    / (w as f32 * 0.55);
                let wave = ((d * 12.0 - t * 3.4).sin() * 0.5 + 0.5) * (1.0 - d).max(0.0);
                let v = wave * (0.4 + ctx.bass() * 0.9);
                let ch = if v > 0.72 {
                    '●'
                } else if v > 0.45 {
                    '○'
                } else if v > 0.18 {
                    '·'
                } else {
                    ' '
                };
                let col = theme::CYAN.mix(theme::MAGENTA, d * 0.8).mul(0.35 + v);
                ctx.put(x, y, ch, col);
            }
        }
        // 已完成的日志淡出上移
        for (i, l) in ctx.ly.lines.iter().enumerate() {
            if l.t > 14.0 {
                break;
            }
            let y = 2 + i as i32;
            if y < h {
                let a = (1.0 - p * 1.8).max(0.0) * 0.5;
                if a > 0.02 {
                    ctx.text(3, y, &format!("[{}] {}", i, l.en), theme::TEXT_FAINT.mul(a));
                }
            }
        }
        // 大标题 SIMULATION
        let rev = fx::reveal_count(t - 12.2, 9.0, 10) as f32;
        let bw = bigfont::width("SIMULATION", 1);
        bigfont::draw_reveal(
            ctx.c,
            ctx.r.x + (w - bw) / 2,
            ctx.r.y + h / 2 - 2,
            "SIMULATION",
            rev,
            theme::WHITE.mix(theme::CYAN, 0.35),
            theme::CYAN_DIM,
            theme::BG,
            1,
        );
        if t > 13.4 {
            let bl = ((t * 3.0) as i32) % 2 == 0;
            if bl {
                ctx.textc(h / 2 + 5, "the world is now running", theme::TEXT_DIM);
            }
        }
        // 起动闪光
        if (t - 11.2).abs() < 0.25 {
            let f = 1.0 - (t - 11.2).abs() / 0.25;
            ctx.flash(f * 0.55, theme::CYAN);
        }
        if (t - 15.95).abs() < 0.35 {
            let f = 1.0 - (t - 15.95).abs() / 0.35;
            ctx.flash(f * 0.8, theme::WHITE);
        }
    }
}

// ════════════════════════════════════════════════════════════
// 2. Title —— 16.000 → 29.709
// ════════════════════════════════════════════════════════════
pub struct Title {
    rain: Option<Rain>,
    /// "world" 的具象化：线框地球 + 大陆点尘（5.5s 后从字符雨后浮现）
    globe: fx3d::Mesh,
    land: Vec<fx3d::Vec3>,
}

impl Title {
    pub fn new() -> Self {
        Title {
            rain: None,
            globe: fx3d::latlong_mesh(14, 9, 1.0),
            land: fx3d::fib_sphere(260),
        }
    }
}

impl Scene for Title {
    fn name(&self) -> &'static str {
        "world.execute(me);"
    }

    fn draw(&mut self, ctx: &mut Ctx) {
        let t = ctx.lt;
        let w = ctx.w();
        let h = ctx.h();
        ctx.clear(theme::VOID);

        // 背景字符雨
        if self.rain.is_none() {
            let mut r = fx::Rng::new(0xC0FFEE);
            self.rain = Some(Rain::new(w, h, &mut r));
        }
        if let Some(rain) = self.rain.as_mut() {
            rain.draw(
                ctx.c,
                ctx.r,
                ctx.dt,
                theme::CYAN.mul(0.85),
                theme::CYAN_DIM.mul(0.5),
                0.62,
            );
        }
        fx::data_dust(ctx.c, ctx.r, t, 0.012, theme::BLUE);

        // "world" 的具象化：线框地球从字符雨后缓缓浮现，转到收束为止
        let ga = if (5.5..12.6).contains(&t) {
            let ain = ((t - 5.5) / 1.6).clamp(0.0, 1.0);
            let gout = if t > 11.4 { ((12.6 - t) / 1.2).clamp(0.0, 1.0) } else { 1.0 };
            fx::smooth(ain) * gout
        } else {
            0.0
        };
        if ga > 0.01 {
            let mut r3 = fx3d::R3::new(w, h);
            r3.cam_z = 3.0;
            r3.focal = h as f32 * 2.0;
            r3.begin();
            let mut tg = fx3d::Target::new(ctx.c, ctx.r.x, ctx.r.y, w, h);
            let lxf = fx3d::Xform::scaled(t * 0.30 + 0.4, 0.28, 0.0, 1.49);
            r3.cloud(&mut tg, &self.land, &lxf, theme::GREEN, theme::GREEN_DIM, fx3d::DEPTH_RAMP, 5.4, 0.85 * ga, 0.9 * ga);
            // 线框最后画：正面赢 Z-Buffer，背面被大陆点尘正确遮挡
            let xf = fx3d::Xform::scaled(t * 0.30, 0.28, 0.0, 1.5);
            r3.wire(&mut tg, &self.globe, &xf, theme::CYAN, theme::WHITE, 5.4, 1.25 * ga, 0.9 * ga);
        }

        // 中央：标题
        let l1 = "world.";
        let l2 = "execute(me);";
        let bw = bigfont::width(l2, 1);
        let x0 = ctx.r.x + (w - bw) / 2;
        let y0 = ctx.r.y + h / 2 - 6;

        // 扫描亮线扫过标题（"编译"过程）
        let sweep = ((t - 3.2) / 2.4).clamp(0.0, 1.0);
        let rev = if t < 3.2 {
            fx::reveal_count(t - 0.35, 7.0, 18) as f32
        } else {
            18.0
        };
        bigfont::draw_reveal(
            ctx.c,
            x0 + (bw - bigfont::width(l1, 1)),
            y0,
            l1,
            rev,
            theme::CYAN,
            theme::CYAN_DIM,
            theme::BG,
            1,
        );
        bigfont::draw_reveal(
            ctx.c,
            x0,
            y0 + 6,
            l2,
            rev - 6.0,
            theme::MAGENTA.mix(theme::WHITE, 0.15),
            theme::MAGENTA_DIM,
            theme::BG,
            1,
        );

        // 扫过时的高亮
        if (0.0..1.0).contains(&sweep) {
            let sx = x0 - 4 + (sweep * (bw + 8) as f32) as i32;
            for y in (y0 - 1)..(y0 + 12) {
                for dx in -2..=2 {
                    ctx.glow(sx + dx, y, theme::WHITE, 0.5 - dx.abs() as f32 * 0.16);
                }
            }
            for y in (y0 - 1)..(y0 + 12) {
                if y >= 0 && y < h {
                    ctx.put(sx, y, '▌', theme::WHITE);
                }
            }
        }

        // 副标题
        if t > 1.6 {
            let sub = "Mili  ·  Miracle Milk  ·  world.execute(me);";
            let a = ((t - 1.6) / 1.2).clamp(0.0, 1.0);
            ctx.textc_glow(h / 2 + 6, sub, theme::TEXT.mul(a), theme::CYAN);
        }

        // 底部：提示符 / 编译输出
        let py = h - 4;
        if t > 5.0 {
            let cmd = "$ world.execute(me);";
            let n = ((t - 5.0) / 0.055) as i32;
            let shown = crate::chrome::reveal_str(cmd, n, &mut ctx.rng, false);
            ctx.text(6, py, &shown, theme::GREEN);
            if n as i32 >= crate::buf::sw(cmd) {
                let bl = ((t * 4.0) as i32) % 2 == 0;
                if bl {
                    ctx.put(6 + crate::buf::sw(cmd), py, '█', theme::GREEN);
                }
                // ENTER → 输出
                if t > 7.4 {
                    let out = [
                        "> allocating world ................ 4 KiB",
                        "> binding me -> world ............ ok",
                        "> entering simulation ............ ok",
                    ];
                    for (i, s) in out.iter().enumerate() {
                        let st = 7.4 + i as f32 * 0.55;
                        if t > st {
                            let a = ((t - st) / 0.5).clamp(0.0, 1.0);
                            ctx.text(8, py + 1 + i as i32, s, theme::TEXT_DIM.mul(a));
                        }
                    }
                }
                // 执行那一刻的整屏白闪
                if (t - 7.35).abs() < 0.22 {
                    let f = 1.0 - (t - 7.35).abs() / 0.22;
                    ctx.flash(f * 0.9, theme::WHITE);
                }
            }
        }

        // 右下角：波形
        if t > 2.2 {
            let by = h - 2;
            let n = 48.min(w - 4);
            let x0 = w - n - 3;
            for i in 0..n {
                let ph = i as f32 / n as f32;
                let v = (ph * 22.0 + t * 5.0).sin() * ctx.bass() * 1.6;
                let ch = if v.abs() < 0.25 {
                    '─'
                } else if v > 0.0 {
                    '‾'
                } else {
                    '_'
                };
                let col = theme::heat(ph).mul(0.4 + ctx.bass());
                ctx.put(x0 + i, by, ch, col);
            }
        }

        // 结束前的收缩（把画面吸向中心）
        if t > 12.6 {
            let p = ((t - 12.6) / 1.1).clamp(0.0, 1.0);
            ctx.darken_outside(p);
            if p > 0.85 {
                ctx.flash((p - 0.85) * 6.0, theme::WHITE);
            }
        }
    }
}
