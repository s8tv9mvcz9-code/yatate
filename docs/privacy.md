# プライバシーポリシー / Privacy Policy

**矢立（Yatate）— 文語 IME**
最終更新: 2026-07-31 ／ Last updated: 2026-07-31

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

## 4. 端末内に保存されるもの

キーボードの設定（表示の切り替え等）が端末内に保存されます。
これらは端末の外へ出ることはなく、アプリを削除すれば消えます。

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

All of this can be verified in the source code — the project is open source under
the MIT license: https://github.com/s8tv9mvcz9-code/yatate

Questions: https://github.com/s8tv9mvcz9-code/yatate/issues
