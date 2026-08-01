# `windows/` — Windows の殻（TSF テキストサービス）

**骨組みの段階である。** 何が出来てゐて何が未了かを、この文書の下の方に正直に書いてある。
設計は [`docs/ime/cross-platform.md`](../docs/ime/cross-platform.md) §4・M7。

## これは殻である

配列（原器）・墨の氣配・旧字確定は**一つも持たない**。すべて核（[`core/`](../core/README.md)）から来る。
ここが受け持つのは Windows の作法だけ —— COM、TSF の契約、候補窓の描画。

| module | OS 依存 | 中身 |
|---|---|---|
| `Session`（核から再エクスポート） | **なし** | 打鍵 → 未確定文字列の状態機械 |
| `registration` | なし（値のみ） | CLSID・プロファイル GUID・カテゴリ |
| `tip` | Windows | `ITfTextInputProcessor`（COM の入口） |

## Windows 機が無くても守れるところまで守る

```bash
cargo test                                   # 頭脳（OS 非依存）の試験
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo check --target x86_64-pc-windows-gnu   # COM の入口が型として成立してゐるか
```

最後の一行が肝で、**Linux 上で Windows ターゲットの型検査が通る**（CI もこれを ubuntu で回す）。
`rustup target add x86_64-pc-windows-gnu` が要る。

## TSF といふ嵌まり方

TIP（Text Input Processor）は **文字を受け取る全アプリのプロセスへ読み込まれる in-proc COM DLL** である。
Word にも Chrome にもメモ帳にも入る。ここから二つの制約が出る。

1. **重い実行時を持ち込めない** — .NET も Electron も不適。C++ か Rust しか現実的でない
2. **候補窓は OS が出してくれない** — 自前で描く（DPI・マルチモニタ・UI-less mode を自分で見る）

## まだ出来てゐないこと

| 項目 | なぜ未了か |
|---|---|
| クラスファクトリ・`DllGetClassObject` 等のエクスポート | 実機でしか正しさを確かめられない |
| レジストリ登録（`ITfInputProcessorProfiles::Register`）とインストーラ | 同上 |
| `ITfKeyEventSink` の配線（仮想キーコード → 文字） | 原器は **JIS 前提**。`;` `:` の位置は配列で動くので実機で詰める |
| `ITfComposition` による未確定文字列の描画 | 同上 |
| 候補窓の自前描画 | 同上 |
| x64 / **ARM64** の両方のビルドと署名 | 署名の道（OSS 枠）の確認が入口 |

**書けば「動くやうに見えて動かないコード」が残るだけ**なので、書かずに印だけ付けてある
（`src/tip.rs` の TODO）。

## 署名について

Microsoft はテキストサービスのバイナリに**デジタル署名**を求めてゐる。
OSS には SignPath Foundation の無償証明書といふ道があり、先行例（`windows-chewing-tsf`）が
実際にそれで配布してゐる。**ARM64 機で x64 の DLL は読み込まれない**ので、両方を出すこと。

## 依存の置き方（注意）

COM 側の依存は **`[target.'cfg(windows)'.dependencies]`** に置いてある。
ここへ入れておかないと `windows` crate の連れてくるものが Linux で壊れ、
`Session` の試験まで道連れになる——それでは層を切り分けた意味が無い。
