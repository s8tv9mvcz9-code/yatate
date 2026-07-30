#!/usr/bin/env bash
# setup-asc-secrets.sh — App Store への自動更新を「一度だけ」仕込む。
#
# これを一回流せば、以後は `git tag v1.0.1 && git push --tags` だけで
# App Store Connect まで無人で上がる（.github/workflows/ios-release.yml）。
#
# ── 人間がやること（これだけ）────────────────────────────
#   1. https://appstoreconnect.apple.com/access/integrations/api を開く
#   2. 「チームキー」タブ →「+」→ 名前は任意、アクセス権は **App Manager**
#   3. 発行された .p8 をダウンロード（**再ダウンロード不可**。無くしたら作り直し）
#   4. 画面に出てゐる **Issuer ID** と **Key ID** を控へる
#
# ── 使ひ方 ────────────────────────────────────────────
#   ./scripts/setup-asc-secrets.sh ~/Downloads/AuthKey_XXXXXXXXXX.p8 <KEY_ID> <ISSUER_ID>
#
# このスクリプトは順に:
#   ・API キーで配布証明書（Apple Distribution）を発行させる（無ければ作られる）
#   ・その証明書を .p12 に書き出す
#   ・GitHub Secrets へ 6 つ登録する（値は表示しない）
set -euo pipefail
cd "$(dirname "$0")/.."

P8="${1:-}"; KEY_ID="${2:-}"; ISSUER_ID="${3:-}"
if [ -z "$P8" ] || [ -z "$KEY_ID" ] || [ -z "$ISSUER_ID" ]; then
  sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'
  exit 1
fi
[ -f "$P8" ] || { echo "✗ .p8 が見つかりません: $P8"; exit 1; }

TEAM=$(security find-certificate -c "Apple Development" -p 2>/dev/null \
  | openssl x509 -noout -subject 2>/dev/null \
  | tr ',' '\n' | awk -F= '/OU/ {gsub(/ /, "", $2); print $2; exit}')
[ -n "$TEAM" ] || { echo "✗ Team ID を証明書から取得できません"; exit 1; }
echo "▶ Team ID: $TEAM"

TMP=$(mktemp -d); trap 'rm -rf "$TMP"' EXIT
KEYDIR="$TMP/asc"; mkdir -p "$KEYDIR"; cp "$P8" "$KEYDIR/key.p8"; chmod 600 "$KEYDIR/key.p8"

# ── 1. 配布証明書を用意させる ───────────────────────────
# App Store 向けの書き出しには「Apple Distribution」が要る。ローカルに無ければ
# API キー経由で発行される（CI で毎回作らせると証明書枠を使ひ切るので、ここで一度だけ）。
if security find-identity -v -p codesigning 2>/dev/null | grep -q "Apple Distribution"; then
  echo "▶ 配布証明書は既にあります"
else
  echo "▶ 配布証明書が無いので API キーで発行させます（アーカイブを一回流します）"
  (cd ios && xcodegen generate >/dev/null)
  xcodebuild archive -project ios/Yatate.xcodeproj -scheme Yatate \
    -destination 'generic/platform=iOS' -configuration Release \
    -archivePath "$TMP/a.xcarchive" \
    -allowProvisioningUpdates \
    -authenticationKeyPath "$KEYDIR/key.p8" \
    -authenticationKeyID "$KEY_ID" -authenticationKeyIssuerID "$ISSUER_ID" \
    DEVELOPMENT_TEAM="$TEAM" CODE_SIGN_STYLE=Automatic > "$TMP/archive.log" 2>&1 \
    || { echo "✗ アーカイブ失敗:"; grep -E "error:" "$TMP/archive.log" | sort -u | head -5; exit 1; }

  cat > "$TMP/eo.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>method</key><string>app-store-connect</string>
  <key>teamID</key><string>$TEAM</string>
  <key>destination</key><string>export</string>
</dict></plist>
PLIST
  xcodebuild -exportArchive -archivePath "$TMP/a.xcarchive" \
    -exportOptionsPlist "$TMP/eo.plist" -exportPath "$TMP/export" \
    -allowProvisioningUpdates \
    -authenticationKeyPath "$KEYDIR/key.p8" \
    -authenticationKeyID "$KEY_ID" -authenticationKeyIssuerID "$ISSUER_ID" \
    > "$TMP/export.log" 2>&1 \
    || { echo "✗ 書き出し失敗:"; grep -E "error:" "$TMP/export.log" | sort -u | head -5; exit 1; }

  security find-identity -v -p codesigning | grep -q "Apple Distribution" \
    || { echo "✗ 配布証明書が作られませんでした。API キーの権限が App Manager か確認を。"; exit 1; }
  echo "✓ 配布証明書を発行しました"
fi

# ── 2. .p12 に書き出す ────────────────────────────────
# 注: security export は指定キーチェーン内の identity をまとめて出すため、
# 開発用証明書も同梱され得る。いづれも本人の証明書であり、CI は署名にしか使はない。
P12_PASS=$(uuidgen)
security export -t identities -f pkcs12 -P "$P12_PASS" -o "$TMP/dist.p12" 2>/dev/null \
  || { echo "✗ .p12 の書き出しに失敗（キーチェーンのロック解除が要るかもしれません）"; exit 1; }
echo "✓ 配布証明書を .p12 へ書き出しました"

# ── 3. GitHub Secrets へ登録（値は表示しない）──────────────
echo "▶ GitHub Secrets を登録します"
gh secret set ASC_KEY_ID              --body "$KEY_ID"                        >/dev/null
gh secret set ASC_ISSUER_ID           --body "$ISSUER_ID"                     >/dev/null
gh secret set ASC_KEY_P8_BASE64       --body "$(base64 < "$KEYDIR/key.p8")"   >/dev/null
gh secret set IOS_DIST_CERT_P12_BASE64 --body "$(base64 < "$TMP/dist.p12")"   >/dev/null
gh secret set IOS_DIST_CERT_PASSWORD  --body "$P12_PASS"                      >/dev/null
gh secret set IOS_TEAM_ID             --body "$TEAM"                          >/dev/null

echo
echo "✓ 仕込み完了。以後の App Store 更新は無人で走ります:"
echo
echo "    git tag v1.0.1 && git push origin v1.0.1"
echo
echo "  → ios-release.yml がビルド・署名・検証・アップロードまで行ひます。"
echo "    審査への提出だけは App Store Connect の画面で押してください（TestFlight は使ひません）。"
echo "  ※ .p8 は再ダウンロードできません。手元の $(basename "$P8") は安全な場所へ保管を。"
