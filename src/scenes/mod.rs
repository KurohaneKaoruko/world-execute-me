//! 场景框架：舞台上下文 + 时间轴。

pub mod exec;
pub mod loss;
pub mod love;
pub mod open;
pub mod organic;
pub mod verse;

use crate::audio::BANDS;
use crate::buf::{Frame as BFrame, Canvas, Rect, Rgb};
use crate::fx::Rng;
use crate::lyrics::{Line, Lyrics};
use crate::theme;
use crate::view::View;

/// 舞台上下文。坐标以舞台左上角为原点。
pub struct Ctx<'a> {
    pub c: &'a mut Canvas,
    pub r: Rect,
    pub t: f32,
    pub lt: f32,
    pub dt: f32,
    #[allow(dead_code)] // 舞台 API：场景可用当前帧号做逐帧抖动
    pub frame: u64,
    pub rng: Rng,
    pub v: &'a View,
    pub ly: &'a Lyrics,
}

// 舞台 API 面：辅助方法按需取用，允许部分暂未被场景引用
#[allow(dead_code)]
impl<'a> Ctx<'a> {
    pub fn new(
        c: &'a mut Canvas,
        r: Rect,
        t: f32,
        lt: f32,
        dt: f32,
        frame: u64,
        v: &'a View,
        ly: &'a Lyrics,
    ) -> Self {
        Ctx {
            c,
            r,
            t,
            lt,
            dt,
            frame,
            rng: Rng::new(frame.wrapping_mul(6364136223846793005) ^ 0x9E3779B97F4A7C15),
            v,
            ly,
        }
    }

    #[inline]
    pub fn w(&self) -> i32 {
        self.r.w
    }
    #[inline]
    pub fn h(&self) -> i32 {
        self.r.h
    }
    #[inline]
    pub fn midx(&self) -> i32 {
        self.r.w / 2
    }
    #[inline]
    pub fn midy(&self) -> i32 {
        self.r.h / 2
    }
    /// 舞台外框（相对坐标）
    #[inline]
    pub fn bounds(&self) -> Rect {
        Rect::new(0, 0, self.r.w, self.r.h)
    }

    #[inline]
    pub fn put(&mut self, x: i32, y: i32, ch: char, fg: Rgb) {
        if x < 0 || y < 0 || x >= self.r.w || y >= self.r.h {
            return;
        }
        let bg = self
            .c
            .get(self.r.x + x, self.r.y + y)
            .map(|c| c.bg)
            .unwrap_or(theme::BG);
        self.c.put(self.r.x + x, self.r.y + y, ch, fg, bg);
    }

    #[inline]
    pub fn putb(&mut self, x: i32, y: i32, ch: char, fg: Rgb, bg: Rgb) {
        if x < 0 || y < 0 || x >= self.r.w || y >= self.r.h {
            return;
        }
        self.c.put(self.r.x + x, self.r.y + y, ch, fg, bg);
    }

    /// 覆盖已有字符的前景色（用于发光/染色）
    #[inline]
    pub fn glow(&mut self, x: i32, y: i32, fg: Rgb, a: f32) {
        if x < 0 || y < 0 || x >= self.r.w || y >= self.r.h {
            return;
        }
        self.c.glow(self.r.x + x, self.r.y + y, fg, a);
    }

    pub fn text(&mut self, x: i32, y: i32, s: &str, fg: Rgb) -> i32 {
        if y < 0 || y >= self.r.h {
            return x;
        }
        let end = self.c.text(self.r.x + x, self.r.y + y, s, fg, theme::BG);
        end - self.r.x
    }

    pub fn textb(&mut self, x: i32, y: i32, s: &str, fg: Rgb, bg: Rgb) -> i32 {
        if y < 0 || y >= self.r.h {
            return x;
        }
        let end = self.c.text(self.r.x + x, self.r.y + y, s, fg, bg);
        end - self.r.x
    }

    pub fn textc(&mut self, y: i32, s: &str, fg: Rgb) {
        let w = crate::buf::sw(s);
        self.text((self.r.w - w) / 2, y, s, fg);
    }

    pub fn textc_glow(&mut self, y: i32, s: &str, fg: Rgb, g: Rgb) {
        let w = crate::buf::sw(s);
        let x = (self.r.w - w) / 2;
        for dx in [-1, 1] {
            let mut px = x + dx;
            for ch in s.chars() {
                let cw = crate::buf::cw(ch);
                if cw == 0 {
                    continue;
                }
                self.glow(px, y, g, 0.4);
                if cw == 2 {
                    self.glow(px + 1, y, g, 0.4);
                }
                px += cw;
            }
        }
        self.text(x, y, s, fg);
    }

    pub fn fill(&mut self, r: Rect, ch: char, fg: Rgb, bg: Rgb) {
        for y in r.y..=r.bottom() {
            for x in r.x..=r.right() {
                self.putb(x, y, ch, fg, bg);
            }
        }
    }

    pub fn frame(&mut self, r: Rect, style: BFrame, fg: Rgb) {
        let (tl, tr, bl, br, h, v) = match style {
            BFrame::Single => ('┌', '┐', '└', '┘', '─', '│'),
            BFrame::Double => ('╔', '╗', '╚', '╝', '═', '║'),
            BFrame::Rounded => ('╭', '╮', '╰', '╯', '─', '│'),
            BFrame::Heavy => ('┏', '┓', '┗', '┛', '━', '┃'),
            BFrame::Dashed => ('┌', '┐', '└', '┘', '┄', '┆'),
            BFrame::None => (' ', ' ', ' ', ' ', ' ', ' '),
        };
        if r.w < 2 || r.h < 2 {
            return;
        }
        for x in (r.x + 1)..r.right() {
            self.put(x, r.y, h, fg);
            self.put(x, r.bottom(), h, fg);
        }
        for y in (r.y + 1)..r.bottom() {
            self.put(r.x, y, v, fg);
            self.put(r.right(), y, v, fg);
        }
        self.put(r.x, r.y, tl, fg);
        self.put(r.right(), r.y, tr, fg);
        self.put(r.x, r.bottom(), bl, fg);
        self.put(r.right(), r.bottom(), br, fg);
    }

    pub fn hline(&mut self, x0: i32, x1: i32, y: i32, ch: char, fg: Rgb) {
        for x in x0..=x1 {
            self.put(x, y, ch, fg);
        }
    }

    pub fn vline(&mut self, x: i32, y0: i32, y1: i32, ch: char, fg: Rgb) {
        for y in y0..=y1 {
            self.put(x, y, ch, fg);
        }
    }

    /// 舞台背景填充
    pub fn clear(&mut self, bg: Rgb) {
        self.c.fill(self.r, ' ', theme::TEXT_FAINT, bg);
    }

    /// 整屏闪光（把底色往 col 混）
    pub fn flash(&mut self, a: f32, col: Rgb) {
        if a <= 0.001 {
            return;
        }
        let a = a.clamp(0.0, 1.0);
        for y in 0..self.r.h {
            for x in 0..self.r.w {
                if let Some(i) = self.c.idx(self.r.x + x, self.r.y + y) {
                    let c = &mut self.c.cells[i];
                    c.bg = c.bg.mix(col, a);
                    c.fg = c.fg.mix(col, a * 0.7);
                }
            }
        }
    }

    /// 把舞台某块区域整体压暗（做"吸向中心"的收束）
    pub fn darken_outside(&mut self, p: f32) {
        let cx = self.r.w / 2;
        let cy = self.r.h / 2;
        let rad = (1.0 - p) * self.r.w as f32 * 0.62;
        for y in 0..self.r.h {
            for x in 0..self.r.w {
                let d = (((x - cx).pow(2) + ((y - cy) * 2).pow(2)) as f32).sqrt();
                if d > rad {
                    self.putb(x, y, ' ', theme::VOID, theme::VOID);
                }
            }
        }
    }

    /// 全屏乱码噪声（转场用）
    pub fn static_noise(&mut self, density: f32, col: Rgb) {
        for y in 0..self.r.h {
            for x in 0..self.r.w {
                if crate::fx::hash2(x, y, self.frame as u32) < density {
                    let g = crate::fx::GARBAGE
                        [((crate::fx::hash2(x + 7, y + 3, self.frame as u32) * 22.0) as usize)
                            .min(21)];
                    self.putb(x, y, g, col, theme::VOID);
                }
            }
        }
    }

    // ── 音频 ──
    #[inline]
    pub fn bass(&self) -> f32 {
        self.v.bass
    }
    #[inline]
    pub fn mid(&self) -> f32 {
        self.v.mid
    }
    #[inline]
    pub fn high(&self) -> f32 {
        self.v.high
    }
    #[inline]
    pub fn rms(&self) -> f32 {
        self.v.rms
    }
    #[inline]
    pub fn hit(&self) -> f32 {
        self.v.hit
    }
    #[inline]
    pub fn bands(&self) -> &[f32; BANDS] {
        &self.v.bands
    }

    // ── 歌词 ──
    pub fn line(&self) -> Option<&Line> {
        self.v.lyric_idx.and_then(|i| self.ly.lines.get(i))
    }
    /// 当前歌词里的关键词原文（清洗过空格）
    pub fn kw(&self) -> Option<String> {
        self.line().and_then(|l| {
            l.tokens
                .iter()
                .find(|t| t.kw.is_some())
                .map(|t| t.s.trim().to_string())
        })
    }
    /// 从某个时间点开始，距今多久（用于"这段歌词刚开始"的判定）
    pub fn since(&self, t0: f32) -> f32 {
        self.t - t0
    }
}

pub trait Scene {
    fn name(&self) -> &'static str;
    fn draw(&mut self, ctx: &mut Ctx);
}

pub struct Seg {
    pub start: f32,
    pub scene: Box<dyn Scene>,
}

pub struct Timeline {
    pub segs: Vec<Seg>,
}

impl Timeline {
    pub fn index_at(&self, t: f32) -> usize {
        let mut idx = 0;
        for (i, s) in self.segs.iter().enumerate() {
            if t >= s.start {
                idx = i;
            } else {
                break;
            }
        }
        idx
    }
    pub fn marks(&self) -> Vec<(f32, &'static str)> {
        self.segs.iter().map(|s| (s.start, s.scene.name())).collect()
    }
}

pub fn build() -> Timeline {
    let mut segs: Vec<Seg> = Vec::new();
    macro_rules! add {
        ($t:expr, $s:expr) => {
            segs.push(Seg {
                start: $t,
                scene: Box::new($s),
            })
        };
    }
    add!(0.000, open::Boot::new());
    add!(16.000, open::Title::new());
    add!(29.709, verse::Geometry::new());
    add!(44.452, verse::Electric::new());
    add!(59.223, verse::Stimulus::new());
    add!(64.045, verse::Trapped::new());
    add!(74.045, organic::Organic::new());
    add!(82.589, organic::Deity::new());
    add!(89.223, organic::Morph::new());
    add!(101.474, organic::Trance::new());
    add!(110.900, loss::Abandon::new());
    add!(118.333, loss::Isolation::new());
    add!(125.708, loss::Fragments::new());
    add!(133.300, loss::Verdict::new());
    add!(147.660, exec::Barrage::new());
    add!(162.632, exec::FinalExec::new());
    add!(177.246, love::Love::new());
    add!(191.356, love::LoveTrapped::new());
    add!(205.811, love::Shutdown::new());
    Timeline { segs }
}
