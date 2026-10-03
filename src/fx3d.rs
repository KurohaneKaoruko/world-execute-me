//! fx3d —— 终端 3D 引擎：透视投影 + Z-Buffer + 兰伯特着色。
//!
//! 为字符网格定制的小型软件渲染器：
//!   · 终端格子约 2:1，投影时 y 方向折半，世界里的正圆在屏幕上也是正圆
//!   · [`R3`] 持有每格深度的 Z-Buffer，云点 / 线框 / 曲面共享同一深度，互相遮挡正确
//!   · 云点按"深度 → 字符/颜色"映射，曲面按兰伯特光照选半块字符的明暗
//!   · 线段做近平面裁剪，物体穿过镜头时不会炸出错误像素
//!
//! 所有生成器（环面、纽结、心形、星系…）都是纯函数：场景在 `new()` 里
//! 预计算网格，`draw()` 里只做旋转 + 投影，离屏渲染逐帧可复现。

// 引擎 API 按需取用，允许暂未被场景引用的部分
#![allow(dead_code)]

use crate::buf::{Canvas, Rgb};
use crate::fx::Rng;

pub const TAU: f32 = std::f32::consts::TAU;

/// 视向光源（相机空间，指向光源方向）。
const LIGHT: Vec3 = Vec3::new(-0.45, 0.62, -0.65);

// ── 向量 ────────────────────────────────────────────────────
#[derive(Clone, Copy, Debug, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }
    pub fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
    pub fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
    pub fn scale(self, f: f32) -> Vec3 {
        Vec3::new(self.x * f, self.y * f, self.z * f)
    }
    pub fn dot(self, o: Vec3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    pub fn len(self) -> f32 {
        self.dot(self).sqrt()
    }
    pub fn norm(self) -> Vec3 {
        let l = self.len();
        if l < 1e-6 {
            Vec3::new(0.0, 1.0, 0.0)
        } else {
            self.scale(1.0 / l)
        }
    }
    pub fn lerp(self, o: Vec3, t: f32) -> Vec3 {
        self.add(o.sub(self).scale(t))
    }
    pub fn rot_y(self, a: f32) -> Vec3 {
        let (s, c) = a.sin_cos();
        Vec3::new(self.x * c - self.z * s, self.y, self.x * s + self.z * c)
    }
    pub fn rot_x(self, a: f32) -> Vec3 {
        let (s, c) = a.sin_cos();
        Vec3::new(self.x, self.y * c - self.z * s, self.y * s + self.z * c)
    }
    pub fn rot_z(self, a: f32) -> Vec3 {
        let (s, c) = a.sin_cos();
        Vec3::new(self.x * c - self.y * s, self.x * s + self.y * c, self.z)
    }
}

/// 欧拉旋转 + 均匀缩放：先绕 Z，再绕 X，最后绕 Y。
/// （"自转轴先定姿态，再整体进动"的运动都用这个顺序组合。）
#[derive(Clone, Copy)]
pub struct Xform {
    pub ry: f32,
    pub rx: f32,
    pub rz: f32,
    pub scale: f32,
}

impl Xform {
    pub fn new(ry: f32, rx: f32, rz: f32) -> Self {
        Xform { ry, rx, rz, scale: 1.0 }
    }
    pub fn scaled(ry: f32, rx: f32, rz: f32, scale: f32) -> Self {
        Xform { ry, rx, rz, scale }
    }
    #[inline]
    pub fn apply(&self, v: Vec3) -> Vec3 {
        v.rot_z(self.rz).rot_x(self.rx).rot_y(self.ry).scale(self.scale)
    }
    /// 只旋转不缩放（法线方向）。
    #[inline]
    pub fn apply_dir(&self, v: Vec3) -> Vec3 {
        v.rot_z(self.rz).rot_x(self.rx).rot_y(self.ry)
    }
}

// ── 网格与点云 ──────────────────────────────────────────────
/// 线框网格：顶点 + 边（顶点索引对）。
pub struct Mesh {
    pub verts: Vec<Vec3>,
    pub edges: Vec<(u16, u16)>,
}

impl Mesh {
    pub fn new() -> Self {
        Mesh { verts: Vec::new(), edges: Vec::new() }
    }
    fn push_vert(&mut self, v: Vec3) -> u16 {
        self.verts.push(v);
        (self.verts.len() - 1) as u16
    }
}

/// 云点/曲面网格：`nrm` 为空时按深度着色，非空时按兰伯特光照着色。
pub struct Cloud {
    pub pts: Vec<Vec3>,
    pub nrm: Vec<Vec3>,
}

impl Cloud {
    pub fn points(pts: Vec<Vec3>) -> Self {
        Cloud { pts, nrm: Vec::new() }
    }
}

/// 深度 → 字符（近 → 远）。
pub const DEPTH_RAMP: &[char] = &['@', '◆', '●', '○', '·', '.'];
/// 兰伯特亮度 → 字符（暗 → 亮）。
pub const SHADE_RAMP: &[char] = &['·', '░', '▒', '▓', '█'];

// ── 网格生成器 ──────────────────────────────────────────────
/// 斐波那契球面均匀点云（单位半径）。
pub fn fib_sphere(n: usize) -> Vec<Vec3> {
    let ga = std::f32::consts::PI * (3.0 - 5.0f32.sqrt());
    (0..n)
        .map(|i| {
            let y = 1.0 - (i as f32 / (n - 1).max(1) as f32) * 2.0;
            let r = (1.0 - y * y).max(0.0).sqrt();
            let th = ga * i as f32;
            Vec3::new(th.cos() * r, y, th.sin() * r)
        })
        .collect()
}

/// 正二十面体线框。
pub fn icosa_mesh() -> Mesh {
    let t = (1.0 + 5.0f32.sqrt()) / 2.0;
    let raw = [
        (-1.0, t, 0.0), (1.0, t, 0.0), (-1.0, -t, 0.0), (1.0, -t, 0.0),
        (0.0, -1.0, t), (0.0, 1.0, t), (0.0, -1.0, -t), (0.0, 1.0, -t),
        (t, 0.0, -1.0), (t, 0.0, 1.0), (-t, 0.0, -1.0), (-t, 0.0, 1.0),
    ];
    let verts: Vec<Vec3> = raw.iter().map(|(x, y, z)| Vec3::new(*x, *y, *z).norm()).collect();
    // 距离最近的顶点对连边
    let mut min = f32::MAX;
    for i in 0..12 {
        for j in (i + 1)..12 {
            min = min.min(verts[i].sub(verts[j]).len());
        }
    }
    let mut edges = Vec::new();
    for i in 0..12u16 {
        for j in (i + 1)..12u16 {
            if verts[i as usize].sub(verts[j as usize]).len() < min * 1.15 {
                edges.push((i, j));
            }
        }
    }
    Mesh { verts, edges }
}

/// 经纬线球（线框"地球"）。lon_n 条经线 × lat_n 条纬线。
pub fn latlong_mesh(lon_n: usize, lat_n: usize, r: f32) -> Mesh {
    let mut m = Mesh::new();
    let lats = lat_n.max(3);
    let lons = lon_n.max(4);
    // 纬线环（避开极点）
    let mut rings: Vec<Vec<u16>> = Vec::new();
    for k in 1..lats {
        let phi = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * k as f32 / lats as f32;
        let (sp, cp) = phi.sin_cos();
        let mut ring = Vec::new();
        for i in 0..lons {
            let th = i as f32 / lons as f32 * TAU;
            ring.push(m.push_vert(Vec3::new(cp * th.cos() * r, sp * r, cp * th.sin() * r)));
        }
        rings.push(ring);
    }
    for ring in &rings {
        for i in 0..lons {
            m.edges.push((ring[i], ring[(i + 1) % lons]));
        }
    }
    // 经线：相邻纬环纵向连接 + 两端接极点
    let pole_n = m.push_vert(Vec3::new(0.0, r, 0.0));
    let pole_s = m.push_vert(Vec3::new(0.0, -r, 0.0));
    for i in (0..lons).step_by(2) {
        m.edges.push((pole_n, rings[0][i]));
        m.edges.push((pole_s, rings[rings.len() - 1][i]));
    }
    for w in rings.windows(2) {
        for i in 0..lons {
            m.edges.push((w[0][i], w[1][i]));
        }
    }
    m
}

/// 立方体线框（半边长 1）。`sub` = 每条棱细分的段数（>1 时棱上出现"栏杆"）。
pub fn box_mesh(sub: usize) -> Mesh {
    let sub = sub.max(1);
    let c: [Vec3; 8] = [
        Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(1.0, 1.0, -1.0), Vec3::new(-1.0, 1.0, -1.0),
        Vec3::new(-1.0, -1.0, 1.0), Vec3::new(1.0, -1.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0), Vec3::new(-1.0, 1.0, 1.0),
    ];
    let e: [(usize, usize); 12] = [
        (0, 1), (1, 2), (2, 3), (3, 0), // 前
        (4, 5), (5, 6), (6, 7), (7, 4), // 后
        (0, 4), (1, 5), (2, 6), (3, 7), // 连接
    ];
    let mut m = Mesh::new();
    let mut idx = [[0u16; 16]; 12];
    for (k, &(a, b)) in e.iter().enumerate() {
        for s in 0..=sub {
            let t = s as f32 / sub as f32;
            idx[k][s] = m.push_vert(c[a].lerp(c[b], t));
        }
    }
    for k in 0..12 {
        for s in 0..sub {
            m.edges.push((idx[k][s], idx[k][s + 1]));
        }
    }
    m
}

/// 平面网格线（XZ 平面，范围 ±1）。`n` = 每方向线数，`y` = 平面高度
///（牢笼场景里烘进顶点后随整体缩放，保证地板贴着笼壁收缩）。
pub fn floor_mesh(n: usize, y: f32) -> Mesh {
    let n = n.max(2);
    let mut m = Mesh::new();
    let step = 2.0 / (n - 1) as f32;
    for k in 0..n {
        let p = -1.0 + k as f32 * step;
        let a = m.push_vert(Vec3::new(-1.0, y, p));
        let b = m.push_vert(Vec3::new(1.0, y, p));
        let c = m.push_vert(Vec3::new(p, y, -1.0));
        let d = m.push_vert(Vec3::new(p, y, 1.0));
        m.edges.push((a, b));
        m.edges.push((c, d));
    }
    m
}

/// 环面（主半径 R、管半径 r），带法线 → 可做兰伯特着色。
pub fn torus_cloud(ring_r: f32, tube_r: f32, nu: usize, nv: usize) -> Cloud {
    let mut pts = Vec::with_capacity(nu * nv);
    let mut nrm = Vec::with_capacity(nu * nv);
    for i in 0..nu {
        let u = i as f32 / nu as f32 * TAU;
        let (cu, su) = u.sin_cos();
        for j in 0..nv {
            let v = j as f32 / nv as f32 * TAU;
            let (cv, sv) = v.sin_cos();
            let rr = ring_r + tube_r * cv;
            pts.push(Vec3::new(rr * cu, rr * su, tube_r * sv));
            nrm.push(Vec3::new(cv * cu, cv * su, sv));
        }
    }
    Cloud { pts, nrm }
}

/// (p,q) 环面纽结管。p=2,q=3 是三叶结。
pub fn knot_cloud(p: u32, q: u32, seg: usize, tube: f32, tube_seg: usize) -> Cloud {
    let pf = p as f32;
    let qf = q as f32;
    let curve = |u: f32| -> Vec3 {
        let t = u * TAU;
        let r = 2.0 + (qf * t).cos();
        Vec3::new(r * (pf * t).cos(), r * (pf * t).sin(), (qf * t).sin() * 1.30).scale(1.0 / 2.65)
    };
    let mut pts = Vec::with_capacity(seg * tube_seg);
    let mut nrm = Vec::with_capacity(seg * tube_seg);
    for i in 0..seg {
        let u = i as f32 / seg as f32;
        let c = curve(u);
        let t = curve(u + 0.003).sub(curve(u - 0.003)).norm();
        let ref_up = if t.z.abs() > 0.9 { Vec3::new(0.0, 1.0, 0.0) } else { Vec3::new(0.0, 0.0, 1.0) };
        let n = ref_up.cross(t).norm();
        let b = t.cross(n).norm();
        for j in 0..tube_seg {
            let a = j as f32 / tube_seg as f32 * TAU;
            let (ca, sa) = a.sin_cos();
            let dir = n.scale(ca).add(b.scale(sa));
            pts.push(c.add(dir.scale(tube)));
            nrm.push(dir);
        }
    }
    Cloud { pts, nrm }
}

/// 心形曲面：把隐式曲线 (x²+y²−1)³ − x²y³ = 0 的轮廓绕 y 轴旋转，
/// 得到有凹槽的"立体心"。带法线。
pub fn heart_cloud(nprof: usize, nring: usize) -> Cloud {    let f = |x: f32, y: f32| -> f32 {
        let a = x * x + y * y - 1.0;
        a * a * a - x * x * y * y * y
    };
    // 逐高度从外部向内找轮廓外边界：最大 |x| 使 F ≤ 0
    let nprof = nprof.max(24);
    let mut prof: Vec<(f32, f32)> = Vec::new(); // (r, y)，y 从顶到底
    for i in 0..nprof {
        let y = 1.30 - 2.32 * i as f32 / (nprof - 1) as f32;
        let mut r = 0.0f32;
        let mut x = 1.42f32;
        while x > 0.0 {
            if f(x, y) <= 0.0 {
                r = x;
                break;
            }
            x -= 1.42 / 220.0;
        }
        if r > 0.015 {
            prof.push((r, y));
        }
    }
    revolve_cloud(&prof, 0.62, nring)
}

/// 通用旋转体：把"半宽轮廓"(r, y) 绕 y 轴旋成曲面（squash 压扁 z 轴）。
/// heart / 茄子 / 番茄共用同一套数值法线着色。
pub fn revolve_cloud(prof: &[(f32, f32)], squash: f32, nring: usize) -> Cloud {
    let nring = nring.max(6);
    let n = prof.len();
    let mut pts = Vec::with_capacity(n * nring);
    let mut nrm = Vec::with_capacity(n * nring);
    for k in 0..n {
        let (r, y) = prof[k];
        let r_prev = prof[k.saturating_sub(1)].0;
        let r_next = prof[(k + 1).min(n - 1)].0;
        let y_prev = prof[k.saturating_sub(1)].1;
        let y_next = prof[(k + 1).min(n - 1)].1;
        let dy = (y_next - y_prev).abs().max(1e-4);
        let dr = (r_next - r_prev) / dy;
        for j in 0..nring {
            let th = j as f32 / nring as f32 * TAU;
            let (ct, st) = th.sin_cos();
            pts.push(Vec3::new(r * ct, y, r * st * squash));
            let nv = Vec3::new(squash * r * ct, -squash * r * dr, r * st);
            nrm.push(nv.norm());
        }
    }
    Cloud { pts, nrm }
}

/// 旋涡星系点云（盘面在 XZ 平面）。`bulge` 返回中心核球（另配色用）。
pub fn galaxy_cloud(n: usize, arms: usize, seed: u64) -> (Cloud, Cloud) {
    let mut rng = Rng::new(seed);
    let mut pts = Vec::with_capacity(n);
    let n_bulge = (n / 7).max(12);
    let mut bulge = Vec::with_capacity(n_bulge);
    for _ in 0..n_bulge {
        let v = Vec3::new(rng.range(-1.0, 1.0), rng.range(-1.0, 1.0), rng.range(-1.0, 1.0));
        bulge.push(v.scale(rng.range(0.02, 0.10)));
    }
    for _ in n_bulge..n {
        let arm = (rng.next_u64() % arms as u64) as f32;
        let t = rng.f().powf(0.62);
        let r = 0.10 + t * 1.05;
        let th = arm / arms as f32 * TAU + t * 4.4 + rng.range(-1.0, 1.0) * 0.34 / (0.25 + t);
        let y = rng.range(-1.0, 1.0) * 0.10 * (1.15 - t);
        pts.push(Vec3::new(r * th.cos(), y, r * th.sin()));
    }
    (Cloud::points(pts), Cloud::points(bulge))
}

// ── 画布桥接 ────────────────────────────────────────────────
/// 把引擎输出的"舞台相对坐标"翻译到画布绝对坐标，并处理透明混合。
pub struct Target<'a> {
    cv: &'a mut Canvas,
    ox: i32,
    oy: i32,
    w: i32,
    h: i32,
}

impl<'a> Target<'a> {
    pub fn new(cv: &'a mut Canvas, ox: i32, oy: i32, w: i32, h: i32) -> Self {
        Target { cv, ox, oy, w, h }
    }
    /// 半透明写入：空格直接写暗色，已有字符做前景混合；宽字符右半（SKIP）跳过。
    #[inline]
    pub fn blend(&mut self, x: i32, y: i32, ch: char, col: Rgb, alpha: f32) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        let ax = self.ox + x;
        let ay = self.oy + y;
        let e = match self.cv.get(ax, ay) {
            Some(c) => *c,
            None => return,
        };
        if e.ch == crate::buf::SKIP {
            return;
        }
        let fg = if e.ch == ' ' {
            col.mul(alpha)
        } else {
            e.fg.mix(col, alpha.clamp(0.0, 1.0))
        };
        self.cv.put_ov(ax, ay, ch, fg);
    }
}

// ── 渲染器 ──────────────────────────────────────────────────
/// 一帧的 3D 视口。相机在 (0,0,cam_z) 朝 −z 看；
/// depth = cam_z − z，越小越近。
pub struct R3 {
    pub w: i32,
    pub h: i32,
    pub cx: f32,
    pub cy: f32,
    /// 焦距（列单位）；y 投影时按 0.5 折算成行。
    pub focal: f32,
    pub cam_z: f32,
    pub near: f32,
    zbuf: Vec<f32>,
}

impl R3 {
    pub fn new(w: i32, h: i32) -> Self {
        let n = (w.max(1) * h.max(1)) as usize;
        R3 {
            w,
            h,
            cx: w as f32 / 2.0,
            cy: h as f32 / 2.0 - 1.0,
            focal: h as f32 * 2.0,
            cam_z: 3.2,
            near: 0.10,
            zbuf: vec![f32::INFINITY; n],
        }
    }
    pub fn begin(&mut self) {
        for z in self.zbuf.iter_mut() {
            *z = f32::INFINITY;
        }
    }
    /// 深度归一化：0 = 镜前，1 = span 处（雾/明暗用）。
    #[inline]
    pub fn depth01(&self, d: f32, span: f32) -> f32 {
        (d / span.max(1e-4)).clamp(0.0, 1.0)
    }
    #[inline]
    fn screens(&self, p: Vec3, d: f32) -> (f32, f32) {
        (self.cx + self.focal * p.x / d, self.cy - self.focal * 0.5 * p.y / d)
    }
    /// 投影一个（已旋转到相机空间的）点。返回 (sx, sy, depth)。
    #[inline]
    pub fn project(&self, p: Vec3) -> Option<(f32, f32, f32)> {
        let d = self.cam_z - p.z;
        if d <= self.near {
            return None;
        }
        let (sx, sy) = self.screens(p, d);
        Some((sx, sy, d))
    }
    /// Z-Buffer 测试 + 混合写入（场景可用 [`Self::project`] 的结果自定义绘制）。
    pub fn plot(&mut self, tg: &mut Target, sx: f32, sy: f32, d: f32, ch: char, col: Rgb, alpha: f32) {
        let ix = sx.round() as i32;
        let iy = sy.round() as i32;
        if ix < 0 || iy < 0 || ix >= self.w || iy >= self.h {
            return;
        }
        let zi = (iy * self.w + ix) as usize;
        if d > self.zbuf[zi] {
            return;
        }
        self.zbuf[zi] = d;
        tg.blend(ix, iy, ch, col, alpha);
    }
    /// 云点：按深度映射字符与颜色（无光照）。
    pub fn cloud(
        &mut self,
        tg: &mut Target,
        pts: &[Vec3],
        xf: &Xform,
        hi: Rgb,
        lo: Rgb,
        ramp: &[char],
        span: f32,
        bright: f32,
        alpha: f32,
    ) {
        for p in pts {
            let rp = xf.apply(*p);
            let Some((sx, sy, d)) = self.project(rp) else { continue };
            let dep = self.depth01(d, span);
            let ri = (((1.0 - dep) * (ramp.len() - 1) as f32).round()) as usize;
            let col = hi.mix(lo, dep).mul(bright * (1.15 - dep * 0.72));
            self.plot(tg, sx, sy, d, ramp[ri.min(ramp.len() - 1)], col, alpha);
        }
    }
    /// 曲面：兰伯特光照（双面）+ 深度雾，用半块字符表现明暗。
    pub fn surface(
        &mut self,
        tg: &mut Target,
        pts: &[Vec3],
        nrms: &[Vec3],
        xf: &Xform,
        base: Rgb,
        hot: Rgb,
        span: f32,
        bright: f32,
        alpha: f32,
    ) {
        for (i, p) in pts.iter().enumerate() {
            let rp = xf.apply(*p);
            let Some((sx, sy, d)) = self.project(rp) else { continue };
            let n = match nrms.get(i) {
                Some(n) => xf.apply_dir(*n),
                None => continue,
            };
            let lam = n.norm().dot(LIGHT).abs().powf(0.6); // gamma 提亮中间调
            let ri = ((lam * (SHADE_RAMP.len() - 1) as f32).round()) as usize;
            let dep = self.depth01(d, span);
            let col = base
                .mul(0.28 + 0.95 * lam)
                .mix(hot, lam * 0.42)
                .mul(bright * (1.12 - dep * 0.78));
            self.plot(tg, sx, sy, d, SHADE_RAMP[ri], col, alpha);
        }
    }
    /// 线框：近平面裁剪 + 沿线深度插值的 Z-Buffer 写入。
    pub fn wire(&mut self, tg: &mut Target, mesh: &Mesh, xf: &Xform, col: Rgb, hi: Rgb, span: f32, bright: f32, alpha: f32) {
        let rv: Vec<Vec3> = mesh.verts.iter().map(|v| xf.apply(*v)).collect();
        for &(a, b) in &mesh.edges {
            let (Some(&va), Some(&vb)) = (rv.get(a as usize), rv.get(b as usize)) else { continue };
            let mut da = self.cam_z - va.z;
            let mut db = self.cam_z - vb.z;
            if da <= self.near && db <= self.near {
                continue;
            }
            let (mut pa, mut pb) = (va, vb);
            if da < self.near {
                let t = (self.near - da) / (db - da);
                pa = va.lerp(vb, t);
                da = self.near;
            } else if db < self.near {
                let t = (self.near - db) / (da - db);
                pb = vb.lerp(va, t);
                db = self.near;
            }
            let (ax, ay) = self.screens(pa, da);
            let (bx, by) = self.screens(pb, db);
            let n = ((bx - ax).abs().max((by - ay).abs())).round() as i32;
            let n = n.clamp(1, 4000);
            for k in 0..=n {
                let t = k as f32 / n as f32;
                let sx = ax + (bx - ax) * t;
                let sy = ay + (by - ay) * t;
                let d = da + (db - da) * t;
                let dep = self.depth01(d, span);
                let c = col.mul(bright * (1.20 - dep * 0.60)).mix(hi, (1.0 - dep) * 0.30);
                self.plot(tg, sx, sy, d, '·', c, alpha);
            }
        }
    }
}

// ── 超时空星流（warp）───────────────────────────────────────/// 经典 hyperspace 星流：星星从深处向镜头扑来，近处拉出光痕。
pub struct Warp {
    stars: Vec<(f32, f32, f32)>, // (ux, uy, z0 ∈ [0,1))
    dmax: f32,
}

impl Warp {
    pub fn new(n: usize, seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let stars = (0..n)
            .map(|_| {
                let a = rng.range(0.0, TAU);
                let r = rng.f().powf(0.5) * 1.05;
                (a.cos() * r, a.sin() * r, rng.f())
            })
            .collect();
        Warp { stars, dmax: 1.9 }
    }
    /// speed：每秒掠过的深度比例。near/far 决定星色两头。
    pub fn draw(&mut self, tg: &mut Target, cx: f32, cy: f32, focal: f32, t: f32, speed: f32, near: Rgb, far: Rgb, bright: f32) {
        for &(ux, uy, z0) in &self.stars {
            let ph = (z0 - t * speed).rem_euclid(1.0);
            let d = 0.07 + ph * (self.dmax - 0.07);
            // 光痕：往更深处拖一小段
            let d2 = 0.07 + (ph + 0.020 + speed * 0.10).min(1.0) * (self.dmax - 0.07);
            let x1 = cx + focal * ux / d;
            let y1 = cy - focal * 0.5 * uy / d;
            let x2 = cx + focal * ux / d2;
            let y2 = cy - focal * 0.5 * uy / d2;
            let prox = 1.0 - (d - 0.07) / (self.dmax - 0.07); // 1 = 最贴近镜头
            let col = near.mix(far, 1.0 - prox).mul(bright * (0.30 + 0.85 * prox));
            let steps = 3;
            for k in 0..steps {
                let tt = k as f32 / steps as f32;
                let a = if k == 0 { 0.95 } else { 0.55 - tt * 0.3 };
                tg.blend(
                    (x1 + (x2 - x1) * tt).round() as i32,
                    (y1 + (y2 - y1) * tt).round() as i32,
                    if k == 0 && prox > 0.8 { '*' } else { '·' },
                    col,
                    a,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buf::SKIP;

    #[test]
    fn wire_draws_pixels() {
        let mut cv = Canvas::new(160, 48);
        let mut r3 = R3::new(160, 48);
        r3.begin();
        let mut tg = Target::new(&mut cv, 0, 0, 160, 48);
        let mesh = icosa_mesh();
        assert!(!mesh.edges.is_empty());
        r3.wire(&mut tg, &mesh, &Xform::new(0.0, 0.0, 0.0), theme_ok(), theme_ok(), 5.0, 1.0, 1.0);
        let n = cv.cells.iter().filter(|c| c.ch != ' ' && c.ch != SKIP).count();
        assert!(n > 50, "wire drew only {n} cells");
    }

    #[test]
    fn latlong_produces_edges() {
        let m = latlong_mesh(14, 9, 1.0);
        assert!(
            m.edges.len() > 100,
            "latlong edges = {}",
            m.edges.len()
        );
        let bad: usize = m
            .edges
            .iter()
            .filter(|(a, b)| (*a as usize) >= m.verts.len() || (*b as usize) >= m.verts.len())
            .count();
        assert_eq!(bad, 0, "dangling edges");
    }

    #[test]
    fn demo_globe_repro() {
        let mut cv = Canvas::new(160, 48);
        let mut r3 = R3::new(160, 48);
        r3.begin();
        let mut tg = Target::new(&mut cv, 0, 0, 160, 48);
        let t = 8.0f32;
        let bright = 1.0f32;
        let land = fib_sphere(340);
        let lxf = Xform::scaled(t * 0.32 + 0.4, 0.30, 0.0, 0.995);
        r3.cloud(&mut tg, &land, &lxf, Rgb::new(92, 228, 118), Rgb::new(28, 92, 48), DEPTH_RAMP, 5.2, bright * 0.85, 0.85);
        let globe = latlong_mesh(14, 9, 1.0);
        let xf = Xform::new(t * 0.32, 0.30, 0.0);
        r3.cam_z = 3.2;
        r3.focal = 48.0 * 2.05;
        r3.wire(&mut tg, &globe, &xf, Rgb::new(58, 214, 226), Rgb::new(240, 246, 255), 5.4, bright * 0.95, 0.9);
        let n = cv.cells.iter().filter(|c| c.ch != ' ' && c.ch != SKIP).count();
        println!("demo_globe_repro: {n} cells drawn, edges={}", globe.edges.len());
        assert!(n > 200, "only {n} cells");
    }

    fn theme_ok() -> Rgb {
        Rgb::new(200, 200, 200)
    }
}
