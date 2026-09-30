//! 全局运行状态：每秒取样的音频能量 + 时间轴信息，供各场景与界面共享。

use crate::audio::{Audio, Spectrum, BANDS};
use crate::lyrics::Lyrics;

/// 与"用户"的连接状态——这是本 MV 的一条隐藏叙事线：
/// 歌里唱到"你走了"的那一刻，状态栏里的连接就断了。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Link {
    Linked,
    Unstable,
    Lost,
}

pub struct View {
    pub t: f32,
    pub dt: f32,
    pub frame: u64,
    pub bands: [f32; BANDS],
    pub bass: f32,
    pub mid: f32,
    pub high: f32,
    pub rms: f32,
    pub pulse: f32,
    /// 本秒内低频的峰值（用于打击感）
    pub hit: f32,
    pub scene_name: &'static str,
    pub scene_idx: usize,
    pub scene_count: usize,
    pub scene_progress: f32,
    pub paused: bool,
    pub finished: bool,
    pub link: Link,
    pub lyric_idx: Option<usize>,
    /// 歌词条最近一次换行的时间（做入场动画）
    pub lyric_since: f32,
    pub show_help: bool,
}

impl View {
    pub fn new() -> Self {
        View {
            t: 0.0,
            dt: 0.016,
            frame: 0,
            bands: [0.0; BANDS],
            bass: 0.0,
            mid: 0.0,
            high: 0.0,
            rms: 0.0,
            pulse: 0.0,
            hit: 0.0,
            scene_name: "",
            scene_idx: 0,
            scene_count: 1,
            scene_progress: 0.0,
            paused: false,
            finished: false,
            link: Link::Linked,
            lyric_idx: None,
            lyric_since: 0.0,
            show_help: false,
        }
    }

    pub fn update(&mut self, audio: &mut Audio, lyrics: &Lyrics, t: f32, dt: f32) {
        self.t = t;
        self.dt = dt;
        self.frame += 1;
        let spec: &Spectrum = &audio.spec;
        spec.bands_at(t, &mut self.bands);
        self.bass = spec.bass_at(t);
        self.mid = spec.mid_at(t);
        self.high = spec.high_at(t);
        self.rms = spec.rms_at(t);
        self.pulse = spec.pulse(t);
        // 打击感：低频相对前 0.15s 的增量
        let prev = spec.bass_at(t - 0.15);
        self.hit = ((self.bass - prev) * 4.0).clamp(0.0, 1.0);

        let idx = lyrics.current(t);
        self.lyric_idx = idx;
        // 用绝对时间差而不是累加 dt：既保证离屏渲染能复现，
        // 也保证拖动进度条之后动画立刻处于正确状态
        self.lyric_since = match idx {
            Some(i) => (t - lyrics.lines[i].t).max(0.0),
            None => 0.0,
        };
        self.link = if t < 106.8 {
            Link::Linked
        } else if t < 118.0 {
            Link::Unstable
        } else {
            Link::Lost
        };
    }

    /// 从某条歌词开始经过的时间
    pub fn elapsed_in_lyric(&self) -> f32 {
        self.lyric_since
    }
}
