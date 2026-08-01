# `core/` — 矢立の決定的核（Rust・依存ゼロ）

**全 OS の殻が共有する、機械で決まる部分だけ。**
配列・旧字確定・氣配・入力の状態機械がここに住み、iOS も macOS も Windows も Android も
「これを呼んで、返つてきたものを描く」だけになる（設計は [`docs/ime/cross-platform.md`](../docs/ime/cross-platform.md)）。

## 何が入つてゐるか

| module | 中身 | SSOT |
|---|---|---|
| `kyuji` | 新字体→旧字体（1:1・ストリーム安全・【ポイント】素通し） | **Python**（`ssot/kyuji.py`） |
| `gojuon` | 五十音の幾何 — 行×段×逸らし、逆引き | **核** |
| `genki` | 原器（縦組五十音配列）— 前置シフト・後置濁点・`^^`＝ん | **核**（仕様は `docs/ime/layout.md` §1） |
| `kehai` | 墨の氣配 — bigram を鍵の墨・段の墨・筆脈の峰へ | **核**（データは `core/data/`） |
| `composer` | 作業帯（確定前の假名バッファ） | **核** |
| `session` | 打鍵 → 未確定文字列の状態機械。**殻はこれを呼ぶ** | **核** |
| `generated/` | 機械生成の表。**手で編集しない** | — |

## ここに入れてよいもの・いけないもの

- **入れてよい**: 決定的（同じ入力 → 同じ出力）・依存ゼロ・UI を知らないもの
- **入れてはいけない**: 画面・ネットワーク・ファイル・OS API

依存ゼロを守ると、テストが **$0・LLM 不要・資格情報不要・OS 不問**で回る。
これは趣味ではなく、CI を ubuntu（課金 1 倍）だけで済ませるための条件である。

## 動かす

```bash
cargo test                 # 単体 ＋ 黄金ベクトル
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo run --bin gen-vectors   # 核 → vectors/{gojuon,kehai,genki}.json
```

## 黄金ベクトル（`vectors/`）

**言語をまたいだ同期を、機械で守るための表。** 入力と期待出力の対が入つてゐる。

| ファイル | 何を縛るか | 書くのは |
|---|---|---|
| `kyuji.json` | 旧字 248 字・境界例・ストリームの分割不変 | `scripts/gen_parity_vectors.py`（Python を実行して書く） |
| `gojuon.json` | 全格子 150 通り・逆引き 85 字・読み順 | `cargo run --bin gen-vectors` |
| `kehai.json` | 代表 12 例の墨・段・峰 | 同上 |
| `genki.json` | 両面 50 鍵・打鍵列 14 例 | 同上 |

**期待値を手で書かない**のが規律である。書いた瞬間に古くなり、ゲートが嘘をつき始める。

いま従つてゐるのは Rust（`tests/parity.rs`）と Swift（`ios/YatateCore/Tests/.../ParityTests.swift`）。
Kotlin・C++ が増えても、**同じファイルを同じやうに流すだけ**でよい。

## 生成物の鮮度

```bash
python3 scripts/gen_rust_tables.py      # ssot/kyuji.py            → src/generated/kyuji_table.rs
python3 scripts/gen_bigram_tables.py    # core/data/kana_bigram.txt → Swift と Rust の bigram 表
```

CI（`core-ci.yml`）は再生成して `git diff --exit-code` する。
「SSOT を変へたのに写しが古い」は人が気をつけて防ぐものではない。

## 殻から使ふ

| OS | 殻 | 核への繋ぎ方 |
|---|---|---|
| Windows | `windows/`（TSF） | 同じ Rust なのでそのまま呼ぶ |
| iOS / macOS | `ios/`（Swift） | いまは Swift の手書き実装がベクトルで縛られてゐる。uniffi 束縛への載せ替へは M5-b2 |
| Android | 未着手 | uniffi（Kotlin） |

`crate-type = ["lib", "staticlib", "cdylib"]` — 静的リンク（iOS）にも動的読み込み（Android・Linux）にも応じられる。
`opt-level = "z"` / `lto` / `strip` はキーボード拡張と in-proc DLL のメモリ予算のため。
