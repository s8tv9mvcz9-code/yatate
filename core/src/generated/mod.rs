//! 機械生成物 — **手で編集しないこと。**
//!
//! Python が SSOT、ここはその写しである（`docs/ime/protocol.md` §5 と同じ規律）。
//! 再生成:
//!
//! ```text
//! python3 scripts/gen_rust_tables.py     # ssot/kyuji.py → kyuji_table.rs
//! ```
//!
//! CI（`core-ci.yml`）は再生成して `git diff --exit-code` することで、
//! 「SSOT を変へたのに写しが古い」を機械的に落とす。

pub mod kyuji_table;
