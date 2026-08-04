#!/usr/bin/env bash
# 矢立の web 殻を組む。**道具は cargo と python3 だけ**（wasm-pack も npm も要らない）。
#
#   ./build.sh          組んで dist/ へ揃へる
#   ./build.sh --serve  組んでから手元で配る（python3 の標準機能だけ）
set -euo pipefail
cd "$(dirname "$0")"

TARGET=wasm32-unknown-unknown
WASM="target/$TARGET/release/yatate_web.wasm"

if ! rustup target list --installed 2>/dev/null | grep -qx "$TARGET"; then
  echo "[*] $TARGET を入れる"
  rustup target add "$TARGET"
fi

echo "[*] 核を wasm へ載せる"
cargo build --release --target "$TARGET"

# 組み上がつた物を検める（出口が揃つてゐるか・import が一つも無いか）。
# Windows 殻の CI が実 DLL のエクスポート表を検べてゐるのと同じ関門である。
echo "[*] wasm を検める"
python3 check_wasm.py "$WASM"

echo "[*] dist/ へ揃へる"
rm -rf dist
mkdir -p dist
cp "$WASM" dist/yatate.wasm
cp index.html yatate.js style.css dist/

printf '[OK] dist/ (%s)\n' "$(du -sh dist | cut -f1)"

if [[ "${1:-}" == "--serve" ]]; then
  # wasm は file:// では読めない（fetch が撥ねる）ので、手元でも配る必要がある。
  echo "[*] http://localhost:8000/ で配る（Ctrl-C で止める）"
  cd dist && exec python3 -m http.server 8000
fi
