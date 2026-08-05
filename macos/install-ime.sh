#!/usr/bin/env bash
# 矢立の入力方式（IMKit）を組んで ~/Library/Input Methods/ へ置く。
#
#   ./macos/install-ime.sh
#
# そのあと**ログアウトして入り直し**、システム設定 → キーボード → 入力ソース →
# 「＋」→ 日本語 →「矢立（文語 IME）」を足す。Ctrl+Space で切り替へる。
#
# ## ログインし直しが要る理由
#
# 入力方式は OS の入力機構に登録される部品で、`~/Library/Input Methods/` の走査は
# ログインセッションの開始時に行はれる。置いただけでは一覧に出ない——
# 「入れたのに出ない」の九割はこれである。
#
# ## 未署名である
#
# 手元の検証はこれで通るが、**配るには Developer ID 署名と公証（notarize）が要る**。
# Mac App Store は Sandbox を要求し IMKit と相性が悪いので、配布経路は
# 直配布の DMG/PKG になる（docs/ime/cross-platform.md §8）。
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$PWD"
DEST="$HOME/Library/Input Methods"
BUILD="$ROOT/ios/build/ime"

command -v xcodegen >/dev/null 2>&1 || { echo "brew install xcodegen が要る" >&2; exit 1; }

echo "▸ 核を Apple の静的ライブラリへ組む"
./scripts/build-apple-ffi.sh --release

echo "▸ プロジェクトを起こす"
( cd ios && xcodegen generate >/dev/null )

echo "▸ YatateIME を組む"
( cd ios && xcodebuild build \
    -project Yatate.xcodeproj -scheme YatateIME \
    -destination 'platform=macOS' \
    -derivedDataPath "$BUILD" \
    -configuration Release \
    CODE_SIGNING_ALLOWED=NO >/dev/null )

APP="$BUILD/Build/Products/Release/YatateIME.app"
[[ -d "$APP" ]] || { echo "組み上がりが見つからない: $APP" >&2; exit 1; }

# 走つてゐる古い版を止めてから置き換へる（掴まれたままだと差し替はらない）
pkill -x YatateIME 2>/dev/null || true

mkdir -p "$DEST"
rm -rf "$DEST/YatateIME.app"
cp -R "$APP" "$DEST/"

echo "▸ 置いた: $DEST/YatateIME.app"
echo
echo "  つぎに:"
echo "    1. ログアウトして入り直す（一覧の走査はログイン時に一度だけ行はれる）"
echo "    2. システム設定 → キーボード → 入力ソース → ＋ → 日本語 → 矢立（文語 IME）"
echo "    3. Ctrl+Space で切り替へる"
