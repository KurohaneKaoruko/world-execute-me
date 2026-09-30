# v1.0.0 效果存档

本目录保存 v1.0.0 发布时的完整视觉快照。后续版本若改动场景效果，
发布时按同样格式存入 `gallery/vX.Y/`，旧版效果即可随时回看。

## frames/ — 全曲离屏渲染（49 帧）

覆盖 0:00 → 3:31 关键时间点，与实时渲染逐像素一致。

重新查看：

```
python tools/render_frames.py out.png gallery/v1.0/frames/all_04.bin gallery/v1.0/frames/all_17.bin
```

或按版本重新离屏生成：

```
cargo run --release -- --no-audio --no-splash --shots "<时间列表>" --out _dev/preview/all.html
```

## stills/ — 精选画面

| 文件 | 场景 |
|---|---|
| h1 | 03 GEOMETRY OF ME（点集 → 圆） |
| h2 | 07 NUTRIENTS（茄子 / 猫变身） |
| h3 | 08 PROOF OF EXISTENCE（神性之眼） / 09 SWITCH MY GENDER（F→M） |
| h4 | 15 EXECUTION ×12（处刑大字）/ 14 VERDICT // PANIC |
| h5 | 15 EXECUTION ×12（迫击炮弹幕） |
| h6 | 18 TRAPPED IN LO-O-OVE（心形牢笼） |
| h7 | 13 ILLEGAL ARGUMENTS（rustc 报错）/ 14 VERDICT // PANIC |
| h8 | 11 USER DISCONNECTED（rm -rf 记忆）/ 12 ERASING FRAGMENTS |
| h9 | 01 BOOT / SELF-TEST、02 标题字卡 |
| k0 | 02 world.execute(me); 标题（字符雨背景） |
| k1 | 08 PROOF OF EXISTENCE |
| k2 | 10 THE TRANCE（催眠隧道） |
| k3 | 17 THE ALGEBRAIC EXPRESSION OF LOVE |
| k6/k8 | 19 PROCESS EXIT（光标熄灭） |
