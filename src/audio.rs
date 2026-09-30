//! 音频：播放、时钟对齐、离线频谱分析。
//!
//! 关键点：为了实现"真·节奏同步"，我们不依赖运行时抓取音频流，
//! 而是在启动时把整首歌离线解码一遍，按 60fps 的粒度算出
//! 32 个频段的能量包络 + 低频/中频/高频三条总线。播放时用
//! 播放器位置直接查表，抖动为零。

use std::fs::File;
use std::path::Path;
use std::time::{Duration, Instant};

use rodio::Source;

pub const FPS: f32 = 60.0;
pub const BANDS: usize = 32;
pub const NBIN: usize = 1024;

/// 音频文件的真实信息（启动时打印，也是 meta 素材）
#[derive(Clone, Debug)]
#[allow(dead_code)] // path/bytes 是元数据，保留给状态栏/日志备用
pub struct TrackInfo {
    pub path: String,
    pub bytes: u64,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration: f32,
    pub frames: usize,
}

/// 节奏包络：按帧索引的频谱与能量
#[derive(Clone)]
pub struct Spectrum {
    pub bands: Vec<[u8; BANDS]>,
    pub bass: Vec<u8>,
    pub mid: Vec<u8>,
    pub high: Vec<u8>,
    pub rms: Vec<u8>,
    pub peak_beats: Vec<f32>,
}

impl Spectrum {
    pub fn empty(dur: f32) -> Self {
        let n = (dur * FPS).ceil() as usize + 2;
        Spectrum {
            bands: vec![[0u8; BANDS]; n],
            bass: vec![0; n],
            mid: vec![0; n],
            high: vec![0; n],
            rms: vec![0; n],
            peak_beats: vec![],
        }
    }
    fn at(&self, v: &Vec<u8>, t: f32) -> f32 {
        if v.is_empty() {
            return 0.0;
        }
        let i = ((t.max(0.0) * FPS) as usize).min(v.len() - 1);
        v[i] as f32 / 255.0
    }
    pub fn bass_at(&self, t: f32) -> f32 {
        self.at(&self.bass, t)
    }
    pub fn mid_at(&self, t: f32) -> f32 {
        self.at(&self.mid, t)
    }
    pub fn high_at(&self, t: f32) -> f32 {
        self.at(&self.high, t)
    }
    pub fn rms_at(&self, t: f32) -> f32 {
        self.at(&self.rms, t)
    }
    pub fn bands_at(&self, t: f32, out: &mut [f32; BANDS]) {
        let i = ((t.max(0.0) * FPS) as usize).min(self.bands.len() - 1);
        for k in 0..BANDS {
            out[k] = self.bands[i][k] as f32 / 255.0;
        }
    }
    /// 低频能量的短时均值，用来驱动"整体脉动"
    pub fn pulse(&self, t: f32) -> f32 {
        let mut s = 0.0;
        for k in 0..6 {
            s += self.bass_at(t - k as f32 / FPS);
        }
        (s / 6.0).clamp(0.0, 1.0)
    }
}

// ── FFT（迭代 radix-2）──────────────────────────────────────
struct Fft {
    rev: Vec<u32>,
    cos: Vec<f32>,
    sin: Vec<f32>,
    n: usize,
    re: Vec<f32>,
    im: Vec<f32>,
}

impl Fft {
    fn new(n: usize) -> Self {
        let bits = n.trailing_zeros();
        let mut rev = vec![0u32; n];
        for i in 0..n {
            rev[i] = (i as u32).reverse_bits() >> (32 - bits);
        }
        let half = n / 2;
        let mut cos = vec![0.0; half];
        let mut sin = vec![0.0; half];
        for i in 0..half {
            let a = -2.0 * std::f32::consts::PI * i as f32 / n as f32;
            cos[i] = a.cos();
            sin[i] = a.sin();
        }
        Fft {
            rev,
            cos,
            sin,
            n,
            re: vec![0.0; n],
            im: vec![0.0; n],
        }
    }

    fn run(&mut self, input: &[f32]) {
        let n = self.n;
        for i in 0..n {
            self.re[i] = input[self.rev[i] as usize];
            self.im[i] = 0.0;
        }
        let mut len = 2;
        while len <= n {
            let half = len / 2;
            let step = n / len;
            let mut i = 0;
            while i < n {
                let mut j = 0;
                while j < half {
                    let wr = self.cos[j * step];
                    let wi = self.sin[j * step];
                    let ur = self.re[i + j];
                    let ui = self.im[i + j];
                    let vr = self.re[i + j + half] * wr - self.im[i + j + half] * wi;
                    let vi = self.re[i + j + half] * wi + self.im[i + j + half] * wr;
                    self.re[i + j] = ur + vr;
                    self.im[i + j] = ui + vi;
                    self.re[i + j + half] = ur - vr;
                    self.im[i + j + half] = ui - vi;
                    j += 1;
                }
                i += len;
            }
            len *= 2;
        }
    }
}

/// 解码整个音频文件并计算包络。返回 None 表示解码失败。
pub fn analyze(path: &Path) -> (TrackInfo, Spectrum) {
    let bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());

    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => {
            let info = TrackInfo {
                path: name,
                bytes,
                sample_rate: 44100,
                channels: 2,
                duration: 0.0,
                frames: 0,
            };
            return (info, Spectrum::empty(0.0));
        }
    };
    let dec = match rodio::Decoder::try_from(file) {
        Ok(d) => d,
        Err(_) => {
            let info = TrackInfo {
                path: name,
                bytes,
                sample_rate: 44100,
                channels: 2,
                duration: 0.0,
                frames: 0,
            };
            return (info, Spectrum::empty(0.0));
        }
    };

    let sample_rate = dec.sample_rate().get();
    let channels = dec.channels().get() as usize;
    let total_est = dec.total_duration().map(|d| d.as_secs_f32()).unwrap_or(212.0);

    let n = NBIN;
    let half = n / 2;
    let mut fft = Fft::new(n);
    let mut hann = vec![0.0f32; n];
    for i in 0..n {
        hann[i] = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (n as f32 - 1.0)).cos();
    }

    // 频段边界：40Hz → 16kHz 对数分布
    let f_lo = 40.0f32;
    let f_hi = 16000.0f32.min(sample_rate as f32 * 0.45);
    let mut edges = [0usize; BANDS + 1];
    for k in 0..=BANDS {
        let f = f_lo * (f_hi / f_lo).powf(k as f32 / BANDS as f32);
        let bin = (f / sample_rate as f32 * n as f32).round() as usize;
        edges[k] = bin.clamp(1, half - 1);
    }
    for k in 1..=BANDS {
        if edges[k] <= edges[k - 1] {
            edges[k] = (edges[k - 1] + 1).min(half - 1);
        }
    }
    // 每个频点属于哪个频段（预计算，避免每帧 32 次比较）
    let mut band_idx = [0u8; NBIN];
    {
        let mut k = 0usize;
        for b in 0..half {
            while k + 1 < BANDS && b >= edges[k + 1] {
                k += 1;
            }
            band_idx[b] = k as u8;
        }
    }
    let (bass_hi, mid_hi) = (edges[6], edges[20]);

    let hop = (sample_rate as f32 / FPS).round() as usize;
    let est_frames = (total_est * FPS).ceil() as usize + 8;
    let mut bands_raw: Vec<[f32; BANDS]> = vec![[0.0; BANDS]; est_frames];
    let mut bass_raw: Vec<f32> = vec![0.0; est_frames];
    let mut mid_raw: Vec<f32> = vec![0.0; est_frames];
    let mut high_raw: Vec<f32> = vec![0.0; est_frames];
    let mut rms_raw: Vec<f32> = vec![0.0; est_frames];
    let mut seen: Vec<bool> = vec![false; est_frames];

    let mut ring: Vec<f32> = vec![0.0; n];
    let mut widx = 0usize;
    let mut since_fft = 0usize;
    let mut total = 0usize;
    let mut frame_idx = 0usize;
    let mut buf = [0.0f32; NBIN];
    let mut acc = 0.0f32;
    let mut ch = 0usize;

    for s in dec {
        if ch == 0 {
            acc = 0.0;
        }
        acc += s / channels as f32;
        ch += 1;
        if ch < channels {
            continue;
        }
        ch = 0;
        let mono = acc;

        ring[widx] = mono;
        widx = (widx + 1) % n;
        total += 1;
        since_fft += 1;
        if total < n || since_fft < hop {
            continue;
        }
        since_fft = 0;

        let mut r = 0.0f32;
        for i in 0..n {
            let v = ring[(widx + i) % n];
            r += v * v;
            buf[i] = v * hann[i];
        }
        fft.run(&buf);

        // 一次遍历同时算出 32 段能量与低/中/高三条总线
        let mut acc_bands = [0.0f32; BANDS];
        let mut ba = 0.0f32;
        let mut mi = 0.0f32;
        let mut hi = 0.0f32;
        for b in 1..half {
            let m = (fft.re[b] * fft.re[b] + fft.im[b] * fft.im[b]).sqrt();
            acc_bands[band_idx[b] as usize] += m;
            if b < bass_hi {
                ba += m;
            } else if b < mid_hi {
                mi += m;
            } else {
                hi += m;
            }
        }
        let rms = (r / n as f32).sqrt();

        // 窗口中心对应的播放时刻
        let t = (total as f32 - n as f32 * 0.5) / sample_rate as f32;
        let idx = (t.max(0.0) * FPS).round() as usize;
        if idx < est_frames {
            bands_raw[idx] = acc_bands;
            bass_raw[idx] = ba;
            mid_raw[idx] = mi;
            high_raw[idx] = hi;
            rms_raw[idx] = rms;
            seen[idx] = true;
            frame_idx = frame_idx.max(idx + 1);
        }
    }

    // 补齐空帧（解码空隙），让包络连续
    let mut last_idx = 0usize;
    for i in 0..frame_idx {
        if seen[i] {
            last_idx = i;
        } else {
            bands_raw[i] = bands_raw[last_idx];
            bass_raw[i] = bass_raw[last_idx];
            mid_raw[i] = mid_raw[last_idx];
            high_raw[i] = high_raw[last_idx];
            rms_raw[i] = rms_raw[last_idx];
        }
    }
    bands_raw.truncate(frame_idx);
    bass_raw.truncate(frame_idx);
    mid_raw.truncate(frame_idx);
    high_raw.truncate(frame_idx);
    rms_raw.truncate(frame_idx);

    // 归一化：每个频段用 95 分位做参考，避免被单帧尖峰拖垮
    let frames = frame_idx.max(1);
    let mut out = Spectrum::empty(frames as f32 / FPS);
    let norm = |v: &Vec<f32>| -> f32 {
        let mut s: Vec<f32> = v.iter().copied().filter(|x| x.is_finite()).collect();
        if s.is_empty() {
            return 1.0;
        }
        s.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let q = s[(s.len() as f32 * 0.95) as usize % s.len()];
        if q <= 1e-9 {
            1.0
        } else {
            q
        }
    };
    let nb = norm(&bass_raw);
    let nm = norm(&mid_raw);
    let nh = norm(&high_raw);
    let nr = norm(&rms_raw);
    let mut band_norms = [1.0f32; BANDS];
    for k in 0..BANDS {
        let v: Vec<f32> = bands_raw.iter().map(|b| b[k]).collect();
        band_norms[k] = norm(&v);
    }

    out.bands = vec![[0u8; BANDS]; frames];
    out.bass = vec![0u8; frames];
    out.mid = vec![0u8; frames];
    out.high = vec![0u8; frames];
    out.rms = vec![0u8; frames];
    let q = |v: f32| -> u8 { (v.clamp(0.0, 1.0).powf(0.7) * 255.0) as u8 };
    for i in 0..frames {
        for k in 0..BANDS {
            out.bands[i][k] = q(bands_raw[i][k] / band_norms[k]);
        }
        out.bass[i] = q(bass_raw[i] / nb);
        out.mid[i] = q(mid_raw[i] / nm);
        out.high[i] = q(high_raw[i] / nh);
        out.rms[i] = q(rms_raw[i] / nr);
    }
    // 低频骤增处标记为"重音"，供场景做冲击
    let mut beats = Vec::new();
    let mut last = -1.0f32;
    for i in 2..frames - 1 {
        let b = out.bass[i] as f32 / 255.0;
        if b > 0.72
            && b >= out.bass[i - 1] as f32 / 255.0
            && b > out.bass[i + 1] as f32 / 255.0
            && (i as f32 / FPS - last) > 0.18
        {
            last = i as f32 / FPS;
            beats.push(last);
        }
    }
    out.peak_beats = beats;

    let info = TrackInfo {
        path: name,
        bytes,
        sample_rate,
        channels: channels as u16,
        duration: frames as f32 / FPS,
        frames,
    };
    (info, out)
}

// ── 播放器 ──────────────────────────────────────────────────
/// 时钟：优先跟随音频播放位置，音频不可用时退化为墙钟。
pub struct Clock {
    base: f32,     // 基准位置（秒）
    anchor: Instant,
    playing: bool,
}

impl Clock {
    pub fn new() -> Self {
        Clock {
            base: 0.0,
            anchor: Instant::now(),
            playing: false,
        }
    }
    pub fn start(&mut self) {
        self.anchor = Instant::now();
        self.playing = true;
    }
    pub fn pause(&mut self) {
        self.base = self.now();
        self.playing = false;
    }
    pub fn resume(&mut self) {
        self.anchor = Instant::now();
        self.playing = true;
    }
    pub fn is_playing(&self) -> bool {
        self.playing
    }
    /// 用音频真实位置校正基准（避免累积漂移）
    pub fn sync(&mut self, audio_pos: f32) {
        self.base = audio_pos;
        self.anchor = Instant::now();
    }
    pub fn now(&self) -> f32 {
        if self.playing {
            self.base + self.anchor.elapsed().as_secs_f32()
        } else {
            self.base
        }
    }
}

pub struct Audio {
    pub info: TrackInfo,
    pub spec: Spectrum,
    player: Option<rodio::Player>,
    _sink: Option<rodio::MixerDeviceSink>,
    fallback_clock: Clock,
    last_pos: f32,
    pub sample_ok: bool,
}

impl Audio {
    /// 纯离线模式（--no-audio / --shots）
    pub fn silent(info: TrackInfo, spec: Spectrum) -> Self {
        Audio {
            info,
            spec,
            player: None,
            _sink: None,
            fallback_clock: Clock::new(),
            last_pos: 0.0,
            sample_ok: false,
        }
    }

    /// 打开扬声器并开始播放
    pub fn play(path: &Path, info: TrackInfo, spec: Spectrum, start_at: f32) -> Self {
        let mut a = Audio::silent(info, spec);
        match open_player(path) {
            Ok((sink, player)) => {
                if start_at > 0.05 {
                    let _ = player.try_seek(Duration::from_secs_f32(start_at));
                }
                // 先挂起：闪屏期间不能出声，等 start() 才正式开播，
                // 否则"按空格开始"之前音频已经跑了一段，画面永远追不齐
                player.pause();
                a.fallback_clock.base = start_at;
                a._sink = Some(sink);
                a.player = Some(player);
                a.sample_ok = true;
            }
            Err(e) => {
                eprintln!("  \x1b[33m[!]\x1b[0m 无法打开音频输出设备：{e}");
                eprintln!("     将以静音模式继续（时间轴走墙钟，画面完全一致）");
                a.sample_ok = false;
            }
        }
        a
    }

    pub fn start(&mut self) {
        // 静音模式下从 --start 指定的位置起播（有音频时由 try_seek 负责）
        let at = self.fallback_clock.base;
        self.fallback_clock = Clock::new();
        self.fallback_clock.base = at;
        self.fallback_clock.start();
        self.last_pos = at;
        // 从闪屏进入正片：把挂起的播放器正式启动
        if let Some(p) = &self.player {
            p.play();
        }
    }

    pub fn pause(&mut self) {
        if let Some(p) = &self.player {
            p.pause();
        }
        self.fallback_clock.pause();
    }

    pub fn resume(&mut self) {
        if let Some(p) = &self.player {
            p.play();
        }
        self.fallback_clock.resume();
    }

    pub fn is_playing(&self) -> bool {
        if let Some(p) = &self.player {
            !p.is_paused()
        } else {
            self.fallback_clock.is_playing()
        }
    }

    pub fn seek(&mut self, t: f32) {
        let t = t.clamp(0.0, (self.info.duration - 0.2).max(0.0));
        if let Some(p) = &self.player {
            let _ = p.try_seek(Duration::from_secs_f32(t));
        }
        self.fallback_clock.base = t;
        self.fallback_clock.anchor = Instant::now();
        self.last_pos = t;
    }

    /// 当前时间轴位置（秒）
    pub fn time(&mut self) -> f32 {
        if let Some(p) = &self.player {
            let pos = p.get_pos().as_secs_f32();
            // 播放器位置只在解码块更新时跳变，用墙钟在两次更新间插值，保持丝滑
            if (pos - self.last_pos).abs() > 1e-4 {
                self.last_pos = pos;
                self.fallback_clock.sync(pos);
            }
            let t = self.fallback_clock.now();
            // 防止墙钟跑飞
            if t > pos + 0.10 {
                self.fallback_clock.sync(pos);
                pos
            } else {
                t
            }
        } else if self.fallback_clock.is_playing() {
            self.fallback_clock.now()
        } else {
            self.fallback_clock.base
        }
    }

}

fn open_player(
    path: &Path,
) -> Result<(rodio::MixerDeviceSink, rodio::Player), Box<dyn std::error::Error>> {
    let mut sink = rodio::DeviceSinkBuilder::open_default_sink()?;
    sink.log_on_drop(false);
    let file = File::open(path)?;
    let player = rodio::stream::play(sink.mixer(), file)?;
    Ok((sink, player))
}
