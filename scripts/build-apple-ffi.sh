#!/usr/bin/env bash
# 核（Rust）を Apple の静的ライブラリへ組み、xcframework に束ねる（M5-b2）。
#
# **これを走らせないと `swift test` も `xcodebuild` も動かない。** Package.swift が
# binaryTarget として YatateFFI.xcframework を指してゐるからである。
# 生成物は git に入れない（.gitignore）——このリポジトリは「表もコードも再生成できる」
# ことを守つてきたので、バイナリだけ例外にしない。
#
#   ./scripts/build-apple-ffi.sh            debug（速い。手元の試し打ち向き）
#   ./scripts/build-apple-ffi.sh --release  release（opt-level=z・LTO・strip）
#
# 出来るもの:
#
#   ios/YatateCore/YatateFFI.xcframework/
#     macos-arm64_x86_64/          … 文机（YatateMac）と IMKit 殻（YatateIME）と swift test
#     ios-arm64/                   … 実機
#     ios-arm64_x86_64-simulator/  … シミュレータ（Apple Silicon と Intel の両方）
#
# 三つに分けるのは xcframework の要求である。ios と ios-sim は**どちらも arm64** なので
# 一つの .a には束ねられない（lipo は同じ arch を二つ入れられない）。
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$PWD"
CRATE="$ROOT/apple"
OUT="$ROOT/ios/YatateCore/YatateFFI.xcframework"

PROFILE_DIR="debug"
CARGO_FLAGS=()
if [[ "${1:-}" == "--release" ]]; then
  PROFILE_DIR="release"
  CARGO_FLAGS=(--release)
fi

if ! command -v cargo >/dev/null 2>&1; then
  # rustup の既定の置き場は PATH に入つてゐないことがある（CI・新しい機）
  if [[ -x "$HOME/.cargo/bin/cargo" ]]; then
    export PATH="$HOME/.cargo/bin:$PATH"
  else
    echo "cargo が無い。https://rustup.rs で入れること" >&2
    exit 1
  fi
fi

# Package.swift の platforms と揃へる（食ひ違ふとリンカが警告を出す）
export MACOSX_DEPLOYMENT_TARGET=13.0
export IPHONEOS_DEPLOYMENT_TARGET=16.0

LIB=libyatate_apple.a

MAC_TARGETS=(aarch64-apple-darwin x86_64-apple-darwin)
IOS_TARGETS=(aarch64-apple-ios)
SIM_TARGETS=(aarch64-apple-ios-sim x86_64-apple-ios)
ALL_TARGETS=("${MAC_TARGETS[@]}" "${IOS_TARGETS[@]}" "${SIM_TARGETS[@]}")

installed=$(rustup target list --installed)
missing=()
for t in "${ALL_TARGETS[@]}"; do
  grep -qx "$t" <<<"$installed" || missing+=("$t")
done
if ((${#missing[@]})); then
  echo "▸ 足りない target を入れる: ${missing[*]}"
  rustup target add "${missing[@]}"
fi

# ⚠ 変数の直後に全角の括弧が来るときは **必ず波括弧で囲む**。
# `$PROFILE_DIR）` と書くと、ロケール次第で bash が全角の閉じ括弧まで
# 変数名の一部として読み、`unbound variable` で落ちる（手元は通り、CI で落ちた）。
echo "▸ 組む（${PROFILE_DIR}）"
for t in "${ALL_TARGETS[@]}"; do
  # macOS 既定の bash は 3.2 で、空配列の展開が `set -u` に触れる
  ( cd "$CRATE" && cargo build ${CARGO_FLAGS[@]+"${CARGO_FLAGS[@]}"} --target "$t" --lib )
done

STAGE="$CRATE/target/xcframework-stage"
rm -rf "$STAGE" "$OUT"
mkdir -p "$STAGE"

# 同じ platform の複数 arch は lipo で一枚に束ねる。
# 名は三枝とも libyatate_apple.a のまま（枝ごとに別の階へ置いて衝突を避ける）。
fatten() {
  local name="$1"; shift
  local dir="$STAGE/$name"
  local out="$dir/$LIB"
  mkdir -p "$dir"
  local libs=()
  for t in "$@"; do
    libs+=("$CRATE/target/$t/$PROFILE_DIR/$LIB")
  done
  if ((${#libs[@]} == 1)); then
    cp "${libs[0]}" "$out"
  else
    lipo -create "${libs[@]}" -output "$out"
  fi
  echo "$out"
}

MAC_LIB=$(fatten macos "${MAC_TARGETS[@]}")
IOS_LIB=$(fatten ios "${IOS_TARGETS[@]}")
SIM_LIB=$(fatten ios-sim "${SIM_TARGETS[@]}")

# 頭書は三つの枝で同じものを使ふ（写しを三枚持たない）。
HEADERS="$STAGE/Headers"
mkdir -p "$HEADERS"
cp "$CRATE/include/yatate_ffi.h" "$CRATE/include/module.modulemap" "$HEADERS/"

echo "▸ 束ねる → ${OUT#"$ROOT"/}"
xcodebuild -create-xcframework \
  -library "$MAC_LIB" -headers "$HEADERS" \
  -library "$IOS_LIB" -headers "$HEADERS" \
  -library "$SIM_LIB" -headers "$HEADERS" \
  -output "$OUT" >/dev/null

# 枝が三つ揃つてゐるか（欠けると「そのプラットフォームだけ黙つてリンクできない」）
for slice in macos-arm64_x86_64 ios-arm64 ios-arm64_x86_64-simulator; do
  [[ -d "$OUT/$slice" ]] || { echo "枝が無い: $slice" >&2; exit 1; }
done

echo "▸ 出来た"
lipo -info "$OUT"/*/"$LIB" | sed 's/^/   /'
