# App Store 提出チェックリスト（矢立）

> 最終更新: 2026-07-31。**アップロードは自動化済み。残るのは App Store Connect の画面作業だけ。**

---

## 現況（2026-07-31 時点で機械的に確認したこと）

| 事項 | 状態 | 根拠 |
|---|---|---|
| ビルドのアップロード | ✅ **build 3 が送信成功** | `altool` が `UPLOAD SUCCEEDED with no errors` / Delivery UUID `f7c8555e-f5aa-41eb-9002-250850f63e1d`（2026-07-30 22:32、Actions run 30587272553） |
| リリース CI | ✅ 端から端まで通る | `ios-release.yml`。タグ push か `workflow_dispatch` で走る |
| ASC API キー・配布証明書 | ✅ Secrets 登録済み | 2026-07-30 22:22〜22:25（`gh secret list` で名前を確認。値は見えない） |
| プライバシーポリシー URL | ⚠️ **貼り替へ要** | 下記「§1」参照 |
| スクリーンショット | ⚠️ **未提出**（素材は `store/screenshots/` に用意した） | 下記「§2」 |
| 審査提出 | ⬜ 未実施（人の操作） | ASC の画面で行ふ |

> **注意**: 「アップロード成功」は「処理成功」ではない。`altool` を通つた後に
> App Store 側の処理で落ち、`ITMS-*` のメールが来ることがある。
> **まづ ASC で build 3 が選択可能な状態になつてゐるかを見ること。**

---

## §1 プライバシーポリシー URL（要注意）

**貼るべき URL:**

```
https://github.com/s8tv9mvcz9-code/yatate/blob/main/docs/privacy.md
```

**貼つてはいけない URL**（bungo-rag は非公開化したので **404**。実測で確認済み）:

```
https://github.com/s8tv9mvcz9-code/bungo-rag/blob/main/docs/privacy.md   ← 死んでゐる
```

到達できないプライバシーポリシー URL は **審査ガイドライン 5.1.1 で一発却下**になる。
既に ASC へ旧 URL を入れてゐる場合は必ず差し替へること。

---

## §2 スクリーンショット

`store/screenshots/` に 6.9 インチ（**1320 × 2868**＝App Store の必須寸法）で用意してある。
iPhone 17 Pro Max シミュレータから `xcrun simctl io <udid> screenshot` で取得したもの。

撮り直す手順:

```bash
cd ios && xcodegen generate
xcrun simctl boot "iPhone 17 Pro Max"
xcrun simctl install booted <Yatate.app のパス>
xcrun simctl launch booted com.bungo.BungoRag
xcrun simctl io booted screenshot store/screenshots/01-yatate.png
```

**キーボード自体が写つた画面を最低 1 枚入れること。** 審査担当は「キーボードが実際に
動くか」を見る。母艦アプリの説明画面だけだと、拡張の実体が伝はらない。
撮り方: 設定 → 一般 → キーボード → キーボード → 新しいキーボードを追加 → 矢立 を
有効にしてから、文字入力欄で地球儀キーから矢立へ切り替へて撮る。

---

## §3 App Review Notes（審査担当への備考欄にそのまま貼れる原稿）

キーボード拡張は 4.4.1 と「空の器」判定で落ちやすい。**先回りして書いておく。**
以下はいづれもコードで裏取り済み（根拠を併記）。

```
本アプリは歴史的仮名遣ひ・旧字体で日本語を書くためのカスタムキーボードです。

・フルアクセス（Full Access）は要求しません。Info.plist の RequestsOpenAccess は
  false で、許可の有無にかかはらず全機能が動作します。打鍵の記録・外部送信は
  行ひません（拡張のコードに通信 API は含まれません）。

・母艦アプリは空の器ではありません。「矢立」タブは有効化手順と鍵盤の作法の解説、
  「文机」タブは画面全体を五十音の紙面として指の運びで綴る入力面で、いづれも
  アプリ内で完結して動作します。

・変換辞書は端末内に同梱しており（青空文庫の公開作品から作成した統計表）、
  サーバとの通信は一切ありません。

・確認手順: 設定 → 一般 → キーボード → キーボード → 新しいキーボードを追加 → 矢立。
  フルアクセスは許可不要です。任意のテキスト入力欄で地球儀キーから矢立へ切り替へ、
  行キー（縦書き二列）を押しながら下へ滑らせると あいうえお の段が選べます。
  右へ逸らすと濁点、左へ逸らすと半濁点・小書き。確定すると新字体は自動で
  旧字体になります（例: 国 → 國）。
```

**裏取り:**
- `RequestsOpenAccess = false` … `ios/YatateKeyboard/Info.plist`（`PlistBuddy` で確認）
- 通信コード無し … `ios/YatateKeyboard/` と `ios/YatateCore/Sources/` に
  `URLSession` 等の出現なし（`grep` で確認）
- 母艦の実体 … `ios/Sources/View/YatateGuideView.swift` / `FuzukueView.swift`

---

## §4 ASC 画面で確認・入力する項目

上から順に潰す。**1 は「今どうなつてゐるか」を読むだけ**なので最初にやること。

1. **アプリレコードの名前** — Bundle ID `com.bungo.BungoRag` のレコードが
   「矢立」名義になつてゐるか。この ID は元々 `文語作文支援`（bungo-rag の iOS アプリ）
   用に作られ、後から矢立のバンドル ID をこのレコードへ**合はせた**経緯がある
   （commit `f762f2f`）。**App Store に出る名前は ASC のメタデータで決まり、
   `CFBundleDisplayName` では決まらない。** 名前が古いままだと 文語作文支援 といふ
   名前で矢立が出てしまふ。
2. **build 3 の状態** — 一覧に出てゐるか、処理中(Processing)や無効になつてゐないか。
3. **プライバシーポリシー URL** — §1 のとほり差し替へ。
4. **App Privacy 質問票** — 収集データ **なし**（矢立は通信しない）。
   ※ bungo-rag 側の Web/Android アプリはサーバへ送るが、**別アプリなので混同しないこと**。
5. **スクリーンショット** — §2。
6. **年齢制限（Age Rating）** — 該当項目なしで 4+ 相当。
7. **カテゴリ** — ユーティリティ（Utilities）が素直。
8. **価格・配信地域** — 無償。
9. **サポート URL** — `https://github.com/s8tv9mvcz9-code/yatate`
10. **App Review Notes** — §3 を貼る。
11. **審査へ提出**（← ここが後戻りできない操作）

---

## §5 やつてはいけないこと

- **`scripts/setup-asc-secrets.sh` の再実行**。Secrets は登録済みで、再実行すると
  Apple Distribution 証明書をもう一枠消費する（枠は有限）。
- **`~/Downloads/AuthKey_*.p8` の紛失**。App Store Connect の API キーは
  **再ダウンロードできない**。無くすと作り直しになる。Downloads に置きつぱなしにせず、
  パスワード管理ソフトなど永続的な場所へ移すこと。
- **`git tag v* && git push --tags`** は「アップロードまで自動で走る」引金。
  検証だけしたいときは `workflow_dispatch` で **`skip_upload: true`** を選ぶ。

---

## §6 積み残し（別途の判断が要る）

- **バンドル ID の衝突** — `bungo-rag/ios/project.yml` も同じ `com.bungo.BungoRag` を
  宣言してゐる（表示名は `文語作文支援`）。ASC のレコードは一つしか無いので、
  bungo-rag 側から archive/upload すると矢立のリリースと衝突する。
  bungo-rag の iOS アプリを将来出す気があるなら、そちらの ID を変へておくこと。
- **矢立ファイルの二重管理** — `bungo-rag` にも `ios/YatateCore/` `ios/YatateKeyboard/`
  `scripts/gen_yatate_ngram.py` が追跡されたまま残つてゐる（13 ファイル）。
  bungo-rag の iOS プロジェクトが今も YatateKeyboard ターゲットを持つので、
  ファイルだけ消すとビルドが壊れる。しかも bungo-rag の `ios-ci.yml` は
  `workflow_dispatch` 専用に落としてあるため、壊れても CI が気づかない。
  消すなら `project.yml` の編集とセットで行ふこと。
