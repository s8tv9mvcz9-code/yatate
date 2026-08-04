//! 辞書引き — 読み（歴史的仮名遣ひ）から表記を引く決定的な層。
//!
//! 表は `core/data/jisho.tsv`（焼き付け成果物）から
//! `scripts/gen_jisho_tables.py` が起こす**機械生成物**である（手で編集しない）。
//! 収穫（青空文庫のルビ・通信あり）と焼き付け（TSV → Rust の表・通信なし）を
//! 分けてあるので、CI は焼き付けだけを鮮度ゲートに載せられる
//! （`docs/ime/artifacts.md`）。
//!
//! ## なぜ核に置くのか
//!
//! 変換のうち「辞書を引く・格子を張る・費用を数へる」は
//! **決定的・依存ゼロ・UI を知らない**——`lib.rs` が核に許した三条件をそのまま満たす。
//! ゆゑに全 OS の殻が同じ答へを得られ、検証は ubuntu の `cargo test` で終はる
//! （`docs/ime/cross-platform.md` §7「核の検証は全部 Linux で終はらせる」）。
//!
//! 逆に**核へ入れないもの**は、埋め込みの計算・近傍検索・情調の推定・文の生成である。
//! それらはモデルと資格情報と通信を要するので、別リポジトリの契約テストに残す
//! （境界の理由は `docs/ime/artifacts.md` §2）。
//!
//! ## 費用は整数で数へる
//!
//! 対数を浮動小数で取ると、言語と環境で丸めが揺れて黄金ベクトルが揺れる。
//! ここでは [`log2_fp`] で **1/256 単位の整数**として持つ。
//! Swift も Kotlin も同じ整数演算を写せば、同じ候補順が出る。

use crate::generated::jisho_table::JISHO;

pub use crate::generated::jisho_table::{ENTRIES, MAX_YOMI_CHARS};

/// 空の辞書は「何を引いても素通り」といふ**沈黙して死ぬゲート**になる。
/// 焼き付けが空を吐いたら、試験を待たず組み立ての時点で落とす。
const _: () = {
    assert!(ENTRIES > 0);
    assert!(MAX_YOMI_CHARS > 0);
};

/// 品詞 — **自立語か付属語か**の二種だけ。
///
/// 文節は「自立語 ひとつ ＋ 付属語 いくつか」なので、分割に効くのはこの二分である。
/// 細かい品詞体系は分割にも候補順にも効かないので持たない（発明しない）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pos {
    /// 自立語 — 文節の頭に立つ。
    Jiritsu,
    /// 付属語 — 直前の自立語にぶら下がる（助詞・助動詞）。
    Fuzoku,
}

impl Pos {
    /// 黄金ベクトル・データファイルで使ふ一字の名前。
    pub fn tag(&self) -> &'static str {
        match self {
            Pos::Jiritsu => "自",
            Pos::Fuzoku => "付",
        }
    }
}

/// 辞書の一語。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Word {
    pub yomi: &'static str,
    pub surface: &'static str,
    pub freq: u32,
    pub pos: Pos,
}

impl Word {
    /// この語を選ぶ費用。度数が大きいほど安い。
    pub fn cost(&self) -> i32 {
        word_cost(self.freq)
    }
}

/// 読みちやうどで引く。無ければ空。
pub fn lookup(yomi: &str) -> Vec<Word> {
    let idx = match JISHO.binary_search_by(|(k, _)| (*k).cmp(yomi)) {
        Ok(i) => i,
        Err(_) => return Vec::new(),
    };
    let (key, items) = JISHO[idx];
    items
        .iter()
        .map(|(surface, freq, pos)| Word {
            yomi: key,
            surface,
            freq: *freq,
            pos: *pos,
        })
        .collect()
}

/// 読みが辞書にあるか。
pub fn contains(yomi: &str) -> bool {
    JISHO.binary_search_by(|(k, _)| (*k).cmp(yomi)).is_ok()
}

/// `rest` の**前方一致**をすべて返す（短い順）。格子を張るのに使ふ。
///
/// 返すのは `(その読みのバイト長, 語)`。上限は [`MAX_YOMI_CHARS`] 文字で、
/// 辞書に無い長さを無駄に引かない。
pub fn prefix_matches(rest: &str) -> Vec<(usize, Word)> {
    let mut out = Vec::new();
    for (n, (byte_idx, _)) in rest.char_indices().enumerate() {
        if n >= MAX_YOMI_CHARS {
            break;
        }
        let _ = byte_idx;
        // n 文字目まで（＝ n+1 文字）の前方一致
        let end = rest
            .char_indices()
            .nth(n + 1)
            .map(|(i, _)| i)
            .unwrap_or(rest.len());
        for w in lookup(&rest[..end]) {
            out.push((end, w));
        }
        if end == rest.len() {
            break;
        }
    }
    out
}

// ── 費用 ────────────────────────────────────────────────────

/// 語を一つ採る基準費用。度数 1 のときこの値になる。
pub const BASE_WORD_COST: i32 = 6000;

/// 辞書に無い一字を仮名のまま置くときの費用（一字あたり）。
///
/// 辞書の**どの語よりも高く**しておく。さうしないと「辞書に有るのに仮名のまま出る」
/// といふ、使ひ手から見て理由の分からない負け方をする。
pub const UNKNOWN_CHAR_COST: i32 = 7000;

/// `log2(x)` の整数近似（**1/256 単位**）。`x == 0` は 0 を返す。
///
/// 浮動小数を使はない——言語ごとの丸めで黄金ベクトルが揺れると、
/// 「Swift だけ候補順が違ふ」が黙つて起きるからである。
/// 手順は正規化（仮数を Q31 の \[1,2) に置く）＋ 8 回の自乗で 8 bit 分の小数を取る。
pub fn log2_fp(x: u32) -> i32 {
    if x == 0 {
        return 0;
    }
    let n = 31 - x.leading_zeros() as i32;
    // 仮数を Q31 の [2^31, 2^32) へ寄せる（＝実数の [1, 2)）
    let mut y: u64 = (x as u64) << (31 - n as u32);
    let mut out = n << 8;
    for i in 0..8 {
        y = (y * y) >> 31; // [1,4) の Q31
        if y >= 1u64 << 32 {
            out += 1 << (7 - i);
            y >>= 1;
        }
    }
    out
}

/// 度数から語の費用へ。度数が大きいほど安い。
///
/// `BASE_WORD_COST - log2(度数)`（1/256 単位）。対数にするのは、度数が
/// 一桁違ふことと二倍違ふことを同じ重みで扱ひたくないからである。
pub fn word_cost(freq: u32) -> i32 {
    BASE_WORD_COST - log2_fp(freq.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 辞書の語数が表と合ふ() {
        assert!(!JISHO.is_empty());
        assert_eq!(
            JISHO.iter().map(|(_, items)| items.len()).sum::<usize>(),
            ENTRIES,
            "ENTRIES が表の実際の語数とずれてゐる"
        );
        assert_eq!(
            JISHO.iter().map(|(y, _)| y.chars().count()).max(),
            Some(MAX_YOMI_CHARS),
            "MAX_YOMI_CHARS が最長の読みとずれてゐる（前方一致が取り零す）"
        );
    }

    #[test]
    fn 読みが昇順に並んでゐる() {
        // 二分探索の前提。焼き付けが並べ替へを忘れたらここで落ちる。
        for pair in JISHO.windows(2) {
            assert!(
                pair[0].0 < pair[1].0,
                "読みが昇順でない: {} → {}",
                pair[0].0,
                pair[1].0
            );
        }
    }

    #[test]
    fn 同じ読みの中は度数降順() {
        for (yomi, items) in JISHO.iter() {
            for w in items.windows(2) {
                assert!(
                    w[0].1 >= w[1].1,
                    "{yomi} の候補が度数降順でない: {} → {}",
                    w[0].0,
                    w[1].0
                );
            }
        }
    }

    #[test]
    fn 引ける() {
        let ws = lookup("けふ");
        assert_eq!(ws.len(), 1);
        assert_eq!(ws[0].surface, "今日");
        assert_eq!(ws[0].pos, Pos::Jiritsu);
        assert!(lookup("そんな読みは無い").is_empty());
    }

    #[test]
    fn 同じ読みに複数の表記が付く() {
        let ws = lookup("もの");
        assert_eq!(ws.len(), 2, "者・物");
        assert_eq!(ws[0].surface, "者", "度数の高い方が先");
    }

    #[test]
    fn 前方一致が短い順に出る() {
        let ms = prefix_matches("てんきなり");
        let lens: Vec<usize> = ms.iter().map(|(n, _)| *n).collect();
        assert!(
            lens.windows(2).all(|w| w[0] <= w[1]),
            "短い順でない: {lens:?}"
        );
        let surfaces: Vec<&str> = ms.iter().map(|(_, w)| w.surface).collect();
        assert!(surfaces.contains(&"天"), "てん が取れる");
        assert!(surfaces.contains(&"天気"), "てんき が取れる");
    }

    #[test]
    fn 前方一致は文字境界で切る() {
        // バイトで切ると UTF-8 が壊れる。壊れてゐたらここで panic する。
        for m in prefix_matches("あかきはなのごとし") {
            assert!(m.0 > 0);
        }
    }

    #[test]
    fn 対数の整数近似が正しい() {
        assert_eq!(log2_fp(0), 0);
        assert_eq!(log2_fp(1), 0);
        assert_eq!(log2_fp(2), 256);
        assert_eq!(log2_fp(4), 512);
        assert_eq!(log2_fp(1024), 2560);
        // log2(3) = 1.58496… → 1.582…（1/256 の刻み以内）
        assert_eq!(log2_fp(3), 405);
        for x in [3u32, 5, 7, 100, 999, 65535, 1 << 20] {
            let want = (x as f64).log2() * 256.0;
            let got = log2_fp(x) as f64;
            assert!((got - want).abs() <= 1.0, "log2_fp({x}) = {got} ≠ {want}");
        }
    }

    #[test]
    fn 対数は単調で溢れない() {
        let mut prev = log2_fp(1);
        for x in [2u32, 3, 10, 1000, u32::MAX] {
            let cur = log2_fp(x);
            assert!(cur >= prev, "単調でない: {x}");
            prev = cur;
        }
        // u32::MAX は 2^32 に**わづかに満たない**ので 32*256 には届かない。
        // 切り捨て側へ倒れることをここで固定しておく（丸めの向きが揺れると費用が揺れる）。
        assert_eq!(log2_fp(u32::MAX), 8191);
    }

    #[test]
    fn 度数が高い語ほど安い() {
        assert!(word_cost(9000) < word_cost(300));
        assert!(word_cost(1) <= BASE_WORD_COST);
        // 辞書の語は必ず未知字より安い（さうでないと辞書が負ける）
        for (_, items) in JISHO.iter() {
            for (surface, freq, _) in items.iter() {
                assert!(
                    word_cost(*freq) < UNKNOWN_CHAR_COST,
                    "{surface}（度数 {freq}）が未知字より高い"
                );
            }
        }
    }
}
