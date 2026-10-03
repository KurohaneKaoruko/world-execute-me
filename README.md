# world.execute(me); — 终端 MV

一个把 Mili 的《world.execute(me);》整首歌演绎成"一台终端机最后时刻"的 Rust CLI 播放器。
整部 MV 就是一次进程的生命周期：开机自检 → 世界生成 → 被执行 → 你离开 → 内存熄灭。

```
$ cargo run --release

# 或者，先看 3D 引擎巡演（不需要音频）：
$ cargo run --release -- --demo3d
```

## 它是什么样的

- **常驻"硬件"界面**：状态栏 / 32 段频谱 / 进度条（带场景刻度）/ 中英双语歌词条，
  中间舞台由 19 个场景按歌词时间轴切换。
- **真·节奏同步**：启动时离线解码整首 MP3，按 60fps 预计算 32 频段能量包络与
  低/中/高三条总线；播放时按播放器位置直接查表——视觉与音频零抖动对齐。
  低频骤增处自动标记 700+ 个重音点供场景做冲击。
- **终端 3D 引擎（fx3d）**：透视投影 + 每格 Z-Buffer + 兰伯特光照 +
  近平面裁剪的线框渲染，全部在字符网格上完成。
  牢笼是迎面合拢的立体铁栏、神明有环绕旋转的陀螺环、
  爱从方程解出旋转的立体心、终局是整座星系被吸回奇点。
  全局辉光（bloom）让高亮处向暗处溢出，像真实的荧光。
- **meta 叙事**：整首歌被处理成一场"系统事件"——
  启动日志、`rustc` 编译报错（expected `Person`, found `Process`）、
  Rust panic backtrace、`$ whoami` → `no such user`、
  `$ ls -la ~/memories` 里被 `rm -rf` 的记忆文件。
- **隐藏线索**：状态栏右侧的 `user@localhost` 在唱到"你走了"的那一刻
  变成 `link unstable`，随后彻底 `NO CARRIER` 掉线，再也不回来。
  结局 `PROCESS EXIT` 后退回终端，只留下 `$ world.execute(me);` 与一行
  "（你还在。它没有了。）"

## 运行

需要真彩终端（Windows Terminal / iTerm2 / 大多数现代终端），建议 ≥ 100×30。

```
cargo run --release
```

音频文件默认从 `assets/audio/world.execute(me); - Mili.mp3` 读取，找不到时以静音模式运行
（时间轴走墙钟，画面完全一致）。

> **版权说明**：仓库内的音频、歌词等素材版权归 **Mili** 及其 respective rights holders
> 所有，仅为个人非商业学习用途随仓库分发；详见 [LICENSE](LICENSE)。
> 如版权方提出要求，将立即移除。

### 选项

```
-f, --file <路径>    指定音频文件
    --no-audio       静音播放
    --no-splash      跳过开始前的标题闪屏
    --demo3d         fx3d 引擎巡演：全屏轮播纽结/地球/心/星系（无需音频）
    --start <秒>     从指定位置开始
    --shots t1,t2,…  离屏渲染若干时间点到 HTML（开发校验；
                     与 --demo3d 同给时校验的是巡演画面）
    --size WxH       离屏渲染尺寸（默认 160x48）
    --out <路径>     离屏渲染输出文件
    --fps <数字>     渲染帧率上限（默认 60）
```

### 按键

| 键 | 功能 |
|---|---|
| `SPACE` | 暂停 / 继续 |
| `←` / `→` | 快退 / 快进 5s |
| `R` | 重播 |
| `H` | 帮助浮层 |
| `Q` / `ESC` | 退出 |

## 场景时间轴（19 幕）

| 时间 | 场景 | 内容 |
|---|---|---|
| 0:00 | BOOT / SELF-TEST | 开机日志 + 3D 内核组装，世界从波纹中成形 |
| 0:16 | world.execute(me); | 大字标题 + 字符雨 + 线框地球浮现 |
| 0:30 | GEOMETRY OF ME | 3D 点云球 + 内核二十面体："如果我是一组点" |
| 0:44 | ELECTRIC STATE | 3D 示波器 / 涡旋隧道 / 年份超时空 / 双球融合 |
| 0:59 | STIMULUS | 3D 反应堆随满足度加速 |
| 1:04 | TRAPPED | 真 3D 铁笼迎面合拢（fx3d） |
| 1:14 | NUTRIENTS / PURR | 3D 旋转体茄子、番茄；猫咪与透视地板 |
| 1:22 | DEITY | 全视之眼 + 3D 陀螺环 |
| 1:29 | MORPH | 3D 立体字变形 F→M / S→M |
| 1:41 | THE TRANCE | 超时空星流 + 3D 陀螺转环 |
| 1:51 | USER DISCONNECTED | `rm -rf` 记忆文件被吸走，链路失稳 |
| 2:00 | ERASING FRAGMENTS | 3D 碎片场随波前卷走 |
| 2:05 | FRAGMENTS | 燃烧的代码 + 余烬，色彩被抽走 |
| 2:13 | VERDICT // PANIC | panic backtrace + 落章冲击波 |
| 2:27 | EXECUTION ×12 | 十二次 SIGKILL 弹幕（3× 砸落 + 辉光） |
| 2:42 | FINAL EXECUTION | 终局处刑 |
| 2:57 | ALGEBRA OF LOVE | 爱的代数式 → 旋转的 3D 立体心 |
| 3:11 | TRAPPED IN LO-O-OVE | 3D 立体心劈成两半：你自由 ↔ 我被困 |
| 3:25 | PROCESS EXIT | 星系坍缩成奇点，光标熄灭 |

## 项目结构

```
world.execute(me)/          ← 仓库根 = cargo 项目根
├── src/                    ← 源码（scenes/ 下按叙事幕分文件）
├── assets/                 ← 参考源素材
│   ├── audio/              ←   歌曲音频
│   ├── lyrics/             ←   内嵌进二进制的歌词（LRC + 中文对照）
│   └── reference/          ←   原始参考资料（原版 LRC / 翻译稿）
├── tools/                  ← 开发工具（离屏帧转 PNG / 截图脚本）
├── gallery/                ← 各版本效果存档（gallery/vX.Y/）
├── Cargo.toml
├── CHANGELOG.md            ← 版本历史与约定
└── README.md
```

## 版本管理

- v1.1.0 起（「DIMENSION」3D 升级）重新进入特性开发；v1.0.x 曾为维护期。
- 每个特性版本递增 minor 号并在 CHANGELOG 记一条，必要时打 tag；
  不保留旧版效果快照，也不为每次修复存 gallery 目录。
- `gallery/v1.0/` 是 v1.0.0 时期的画面留档，仅作历史参考，与后续版本不再同步。
- 开发期临时输出放 `_dev/`、`_check/`，渲染产物放 `_video/`（均已 gitignore），不进版本库。

## 架构

```
src/
├── main.rs     启动流程 / 主循环 / 离屏渲染
├── audio.rs    rodio 播放 + 离线 FFT 频谱包络（自写 radix-2）
├── lyrics.rs   LRC + 中英双语对齐解析（关键词标注）
├── view.rs     全局帧状态（频段能量、link 状态、歌词游标）
├── buf.rs      字符画布：Cell/Canvas/图元，宽字符对位
├── term.rs     差分刷新终端 + 离屏 HTML/二进制导出
├── chrome.rs   常驻界面：状态栏/频谱/进度/歌词条/帮助
├── theme.rs    调色板与关键词高亮
├── fx.rs       特效库：粒子/故障/字符雨/辉光/心形曲线
├── fx3d.rs     终端 3D 引擎：透视投影 + Z-Buffer + 兰伯特着色
├── demo3d.rs   --demo3d 引擎巡演模式
├── bigfont.rs  5x5 像素大字模
└── scenes/     19 个场景 + 时间轴
    ├── open.rs    boot / title
    ├── verse.rs   主歌段
    ├── organic.rs 有机段
    ├── loss.rs    失去段
    ├── exec.rs    处刑段
    └── love.rs    爱与终局
```

渲染流程：每帧 `场景.draw(ctx)` 画舞台 → 舞台后处理（扫描线/辉光/渐晕/故障）→
外壳四件套 → 全局调色（"孤独"段落整体褪色）→ 差分刷新。
任意 panic 都会先还原终端状态再退出。

## 开发工具

离屏渲染一帧 = 与实时完全同一份 `render_frame`，从 t-4s 预热推进，
因此截到的就是"那一刻真实的样子"：

```
cargo run --release -- --no-audio --shots "31,85,150" --out _dev/preview/all.html
python tools/render_frames.py out.png _dev/preview/all_00.bin _dev/preview/all_01.bin   # 二进制帧 → PNG
./tools/shoot.sh 04 17 33                                            # 打开 HTML 截图（→ _dev/shots）
```

## 渲染成视频

把整首歌逐帧离屏渲染（严格 60fps、零丢帧，与实时播放共用同一渲染管线），
再用 ffmpeg 与歌曲音频合成 1080p MP4：

```
# 1) 逐帧导出（≈12,800 帧 .bin，几十秒完成）
cargo run --release -- --render-video _video/frames

# 2) 合成 MP4（需要 Pillow；ffmpeg 放到 tools/ffmpeg/ffmpeg.exe 或加入 PATH）
pip install pillow
python tools/render_video.py _video/frames "assets/audio/world.execute(me); - Mili.mp3" _video/world-execute-me.mp4
```

便携版 ffmpeg 下载解压后把 `bin/ffmpeg.exe` 放进 `tools/ffmpeg/` 即可（已 gitignore）。
