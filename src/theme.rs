//! 调色板与颜色工具。
//!
//! 整部 MV 的色彩语言：
//!   冷青 = 机器/数据  品红 = 爱  正红 = 执行/死亡  琥珀 = 警示  灰蓝 = 孤独

use crate::buf::Rgb;

// ── 基础背景 ────────────────────────────────────────────────
pub const VOID: Rgb = Rgb::new(3, 5, 9);
pub const BG: Rgb = Rgb::new(5, 8, 14);
pub const PANEL: Rgb = Rgb::new(10, 16, 26);
pub const PANEL_HI: Rgb = Rgb::new(16, 26, 40);

// ── 前景文本 ────────────────────────────────────────────────
pub const TEXT: Rgb = Rgb::new(198, 214, 234);
pub const TEXT_DIM: Rgb = Rgb::new(92, 110, 136);
pub const TEXT_FAINT: Rgb = Rgb::new(46, 58, 76);

// ── 主题色 ──────────────────────────────────────────────────
pub const CYAN: Rgb = Rgb::new(58, 214, 226);
pub const CYAN_DIM: Rgb = Rgb::new(24, 92, 104);
pub const BLUE: Rgb = Rgb::new(74, 124, 255);
pub const BLUE_DIM: Rgb = Rgb::new(26, 46, 104);
pub const MAGENTA: Rgb = Rgb::new(255, 78, 158);
pub const MAGENTA_DIM: Rgb = Rgb::new(112, 26, 68);
pub const RED: Rgb = Rgb::new(255, 58, 48);
pub const RED_DIM: Rgb = Rgb::new(104, 20, 18);
pub const AMBER: Rgb = Rgb::new(255, 180, 84);
pub const AMBER_DIM: Rgb = Rgb::new(104, 70, 28);
pub const GREEN: Rgb = Rgb::new(92, 228, 118);
pub const GREEN_DIM: Rgb = Rgb::new(28, 92, 48);
pub const PURPLE: Rgb = Rgb::new(168, 108, 255);
pub const PURPLE_DIM: Rgb = Rgb::new(62, 38, 104);
pub const WHITE: Rgb = Rgb::new(240, 246, 255);

// ── 关键词配色 ──────────────────────────────────────────────
/// 关键词的视觉分类，决定它在歌词条里被渲染成什么颜色的"数据块"。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kw {
    /// 数学：次元、圆周、切线、极限
    Math,
    /// 机器指令：执行、初始化、模拟
    Machine,
    /// 生物：营养、抗氧化物、咕噜声
    Organic,
    /// 情感：爱
    Love,
    /// 神性/存在
    Divine,
    /// 负面：孤独、失望、非法参数
    Negative,
}

impl Kw {
    /// (前景亮色, 背景底色)
    pub fn colors(self) -> (Rgb, Rgb) {
        match self {
            Kw::Math => (Rgb::new(150, 240, 250), Rgb::new(14, 74, 86)),
            Kw::Machine => (Rgb::new(160, 200, 255), Rgb::new(20, 52, 104)),
            Kw::Organic => (Rgb::new(180, 246, 160), Rgb::new(30, 78, 34)),
            Kw::Love => (Rgb::new(255, 168, 210), Rgb::new(110, 22, 66)),
            Kw::Divine => (Rgb::new(255, 232, 168), Rgb::new(96, 68, 20)),
            Kw::Negative => (Rgb::new(255, 158, 150), Rgb::new(96, 22, 20)),
        }
    }
    /// 该类关键词的"主题色"（用于粒子、光晕）
    pub fn accent(self) -> Rgb {
        match self {
            Kw::Math => CYAN,
            Kw::Machine => BLUE,
            Kw::Organic => GREEN,
            Kw::Love => MAGENTA,
            Kw::Divine => AMBER,
            Kw::Negative => RED,
        }
    }
}

/// 根据大写关键词判断视觉分类。返回 None 表示这是普通词。
pub fn classify(word: &str) -> Option<Kw> {
    let w: String = word
        .chars()
        .filter(|c| c.is_ascii_alphabetic() || *c == '-' || *c == ' ')
        .collect::<String>()
        .to_ascii_uppercase();
    let w = w.trim();
    Some(match w {
        "DIMENSION" | "CIRCUMFERENCE" | "TANGENTS" | "LIMITATIONS" | "VIBRATIONS" => Kw::Math,
        "PROTECTION" | "OBJECT CREATION" | "OBJECT" | "CREATION" | "INITIALIZATION"
        | "SIMULATION" | "EXECUTION" | "STIMULATIONS" | "SATISFACTION" | "COMPLETION"
        | "FRAGMENTS" => Kw::Machine,
        "NUTRIENTS" | "ANTIOXIDANTS" | "ENJOYMENT" => Kw::Organic,
        "LO-O-OVE" | "LOVE" => Kw::Love,
        "EXISTENCE" => Kw::Divine,
        "ISOLATION" | "DISHEARTENED" | "ILLEGAL ARGUMENTS" | "ILLEGAL" | "ARGUMENTS" => {
            Kw::Negative
        }
        _ => return None,
    })
}

/// 判断一个词是否是"全大写关键词"（含连字符 / 空格）。
pub fn is_keyword(word: &str) -> bool {
    let letters = word.chars().filter(|c| c.is_ascii_alphabetic()).count();
    letters >= 2
        && word
            .chars()
            .all(|c| !c.is_ascii_lowercase() && (c.is_ascii_alphabetic() || c == '-' || c == ' '))
}

/// 冷 → 暖的连续色带，用于进度条 / 波形。
pub fn heat(t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    if t < 0.34 {
        CYAN.mix(BLUE, t / 0.34)
    } else if t < 0.67 {
        BLUE.mix(MAGENTA, (t - 0.34) / 0.33)
    } else {
        MAGENTA.mix(AMBER, (t - 0.67) / 0.33)
    }
}
