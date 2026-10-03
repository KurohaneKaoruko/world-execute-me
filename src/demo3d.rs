//! demo3d —— 独立 3D 巡演模式（--demo3d）。
//!
//! 不读音频、不进主时间轴：用 fx3d 引擎全屏轮播四个招牌对象
//! （环面纽结 / 线框地球 / 旋转心 / 旋涡星系），作为引擎的可视化自检与彩蛋。

use std::io::Write;
use std::time::{Duration, Instant};

use crossterm::event::{poll, read, Event, KeyCode, KeyEventKind};

use crate::bigfont;
use crate::buf::Canvas;
use crate::fx;
use crate::fx3d::{self, R3, Target, Xform};
use crate::term::Term;
use crate::theme;

const MODE_LEN: f32 = 7.0;
const MODES: usize = 4;

const MODE_NAMES: [&str; 4] = ["TORUS KNOT", "WIREFRAME WORLD", "HEART OF REVOLUTION", "SPIRAL GALAXY"];
const MODE_TAGS: [&str; 4] = ["(2,3) trefoil", "lon×lat + fibonacci dust", "implicit curve → solid", "8000 stars · 3 arms"];

pub fn run() {
    let (tw, th) = if let Ok((w, h)) = crossterm::terminal::size() {
        (w as i32, h as i32)
    } else {
        (120, 40)
    };
    let mut canvas = Canvas::new(tw, th);
    let mut t = match Term::new(tw, th) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("无法初始化终端：{e}");
            return;
        }
    };
    let t0 = Instant::now();
    loop {
        let frame_start = Instant::now();
        if poll(Duration::from_millis(0)).unwrap_or(false) {
            if let Ok(Event::Key(k)) = read() {
                if k.kind == KeyEventKind::Press && matches!(k.code, KeyCode::Char('q') | KeyCode::Esc) {
                    break;
                }
            }
        }
        let time = t0.elapsed().as_secs_f32();
        draw(&mut canvas, time);
        let _ = t.present(&canvas);
        let el = frame_start.elapsed();
        let target = Duration::from_secs_f32(1.0 / 60.0);
        if el < target {
            std::thread::sleep(target - el);
        }
    }
    t.exit();
    let _ = std::io::stdout().flush();
    println!("  \x1b[2mfx3d demo exited — the engine keeps executing.\x1b[0m");
}

/// 渲染一帧巡演画面（纯时间函数，离屏校验与实时共用）。
pub fn draw(cv: &mut Canvas, t: f32) {
    cv.clear();
    let w = cv.w;
    let h = cv.h;
    let mode = ((t / MODE_LEN) as usize) % MODES;
    let mt = t % MODE_LEN;
    // 模式首尾各 0.7s 淡入淡出
    let fade = (mt / 0.7).clamp(0.0, 1.0).min(((MODE_LEN - mt) / 0.7).clamp(0.0, 1.0));
    let bright = fx::smooth(fade);

    let mut r3 = R3::new(w, h);
    r3.begin();
    let n_pts;
    {
        let mut tg = Target::new(cv, 0, 0, w, h);
        match mode {
            0 => {
                // 环面纽结：管面兰伯特着色；初始姿态摆到三叶剪影最可读的角度
                let cloud = fx3d::knot_cloud(2, 3, 170, 0.20, 16);
                n_pts = cloud.pts.len();
                let xf = Xform::scaled(0.6 + t * 0.35, 0.5 + 0.30 * (t * 0.33).sin(), t * 0.2, 0.85);
                r3.cam_z = 3.1;
                r3.focal = h as f32 * 2.1;
                r3.surface(
                    &mut tg,
                    &cloud.pts,
                    &cloud.nrm,
                    &xf,
                    theme::CYAN.mix(theme::BLUE, 0.25),
                    theme::WHITE,
                    5.0,
                    bright * (1.0 + 0.15 * (t * 2.1).sin()),
                    0.95,
                );
                // 伴飞尘
                let dust = fx3d::fib_sphere(160);
                let dxf = Xform::new(-t * 0.2, 0.5, 0.0);
                r3.cloud(&mut tg, &dust, &dxf, theme::PURPLE, theme::PURPLE_DIM, fx3d::DEPTH_RAMP, 6.0, bright * 0.7, 0.7);
            }
            1 => {
                // 线框地球：斐波那契"大陆"点尘 + 经纬线框 + 土星环
                n_pts = 0;
                let land = fx3d::fib_sphere(340);
                let lxf = Xform::scaled(t * 0.32 + 0.4, 0.30, 0.0, 0.995);
                r3.cloud(&mut tg, &land, &lxf, theme::GREEN, theme::GREEN_DIM, fx3d::DEPTH_RAMP, 5.2, bright * 0.85, 0.85);
                // 线框最后画：半径更大，在正面赢下 Z-Buffer，背面被正确遮挡
                let globe = fx3d::latlong_mesh(14, 9, 1.0);
                let xf = Xform::new(t * 0.32, 0.30, 0.0);
                r3.cam_z = 3.2;
                r3.focal = h as f32 * 2.05;
                r3.wire(&mut tg, &globe, &xf, theme::CYAN, theme::WHITE, 5.4, bright * 1.35, 0.9);
                let ring = fx3d::torus_cloud(1.45, 0.015, 90, 4);
                let rxf = Xform::new(-t * 0.12, 1.15, 0.35);
                r3.surface(&mut tg, &ring.pts, &ring.nrm, &rxf, theme::AMBER, theme::WHITE, 5.4, bright * 0.8, 0.85);
            }
            2 => {
                // 心形曲面：绕 y 旋转的"立体心"，随时间搏动
                n_pts = 0;
                let heart = fx3d::heart_cloud(120, 40);
                let beat = 1.0 + 0.06 * (t * 2.4).sin().max(0.0).powf(3.0);
                let xf = Xform::scaled(t * 0.4, 0.20 * (t * 0.5).sin(), 0.0, 0.92 * beat);
                r3.cam_z = 3.6;
                r3.focal = h as f32 * 2.0;
                r3.surface(&mut tg, &heart.pts, &heart.nrm, &xf, theme::MAGENTA, theme::WHITE, 5.6, bright * (1.0 + 0.25 * (t * 2.4).sin().max(0.0)), 0.95);
                // 环绕的 ♥ 轨道
                let mut orb: Vec<fx3d::Vec3> = Vec::new();
                for k in 0..48 {
                    let a = k as f32 / 48.0 * fx3d::TAU + t * 0.8;
                    orb.push(fx3d::Vec3::new(a.cos() * 1.6, (a * 2.0).sin() * 0.22, a.sin() * 1.6));
                }
                let oxf = Xform::new(0.0, 0.45, 0.0);
                r3.cloud(&mut tg, &orb, &oxf, theme::MAGENTA.mix(theme::WHITE, 0.4), theme::MAGENTA_DIM, &['♥', '·'], 5.6, bright, 0.9);
            }
            _ => {
                // 旋涡星系：缓慢进动的悬臂 + 白热核球
                n_pts = 0;
                let (arms, bulge) = fx3d::galaxy_cloud(3400, 3, 0x6A11_0C);
                let xf = Xform::new(t * 0.16, 0.9 + 0.12 * (t * 0.21).sin(), 0.0);
                r3.cam_z = 3.4;
                r3.focal = h as f32 * 2.05;
                r3.cloud(&mut tg, &arms.pts, &xf, theme::CYAN, theme::PURPLE_DIM, fx3d::DEPTH_RAMP, 6.2, bright * 0.95, 0.92);
                r3.cloud(&mut tg, &bulge.pts, &xf, theme::WHITE, theme::AMBER_DIM, &['@', '●', '·'], 6.2, bright, 0.95);
            }
        }
    }

    // CRT 后处理（3D 先画，HUD 后画——保证文字清晰不被辉光糊住）
    fx::bloom(cv, 0.52, 0.55);
    fx::scanlines(cv, t, 0.05);
    fx::vignette(cv, 0.22);

    // HUD：模式名 + 副标题
    let name = MODE_NAMES[mode];
    bigfont::center(
        cv,
        w / 2,
        1,
        name,
        theme::WHITE.mul(0.95 * bright),
        theme::BG,
        1,
    );
    let tag = format!(
        "fx3d // {} · pts {:>5} · z-buffer {}×{} · lambert on",
        MODE_TAGS[mode], n_pts, w, h
    );
    cv.text_center(w / 2, 8, &tag, theme::TEXT_FAINT.mul(bright), theme::BG);

    // 底部：模式进度点（左）+ 退出提示（中）
    for k in 0..MODES as i32 {
        let on = k == mode as i32;
        let ch = if on { '◆' } else { '◇' };
        cv.put(6 + k * 2, h - 2, ch, if on { theme::CYAN } else { theme::TEXT_FAINT }, theme::BG);
    }
    cv.text_center(w / 2, h - 2, "Q / ESC 退出", theme::TEXT_DIM, theme::BG);
}
