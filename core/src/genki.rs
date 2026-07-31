//! 原器 — 縦組五十音配列（`docs/ime/layout.md` §1）。**物理鍵盤の座標系**である。
//!
//! 使用者が約 10 年使ひ込んできた自作 PC 配列で、モバイルの二行配列はこれを硝子へ
//! 折り畳んだものにあたる。iOS のキーボード拡張は**ハードキーを受け取れない**ので、
//! 原器がそのまま活きる場は macOS と Windows しかない
//! ——だから原器の実装は核に置き、両方の殻が同じものを使ふ
//! （[`docs/ime/cross-platform.md`] §4）。
//!
//! ## 紙面の見立て
//!
//! 右半面が縦書きの第一行。**段（あいうえお）が右から左へ**進み、
//! **行（あ→か→さ）が上から下へ**降りる。`j`（そ）まで書き切ると紙面の行を替へるやうに
//! 左半面へ折り返し、第二の行（た→な→は）が再び上から降りる。
//!
//! ```text
//!       左半面（第二の行）              右半面（第一の行）
//!   ［1］［2］［3］［4］［5］    ［6］［7］［8］［9］［0］   ［^］
//!    と   て   つ   ち   た      お   え   う   い   あ     前置シフト
//!   ［q］［w］［e］［r］［t］    ［y］［u］［i］［o］［p］
//!    の   ね   ぬ   に   な      こ   け   く   き   か
//!   ［a］［s］［d］［f］［g］    ［j］［k］［l］［;］［:］
//!    ほ   へ   ふ   ひ   は      そ   せ   す   し   さ
//! ```
//!
//! 第二面（`^` 前置後）は同じ紙面構造の写しで、右半面に ま・や、左半面に ら・わ。
//! **や行・わ行とも五段が埋まる**（や行に い・え、わ行に う を重複配置——
//! 地図の完全性を欠けより優先する）。ん は行にも段にも属さぬ唯一の仮名として、
//! シフトそのものの重ね打ち `^^` に住む。
//!
//! ## 確定してゐること / まだ決めてゐないこと
//!
//! 鍵盤は **JIS**、濁点は `b`・半濁点は `v` の**後置打鍵**（本人確認済み・2026-07-30）。
//! 記号の配置と最下段の残り、小書き（ゃゅょっ）の打ち方は**未定**であり、
//! ここでは**発明しない**（未定のものを勝手に決めると、原器が原器でなくなる）。

use crate::gojuon::{self, Deflect};

/// 前置シフトの鍵。これ自身を重ね打つと「ん」になる。
pub const SHIFT: char = '^';
/// 濁点（後置打鍵）。
pub const DAKUTEN: char = 'b';
/// 半濁点（後置打鍵）。
pub const HANDAKUTEN: char = 'v';

/// 第一面（無シフト）— 清音 30 字。左半面が第二の行、右半面が第一の行。
#[rustfmt::skip]
pub static FIRST_PLANE: [(char, &str); 30] = [
    // 段: 右から左へ（0=あ段 … 6=お段）／行: 上から下へ
    ('0', "あ"), ('9', "い"), ('8', "う"), ('7', "え"), ('6', "お"),
    ('p', "か"), ('o', "き"), ('i', "く"), ('u', "け"), ('y', "こ"),
    (':', "さ"), (';', "し"), ('l', "す"), ('k', "せ"), ('j', "そ"),
    // ここで紙面を折り返して左半面へ
    ('5', "た"), ('4', "ち"), ('3', "つ"), ('2', "て"), ('1', "と"),
    ('t', "な"), ('r', "に"), ('e', "ぬ"), ('w', "ね"), ('q', "の"),
    ('g', "は"), ('f', "ひ"), ('d', "ふ"), ('s', "へ"), ('a', "ほ"),
];

/// 第二面（`^` 前置後）— ま・や・ら・わ行。`^^` は「ん」（表には入れない）。
#[rustfmt::skip]
pub static SECOND_PLANE: [(char, &str); 20] = [
    ('0', "ま"), ('9', "み"), ('8', "む"), ('7', "め"), ('6', "も"),
    ('p', "や"), ('o', "い"), ('i', "ゆ"), ('u', "え"), ('y', "よ"),
    ('5', "ら"), ('4', "り"), ('3', "る"), ('2', "れ"), ('1', "ろ"),
    ('t', "わ"), ('r', "ゐ"), ('e', "う"), ('w', "ゑ"), ('q', "を"),
];

/// 打鍵が作業帯へ及ぼす変化。**核は文を持たない**——殻が持つ文への指示だけを返す。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Edit {
    /// 仮名を足す。
    Insert(&'static str),
    /// 直前の一字を差し替へる（濁点・半濁点の後置打鍵）。
    ReplaceLast(char),
    /// 目に見える変化なし（前置シフトが立つた等）。
    None,
    /// この鍵は原器に無い（記号・機能キーは殻の領分）。
    Unmapped,
}

fn lookup(plane: &[(char, &'static str)], key: char) -> Option<&'static str> {
    plane.iter().find(|(k, _)| *k == key).map(|(_, kana)| *kana)
}

/// 濁点を打つた結果。濁れない字なら `None`。
pub fn dakuten(kana: char) -> Option<char> {
    let (gyo, dan) = gojuon::reverse_lookup(kana)?;
    gojuon::gyo_named(gyo)?
        .kana(dan, Deflect::Daku)
        .and_then(|s| s.chars().next())
}

/// 半濁点を打つた結果。は行だけが半濁点を持つ。
///
/// 小書き（ゃゅょっ）は原器では**未定**なので、や行・た行には及ぼさない
/// （二行配列では同じ左逸らし面に小書きが同居するが、それは硝子の話である）。
pub fn handakuten(kana: char) -> Option<char> {
    let (gyo, dan) = gojuon::reverse_lookup(kana)?;
    if gyo != "は" {
        return None;
    }
    gojuon::gyo_named(gyo)?
        .kana(dan, Deflect::Ko)
        .and_then(|s| s.chars().next())
}

/// 原器の状態機械。前置シフトは**逐次打鍵**（同時押しでない）ゆゑ状態が要る。
#[derive(Clone, Default, Debug)]
pub struct Genki {
    shifted: bool,
}

impl Genki {
    pub fn new() -> Self {
        Self::default()
    }

    /// 前置シフトが立つてゐるか（殻が面の表示を切り替へるために見る）。
    pub fn is_shifted(&self) -> bool {
        self.shifted
    }

    /// 一打を食はせる。`last` は作業帯の末尾の一字（濁点の後置打鍵に要る）。
    pub fn press(&mut self, key: char, last: Option<char>) -> Edit {
        if key == SHIFT {
            // ん は行にも段にも属さぬ唯一の仮名——シフトの重ね打ちに住む
            if self.shifted {
                self.shifted = false;
                return Edit::Insert("ん");
            }
            self.shifted = true;
            return Edit::None;
        }

        if self.shifted {
            self.shifted = false;
            return match lookup(&SECOND_PLANE, key) {
                Some(kana) => Edit::Insert(kana),
                None => Edit::Unmapped,
            };
        }

        match key {
            DAKUTEN => last
                .and_then(dakuten)
                .map(Edit::ReplaceLast)
                .unwrap_or(Edit::Unmapped),
            HANDAKUTEN => last
                .and_then(handakuten)
                .map(Edit::ReplaceLast)
                .unwrap_or(Edit::Unmapped),
            _ => match lookup(&FIRST_PLANE, key) {
                Some(kana) => Edit::Insert(kana),
                None => Edit::Unmapped,
            },
        }
    }

    /// 前置シフトを取り消す（別の入力欄へ移る等、殻の都合で状態を捨てるとき）。
    pub fn reset(&mut self) {
        self.shifted = false;
    }
}

/// 打鍵の列を仮名の列へ（試験と稽古場のための便宜。殻は [`Genki::press`] を使ふ）。
pub fn type_keys(keys: &str) -> String {
    let mut g = Genki::new();
    let mut out = String::new();
    for key in keys.chars() {
        match g.press(key, out.chars().last()) {
            Edit::Insert(kana) => out.push_str(kana),
            Edit::ReplaceLast(c) => {
                out.pop();
                out.push(c);
            }
            Edit::None | Edit::Unmapped => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 紙面は右から左へ段_上から下へ行() {
        assert_eq!(type_keys("0987"), "あいうえ"); // 段は右から左
        assert_eq!(type_keys("0pj"), "あかそ"); // 行は上から下、j で折り返し
        assert_eq!(type_keys("5ta"), "たなほ"); // 左半面も上から下
    }

    #[test]
    fn 第一面は清音三十字が揃ふ() {
        assert_eq!(FIRST_PLANE.len(), 30);
        let kana: Vec<&str> = FIRST_PLANE.iter().map(|(_, k)| *k).collect();
        for k in ["あ", "か", "さ", "た", "な", "は", "ほ", "そ"] {
            assert!(kana.contains(&k), "{k} が第一面に無い");
        }
    }

    #[test]
    fn 第二面はや行わ行とも五段が埋まる() {
        // や行に い・え、わ行に う を重複配置する（地図の完全性を優先）
        assert_eq!(type_keys("^p^o^i^u^y"), "やいゆえよ");
        assert_eq!(type_keys("^t^r^e^w^q"), "わゐうゑを");
    }

    #[test]
    fn ゐゑが第一級の鍵を持つ() {
        assert_eq!(type_keys("^r"), "ゐ");
        assert_eq!(type_keys("^w"), "ゑ");
    }

    #[test]
    fn シフトの重ね打ちが_ん() {
        assert_eq!(type_keys("^^"), "ん");
        assert_eq!(type_keys("t^^"), "なん");
    }

    #[test]
    fn 前置シフトは逐次打鍵で一打だけ効く() {
        let mut g = Genki::new();
        assert_eq!(g.press('^', None), Edit::None);
        assert!(g.is_shifted());
        assert_eq!(g.press('0', None), Edit::Insert("ま")); // 第二面
        assert!(!g.is_shifted());
        assert_eq!(g.press('0', None), Edit::Insert("あ")); // 戻つてゐる
    }

    #[test]
    fn 濁点と半濁点は後置打鍵() {
        assert_eq!(type_keys("pb"), "が"); // か → が
        assert_eq!(type_keys("gb"), "ば"); // は → ば
        assert_eq!(type_keys("gv"), "ぱ"); // は → ぱ
        assert_eq!(type_keys("0b"), "あ"); // 濁れない字は変はらない
        assert_eq!(type_keys("pv"), "か"); // か行に半濁点は無い
    }

    #[test]
    fn 小書きは原器では未定ゆゑ発明しない() {
        // や行・た行に半濁点（v）を及ぼして小書きを作つたりはしない
        assert_eq!(handakuten('や'), None);
        assert_eq!(handakuten('つ'), None);
    }

    #[test]
    fn 原器に無い鍵は殻へ返す() {
        let mut g = Genki::new();
        assert_eq!(g.press('z', None), Edit::Unmapped);
        assert_eq!(g.press('/', None), Edit::Unmapped);
    }

    #[test]
    fn 文を打てる() {
        // 「けふはよきてんきなり」= け(u) ふ(d) は(g) よ(^y) き(o) て(2) ん(^^) き(o) な(t) り(^4)
        assert_eq!(type_keys("udg^yo2^^ot^4"), "けふはよきてんきなり");
        // 濁点つき: 「かぜ」= か(p) 濁(b)…ではなく「が」、ぜ = せ(k)＋濁(b)
        assert_eq!(type_keys("pbkb"), "がぜ");
    }
}
