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

# ## sudo で走らせてはいけない
#
# 入力方式は**ログインしてゐる本人の権限**で読まれる部品である。sudo で置くと
# ~/Library/Input Methods/YatateIME.app が root 所有になり、セッションが登録できず
# 一覧に出ない。しかも組み上げも設置も成功するので、**成功したやうに見えて出ない**。
# 「入れたのに出ない」の残り一割はこれ（九割はログインし直し忘れ）。実際に踏んだ。
if [[ "${EUID:-$(id -u)}" -eq 0 ]]; then
  cat >&2 <<'MSG'
✗ sudo で走らせないこと。

  入力方式は本人の権限で読まれるため、root で置くと登録されない
  （組み上げも設置も成功するのに一覧に出ない、といふ形で失敗する）。

  既に sudo で入れてしまつた場合は、先に消してから入れ直す:

      sudo rm -rf ~/Library/Input\ Methods/YatateIME.app
      ./macos/install-ime.sh          # ← sudo を付けない
MSG
  exit 1
fi

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
# 以前 sudo で入れてゐると root 所有で消せない。黙つて古い版が residue として
# 残ると、また「入れたのに出ない」に戻るので、ここで手を止めて理由を告げる。
if [[ -e "$DEST/YatateIME.app" ]] && ! rm -rf "$DEST/YatateIME.app" 2>/dev/null; then
  cat >&2 <<MSG
✗ 既にある $DEST/YatateIME.app を消せない（恐らく sudo で入れた root 所有の版）。

  一度だけ消してから、sudo を付けずに入れ直すこと:

      sudo rm -rf ~/Library/Input\\ Methods/YatateIME.app
      ./macos/install-ime.sh
MSG
  exit 1
fi
cp -R "$APP" "$DEST/"
IMEAPP="$DEST/YatateIME.app"

# ── 表示名の入れ物 ──────────────────────────────────
# 入力ソースの名前は Resources/<lang>.lproj/InfoPlist.strings から引かれる。
# CODE_SIGNING_ALLOWED=NO の素の組み上げには Resources が一つも入らないため、
# ここで補ふ。鍵は入力モードの ID（ComponentInputModeDict の tsInputModeListKey）。
for L in en ja; do
  mkdir -p "$IMEAPP/Contents/Resources/$L.lproj"
  cat > "$IMEAPP/Contents/Resources/$L.lproj/InfoPlist.strings" <<'STRINGS'
"CFBundleName" = "矢立";
"CFBundleDisplayName" = "矢立（文語 IME）";
"jp.yatate.inputmethod.Japanese" = "矢立（文語 IME）";
STRINGS
done

# ── 束に封をする ────────────────────────────────────
# xcodebuild の CODE_SIGNING_ALLOWED=NO では、実行ファイルに linker が付ける
# ad-hoc 署名しか無く `_CodeSignature` が作られない。この状態の束は
#   codesign --verify → "code has no resources but signature indicates they must be present"
# となり、**署名の妥当でない入力方式は OS が読み込まない**。束ごと ad-hoc で
# 署名し直して封をする（配布には Developer ID 署名と公証が別途要る）。
codesign --force --deep --sign - "$IMEAPP" 2>/dev/null
codesign --verify --deep --strict "$IMEAPP" 2>/dev/null \
  || { echo "✗ 署名の検証に通らない。この束は OS に読み込まれない" >&2; exit 1; }

# IMKit の殻は初回起動時に自分を入力方式として名乗る。置いただけでも
# ログイン時の走査で拾はれるが、ここで一度立ち上げておくと登録が確実になる
# （LSBackgroundOnly なので画面には何も出ない）。
open "$IMEAPP" 2>/dev/null || true

echo "▸ 置いた: $DEST/YatateIME.app"
echo
echo "  つぎに:"
echo "    1. ログアウトして入り直す（一覧の走査はログイン時に一度だけ行はれる）"
echo "    2. システム設定 → キーボード → 入力ソース → ＋ → 日本語 → 矢立（文語 IME）"
echo "    3. Ctrl+Space で切り替へる"
