# プライバシーポリシー / Privacy Policy

**矢立（Yatate）— 文語 IME**
最終更新: 2026-08-04 ／ Last updated: 2026-08-04

---

## 要約

**このアプリとキーボードは、いかなる情報も収集・送信しません。**
通信する機能そのものを持ちません。

## 1. 収集しない情報

氏名・メールアドレス・電話番号・住所・生年月日・位置情報・連絡先・写真・
端末識別子・広告 ID・利用状況の統計——**いずれも収集しません**。
アカウント登録はなく、利用者を識別する仕組みもありません。

## 2. 送信しない

本アプリおよびキーボード拡張「矢立」には、**ネットワーク通信を行うコードが含まれていません**。
入力された文字、変換の結果、設定、利用の記録——**何一つ、いかなる宛先にも送信されません**。
解析基盤も、広告も、第三者 SDK も使用していません。

変換に必要な辞書と統計（青空文庫の著作権満了作品から作成したもの）は
**アプリに同梱**されており、端末の中だけで処理されます。

## 3. キーボード拡張「矢立」について

iOS のカスタムキーボードには「フルアクセス」という許可の仕組みがありますが、

- **矢立はフルアクセスを要求しません**（`RequestsOpenAccess` は `false`）。
- 許可の有無にかかわらず、**すべての機能が同じように動作します**。
- **入力された文字を記録・保存・送信しません。**
- **他のアプリの文章を読みません。** 入力欄の前後の文脈を取得する API
  （`documentContextBeforeInput` 等）を一切使用していません。

これらはソースコードで確認できます。本アプリは MIT ライセンスの
オープンソースです: https://github.com/s8tv9mvcz9-code/yatate

## 3-2. web 版について（頁として配るもの）

**web 版だけは、頁と核（wasm）を取りに行くところが通信になります。** ここは
アプリ版と性質が違ふので、曖昧にせず書き分けます。

| もの | どこで起きるか |
|---|---|
| 頁と wasm の取得 | **通信**（初回のみ。以後はブラウザのキャッシュで開けます） |
| 打鍵・変換・旧字確定 | ブラウザの中だけ。どこへも送りません |
| 学習した対（読み → 表記） | そのブラウザの `localStorage` のみ。送信も同期もしません |

配るのは**静的な頁だけ**で、こちらへ何かを送り返す仕組みはありません。

そしてこれは約束の文だけで支へてゐるのではありません。組み上がつた wasm には
**`import` が一つも無く**、外の世界の関数を一つも呼べません——通信も保存も、
そもそも出来ない造りです。この性質は `web/check_wasm.py` が CI で毎回検めてゐます。

「そのブラウザだけ」は制約ではなく約束の一部です。機を替へて持ち運びたいときは、
頁の「書き出す／読み込む」（単一の JSON ファイル）を使ひます——これも端末の中で
完結し、どこへも送りません。

## 4. 端末内に保存されるもの

キーボードの設定（表示の切り替え等）が端末内に保存されます。
web 版では、学習した「読み → 表記」の対がそのブラウザの `localStorage` に保存されます。
これらは端末の外へ出ることはなく、アプリを削除する（web 版はブラウザの保存領域を消す）
と消えます。

## 5. 子どもの利用について

本アプリは特定の年齢層を対象としておらず、そもそも情報を収集しないため、
子どもから情報を集めることもありません。

## 6. お問い合わせ・変更

本ポリシーの変更は本ページで告知します。お問い合わせは GitHub の Issue へ:
https://github.com/s8tv9mvcz9-code/yatate/issues

---

## English

**This app and its keyboard collect and transmit no information whatsoever.**
They contain no networking code at all.

We collect no personal data: no name, email, phone, address, location, contacts,
photos, device identifiers, advertising identifiers, or usage analytics. There are
no accounts and no way to identify a user. There is no analytics platform, no
advertising, and no third-party SDK.

The **Yatate keyboard does not request Full Access** (`RequestsOpenAccess` is
`false`), and behaves identically whether or not it is granted. It never records,
stores, or transmits keystrokes, and it never reads the surrounding text of other
apps — it does not use `documentContextBeforeInput` or related APIs at all.

The dictionary and statistics used for conversion are derived from public-domain
works of Aozora Bunko and are **bundled in the app**; all processing happens on
your device.

**The web version** is the one exception worth stating plainly: fetching the page
and the wasm module is network traffic. Everything after that — typing, conversion,
old-glyph finalisation, and learning — happens inside your browser, and learned
readings live only in that browser's `localStorage`. We serve a static page and
have no endpoint to send anything back to. This is not merely a promise: the
compiled wasm module has **no imports at all**, so it cannot call out to anything;
`web/check_wasm.py` verifies this on every CI run.

All of this can be verified in the source code — the project is open source under
the MIT license: https://github.com/s8tv9mvcz9-code/yatate

Questions: https://github.com/s8tv9mvcz9-code/yatate/issues
