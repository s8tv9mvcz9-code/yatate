#!/usr/bin/env bash
# build-device.sh — 実機（接続済み iPhone）へ署名ビルドして導入する。
#
# 前提（人間側の一度きりの作業）:
#   Xcode → Settings… → Accounts → 「+」→ Apple ID でサインイン
#   （Apple Developer Program 加入済みの Apple ID。これが無いと
#    「No Account for Team …」で失敗する。CLI からは登録できない）
#
# 使ひ方:
#   cd ios && ./build-device.sh              # ビルド＋導入
#   TEAM=XXXXXXXXXX ./build-device.sh        # 別チームで署名
#   ./build-device.sh --build-only           # 導入せずビルドのみ
set -euo pipefail
cd "$(dirname "$0")"

# Team ID は証明書の **OU** から取る。
# ⚠ CN の括弧内（"Apple Development: foo@bar (JG3PUF6X49)"）は Team ID ではなく
#   証明書側の識別子。これを DEVELOPMENT_TEAM に渡すと、アカウントが正しく
#   登録されてゐても "No Account for Team …" で落ちる（実際に嵌つた）。
detect_team() {
  security find-certificate -c "Apple Development" -p 2>/dev/null \
    | openssl x509 -noout -subject 2>/dev/null \
    | tr ',' '\n' | awk -F= '/OU/ {gsub(/ /, "", $2); print $2; exit}'
}
TEAM="${TEAM:-$(detect_team)}"
if [ -z "$TEAM" ]; then
  echo "✗ 開発証明書が見つかりません。Xcode → Settings… → Accounts で Apple ID を追加し、"
  echo "  Manage Certificates… から Apple Development 証明書を作成してください。"
  exit 1
fi
CONFIG="${CONFIG:-Debug}"
DERIVED="build-device"

echo "▶ xcodegen generate"
xcodegen generate >/dev/null

echo "▶ 署名ビルド（team=$TEAM, config=$CONFIG）"
set +e
xcodebuild build \
  -project Yatate.xcodeproj -scheme Yatate \
  -destination 'generic/platform=iOS' -configuration "$CONFIG" \
  -derivedDataPath "$DERIVED" \
  -allowProvisioningUpdates \
  DEVELOPMENT_TEAM="$TEAM" CODE_SIGN_STYLE=Automatic \
  > "$DERIVED.log" 2>&1
rc=$?
set -e
if [ $rc -ne 0 ]; then
  echo "✗ ビルド失敗。主なエラー:"
  grep -E "error:" "$DERIVED.log" | sort -u | head -5
  if grep -q "No Account for Team" "$DERIVED.log"; then
    echo
    echo "→ Xcode に Apple ID が未登録です。"
    echo "   Xcode → Settings… → Accounts → 「+」→ Apple ID を追加してから再実行してください。"
  fi
  exit $rc
fi

APP=$(find "$DERIVED/Build/Products/$CONFIG-iphoneos" -maxdepth 1 -name "*.app" | head -1)
echo "✓ ビルド成功: $APP"
codesign -dv "$APP" 2>&1 | grep -E "Authority|TeamIdentifier" | head -2 || true

[ "${1:-}" = "--build-only" ] && exit 0

# UDID は列位置でなく形（UUID）で拾ふ（機種名の語数で列がずれるため）
DEVICE=$(xcrun devicectl list devices 2>/dev/null \
  | awk '/connected/ {for (i = 1; i <= NF; i++) if ($i ~ /^[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-/) print $i}' \
  | head -1)
if [ -z "$DEVICE" ]; then
  echo "! 接続済みの実機が見つかりません（USB 接続とロック解除・信頼を確認）。"
  echo "  手動導入: xcrun devicectl device install app --device <UDID> \"$APP\""
  exit 0
fi

echo "▶ 導入先デバイス: $DEVICE"
xcrun devicectl device install app --device "$DEVICE" "$APP"
echo
echo "✓ 導入完了。初回は端末側で信頼設定が要ります:"
echo "   設定 → 一般 → VPN とデバイス管理 → デベロッパApp → 信頼"
echo "  矢立（キーボード）を使ふには:"
echo "   設定 → 一般 → キーボード → キーボード → 新しいキーボードを追加 → 文語作文支援"
