#!/usr/bin/env bash
# 把 _dev/preview 里的逐帧页面截图成 PNG（需 agent-browser），输出到 _dev/shots。
# 用法: ./tools/shoot.sh 05 10 17 …（编号对应 --shots 输出的 all_XX_*.html）
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BASE="$ROOT/_dev/preview"
OUT="$ROOT/_dev/shots"
mkdir -p "$OUT"
cd "$BASE" || exit 1
for i in "$@"; do
  f=$(ls all_${i}_*.html 2>/dev/null | head -1)
  if [ -z "$f" ]; then echo "miss $i"; continue; fi
  agent-browser open "file:///$BASE/$f" >/dev/null 2>&1
  agent-browser screenshot "$OUT/f$i.png" >/dev/null 2>&1
  echo "$i ok $f"
done
