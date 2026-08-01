//! 矢立の Windows 殻 — TSF（Text Services Framework）のテキストサービス。
//!
//! **これは殻である。**配列（原器）・墨の氣配・旧字確定は一つも持たず、
//! すべて核（`yatate-core`）から来る（`docs/ime/cross-platform.md` §3）。
//! ここが受け持つのは Windows の作法だけ——COM、TSF の契約、候補窓の描画。
//!
//! ## 構成
//!
//! | module | OS 依存 | 中身 |
//! |---|---|---|
//! | [`Session`]（核から再エクスポート） | **なし** | 打鍵 → 未確定文字列の状態機械 |
//! | [`registration`] | なし（値のみ） | CLSID・プロファイル GUID・登録に要る定数 |
//! | [`tip`] | Windows | `ITfTextInputProcessor` 実装（COM の入口） |
//!
//! この切り方が肝である。IME の頭脳（`Session`）は**核に居る**——macOS も Android も
//! 同じ物を要るからで、殻ごとに書けば三度書くことになる。
//! COM を知らない層なので **Windows 機が無くても試験できる**。CI は ubuntu（課金 1 倍）で
//! `cargo test` と `cargo check --target x86_64-pc-windows-gnu` を回す。
//!
//! ## まだ出来てゐないこと（正直に）
//!
//! M7 の骨組みである。COM の入口と登録の値は在るが、
//! **クラスファクトリ・レジストリ書き込み・候補窓の描画・実機での動作確認は未了**。
//! それらは Windows 機の上でしか詰められない（[`tip`] の各所に印を付けてある）。

pub mod registration;

#[cfg(windows)]
pub mod tip;

/// 入力の状態機械は**核**にある（殻ごとに書かない）。使ひ勝手のため再エクスポートする。
pub use yatate_core::session::{KeyAction, Session};
