//! 终端控制：初始化 / 还原 / 差分刷新 / 离屏导出（HTML 预览）。

use std::io::{Stdout, Write};

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::queue;
use crossterm::style::{Color, Colors, Print, ResetColor, SetColors};
use crossterm::terminal::{
    Clear, ClearType, DisableLineWrap, EnableLineWrap, EnterAlternateScreen, LeaveAlternateScreen,
};

use crate::buf::{Cell, Canvas, SKIP};

pub struct Term {
    pub out: Stdout,
    prev: Canvas,
}

fn cvt(c: crate::buf::Rgb) -> Color {
    Color::Rgb {
        r: c.r,
        g: c.g,
        b: c.b,
    }
}

impl Term {
    pub fn new(w: i32, h: i32) -> std::io::Result<Self> {
        let mut out = std::io::stdout();
        crossterm::terminal::enable_raw_mode()?;
        execute!(
            out,
            EnterAlternateScreen,
            DisableLineWrap,
            EnableMouseCapture,
            Hide,
            Clear(ClearType::All)
        )?;
        out.flush()?;
        Ok(Term {
            out,
            prev: Canvas::new(w, h),
        })
    }

    pub fn exit(&mut self) {
        let _ = execute!(
            self.out,
            ResetColor,
            DisableMouseCapture,
            EnableLineWrap,
            Show,
            LeaveAlternateScreen
        );
        let _ = self.out.flush();
        let _ = crossterm::terminal::disable_raw_mode();
    }

    /// 尺寸变化时重建差分缓冲，下一帧全量重画
    pub fn resize(&mut self, w: i32, h: i32) {
        self.prev = Canvas::new(w, h);
        let _ = execute!(self.out, Clear(ClearType::All));
    }

    /// 差分刷新。只输出变化的单元格，并按颜色分组以压缩转义序列。
    pub fn present(&mut self, cur: &Canvas) -> std::io::Result<()> {
        if self.prev.w != cur.w || self.prev.h != cur.h {
            self.prev = Canvas::new(cur.w, cur.h);
            let _ = execute!(self.out, Clear(ClearType::All));
        }
        let mut buf: Vec<u8> = Vec::with_capacity(32 * 1024);
        let mut cur_fg: Option<crate::buf::Rgb> = None;
        let mut cur_bg: Option<crate::buf::Rgb> = None;
        let mut pen_x: i32 = -1;
        let mut pen_y: i32 = -1;

        for y in 0..cur.h {
            let row = (y * cur.w) as usize;
            for x in 0..cur.w {
                let i = row + x as usize;
                let c = cur.cells[i];
                if c.ch == SKIP {
                    continue;
                }
                let p = self.prev.cells[i];
                if p.ch == c.ch && p.fg == c.fg && p.bg == c.bg {
                    continue;
                }
                if pen_y != y || pen_x != x {
                    let _ = queue!(buf, MoveTo(x as u16, y as u16));
                    pen_y = y;
                }
                if cur_fg != Some(c.fg) || cur_bg != Some(c.bg) {
                    let _ = queue!(buf, SetColors(Colors::new(cvt(c.fg), cvt(c.bg))));
                    cur_fg = Some(c.fg);
                    cur_bg = Some(c.bg);
                }
                let _ = queue!(buf, Print(c.ch));
                pen_x = x + crate::buf::cw(c.ch);
            }
        }
        self.out.write_all(&buf)?;
        self.out.flush()?;
        self.prev.cells.copy_from_slice(&cur.cells);
        Ok(())
    }
}

// ── 离屏导出（用于开发期视觉校验，不影响正常播放）────────────────
/// 把一帧渲染成 HTML：每个 cell 一个 span，宽字符用 2ch 宽度保证对位。
pub fn frame_to_html(cv: &Canvas, scale: f32) -> String {
    let mut s = String::with_capacity(cv.cells.len() * 24);
    s.push_str(&format!(
        "<div class=\"frame\" style=\"font-size:{}px\"><pre>",
        scale
    ));
    for y in 0..cv.h {
        let mut i = 0;
        let mut x = 0;
        while x < cv.w {
            let c = cv.cells[(y * cv.w + x) as usize];
            if c.ch == SKIP {
                x += 1;
                continue;
            }
            let w = crate::buf::cw(c.ch);
            // 合并同色同宽连续段
            let mut run = String::new();
            run.push(c.ch);
            let mut x2 = x + w;
            while x2 < cv.w {
                let n = cv.cells[(y * cv.w + x2) as usize];
                if n.ch == SKIP {
                    break;
                }
                if crate::buf::cw(n.ch) != w {
                    break;
                }
                if n.fg != c.fg || n.bg != c.bg {
                    break;
                }
                run.push(n.ch);
                x2 += w;
            }
            s.push_str(&format!(
                "<span style=\"color:#{:02x}{:02x}{:02x};background:#{:02x}{:02x}{:02x};width:{}ch\">{}</span>",
                c.fg.r,
                c.fg.g,
                c.fg.b,
                c.bg.r,
                c.bg.g,
                c.bg.b,
                run.chars().count() as i32 * w,
                escape(&run)
            ));
            x = x2;
            i += 1;
            if i > 5000 {
                break;
            }
        }
        s.push('\n');
    }
    s.push_str("</pre></div>");
    s
}

fn escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            ' ' => o.push_str("&nbsp;"),
            _ => o.push(c),
        }
    }
    o
}

/// 生成一个可截图校验的 HTML 页面
pub fn shots_html(frames: &[(f32, Canvas)], font_px: f32) -> String {
    let mut body = String::new();
    for (t, cv) in frames {
        body.push_str(&format!(
            "<div class=\"cap\">t = {:.2}s &nbsp;|&nbsp; {}×{}</div>",
            t, cv.w, cv.h
        ));
        body.push_str(&frame_to_html(cv, font_px));
    }
    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><style>
*{{box-sizing:border-box}}
body{{margin:0;background:#000;font-family:"Cascadia Mono","Consolas","DejaVu Sans Mono",monospace;}}
.cap{{color:#6f8;font:12px/20px monospace;padding:6px 10px;background:#08120c;border-bottom:1px solid #1b3a26;position:sticky;top:0}}
.frame pre{{margin:0;padding:0;line-height:1.0;letter-spacing:0;font-family:inherit;white-space:pre;tab-size:1}}
.frame span{{display:inline-block;overflow:visible;text-align:center;vertical-align:top}}
</style></head><body>{}</body></html>"#,
        body
    )
}

/// 未使用的占位，保留 Cell 引入避免告警
pub fn _touch(_c: &Cell) {}

/// 单帧独立页面（开发期逐帧截图用）
pub fn frame_html_page(cv: &Canvas, font_px: f32) -> String {
    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><style>
*{{box-sizing:border-box}}
html,body{{margin:0;padding:0;background:#000}}
.frame pre{{margin:0;padding:0;line-height:1.0;letter-spacing:0;white-space:pre;font-family:"Cascadia Mono","Consolas","DejaVu Sans Mono",monospace}}
.frame span{{display:inline-block;overflow:visible;text-align:center;vertical-align:top}}
</style></head><body>{}</body></html>"#,
        frame_to_html(cv, font_px)
    )
}

/// 导出二进制帧（供外部脚本用 PIL 精确渲染成位图，做逐帧视觉校验）
pub fn frame_to_bin(cv: &Canvas) -> Vec<u8> {
    let mut v = Vec::with_capacity(16 + cv.cells.len() * 10);
    v.extend_from_slice(b"WEMV");
    v.extend_from_slice(&(cv.w as u32).to_le_bytes());
    v.extend_from_slice(&(cv.h as u32).to_le_bytes());
    for c in &cv.cells {
        v.push(c.fg.r);
        v.push(c.fg.g);
        v.push(c.fg.b);
        v.push(c.bg.r);
        v.push(c.bg.g);
        v.push(c.bg.b);
        v.extend_from_slice(&(c.ch as u32).to_le_bytes());
    }
    v
}
