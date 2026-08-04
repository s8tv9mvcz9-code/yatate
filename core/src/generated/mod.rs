//! 機械生成物 — **手で編集しないこと。**
//!
//! Python が SSOT、ここはその写しである（`docs/ime/protocol.md` §5 と同じ規律）。
//! 再生成:
//!
//! ```text
//! python3 scripts/gen_rust_tables.py      # ssot/kyuji.py          → kyuji_table.rs
//! python3 scripts/gen_bigram_tables.py    # core/data/kana_bigram.txt → kana_bigram.rs
//! python3 scripts/gen_jisho_tables.py     # core/data/jisho.tsv       → jisho_table.rs
//! ```
//!
//! いづれも**通信しない**。通信する側（青空文庫からの収穫）は
//! `gen_yatate_ngram.py` と `harvest_jisho.py` に分けてあり、CI では回さない
//! （`docs/ime/artifacts.md`）。
//!
//! 仮名 bigram は **Swift 版と同じ一つのデータ**（`core/data/kana_bigram.txt`）から
//! 起こす。別々に収穫すると青空文庫のカタログの変化で二つの表が静かにずれ、
//! iOS と Windows で違ふ絵を描くことになる。
//!
//! CI（`core-ci.yml`）は再生成して `git diff --exit-code` することで、
//! 「SSOT を変へたのに写しが古い」を機械的に落とす。

pub mod jisho_table;
pub mod kana_bigram;
pub mod kyuji_table;
