# `apple/` — 核を Apple へ出す束縛（素の C ABI）

**iOS・macOS の殻が核（[`core/`](../core/README.md)・Rust）を呼ぶための一枚。**
`web/`（wasm）が JS へ出してゐるのと同じ形で、Swift へ出す。設計は
[`docs/ime/cross-platform.md`](../docs/ime/cross-platform.md) §5・§10（M5-b2）。

## uniffi を使はない

設計書は当初 uniffi（Mozilla の束縛生成器）を想定してゐたが、**実装の段でやめた**。

核の入出力は「文字を入れる／文字列を貰ふ」だけで、それ以上の型を運んでゐない。
それなら `extern "C"` で足りる。uniffi を採ると

- 核（`yatate-core`）の**依存ゼロ方針**に手を入れるか、さもなくば束縛用の
  中間 crate を結局書くことになる
- 生成器と実行時ライブラリの版が、道具立てとして増える

ので、得るものが無い。いま**四つの殻（web・Windows・macOS・iOS）が同じ形で核に触る**
——Windows は Rust なので直に、他は C ABI で。**道具立ては cargo だけ**である。

## ここに知識を置かない

原器の配列も、旧字の 248 字も、氣配の重みも、辞書も、この束縛は**一つも知らない**。
`yatate_kagi_table()` などが返すのは核の表をその場で舐めた結果で、Swift 側も
それを読むだけである。地図は最後まで一枚のままになる。

| 出す物 | 何を | 誰が使ふ |
|---|---|---|
| `yatate_henkan_*` | 変換まで含む状態機械（段・候補・区切り修正・確定） | macOS の IMKit 殻・iOS の稽古場・鍵盤拡張 |
| `yatate_session_*` | 打鍵 → 仮名だけの細い道 | 変換を持たない場面 |
| `yatate_genki_*` | 原器の状態機械（前置シフトの逐次性） | 試験・図 |
| `yatate_*_table` / `yatate_kehai_field` | 表と氣配（TSV で渡す） | Swift の `Gojuon` `Kagi` `Kyuji` `Kehai` |

## 約束

| 事 | 決め |
|---|---|
| 文字列 | 返り値は UTF-8 の NUL 終端。**呼んだ側が `yatate_string_free` で返す** |
| 文字 | `u32` のスカラ値。`0` は「無い」 |
| 手綱 | `NULL` を渡しても落ちない（殻の取り回しの誤りでアプリごと死なせない） |
| 墨の値 | **丸めずに**往復可能な表現で渡す（3 桁で切ると黄金ベクトルの 1e-6 と食ひ違ふ） |

## 頭書は手で書いてゐるが、機械が縛つてゐる

[`include/yatate_ffi.h`](include/yatate_ffi.h) は手書きである。だから
`cargo test` の `頭書` が、この頭書と `#[no_mangle]` の集合・符号の値を突き合はせる。
片方だけ直せば落ちるので、静かにずれることはない
——Windows 殻が DLL のエクスポート表を検めてゐるのと同じ関門である。

## 組む

```bash
cargo test                       # 21 件。ubuntu で回る（課金 1 倍）
../scripts/build-apple-ffi.sh    # 三つの枝を xcframework へ束ねる
```

`scripts/build-apple-ffi.sh` が作るのは

```
ios/YatateCore/YatateFFI.xcframework/
  macos-arm64_x86_64/          文机（YatateMac）・入力方式（YatateIME）・swift test
  ios-arm64/                   実機
  ios-arm64_x86_64-simulator/  シミュレータ
```

三つに分けるのは xcframework の要求である。`ios` と `ios-sim` は**どちらも arm64** なので、
一つの `.a` には束ねられない（`lipo` は同じ arch を二つ入れられない）。

生成物なので git には入れてゐない。**先に組まないと `swift test` も `xcodebuild` も
「artifact が無い」で止まる**——壊れてゐるのではなく、先に一行走らせよといふ意味である。
