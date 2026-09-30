//! LRC 解析 + 中英对齐。
//!
//! 双语文件的英文行数比 LRC 多一条（间奏处多出的重复句），
//! 所以对齐用"向前看窗口 + 精确匹配"，而不是简单按序号配对。

// 解析器附带元数据与查询 API，部分供场景侧按需取用
#![allow(dead_code)]

use crate::buf::sw;
use crate::theme::{self, Kw};

#[derive(Clone, Debug)]
pub struct Token {
    pub s: String,
    pub kw: Option<Kw>,
    /// 该 token 在整行中的起始列（以显示宽度计）
    pub x: i32,
}

#[derive(Clone, Debug)]
pub struct Line {
    pub t: f32,
    pub en: String,
    pub zh: Option<String>,
    pub tokens: Vec<Token>,
    pub width: i32,
    /// 同一条歌词连续重复出现时的序号（从 1 开始）
    pub rep: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Lyrics {
    pub lines: Vec<Line>,
}

pub const LRC: &str = include_str!("../assets/lyrics/lyrics.lrc");
pub const ZH: &str = include_str!("../assets/lyrics/lyrics.zh.txt");

fn parse_timestamp(s: &str) -> Option<f32> {
    // [mm:ss.xxx]
    let s = s.trim();
    if !s.starts_with('[') {
        return None;
    }
    let end = s.find(']')?;
    let body = &s[1..end];
    let mut it = body.split(':');
    let m: f32 = it.next()?.parse().ok()?;
    let sec: f32 = it.next()?.parse().ok()?;
    Some(m * 60.0 + sec)
}

fn is_cjk_any(s: &str) -> bool {
    s.chars().any(|c| (0x3000..=0x9FFF).contains(&(c as u32)))
}

/// 把一行英文拆成 token（关键词单独成块）
fn tokenize(en: &str) -> (Vec<Token>, i32) {
    let mut toks: Vec<Token> = Vec::new();
    let mut x = 0i32;
    let words: Vec<&str> = en.split(' ').filter(|w| !w.is_empty()).collect();
    for (i, w) in words.iter().enumerate() {
        let kw = if theme::is_keyword(w) {
            theme::classify(w)
        } else {
            None
        };
        let piece = if i == 0 {
            w.to_string()
        } else {
            format!(" {w}")
        };
        let wid = sw(&piece);
        toks.push(Token {
            s: piece,
            kw,
            x,
        });
        x += wid;
    }
    (toks, x)
}

pub fn load() -> Lyrics {
    // ── 读取双语对照 ──
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut buf: Vec<String> = Vec::new();
    for raw in ZH.lines() {
        let l = raw.trim();
        if l.is_empty() {
            continue;
        }
        buf.push(l.to_string());
        if buf.len() == 2 {
            pairs.push((buf[0].clone(), buf[1].clone()));
            buf.clear();
        }
    }
    if buf.len() == 1 {
        pairs.push((buf[0].clone(), String::new()));
    }

    // ── 读取 LRC ──
    let mut raw_lines: Vec<(f32, String)> = Vec::new();
    for raw in LRC.lines() {
        let l = raw.trim();
        if l.is_empty() {
            continue;
        }
        if let Some(t) = parse_timestamp(l) {
            if let Some(e) = l.find(']') {
                let text = l[e + 1..].trim().to_string();
                if !text.is_empty() {
                    raw_lines.push((t, text));
                }
            }
        }
    }

    // ── 对齐 ──
    let norm = |s: &str| -> String { s.trim().to_ascii_uppercase().replace("  ", " ") };
    let mut j = 0usize;
    let mut lines: Vec<Line> = Vec::new();
    let mut prev_en = String::new();
    let mut rep = 0usize;
    for (t, en) in raw_lines.iter() {
        let key = norm(en);
        let mut zh: Option<String> = None;
        for k in j..(j + 5).min(pairs.len()) {
            if norm(&pairs[k].0) == key {
                zh = Some(pairs[k].1.clone());
                j = k + 1;
                break;
            }
        }
        if norm(en) == norm(&prev_en) {
            rep += 1;
        } else {
            rep = 1;
            prev_en = en.clone();
        }
        let (tokens, width) = tokenize(en);
        lines.push(Line {
            t: *t,
            en: en.clone(),
            zh,
            tokens,
            width,
            rep,
        });
    }
    Lyrics { lines }
}

impl Lyrics {
    /// 当前正在唱的歌词下标
    pub fn current(&self, t: f32) -> Option<usize> {
        if self.lines.is_empty() {
            return None;
        }
        if t < self.lines[0].t {
            return None;
        }
        let mut lo = 0usize;
        let mut hi = self.lines.len() - 1;
        while lo < hi {
            let mid = (lo + hi + 1) / 2;
            if self.lines[mid].t <= t {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        Some(lo)
    }

    /// 从 t 起、这条歌词同文重复了几次（用于 EXECUTION 连打 / You have left）
    pub fn repetition(&self, idx: usize) -> usize {
        self.lines[idx].rep
    }

    /// 关于某关键词、在某时间点之前一共出现过几次
    pub fn count_kw_before(&self, text: &str, idx: usize) -> usize {
        self.lines[..idx].iter().filter(|l| l.en == text).count()
    }

    /// 歌词条元信息：(本条含有的关键词, 该句在全曲中的第几次)
    pub fn meta(&self, idx: usize) -> (String, usize) {
        let l = &self.lines[idx];
        let kws: Vec<String> = l
            .tokens
            .iter()
            .filter(|t| t.kw.is_some())
            .map(|t| t.s.trim().to_string())
            .collect();
        (kws.join(" · "), l.rep)
    }
}
