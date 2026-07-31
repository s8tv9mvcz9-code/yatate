//! 墨の氣配 — 言語の期待を行為空間（どの鍵か）へ射影する（`docs/ime/vla.md` A0）。
//!
//! 仮名連 bigram（青空文庫・機械生成）を、二行配列の**鍵ごとの墨の濃淡**（0…1）へ落とす。
//! 決定的・訓練なし・ネットワーク不要。`Kehai.swift` のパリティ実装である。
//!
//! **弱信号は無地**（実用性ガード）。観測が [`MIN_EVIDENCE`] に満たなければ何も描かない
//! ——共感覚レイヤーが弱信号で中立色に落ちるのと同じ思想で、
//! 道具が確信の無い助言で画面を汚さないための線である。
//!
//! ## Swift 版との一点の違ひ（意図的）
//!
//! Swift 版は峰（`peak`）を `Dictionary.max{...}` で選ぶため、**同点のとき結果が
//! 辞書の内部順序に依存する**（＝実行ごとに変はり得る）。核は全 OS の基準になるので、
//! ここでは同点を**読み順（あ→か→…→わ、次に ん・、・。）の先着**で割る。
//! M5-b で Swift がこの核へ載れば、その不安定さは消える。

use crate::generated::kana_bigram::KANA_BIGRAM;
use crate::gojuon::{self, reverse_lookup};

/// 弱信号は無地（`docs/ime/vla.md` §3）。観測がこの回数に満たねば描かない。
pub const MIN_EVIDENCE: u32 = 20;

/// 体系の外に置く字（行×段の梯子に乗らないが、鍵盤には在るもの）。
pub const EXTRA_KEYS: [char; 3] = ['ん', '、', '。'];

/// 鍵の同一性。行キーは行名で、体系外の字は字そのもので指す。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyId {
    /// "あ" "か" … "わ"
    Gyo(&'static str),
    /// 'ん' '、' '。'
    Moji(char),
}

impl KeyId {
    /// 読み順の位置（同点の割り方を決めるための正準順序）。
    fn order(&self) -> usize {
        match self {
            KeyId::Gyo(name) => gojuon::all().position(|g| g.name == *name).unwrap_or(99),
            KeyId::Moji(c) => 10 + EXTRA_KEYS.iter().position(|k| k == c).unwrap_or(9),
        }
    }
}

/// 行為空間に描かれた場。`ink` は鍵の墨（最大値 1 に正規化）、`dan` は梯子内の段の墨。
///
/// どちらも**正準順序で並んでゐる**（言語をまたいでも同じ並びになる＝ベクトルで縛れる）。
#[derive(Clone, Debug, Default)]
pub struct ActionField {
    pub ink: Vec<(KeyId, f64)>,
    pub dan: Vec<(&'static str, [f64; 5])>,
    /// 筆脈の先（最有力の一手）。弱信号なら `None`。
    pub peak: Option<KeyId>,
}

impl ActionField {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.ink.is_empty()
    }

    /// 鍵の墨。描かれてゐなければ 0。
    pub fn ink_of(&self, key: &KeyId) -> f64 {
        self.ink
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| *v)
            .unwrap_or(0.0)
    }

    /// 行の梯子の段の墨。描かれてゐなければ全て 0。
    pub fn dan_of(&self, gyo: &str) -> [f64; 5] {
        self.dan
            .iter()
            .find(|(g, _)| *g == gyo)
            .map(|(_, v)| *v)
            .unwrap_or([0.0; 5])
    }
}

fn row_for(key: &str) -> &'static [(char, u32)] {
    let find = |k: &str| {
        KANA_BIGRAM
            .iter()
            .find(|(prev, _)| *prev == k)
            .map(|(_, items)| *items)
    };
    // 句読点・表に無い字の後は「連なりの開始」分布へ退避する
    find(key).or_else(|| find("^")).unwrap_or(&[])
}

/// 直前の一字（無ければ連なりの開始 `^`）から、鍵盤に滲む氣配を出す。
pub fn field(prev: Option<char>) -> ActionField {
    let key = prev.map(String::from).unwrap_or_else(|| "^".to_string());
    let rows = row_for(&key);

    let mut ink: Vec<(KeyId, u32)> = Vec::new();
    let mut dan: Vec<(&'static str, [u32; 5])> = Vec::new();
    let mut total: u32 = 0;

    for &(next, count) in rows {
        total += count;
        if let Some(pos) = EXTRA_KEYS.iter().position(|k| *k == next) {
            let id = KeyId::Moji(EXTRA_KEYS[pos]);
            match ink.iter_mut().find(|(k, _)| *k == id) {
                Some((_, v)) => *v += count,
                None => ink.push((id, count)),
            }
        } else if let Some((gyo, d)) = reverse_lookup(next) {
            let id = KeyId::Gyo(gyo);
            match ink.iter_mut().find(|(k, _)| *k == id) {
                Some((_, v)) => *v += count,
                None => ink.push((id, count)),
            }
            match dan.iter_mut().find(|(g, _)| *g == gyo) {
                Some((_, row)) => row[d] += count,
                None => {
                    let mut row = [0u32; 5];
                    row[d] = count;
                    dan.push((gyo, row));
                }
            }
        }
    }

    let max_ink = ink.iter().map(|(_, v)| *v).max().unwrap_or(0);
    if total < MIN_EVIDENCE || max_ink == 0 {
        return ActionField::empty();
    }

    // 正準順序へ並べ替へる（言語をまたいで同じ並びにする）
    ink.sort_by_key(|(k, _)| k.order());
    dan.sort_by_key(|(g, _)| gojuon::all().position(|x| x.name == *g).unwrap_or(99));

    // 峰は「墨の濃い順、同点なら読み順の先着」——同点で結果が揺れないやうに
    let peak = ink
        .iter()
        .max_by(|a, b| a.1.cmp(&b.1).then(b.0.order().cmp(&a.0.order())))
        .map(|(k, _)| *k);

    ActionField {
        ink: ink
            .into_iter()
            .map(|(k, v)| (k, f64::from(v) / f64::from(max_ink)))
            .collect(),
        dan: dan
            .into_iter()
            .map(|(g, row)| {
                let m = row.iter().copied().max().unwrap_or(0);
                let norm = if m > 0 {
                    let mut out = [0.0; 5];
                    for (i, v) in row.iter().enumerate() {
                        out[i] = f64::from(*v) / f64::from(m);
                    }
                    out
                } else {
                    [0.0; 5]
                };
                (g, norm)
            })
            .collect(),
        peak,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 連なりの開始に氣配が立つ() {
        let f = field(None);
        assert!(!f.is_empty());
        // 開始分布の最頻は「の」＝な行。峰もそこを指す
        assert_eq!(f.peak, Some(KeyId::Gyo("な")));
        assert!(
            (f.ink_of(&KeyId::Gyo("な")) - 1.0).abs() < 1e-9,
            "峰は 1 に正規化される"
        );
        assert!(f.ink_of(&KeyId::Gyo("あ")) < 1.0);
    }

    #[test]
    fn 表に無い字は開始分布へ退避する() {
        // 「々」は仮名連に出ないので開始分布と同じ場になる
        assert_eq!(field(Some('々')).peak, field(None).peak);
    }

    #[test]
    fn 段の墨は行ごとに正規化される() {
        let f = field(None);
        for (_, row) in &f.dan {
            let m = row.iter().cloned().fold(0.0_f64, f64::max);
            assert!(
                (m - 1.0).abs() < 1e-9 || m == 0.0,
                "行内の最大が 1 でない: {row:?}"
            );
            assert!(row.iter().all(|v| (0.0..=1.0).contains(v)));
        }
    }

    #[test]
    fn 弱信号は無地() {
        // 度数の乏しい前字は総観測が MIN_EVIDENCE に届かず、何も描かない
        let weak = KANA_BIGRAM
            .iter()
            .find(|(_, items)| items.iter().map(|(_, c)| *c).sum::<u32>() < MIN_EVIDENCE);
        if let Some((prev, _)) = weak {
            let c = prev.chars().next().unwrap();
            assert!(field(Some(c)).is_empty(), "{prev} で無地にならない");
        }
    }

    #[test]
    fn 句読点も鍵として墨を持つ() {
        // 「い」の後は「。」が高頻度（文の終はる氣配）
        let f = field(Some('い'));
        assert!(f.ink_of(&KeyId::Moji('。')) > 0.0);
    }

    #[test]
    fn 峰は同点でも揺れない() {
        // 同じ入力を何度呼んでも同じ峰（Swift の Dictionary 順依存を排した点）
        for _ in 0..50 {
            assert_eq!(field(Some('か')).peak, field(Some('か')).peak);
        }
    }
}
