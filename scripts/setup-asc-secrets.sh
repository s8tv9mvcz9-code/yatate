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
#   ・API キーで配布証明書（Apple Distribution）を発行する（秘密鍵はこちらで生成）
#   ・鍵と証明書を .p12 に束ねる
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

# ── 1〜2. 配布証明書を発行し .p12 に束ねる ───────────────
# Xcode が作る証明書はデータ保護キーチェーンに入り `security export` で取り出せない
# （秘密鍵が出ないので .p12 を作れない）。よつて秘密鍵はこちらで生成し、CSR を
# App Store Connect API に投げて証明書を受け取る。鍵と証明書が揃ふので CI へ渡せる。
P12_PASS=$(uuidgen)
python3 scripts/asc_cert.py "$KEY_ID" "$ISSUER_ID" "$KEYDIR/key.p8" "$TMP/dist.p12" "$P12_PASS" \
  || { echo "✗ 配布証明書の発行に失敗しました"; exit 1; }

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
