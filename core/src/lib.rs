//! 矢立の決定的核 — 全 OS の殻（iOS・macOS・Windows・Android）が共有する純関数群。
//!
//! **殻は OS ごとに native、核は一つ**（[`docs/ime/cross-platform.md`]）。
//! IME の殻は OS の入力機構そのものに嵌め込まれる部品なので標準化できないが、
//! 矢立の中身は約 2/3 が核であり、核は完全に共有できる。
//!
//! ここに入れてよいもの:
//!
//! - **決定的**であること（同じ入力 → 同じ出力。時刻・乱数・環境に依らない）
//! - **依存ゼロ**であること（標準ライブラリのみ。テストが $0 で回る）
//! - **UI を知らない**こと（色・描画・ジェスチャは殻の仕事）
//!
//! 入れてはいけないもの: 画面・ネットワーク・ファイル・OS API。
//!
//! ## パリティ
//!
//! 旧字表の SSOT は Python（`ssot/kyuji.py`）で、`src/generated/` は
//! `scripts/gen_rust_tables.py` が写した**機械生成物**である（手で編集しない）。
//! 挙動の一致は `core/vectors/*.json`（Python が生成する黄金ベクトル）を
//! `cargo test` が流して機械的に検査する。

pub mod bunsetsu;
pub mod composer;
pub mod generated;
pub mod genki;
pub mod gojuon;
pub mod henkan;
pub mod jisho;
pub mod kagi;
pub mod kehai;
pub mod kyuji;
pub mod session;

pub use bunsetsu::{coverage_error, segment, Bunsetsu, Cand};
pub use composer::Composer;
pub use genki::{Edit, Genki};
pub use gojuon::{Deflect, Gyo};
pub use henkan::{Act, Henkan, Phase};
pub use jisho::{Pos, Word};
pub use kagi::{genki_of_code, genki_of_hid, genki_of_mac, genki_of_scan, Kagi, KAGI};
pub use kehai::{field, ActionField, KeyId};
pub use kyuji::{kyuji_stream, to_kyuji, to_kyuji_body, KyujiStream, POINT_MARKER};
pub use session::{KeyAction, Session};
