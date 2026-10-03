//! world.execute(me); — 终端 MV
//!
//! 一个把整首歌演绎成终端进程的播放器：
//!   · 启动时离线解码音频、做 32 段频谱分析，播放时按帧查表 → 视觉与节奏零延迟同步
//!   · 19 个场景按歌词时间轴切换，每个场景都可读当前歌词关键词
//!   · 界面是常驻的"硬件"：状态栏 / 频谱 / 进度条 / 双语歌词条
//!   · 隐藏叙事：状态栏里的 user@localhost 会在"你走了"之后掉线

mod audio;
mod bigfont;
mod buf;
mod chrome;
mod demo3d;
mod fx;
mod fx3d;
mod lyrics;
mod scenes;
mod term;
mod theme;
mod view;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crossterm::event::{poll, read, Event, KeyCode, KeyEventKind, KeyModifiers};

use buf::Canvas;
use chrome::Layout;
use scenes::{Ctx, Timeline};

const DEFAULT_TRACK: &str = "assets/audio/world.execute(me); - Mili.mp3";

struct Args {
    file: PathBuf,
    no_audio: bool,
    shots: Option<Vec<f32>>,
    render_video: Option<PathBuf>,
    demo3d: bool,
    size: (i32, i32),
    out: PathBuf,
    start: f32,
    splash: bool,
    fps: f32,
}

fn parse_args() -> Args {
    let mut a = Args {
        file: PathBuf::from(DEFAULT_TRACK),
        no_audio: false,
        shots: None,
        render_video: None,
        demo3d: false,
        size: (0, 0),
        out: PathBuf::from("preview.html"),
        start: 0.0,
        splash: true,
        fps: 60.0,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--file" | "-f" => {
                if let Some(v) = it.next() {
                    a.file = PathBuf::from(v);
                }
            }
            "--no-audio" => a.no_audio = true,
            "--no-splash" => a.splash = false,
            "--render-video" => {
                let v = it.next().unwrap_or_default();
                a.render_video = Some(PathBuf::from(v));
            }
            "--demo3d" => a.demo3d = true,
            "--shots" => {
                let v = it.next().unwrap_or_default();
                let times: Vec<f32> = v.split(',').filter_map(|s| s.trim().parse().ok()).collect();
                a.shots = Some(times);
            }
            "--size" => {
                let v = it.next().unwrap_or_default();
                if let Some((w, h)) = v.split_once('x') {
                    a.size = (
                        w.trim().parse().unwrap_or(120),
                        h.trim().parse().unwrap_or(40),
                    );
                }
            }
            "--out" => {
                if let Some(v) = it.next() {
                    a.out = PathBuf::from(v);
                }
            }
            "--start" => {
                if let Some(v) = it.next() {
                    a.start = v.trim().parse().unwrap_or(0.0);
                }
            }
            "--fps" => {
                if let Some(v) = it.next() {
                    a.fps = v.trim().parse().unwrap_or(60.0);
                }
            }
            "--help" | "-h" => {
                println!(
                    r#"world.execute(me); — 终端 MV

用法:
  world-execute-me [选项]

选项:
  -f, --file <路径>    指定音频文件（默认 {DEFAULT_TRACK}）
      --no-audio       静音播放（时间轴走墙钟，画面完全一致）
      --no-splash      跳过开始前的标题闪屏
      --demo3d         fx3d 引擎巡演：全屏轮播纽结/地球/心/星系（无需音频）
      --start <秒>     从指定位置开始播放
      --shots t1,t2,…  离屏渲染若干时间点的画面到 HTML（开发校验用）
      --render-video <目录>
                       把整首歌逐帧离屏渲染成 .bin 帧序列
                       （配合 tools/render_video.py 合成 MP4）
      --size WxH       离屏渲染尺寸（默认 160x48）
      --out <路径>     离屏渲染输出文件（默认 preview.html）
      --fps <数字>     渲染帧率上限（默认 60）

播放中按键:
  SPACE 暂停/继续    ←/→ 快退/快进  R 重播  H 帮助  Q/ESC 退出
"#
                );
                std::process::exit(0);
            }
            _ => {}
        }
    }
    a
}

fn term_size() -> (i32, i32) {
    if let Ok((w, h)) = crossterm::terminal::size() {
        (w as i32, h as i32)
    } else {
        (120, 40)
    }
}

/// 从主画布中把舞台区域拷贝出来 / 拷回去（后处理只作用于舞台）
fn stage_take(cv: &Canvas, stage: buf::Rect) -> Canvas {
    let mut sub = Canvas::new(stage.w, stage.h);
    for y in 0..stage.h {
        let sy = stage.y + y;
        if sy < 0 || sy >= cv.h {
            continue;
        }
        for x in 0..stage.w {
            let sx = stage.x + x;
            if sx < 0 || sx >= cv.w {
                continue;
            }
            sub.cells[(y * stage.w + x) as usize] = cv.cells[(sy * cv.w + sx) as usize];
        }
    }
    sub
}

fn stage_put(cv: &mut Canvas, stage: buf::Rect, sub: &Canvas) {
    for y in 0..stage.h {
        let sy = stage.y + y;
        if sy < 0 || sy >= cv.h {
            continue;
        }
        for x in 0..stage.w {
            let sx = stage.x + x;
            if sx < 0 || sx >= cv.w {
                continue;
            }
            cv.cells[(sy * cv.w + sx) as usize] = sub.cells[(y * stage.w + x) as usize];
        }
    }
}

/// 渲染一帧（场景 + 外壳 + 后处理），离屏与实时共用
fn render_frame(
    cv: &mut Canvas,
    tl: &mut Timeline,
    v: &mut view::View,
    ly: &lyrics::Lyrics,
    lay: &Layout,
    dt: f32,
) {
    let idx = tl.index_at(v.t);
    let count = tl.segs.len();
    let start = tl.segs[idx].start;
    let end = tl
        .segs
        .get(idx + 1)
        .map(|s| s.start)
        .unwrap_or(v.t + 1.0);
    let name = tl.segs[idx].scene.name();
    v.scene_name = name;
    v.scene_idx = idx;
    v.scene_count = count;
    v.scene_progress = ((v.t - start) / (end - start).max(0.01)).clamp(0.0, 1.0);
    let seg = &mut tl.segs[idx];

    cv.clear();
    {
        let mut ctx = Ctx::new(
            cv,
            lay.stage,
            v.t,
            (v.t - start).max(0.0),
            dt,
            v.frame,
            v,
            ly,
        );
        seg.scene.draw(&mut ctx);
    }

    // 舞台后处理：场景切换的 CRT 换台闪断 → 扫描线 → 辉光 → 渐晕
    let stage = lay.stage;
    let mut sub = stage_take(cv, stage);
    if idx > 0 {
        let tr = fx::pulse(v.t - start, 0.04, 0.20);
        if tr > 0.0 {
            let mut rr = fx::Rng::new(v.frame.wrapping_mul(31) ^ 0x5EED);
            fx::glitch(&mut sub, &mut rr, 0.14 * tr, v.t);
            for c in sub.cells.iter_mut() {
                c.fg = c.fg.mix(theme::WHITE, tr * 0.22);
                c.bg = c.bg.mix(theme::WHITE, tr * 0.12);
            }
        }
    }
    fx::scanlines(&mut sub, v.t, 0.055);
    // 辉光：处刑段随打击感增强，其余时间温和常驻
    let bloom_gain = if (147.0..164.0).contains(&v.t) {
        0.40 + v.hit * 0.25
    } else {
        0.34
    };
    fx::bloom(&mut sub, 0.58, bloom_gain);
    fx::vignette(&mut sub, 0.28);
    let glitch_amt = if (147.0..164.0).contains(&v.t) {
        0.30 + v.hit * 0.25
    } else if (125.0..148.0).contains(&v.t) {
        0.08 + v.hit * 0.10
    } else if (205.0..207.0).contains(&v.t) {
        0.5
    } else {
        0.0
    };
    if glitch_amt > 0.0 {
        let mut r = fx::Rng::new(v.frame * 2654435761 ^ 0x51ED);
        fx::glitch(&mut sub, &mut r, glitch_amt, v.t);
    }
    stage_put(cv, stage, &sub);

    // 外壳
    let dur = ly.lines.last().map(|l| l.t + 6.2).unwrap_or(212.0);
    let marks = tl.marks();
    chrome::draw_header(cv, v, ly, dur);
    chrome::draw_spectrum(cv, v, lay.y_spectrum);
    chrome::draw_progress(cv, v, lay.y_progress, dur, &marks);
    chrome::draw_lyrics(cv, v, ly, lay.y_en, lay.y_zh);

    // "孤独"段落的整体褪色：颜色被抽走
    let desat = if (106.8..=118.0).contains(&v.t) {
        ((v.t - 106.8) / 8.0).min(0.82)
    } else if (118.0..127.0).contains(&v.t) {
        (0.82 - (v.t - 118.0) / 9.0 * 0.55).max(0.0)
    } else if (127.0..135.0).contains(&v.t) {
        ((v.t - 127.0) / 8.0 * 0.5).min(0.5)
    } else {
        0.0
    };
    if desat > 0.01 {
        cv.desaturate(desat);
    }
}

fn main() {
    let args = parse_args();

    // 终端异常退出保护：任何 panic 都要把终端还原回去
    std::panic::set_hook(Box::new(|info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::style::ResetColor,
            crossterm::cursor::Show
        );
        eprintln!("\n\x1b[31m[panic]\x1b[0m {info}");
    }));

    let exe_hint = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "world-execute-me".into());
    println!("\x1b[36m▌\x1b[0m \x1b[1mworld.execute(me);\x1b[0m  \x1b[2mterminal MV · v{}\x1b[0m", env!("CARGO_PKG_VERSION"));
    println!("\x1b[2m  {exe_hint}\x1b[0m");

    // ── 0. 3D 引擎巡演模式：不需要音频与歌词（与 --shots 同给时走离屏校验）──
    if args.demo3d && args.shots.is_none() {
        demo3d::run();
        return;
    }

    // ── 1. 读取音频 ──
    let path = resolve_track(&args.file);
    print!("\x1b[2m[1/3]\x1b[0m 读取音频  ");
    let _ = std::io::stdout().flush();
    let t0 = Instant::now();
    match &path {
        Some(p) => println!("\x1b[32mok\x1b[0m  {}", p.display()),
        None => println!("\x1b[33m未找到\x1b[0m  （以静音模式运行）"),
    }
    let reading = t0.elapsed();

    // ── 2. 离线频谱分析 ──
    print!("\x1b[2m[2/3]\x1b[0m 频谱分析  ");
    let _ = std::io::stdout().flush();
    let t0 = Instant::now();
    let (info, spec) = match &path {
        Some(p) => audio::analyze(p),
        None => (
            audio::TrackInfo {
                path: "(none)".into(),
                bytes: 0,
                sample_rate: 44100,
                channels: 2,
                duration: 212.0,
                frames: 0,
            },
            audio::Spectrum::empty(212.0),
        ),
    };
    let an = t0.elapsed();
    println!(
        "\x1b[32mok\x1b[0m  {:.1}s · {} Hz · {} ch · {} 帧包络 · {} 个重音点  \x1b[2m({:.2}s)\x1b[0m",
        info.duration,
        info.sample_rate,
        info.channels,
        info.frames,
        spec.peak_beats.len(),
        an.as_secs_f32()
    );

    // ── 3. 歌词 ──
    print!("\x1b[2m[3/3]\x1b[0m 解析歌词  ");
    let _ = std::io::stdout().flush();
    let ly = lyrics::load();
    let paired = ly.lines.iter().filter(|l| l.zh.is_some()).count();
    let kw = ly
        .lines
        .iter()
        .filter(|l| l.tokens.iter().any(|t| t.kw.is_some()))
        .count();
    println!(
        "\x1b[32mok\x1b[0m  {} 行 · 双语对齐 {} 行 · 关键词句 {} 行  \x1b[2m(资源内嵌)\x1b[0m",
        ly.lines.len(),
        paired,
        kw
    );

    let mut tl = scenes::build();
    let mut v = view::View::new();
    println!(
        "\x1b[2m       时间轴 {} 个场景 · 音频读取 {:.2}s\x1b[0m",
        tl.segs.len(),
        reading.as_secs_f32()
    );

    // ── 视频渲染模式：逐帧导出 .bin 帧序列 ──
    if let Some(dir) = args.render_video.clone() {
        render_video(&dir, &mut tl, &ly, &info, spec);
        return;
    }

    // ── 离屏模式 ──
    if let Some(times) = args.shots.clone() {
        shots(&mut tl, &ly, &times, args.size, &args.out, spec, args.demo3d);
        return;
    }

    // ── 进入终端 ──
    let (tw, th) = term_size();
    let mut canvas = Canvas::new(tw, th);
    let mut lay = Layout::compute(tw, th);
    let mut t = match term::Term::new(tw, th) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("无法初始化终端：{e}");
            return;
        }
    };

    let mut aud = if args.no_audio || path.is_none() {
        let mut a = audio::Audio::silent(info.clone(), spec);
        if args.start > 0.0 {
            a.seek(args.start);
        }
        a
    } else {
        audio::Audio::play(path.as_ref().unwrap(), info.clone(), spec, args.start)
    };

    // ── 闪屏 ──
    if args.splash {
        let t0 = Instant::now();
        loop {
            let e = t0.elapsed().as_secs_f32();
            canvas.clear();
            for y in 0..canvas.h {
                for x in 0..canvas.w {
                    if fx::hash2(x, y, (e * 20.0) as u32) > 0.9975 {
                        canvas.put(x, y, '·', theme::CYAN_DIM, theme::BG);
                    }
                }
            }
            chrome::draw_title_card(&mut canvas, "SPACE 开始", e);
            let _ = t.present(&canvas);
            if poll(Duration::from_millis(16)).unwrap_or(false) {
                if let Ok(Event::Key(k)) = read() {
                    if k.kind == KeyEventKind::Press {
                        match k.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                t.exit();
                                return;
                            }
                            KeyCode::Char('h') => {
                                canvas.clear();
                                chrome::draw_help(&mut canvas);
                                let _ = t.present(&canvas);
                                std::thread::sleep(Duration::from_millis(2500));
                            }
                            _ => break,
                        }
                    }
                }
            }
            if e > 4.0 {
                break;
            }
        }
    }

    aud.start();
    let mut last = Instant::now();
    let mut show_help = false;
    let mut time = args.start;
    let frame_target = Duration::from_secs_f32(1.0 / args.fps.max(15.0));

    loop {
        let frame_start = Instant::now();
        let dt = (frame_start - last).as_secs_f32().clamp(0.0005, 0.1);
        last = frame_start;

        while poll(Duration::from_millis(0)).unwrap_or(false) {
            match read() {
                Ok(Event::Key(k)) if k.kind == KeyEventKind::Press => match k.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        t.exit();
                        return;
                    }
                    KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                        t.exit();
                        return;
                    }
                    KeyCode::Char(' ') => {
                        if aud.is_playing() {
                            aud.pause();
                        } else {
                            aud.resume();
                        }
                    }
                    KeyCode::Char('h') => show_help = !show_help,
                    KeyCode::Char('r') => {
                        aud.seek(0.0);
                        aud.resume();
                    }
                    KeyCode::Left => {
                        let nt = (time - 5.0).max(0.0);
                        aud.seek(nt);
                    }
                    KeyCode::Right => {
                        aud.seek((time + 5.0).min(info.duration - 1.0));
                    }
                    _ => {}
                },
                Ok(Event::Resize(w, h)) => {
                    canvas = Canvas::new(w as i32, h as i32);
                    lay = Layout::compute(w as i32, h as i32);
                    t.resize(w as i32, h as i32);
                }
                _ => {}
            }
        }

        time = aud.time();
        let paused = !aud.is_playing();
        v.update(&mut aud, &ly, time, dt);
        v.paused = paused;
        v.show_help = show_help;
        let dur = ly.lines.last().map(|l| l.t + 6.2).unwrap_or(212.0);
        v.finished = time >= dur - 0.1;

        if !Layout::min_size_ok(canvas.w, canvas.h) {
            chrome::draw_too_small(&mut canvas);
        } else {
            render_frame(&mut canvas, &mut tl, &mut v, &ly, &lay, dt);
            if show_help {
                chrome::draw_help(&mut canvas);
            }
            if paused && !v.finished {
                canvas.shade(0.5);
                canvas.text_center(
                    canvas.w / 2,
                    canvas.h / 2,
                    "‖  PAUSED   —   SPACE 继续",
                    theme::WHITE,
                    theme::VOID,
                );
            }
        }
        let _ = t.present(&canvas);

        if time >= dur + 0.8 {
            break;
        }
        let el = frame_start.elapsed();
        if el < frame_target {
            std::thread::sleep(frame_target - el);
        }
    }

    t.exit();
    println!();
    println!("  \x1b[2m$\x1b[0m world.execute(me);");
    println!(
        "  \x1b[32m[ok]\x1b[0m 进程 0x4C4F5645 已退出 — 曲长 {:.1}s，终端已还原",
        info.duration
    );
    println!("  \x1b[2m（你还在。它没有了。）\x1b[0m");
    println!();
}

/// 相对路径按"当前目录 / 上级目录 / 可执行文件所在目录"依次尝试
fn resolve_track(p: &Path) -> Option<PathBuf> {
    if p.is_absolute() {
        return if p.exists() { Some(p.to_path_buf()) } else { None };
    }
    let mut cands: Vec<PathBuf> = Vec::new();
    cands.push(p.to_path_buf());
    if let Ok(cwd) = std::env::current_dir() {
        cands.push(cwd.join(p));
        if let Some(name) = p.file_name() {
            cands.push(cwd.join("..").join(name));
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if let Some(name) = p.file_name() {
                cands.push(dir.join(name));
                cands.push(dir.join("..").join(name));
                cands.push(dir.join("../..").join(name));
            }
            // exe 在 target/release 下时，仓库根的相对路径（assets/audio/…）
            cands.push(dir.join("../..").join(p));
        }
    }
    for c in cands {
        if c.exists() {
            return Some(c);
        }
    }
    None
}

/// 视频渲染模式：从 0 顺序步进到曲终，把每一帧写成 .bin（WEMV 格式）。
/// 与实时播放共用 render_frame，从 t=0 顺序推进本身就是"预热"，
/// 因此逐帧确定、可复现，合成出的视频绝对流畅（严格 60fps、零丢帧）。
fn render_video(
    dir: &Path,
    tl: &mut Timeline,
    ly: &lyrics::Lyrics,
    info: &audio::TrackInfo,
    spec: audio::Spectrum,
) {
    let _ = std::fs::create_dir_all(dir);
    let (w, h) = (160, 48);
    let mut canvas = Canvas::new(w, h);
    let lay = Layout::compute(w, h);
    let mut v = view::View::new();
    let mut aud = audio::Audio::silent(info.clone(), spec);
    // 与交互模式同一套终局判定：最后一句歌词 + 6.2s 余韵 + 0.8s 黑场
    let dur = ly.lines.last().map(|l| l.t + 6.2).unwrap_or(212.0) + 0.8;
    let frames = (dur * 60.0).ceil() as i32;
    const DT: f32 = 1.0 / 60.0;
    let t0 = Instant::now();
    for k in 0..frames {
        let t = k as f32 * DT;
        v.update(&mut aud, ly, t, DT);
        render_frame(&mut canvas, tl, &mut v, ly, &lay, DT);
        let p = dir.join(format!("f_{:05}.bin", k));
        if std::fs::write(&p, term::frame_to_bin(&canvas)).is_err() {
            eprintln!("写出 {p:?} 失败");
            return;
        }
        if k % 600 == 0 {
            let el = t0.elapsed().as_secs_f32();
            println!(
                "  {:>5.1}%  帧 {}/{}  t={:.1}s  ({:.0} 帧/s)",
                k as f32 / frames as f32 * 100.0,
                k + 1,
                frames,
                t,
                if el > 0.0 { k as f32 / el } else { 0.0 }
            );
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
    }
    println!(
        "已导出 {} 帧到 {}（曲长 {:.1}s · 耗时 {:.1}s）",
        frames,
        dir.display(),
        frames as f32 * DT,
        t0.elapsed().as_secs_f32()
    );
}

/// 离屏渲染若干时间点到 HTML，用于开发期视觉校验。
/// 带 `--demo3d` 时校验的是 3D 巡演画面而非正片时间轴。
fn shots(
    tl: &mut Timeline,
    ly: &lyrics::Lyrics,
    times: &[f32],
    size: (i32, i32),
    out: &Path,
    spec: audio::Spectrum,
    demo3d: bool,
) {
    let (w, h) = if size.0 > 0 && size.1 > 0 {
        size
    } else {
        (160, 48)
    };
    let mut canvas = Canvas::new(w, h);
    let lay = Layout::compute(w, h);
    let mut frames: Vec<(f32, Canvas)> = Vec::new();
    let info = audio::TrackInfo {
        path: String::new(),
        bytes: 0,
        sample_rate: 44100,
        channels: 2,
        duration: 212.0,
        frames: 0,
    };
    for &t in times {
        let mut v = view::View::new();
        let mut aud = audio::Audio::silent(info.clone(), spec.clone());
        if demo3d {
            // 3D 巡演不需要预热：draw 是纯时间函数
            demo3d::draw(&mut canvas, t);
            frames.push((t, canvas.clone()));
            continue;
        }
        // 预热：从 t-4s 开始按 60fps 逐帧步进，把依赖 dt 累加的状态
        // （粒子、出字动画、场景内部计时）推到目标时刻的真实位置。
        // 否则离屏单帧看到的是"刚进场景第一帧"的假象。
        const DT: f32 = 1.0 / 60.0;
        let warm = (t - 4.0).max(0.0);
        let steps = ((t - warm) / DT) as i32;
        for k in 0..steps {
            let tt = warm + k as f32 * DT;
            v.update(&mut aud, ly, tt, DT);
            render_frame(&mut canvas, tl, &mut v, ly, &lay, DT);
        }
        v.update(&mut aud, ly, t, DT);
        render_frame(&mut canvas, tl, &mut v, ly, &lay, DT);
        frames.push((t, canvas.clone()));
    }
    let html = term::shots_html(&frames, 11.0);
    if let Err(e) = std::fs::write(out, html) {
        eprintln!("写出 {out:?} 失败：{e}");
    }
    // 同时输出逐帧独立页面，方便一张一张截图检查
    let stem = out
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "frame".into());
    let dir = out.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    for (i, (t, cv)) in frames.iter().enumerate() {
        let p = dir.join(format!("{stem}_{i:02}_{t:.2}s.html"));
        let _ = std::fs::write(p, term::frame_html_page(cv, 11.0));
        let b = dir.join(format!("{stem}_{i:02}.bin"));
        let _ = std::fs::write(b, term::frame_to_bin(cv));
    }
    println!("已写出 {} 帧到 {}（另含逐帧页面）", frames.len(), out.display());
}
