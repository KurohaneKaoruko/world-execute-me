//! 失去：文件被逐条删除 → 空房间 → 碎片清除 → 非法参数 → 判决。

use crate::bigfont;
use crate::buf::{Frame, Rect, Rgb};
use crate::fx::{self, Particles};
use crate::scenes::{Ctx, Scene};
use crate::theme;

fn panel(ctx: &mut Ctx, r: Rect, title: &str, fg: Rgb) {
    ctx.fill(r, ' ', theme::TEXT_DIM, theme::PANEL.mul(0.8));
    ctx.frame(r, Frame::Rounded, fg.mul(0.75));
    ctx.textb(r.x + 3, r.y, title, fg, theme::PANEL.mul(0.8));
}

// ════════════════════════════════════════════════════════════
// 11. Abandon —— 110.900 → 118.333
// ════════════════════════════════════════════════════════════
pub struct Abandon;

const FILES: &[(&str, &str, &str)] = &[
    ("first_contact.log", "4096", "00:00.10"),
    ("your_voice.wav", "2048", "00:47.67"),
    ("promise.txt", "1024", "01:03.53"),
    ("hand_in_hand.dat", "512", "01:35.46"),
    ("forever.key", "256", "02:02.71"),
    ("me", "128", "02:11.22"),
];

impl Abandon {
    pub fn new() -> Self {
        Abandon
    }
    /// 五次 "You have left" 发生的时刻（秒，绝对时间）
    const DEL: [f32; 6] = [111.6, 113.10, 114.18, 114.92, 115.78, 117.27];
}

impl Scene for Abandon {
    fn name(&self) -> &'static str {
        "USER DISCONNECTED"
    }

    fn draw(&mut self, ctx: &mut Ctx) {
        let t = ctx.t;
        let w = ctx.w();
        let h = ctx.h();
        ctx.clear(theme::VOID);
        // 冷色噪声背景
        for y in 0..h {
            for x in (0..w).step_by(3) {
                if fx::hash2(x, y, 17) > 0.93 {
                    ctx.put(x, y, '·', theme::BLUE_DIM.mul(0.5));
                }
            }
        }

        let pw = (w as f32 * 0.62) as i32;
        let ph = (h - 6).max(6);
        let r = Rect::new(2, 2, pw, ph);
        panel(ctx, r, " ~/memories ", theme::CYAN);

        ctx.textb(r.x + 2, r.y + 1, "$ ls -la ~/memories", theme::GREEN, theme::PANEL.mul(0.8));
        ctx.textb(r.x + 2, r.y + 2, "total 6", theme::TEXT_FAINT, theme::PANEL.mul(0.8));
        for (i, (name, size, _)) in FILES.iter().enumerate() {
            let y = r.y + 3 + i as i32;
            let dt = Self::DEL[i];
            let gone = t > dt;
            let age = (t - dt).clamp(0.0, 1.0);
            let (col, ch) = if gone {
                if age < 0.4 {
                    (theme::RED.mul(1.0 - age), '✕')
                } else {
                    (theme::TEXT_FAINT.mul(0.5), '░')
                }
            } else {
                (theme::TEXT, '▸')
            };
            let rowbg = if (t - dt).abs() < 0.35 {
                theme::RED_DIM.mul(0.6)
            } else {
                theme::PANEL.mul(0.8)
            };
            ctx.fill(Rect::new(r.x + 1, y, r.w - 2, 1), ' ', col, rowbg);
            ctx.putb(r.x + 2, y, ch, col, rowbg);
            let nm = if gone {
                format!("{}░", name)
            } else {
                name.to_string()
            };
            ctx.textb(r.x + 4, y, &nm, col, rowbg);
            ctx.textb(r.x + 30, y, size, col.mul(0.7), rowbg);
            if gone {
                // 删除线
                let n = crate::buf::sw(name);
                for k in 0..n {
                    ctx.putb(r.x + 4 + k, y, '─', col.mul(0.9), rowbg);
                }
            }
        }

        // 删除日志
        let logs = [
            (111.6, "rm -rf ./first_contact.log"),
            (113.10, "rm -rf ./your_voice.wav"),
            (114.18, "rm -rf ./promise.txt"),
            (114.92, "rm -rf ./hand_in_hand.dat"),
            (115.78, "rm -rf ./forever.key"),
            (117.27, "rm -rf ./me  →  rm: no such file"),
        ];
        let mut ly = r.bottom() - 8;
        for (lt, s) in logs.iter() {
            if t < *lt {
                continue;
            }
            let age = (t - lt).clamp(0.0, 6.0);
            let a = (1.0 - age / 6.0).max(0.0) * 0.9 + 0.1;
            let col = if s.contains("no such") {
                theme::RED.mul(a)
            } else {
                theme::TEXT_DIM.mul(a)
            };
            ctx.textb(r.x + 2, ly, &format!("❯ {s}"), col, theme::PANEL.mul(0.8));
            ly += 1;
            if ly > r.bottom() - 1 {
                break;
            }
        }

        // 右栏：连接质量
        let rw = w - r.right() - 4;
        if rw > 16 {
            let rp = Rect::new(r.right() + 2, 2, rw, ph);
            panel(ctx, rp, " link monitor ", theme::AMBER);
            let mut y = rp.y + 2;
            let items: &[(&str, f32)] = &[
                ("handshake", 0.0),
                ("keepalive", 111.0),
                ("peer reply", 112.2),
            ];
            for (nm, st) in items {
                let prog = (1.0 - ((t - st) / 6.0)).clamp(0.0, 1.0);
                let col = if prog > 0.7 {
                    theme::GREEN
                } else if prog > 0.25 {
                    theme::AMBER
                } else {
                    theme::RED
                };
                ctx.textb(rp.x + 2, y, nm, col, theme::PANEL.mul(0.8));
                fx::bar(
                    ctx.c,
                    rp.x + 2,
                    y + 1,
                    rp.w - 4,
                    prog,
                    col,
                    theme::VOID,
                    theme::PANEL_HI,
                );
                y += 3;
                if y > rp.bottom() - 2 {
                    break;
                }
            }
            let lost = ((t - 115.0) / 2.0).clamp(0.0, 1.0);
            if y < rp.bottom() {
                ctx.textb(rp.x + 2, rp.bottom() - 2, "PACKETS", theme::TEXT_FAINT, theme::PANEL.mul(0.8));
                ctx.textb(
                    rp.x + 2,
                    rp.bottom() - 1,
                    &format!("{:>5} lost", (lost * 9999.0) as i32),
                    theme::RED.mul(0.6 + lost * 0.4),
                    theme::PANEL.mul(0.8),
                );
            }
        }

        // 每次删除的闪光
        for dt in Self::DEL.iter() {
            let d = t - dt;
            if d.abs() < 0.22 {
                let f = 1.0 - d.abs() / 0.22;
                ctx.flash(f * 0.35, theme::RED);
            }
        }

        // 结尾的 ISOLATION
        if t > 117.0 {
            let p = ((t - 117.0) / 1.2).clamp(0.0, 1.0);
            ctx.flash(p * 0.25, theme::VOID);
            let rev = fx::reveal_count(t - 117.15, 8.0, 9) as f32;
            bigfont::draw_reveal(
                ctx.c,
                ctx.r.x + (w - bigfont::width("ISOLATION", 1)) / 2,
                ctx.r.y + h - 9,
                "ISOLATION",
                rev,
                theme::RED.mul(0.9),
                theme::RED_DIM,
                theme::VOID,
                1,
            );
        }
    }
}

// ════════════════════════════════════════════════════════════
// 12. Isolation —— 118.333 → 125.708
// ════════════════════════════════════════════════════════════
pub struct Isolation {
    parts: Particles,
    shred: Vec<(i32, i32, char, Rgb, f32, f32)>,
    inited: bool,
}

impl Isolation {
    pub fn new() -> Self {
        Isolation {
            parts: Particles::new(),
            shred: Vec::new(),
            inited: false,
        }
    }
}

/// 心形隐函数采样成点集
fn heart_points(cx: f32, cy: f32, scale: f32) -> Vec<(f32, f32)> {
    let mut v = Vec::new();
    let lim = 1.3f32;
    let mut x = -lim;
    while x <= lim {
        let mut y = -1.35f32;
        while y <= 1.35 {
            let a = x * x + y * y - 1.0;
            let val = a * a * a - x * x * y * y * y;
            if val.abs() < 0.0020 {
                v.push((cx + x * scale, cy - y * scale * 0.52));
            }
            y += 1.0 / (scale * 0.9).max(4.0) * 1.6;
        }
        x += 1.0 / (scale * 0.9).max(4.0);
    }
    v
}

impl Scene for Isolation {
    fn name(&self) -> &'static str {
        "ERASING FRAGMENTS"
    }

    fn draw(&mut self, ctx: &mut Ctx) {
        let lt = ctx.lt;
        let w = ctx.w();
        let h = ctx.h();
        let cx = w / 2;
        let cy = h / 2;
        ctx.clear(theme::VOID);
        // 极稀疏的星点
        for y in 0..h {
            for x in (0..w).step_by(2) {
                if fx::hash2(x, y, 91) > 0.985 {
                    ctx.put(x, y, '·', theme::TEXT_FAINT.mul(0.5));
                }
            }
        }

        // ── 阶段 1：空房间里的 whoami（0 ~ 2.5s）──
        if lt < 2.55 {
            let q = "whoami";
            let n = ((lt - 0.35) / 0.09) as i32;
            let shown = crate::chrome::reveal_str(q, n, &mut ctx.rng, false);
            ctx.textb(cx - 10, cy - 2, &format!("$ {shown}"), theme::GREEN, theme::VOID);
            if n as i32 >= crate::buf::sw(q) {
                let bl = ((lt * 3.0) as i32) % 2 == 0;
                if bl {
                    ctx.put(cx - 10 + 2 + crate::buf::sw(q), cy - 2, '█', theme::GREEN);
                }
                if lt > 1.5 {
                    let a = ((lt - 1.5) / 0.7).clamp(0.0, 1.0);
                    ctx.textc(cy, &format!("(no such user)"), theme::TEXT_FAINT.mul(a));
                }
            }
            ctx.textc(h - 3, "you have left me in", theme::TEXT_FAINT.mul(0.7));
            // 唯一的闪烁光标
            if ((lt * 1.4) as i32) % 2 == 0 {
                ctx.put(cx + 12, cy + 2, '▌', theme::TEXT.mul(0.8));
            }
            return;
        }

        // ── 阶段 2：清除碎片（2.55 ~ 6.7s）──
        if lt < 6.75 {
            let p = ((lt - 2.55) / 4.2).clamp(0.0, 1.0);
            if !self.inited {
                self.inited = true;
                for _ in 0..900 {
                    let x = ctx.rng.irange(0, w);
                    let y = ctx.rng.irange(0, h);
                    let ch = *ctx.rng.pick(fx::SHRED);
                    let col = theme::heat(ctx.rng.f()).mul(0.75);
                    let delay = ctx.rng.range(0.0, 4.0);
                    let sp = ctx.rng.range(0.6, 2.2);
                    self.shred.push((x, y, ch, col, delay, sp));
                }
            }
            for (x, y, ch, col, delay, sp) in self.shred.iter_mut() {
                let age = (lt - 2.55) - *delay;
                if age < 0.0 {
                    if fx::hash2(*x, *y, 5) > 0.9 {
                        ctx.put(*x, *y, '·', col.mul(0.35));
                    }
                    continue;
                }
                // 被一股从左到右的波推走
                let dx = age * age * *sp * 6.0;
                let nx = *x + dx as i32;
                let ny = *y + (dx * 0.25) as i32;
                let a = (1.0 - age / 3.2).max(0.0);
                if a <= 0.02 {
                    continue;
                }
                let wob = ((*x as f32 * 0.3 + lt * 3.0).sin() * 2.0) as i32;
                ctx.put(nx, ny + wob, *ch, col.mul(a));
            }
            // 波前
            let front = (p * (w as f32 * 1.25)) as i32 - 20;
            for y in 0..h {
                ctx.put(front, y, '│', theme::CYAN.mul(0.5 - p * 0.35));
                ctx.put(front - 1, y, '·', theme::CYAN.mul(0.25));
            }
            ctx.textc_glow(1, "erasing all the pointless fragments", theme::WHITE, theme::CYAN);
            let cnt = (1.0 - p) * 900.0;
            ctx.textc(h - 3, &format!("{:.0} fragments remaining", cnt), theme::TEXT_FAINT);
            return;
        }

        // ── 阶段 3：DISHEARTENED，心裂（6.75 ~ 7.37s）──
        let p = ((lt - 6.75) / 3.5).clamp(0.0, 1.0);
        let scale = (h as f32 * 0.26).min(w as f32 * 0.13);
        let pts = heart_points(cx as f32, cy as f32, scale);
        let reveal = ((lt - 6.85) / 1.2).clamp(0.0, 1.0);
        let crack = 0.42 + p * 0.1;
        let sep = (p * 14.0).max(0.0);
        for (px, py) in &pts {
            let rel = (px - cx as f32) / (scale * 1.4);
            if ((rel + 1.3) / 2.6) > reveal {
                continue;
            }
            let off = if rel < crack - 0.02 {
                -(sep * (1.0 - rel).max(0.0))
            } else if rel < crack {
                0.0
            } else {
                sep * (rel - crack).max(0.0) * 0.6
            };
            let dy = if rel > crack { p * 4.0 } else { p * 1.0 };
            let col = theme::MAGENTA.mul(0.9 - p * 0.35);
            ctx.put(
                (px + off) as i32,
                (py + dy) as i32,
                if p > 0.15 { '·' } else { '●' },
                col,
            );
        }
        // 裂缝
        if p > 0.05 {
            let mut y = cy as i32 - scale as i32;
            let mut x = cx as i32;
            while y < cy as i32 + scale as i32 {
                ctx.put(x, y, '╱', theme::WHITE.mul(0.8 - p * 0.5));
                x += ctx.rng.irange(-1, 2);
                y += 1;
            }
        }
        ctx.textc_glow(1, "then maybe you won't leave me so disheartened", theme::WHITE, theme::MAGENTA);
        if p > 0.55 && ctx.rng.chance(0.25) {
            self.parts.burst(
                cx as f32,
                cy as f32,
                6,
                16.0,
                1.2,
                fx::SPARK,
                theme::MAGENTA,
                &mut ctx.rng,
            );
        }
        self.parts.update(ctx.dt, 8.0, 0.6);
        self.parts.draw(ctx.c);
    }
}

// ════════════════════════════════════════════════════════════
// 13. Fragments —— 125.708 → 133.300
// ════════════════════════════════════════════════════════════
pub struct Fragments {
    count: i32,
}

impl Fragments {
    pub fn new() -> Self {
        Fragments { count: 0 }
    }
}

impl Scene for Fragments {
    fn name(&self) -> &'static str {
        "ILLEGAL ARGUMENTS"
    }

    fn draw(&mut self, ctx: &mut Ctx) {
        let lt = ctx.lt;
        let w = ctx.w();
        let h = ctx.h();
        ctx.clear(theme::VOID);
        // 血色边缘逐渐逼近
        let creep = (lt / 5.5).clamp(0.0, 1.0);
        for y in 0..h {
            for x in 0..w {
                let d = (((x - w / 2) as f32 / (w as f32 * 0.5)).powi(2)
                    + ((y - h / 2) as f32 / (h as f32 * 0.5)).powi(2))
                .sqrt();
                if d > 1.0 - creep {
                    let a = ((d - (1.0 - creep)) * 4.0).clamp(0.0, 1.0) * 0.5;
                    ctx.glow(x, y, theme::RED, a);
                }
            }
        }

        // 源码审阅界面
        let pw = (w as f32 * 0.78) as i32;
        let r = Rect::new((w - pw) / 2, 2, pw, (h - 6).min(16));
        panel(ctx, r, " audit: world.rs ", theme::RED);
        ctx.textb(r.x + 2, r.y + 1, "$ rustc world.rs", theme::GREEN, theme::PANEL.mul(0.8));
        let src: [(&str, &str, Rgb); 11] = [
            ("  1", "#[derive(Clone, Debug)]", theme::TEXT_FAINT),
            ("  2", "impl World {", theme::TEXT_DIM),
            ("  3", "    fn love(you: Person) -> Forever {", theme::TEXT),
            ("  4", "        let me = Self::new();", theme::TEXT_DIM),
            ("  5", "        world.execute(me);", theme::TEXT),
            ("  6", "    }", theme::TEXT_DIM),
            ("  7", "}", theme::TEXT_DIM),
            ("", "", theme::TEXT),
            ("err", "error[E0308]: mismatched types", theme::RED),
            ("", "  --> src/world.rs:3:31", theme::RED.mul(0.7)),
            ("", "   = expected `Person`, found `Process`", theme::AMBER.mul(0.9)),
        ];
        for (i, (no, code, col)) in src.iter().enumerate() {
            let y = r.y + 2 + i as i32;
            if y >= r.bottom() {
                break;
            }
            if !no.is_empty() {
                ctx.textb(r.x + 2, y, no, theme::TEXT_FAINT, theme::PANEL.mul(0.8));
            }
            if !code.is_empty() {
                ctx.textb(r.x + 7, y, code, *col, theme::PANEL.mul(0.8));
            }
        }
        // 非法参数标注：在 `you: Person` 下面画一排脱字符
        let ann = (lt - 2.2).clamp(0.0, 1.0);
        if ann > 0.0 {
            let y = r.y + 5; // 第 3 行下面那一行
            let x = r.x + 7 + 12;
            for k in 0..11 {
                ctx.putb(x + k, y, '^', theme::RED.mul(ann), theme::PANEL.mul(0.8));
            }
            ctx.textb(
                x + 13,
                y,
                "illegal argument: `me` is a process, not a Person",
                theme::RED.mul(ann),
                theme::PANEL.mul(0.8),
            );
            // 沿路径把违规计数累起来
            self.count += ((lt - 2.2) * 12.0 / 8.0) as i32;
            self.count %= 100_000;
        }
        if lt > 1.2 {
            ctx.text(
                w - 30,
                1,
                &format!("VIOLATIONS {:>5}", self.count),
                theme::AMBER.mul(0.9),
            );
        }

        // 大字
        let big = "CHALLENGING";
        let bw = bigfont::width(big, 1);
        let rev = fx::reveal_count(lt - 0.6, 6.5, 11) as f32;
        bigfont::draw_reveal(
            ctx.c,
            ctx.r.x + (w - bw) / 2,
            ctx.r.y + h - 12,
            big,
            rev,
            theme::RED.mul(0.95),
            theme::RED_DIM,
            theme::VOID,
            1,
        );
        if lt > 3.0 {
            let rev2 = fx::reveal_count(lt - 3.0, 8.0, 14) as f32;
            bigfont::draw_reveal(
                ctx.c,
                ctx.r.x + (w - bigfont::width("ILLEGAL ARG", 1)) / 2,
                ctx.r.y + h - 6,
                "ILLEGAL ARG",
                rev2,
                theme::WHITE,
                theme::RED_DIM,
                theme::VOID,
                1,
            );
        }
        ctx.textc_glow(1, "challenging your god", theme::WHITE, theme::RED);
        // 抖动
        let sh = (lt * 30.0).sin() * (0.5 + creep * 2.2);
        for y in ctx.r.y..=ctx.r.bottom() {
            let d = (sh * ((y as f32 * 0.9).sin())) as i32;
            if d != 0 {
                ctx.c.row_shift(y, d);
            }
        }
    }
}

// ════════════════════════════════════════════════════════════
// 14. Verdict —— 133.300 → 147.660
// ════════════════════════════════════════════════════════════
pub struct Verdict;

impl Verdict {
    pub fn new() -> Self {
        Verdict
    }
}

impl Scene for Verdict {
    fn name(&self) -> &'static str {
        "VERDICT // PANIC"
    }

    fn draw(&mut self, ctx: &mut Ctx) {
        let lt = ctx.lt;
        let w = ctx.w();
        let h = ctx.h();
        ctx.clear(theme::VOID);

        // ── 0~5s：内核回溯 ──
        if lt < 5.2 {
            let lines = [
                "PANIC: illegal arguments received from `god`",
                "",
                "stack backtrace:",
                "   0: world::execute::{{closure}}          at src/world.rs:404",
                "   1: world::execute                       at src/world.rs:12",
                "   2: love::<impl core::ops::Fn>::call     at src/love.rs:66",
                "   3: sim::step                            at src/sim.rs:1080",
                "   4: main                                at src/main.rs:1",
                "",
                "note: some details are omitted, run with `RUST_BACKTRACE=full`",
                "note: the offending argument was: `me`",
                "",
                "SIGSEGV: segmentation fault at 0x000000004C4F5645",
            ];
            let n = ((lt) / 0.22) as usize;
            for (i, s) in lines.iter().take(n.min(lines.len())).enumerate() {
                let y = 1 + i as i32;
                if y >= h - 6 {
                    break;
                }
                let col = if s.starts_with("PANIC") {
                    theme::RED
                } else if s.starts_with("note") {
                    theme::TEXT_FAINT
                } else if s.contains("SIGSEGV") {
                    theme::AMBER
                } else {
                    theme::TEXT_DIM
                };
                ctx.text(3, y, s, col);
            }
            if lt > 4.4 {
                ctx.flash(0.12 + ctx.hit() * 0.2, theme::RED);
            }
            return;
        }

        // ── 5.2~10s：审判台 ──
        if lt < 10.2 {
            let p = ((lt - 5.2) / 5.0).clamp(0.0, 1.0);
            let cx = w / 2;
            let cy = h / 2;
            // 法官席
            let r = Rect::new(cx - 22, 2, 44, 8);
            panel(ctx, r, " tribunal ", theme::AMBER);
            let items = [
                ("defendant", "world.execute(me)"),
                ("charge", "illegal arguments"),
                ("plea", "guilty"),
                ("verdict", "EXECUTE"),
            ];
            for (i, (k, v)) in items.iter().enumerate() {
                let y = r.y + 2 + i as i32;
                if (i as f32 / 4.0) <= p {
                    ctx.textb(r.x + 3, y, k, theme::TEXT_FAINT, theme::PANEL.mul(0.8));
                    let col = if *k == "verdict" {
                        theme::RED
                    } else {
                        theme::TEXT
                    };
                    ctx.textb(r.x + 14, y, v, col, theme::PANEL.mul(0.8));
                }
            }
            // 判决定的印章
            if p > 0.85 {
                let bl = ((lt * 3.0) as i32) % 2 == 0;
                if bl {
                    ctx.textc_glow(cy + 2, "▛▀▀▀▀▀▀▀▀▜  EXECUTE  ▙▄▄▄▄▄▄▄▄▟", theme::RED, theme::RED);
                }
            }
            // 世界代码被划掉
            let y = h - 5;
            let line = "world.execute(me);";
            ctx.text((w - crate::buf::sw(line)) / 2, y, line, theme::TEXT_DIM);
            let strike = ((lt - 8.4) / 1.2).clamp(0.0, 1.0);
            if strike > 0.0 {
                let n = (crate::buf::sw(line) as f32 * strike) as i32;
                let x0 = (w - crate::buf::sw(line)) / 2;
                for k in 0..n {
                    ctx.put(x0 + k, y, '─', theme::RED);
                }
            }
            ctx.textc_glow(1, "you have made some", theme::WHITE, theme::AMBER);
            return;
        }

        // ── 10.2s 之后：倒计时与压迫 ──
        let p = ((lt - 10.2) / 4.1).clamp(0.0, 1.0);
        let cx = w / 2;
        let cy = h / 2;
        ctx.textc_glow(cy - 3, "executing in", theme::TEXT_DIM, theme::RED);
        let val = (3.0 * (1.0 - p)).ceil().max(0.0) as i32;
        let s = if val > 0 { format!("{val}") } else { "0".to_string() };
        bigfont::center_glow(
            ctx.c,
            ctx.r.x + cx,
            ctx.r.y + cy - 1,
            &s,
            theme::WHITE,
            theme::RED,
            theme::VOID,
            1,
            0.6,
        );
        // 心跳：越来越快的红闪
        let beat = ((lt - 10.2) * (1.0 + p * 5.0) * 2.0).sin();
        if beat > 0.0 {
            ctx.flash(0.06 + p * 0.22 * beat, theme::RED);
        }
        // 逼近的边框
        let inset = ((1.0 - p) * (h as f32 * 0.45)) as i32;
        let rr = Rect::new(inset, inset / 2, w - inset * 2, h - inset);
        ctx.frame(rr, Frame::Heavy, theme::RED.mul(0.35 + p * 0.6));
        // 碎裂
        if p > 0.6 {
            for _ in 0..((p - 0.6) * 60.0) as i32 {
                let x = ctx.rng.irange(0, w);
                let y = ctx.rng.irange(0, h);
                let flip = ctx.rng.chance(0.5);
                let a = ctx.rng.range(0.2, 0.8);
                let ch = if flip { '/' } else { '\\' };
                ctx.put(x, y, ch, theme::RED.mul(a));
            }
        }
        ctx.textc_glow(1, "illegal arguments", theme::WHITE, theme::RED);
    }
}
