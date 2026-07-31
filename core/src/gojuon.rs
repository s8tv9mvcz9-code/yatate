//! 五十音の幾何学 — 行×段の地図（`docs/ime/layout.md` §2、`Gojuon.swift` のパリティ実装）。
//!
//! 梯子は常に **5 スロット固定**。や行・わ行の空きは `None` で残し、位置の筋肉記憶を守る。
//! ゐ・ゑ はわ行の 1・3 段目の第一級市民である。

/// 逸らし（deflection）— 書き下ろしスライド中の横方向の意味。
/// 濁点は字の右肩に打たれるものだから、**右への逸らしが濁点**になる。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Deflect {
    /// 清音
    None,
    /// 右: 濁点（か→が）
    Daku,
    /// 左: 半濁点（は→ぱ）または小書き（や→ゃ、つ→っ、あ→ぁ）
    Ko,
}

/// 一つの行（ぎゃう）。キーの刻印と三態の梯子。
pub struct Gyo {
    /// 行頭の仮名（キー刻印）
    pub name: &'static str,
    /// 5 スロット（あ〜お段）
    pub seion: [Option<&'static str>; 5],
    /// 右逸らし面（無い行は `None`）
    pub daku: Option<[Option<&'static str>; 5]>,
    /// 左逸らし面（無い行は `None`）
    pub ko: Option<[Option<&'static str>; 5]>,
}

impl Gyo {
    /// 段と逸らしから仮名を引く。空きスロットは `None`（何も入力しない）。
    pub fn kana(&self, dan: usize, deflect: Deflect) -> Option<&'static str> {
        if dan >= 5 {
            return None;
        }
        match deflect {
            Deflect::None => self.seion[dan],
            Deflect::Daku => self.daku.as_ref().and_then(|p| p[dan]),
            Deflect::Ko => self.ko.as_ref().and_then(|p| p[dan]),
        }
    }

    /// この行が持つ全ての面を（清音・濁点・小書きの順に）辿る。
    fn planes(&self) -> [Option<&[Option<&'static str>; 5]>; 3] {
        [Some(&self.seion), self.daku.as_ref(), self.ko.as_ref()]
    }
}

macro_rules! gyo {
    ($name:expr, [$($s:expr),*]) => {
        Gyo { name: $name, seion: [$($s),*], daku: None, ko: None }
    };
    ($name:expr, [$($s:expr),*], daku: [$($d:expr),*]) => {
        Gyo { name: $name, seion: [$($s),*], daku: Some([$($d),*]), ko: None }
    };
    ($name:expr, [$($s:expr),*], ko: [$($k:expr),*]) => {
        Gyo { name: $name, seion: [$($s),*], daku: None, ko: Some([$($k),*]) }
    };
    ($name:expr, [$($s:expr),*], daku: [$($d:expr),*], ko: [$($k:expr),*]) => {
        Gyo { name: $name, seion: [$($s),*], daku: Some([$($d),*]), ko: Some([$($k),*]) }
    };
}

// rustfmt を止める — この表の桁揃へは装飾ではなく**地図**である。
// 5 スロットが横一列に並んでゐるから「や行の 2 段目が空き」が目で読める。
// 自動整形は 1 行 1 引数へ崩し、行×段の対応が読めなくなる。
#[rustfmt::skip]
pub static A: Gyo = gyo!("あ", [Some("あ"), Some("い"), Some("う"), Some("え"), Some("お")],
    daku: [None, None, Some("ゔ"), None, None],
    ko:   [Some("ぁ"), Some("ぃ"), Some("ぅ"), Some("ぇ"), Some("ぉ")]);
#[rustfmt::skip]
pub static KA: Gyo = gyo!("か", [Some("か"), Some("き"), Some("く"), Some("け"), Some("こ")],
    daku: [Some("が"), Some("ぎ"), Some("ぐ"), Some("げ"), Some("ご")]);
#[rustfmt::skip]
pub static SA: Gyo = gyo!("さ", [Some("さ"), Some("し"), Some("す"), Some("せ"), Some("そ")],
    daku: [Some("ざ"), Some("じ"), Some("ず"), Some("ぜ"), Some("ぞ")]);
#[rustfmt::skip]
pub static TA: Gyo = gyo!("た", [Some("た"), Some("ち"), Some("つ"), Some("て"), Some("と")],
    daku: [Some("だ"), Some("ぢ"), Some("づ"), Some("で"), Some("ど")],
    ko:   [None, None, Some("っ"), None, None]);
#[rustfmt::skip]
pub static NA: Gyo = gyo!("な", [Some("な"), Some("に"), Some("ぬ"), Some("ね"), Some("の")]);
#[rustfmt::skip]
pub static HA: Gyo = gyo!("は", [Some("は"), Some("ひ"), Some("ふ"), Some("へ"), Some("ほ")],
    daku: [Some("ば"), Some("び"), Some("ぶ"), Some("べ"), Some("ぼ")],
    ko:   [Some("ぱ"), Some("ぴ"), Some("ぷ"), Some("ぺ"), Some("ぽ")]);
#[rustfmt::skip]
pub static MA: Gyo = gyo!("ま", [Some("ま"), Some("み"), Some("む"), Some("め"), Some("も")]);
#[rustfmt::skip]
pub static YA: Gyo = gyo!("や", [Some("や"), None, Some("ゆ"), None, Some("よ")],
    ko:   [Some("ゃ"), None, Some("ゅ"), None, Some("ょ")]);
#[rustfmt::skip]
pub static RA: Gyo = gyo!("ら", [Some("ら"), Some("り"), Some("る"), Some("れ"), Some("ろ")]);
#[rustfmt::skip]
pub static WA: Gyo = gyo!("わ", [Some("わ"), Some("ゐ"), None, Some("ゑ"), Some("を")]);

/// 二行配列の第一の行（画面右列・上→下）— 縦書きの読み順で先。
pub static FIRST_LINE: [&Gyo; 5] = [&A, &KA, &SA, &TA, &NA];
/// 第二の行（左列・上→下）。
pub static SECOND_LINE: [&Gyo; 5] = [&HA, &MA, &YA, &RA, &WA];

/// 二行配列の全 10 行を読み順で辿る。
pub fn all() -> impl Iterator<Item = &'static Gyo> {
    FIRST_LINE.into_iter().chain(SECOND_LINE)
}

/// 行名から行を引く（"か" → か行）。
pub fn gyo_named(name: &str) -> Option<&'static Gyo> {
    all().find(|g| g.name == name)
}

/// 仮名 → (行名, 段)。濁点・半濁点・小書きも**基底の鍵へ畳む**。
///
/// 「墨の気配」が bigram を鍵盤へ射影するときの逆写像で、
/// `Kehai.swift` の `reverseMap` と同じ規則で作る。
pub fn reverse_lookup(c: char) -> Option<(&'static str, usize)> {
    for g in all() {
        for plane in g.planes().into_iter().flatten() {
            for (dan, slot) in plane.iter().enumerate() {
                if let Some(kana) = slot {
                    if kana.starts_with(c) {
                        return Some((g.name, dan));
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 梯子は常に五スロット() {
        for g in all() {
            assert_eq!(g.seion.len(), 5, "{}", g.name);
            if let Some(p) = &g.daku {
                assert_eq!(p.len(), 5, "{}", g.name);
            }
            if let Some(p) = &g.ko {
                assert_eq!(p.len(), 5, "{}", g.name);
            }
        }
    }

    #[test]
    fn ゐゑは第一級市民() {
        assert_eq!(WA.kana(1, Deflect::None), Some("ゐ"));
        assert_eq!(WA.kana(3, Deflect::None), Some("ゑ"));
        assert_eq!(WA.kana(2, Deflect::None), None); // 空きは空きのまま
    }

    #[test]
    fn 右逸らしが濁点_左逸らしが半濁と小書き() {
        assert_eq!(KA.kana(0, Deflect::Daku), Some("が"));
        assert_eq!(HA.kana(0, Deflect::Ko), Some("ぱ"));
        assert_eq!(TA.kana(2, Deflect::Ko), Some("っ"));
        assert_eq!(NA.kana(0, Deflect::Daku), None); // 濁点面の無い行
        assert_eq!(KA.kana(9, Deflect::None), None); // 範囲外
    }

    #[test]
    fn 逆引きは基底の鍵へ畳む() {
        assert_eq!(reverse_lookup('が'), Some(("か", 0)));
        assert_eq!(reverse_lookup('ぱ'), Some(("は", 0)));
        assert_eq!(reverse_lookup('ゐ'), Some(("わ", 1)));
        assert_eq!(reverse_lookup('ん'), None); // 体系外は殻が字として扱ふ
    }
}
