//! 矢立の Windows 殻 — TSF（Text Services Framework）のテキストサービス。
//!
//! **これは殻である。**配列（原器）・墨の氣配・旧字確定は一つも持たず、
//! すべて核（`yatate-core`）から来る（`docs/ime/cross-platform.md` §3）。
//! ここが受け持つのは Windows の作法だけ——COM、TSF の契約、鍵盤の翻訳。
//!
//! ## 構成
//!
//! | module | OS 依存 | 中身 |
//! |---|---|---|
//! | [`Session`]（核から再エクスポート） | **なし** | 打鍵 → 未確定文字列の状態機械 |
//! | [`keymap`] | **なし**（値のみ） | **JIS 物理鍵盤 → 原器**。配列で動く 3 鍵を物理位置で縛る |
//! | [`registration`] | 値は無依存／書込は Windows | CLSID・プロファイル・カテゴリ |
//! | [`dll`] | Windows | COM の四つの出口（`DllGetClassObject` 等） |
//! | [`factory`] | Windows | `IClassFactory` |
//! | [`tip`] | Windows | `ITfTextInputProcessor` ＋ `ITfKeyEventSink` |
//! | [`edit`] | Windows | 未確定文字列を文書へ置く編集セッション |
//!
//! この切り方が肝である。IME の頭脳（`Session`）は**核に居る**——macOS も Android も
//! 同じ物を要るからで、殻ごとに書けば三度書くことになる。
//! さらに**鍵盤の翻訳（`keymap`）も OS API を呼ばない**ので、
//! 原器の心臓部は Windows 機なしで丸ごと試験できる。
//!
//! ## Windows 機が無くても守れるところ
//!
//! ```bash
//! cargo test                                   # 頭脳と鍵盤の翻訳（OS 非依存）
//! cargo clippy --all-targets -- -D warnings
//! cargo fmt --check
//! cargo build --target x86_64-pc-windows-gnu   # 実 DLL としてリンクが通るか
//! # さらに: エクスポート表に DllGetClassObject 等が実在するかを objdump で検査
//! ```
//!
//! **実機でしか確かめられないのは「TSF に読まれて実際に打てるか」だけ**である。
//! 手順は `windows/README.md` の「実機での確認」にある。

pub mod keymap;
pub mod registration;

#[cfg(windows)]
pub mod dll;
#[cfg(windows)]
pub mod edit;
#[cfg(windows)]
pub mod factory;
#[cfg(windows)]
pub mod tip;

/// 入力の状態機械は**核**にある（殻ごとに書かない）。使ひ勝手のため再エクスポートする。
pub use yatate_core::session::{KeyAction, Session};
