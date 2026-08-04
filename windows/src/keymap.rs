//! JIS 物理鍵盤 → 原器の鍵（`docs/ime/layout.md` §1）。**この module に COM は無い。**
//!
//! ## なぜ独立した層が要るのか
//!
//! 原器は「`0` が あ、`:` が さ」のやうに**文字**で定義されてゐる。
//! 一方 TSF が渡してくるのは**仮想キーコード**（VK）と**走査符号**（scancode）である。
//! その間を埋めるのがここで、**写像は配列によつて動く**。
//!
//! ```text
//!         物理位置      JIS の VK              US の VK
//!   さ    sc 0x28   VK_OEM_1   (0xBA)     VK_OEM_7   (0xDE)   ← 刻印は「:」対「'」
//!   し    sc 0x27   VK_OEM_PLUS(0xBB)     VK_OEM_1   (0xBA)   ← **刻印は両方「;」**
//!   ^     sc 0x0D   VK_OEM_7   (0xDE)     VK_OEM_PLUS(0xBB)   ← 刻印は「^」対「=」
//! ```
//!
//! 「し」の行が最も危い。**刻印も物理位置も同じなのに VK だけが違ふ**ので、
//! 世間の（ほぼ US 前提の）VK 表を写すと、実機で「し」が黙つて「さ」になる。
//! 例外も警告も出ない。だから表を持ち、試験で縛る。
//!
//! ## 走査符号を主とする（配列判定を要らなくする）
//!
//! 原器は**指の位置の地図**であり、字面ではなく鍵の位置に意味がある
//! （`docs/ime/layout.md` §1「五十音の幾何学」）。走査符号は配列に依らず
//! **物理位置**を指すので、原器の思想とそのまま噛み合ふ。
//!
//! ゆゑに配列で意味の変はる 3 鍵は **scancode で引き**、残る 30 鍵
//! （数字 10・英字 20）は全配列共通なので VK で引く。
//! これで「JIS か US か」を実行時に当てる必要が消える——
//! 当て損なひが無音の誤爆になる箇所を、そもそも作らない。
//!
//! ## `ToUnicodeEx` を使はない理由（重要）
//!
//! 「今の配列で何の字が出るか」を返す `ToUnicodeEx` は一見正しく見えるが、
//! **かなロック（VK_KANA）が立つと壊れる**。実測では VK_OEM_1 が `:` ではなく
//! 半角カタカナ `ｹ`(U+FF79) を返し、VK_OEM_PLUS→`ﾚ`、VK_OEM_7→`ﾍ` となる。
//! 文字を鍵にした写像表は、かな入力モードの環境で全面的に破綻する。
//! 加へて `ITfKeyEventSink::OnTestKeyDown` は**副作用禁止**の契約であり、
//! `ToUnicodeEx` はデッドキー緩衝を書き換へ得るので、そもそも呼んではならない。
//!
//! ## 依存を持たない
//!
//! VK も scancode も単なる整数なので、`windows` crate を参照しない。
//! おかげでこの module は **Linux の `cargo test` で丸ごと試験できる**
//! ——原器の心臓部が、Windows 機を持たずに守れる。

/// 仮想キーコード（Win32 の `VK_*`）。`windows` crate に依存しないため素の `u16` で持つ。
pub type Vk = u16;
/// 走査符号（set 1）。`lParam` の 16〜23 bit に入つてくる**物理位置**。
pub type Scan = u16;

// ── 配列で意味の変はる 3 鍵（走査符号で引く）───────────────
/// JIS `^`（原器の**前置シフト**）。US 配列の同位置は `=`。
pub const SC_CARET: Scan = 0x0D;
/// JIS `;`（原器の し）。US 配列の同位置も刻印は `;` だが VK が違ふ。
pub const SC_SEMICOLON: Scan = 0x27;
/// JIS `:`（原器の さ）。US 配列の同位置は `'`。
pub const SC_COLON: Scan = 0x28;

// ── 参考: 同じ 3 鍵の VK（JIS）。走査符号が取れない経路の予備に使ふ。
pub const VK_OEM_1: Vk = 0xBA; // JIS: `:`  / US: `;`
pub const VK_OEM_PLUS: Vk = 0xBB; // JIS: `;`  / US: `=`
pub const VK_OEM_7: Vk = 0xDE; // JIS: `^`  / US: `'`

// ── 殻が別に扱ふ機能キー ────────────────────────────────
pub const VK_BACK: Vk = 0x08;
pub const VK_RETURN: Vk = 0x0D;
pub const VK_ESCAPE: Vk = 0x1B;
pub const VK_SPACE: Vk = 0x20;
pub const VK_SHIFT: Vk = 0x10;
pub const VK_CONTROL: Vk = 0x11;
pub const VK_MENU: Vk = 0x12;

// ── 変換の段で使ふ矢印（`yatate_core::henkan`）────────────
//
// 走査符号では引かない。矢印は拡張キー（`E0` 前置）で、原器の三鍵とは
// 走査符号の空間が重ならないので VK で足りる。
/// 注目する文節を左へ（Shift 併用で**縮める**）。
pub const VK_LEFT: Vk = 0x25;
/// 前の候補へ。
pub const VK_UP: Vk = 0x26;
/// 注目する文節を右へ（Shift 併用で**伸ばす**）。
pub const VK_RIGHT: Vk = 0x27;
/// 次の候補へ。
pub const VK_DOWN: Vk = 0x28;

/// `GetKeyboardType(0)` が日本語鍵盤を指す値。**配列の判定はこれで行ふ**
/// （`GetKeyboardLayout` の HKL では 106 と 101 を見分けられない——
/// 日本語環境ではどちらも入力ロケールが同一で、差は下層の
/// `LayerDriver JPN`（kbd106.dll か kbd101.dll か）に在るため）。
pub const KEYBOARD_TYPE_JAPANESE: u32 = 7;
/// 同 `4` は Enhanced 101/102（US 等）。
pub const KEYBOARD_TYPE_ENHANCED: u32 = 4;

/// 全配列で共通の 30 鍵（数字 10・英字 20）。
///
/// これらは Windows の鍵盤定義で全 `KBD_TYPE` 共通と決められてゐるので、
/// 配列を問はず VK で引いてよい。
#[rustfmt::skip]
static UNIVERSAL_TABLE: &[(Vk, char)] = &[
    // 数字列（右半面 あ〜お ＝ 0,9,8,7,6 ／ 左半面 た行 ＝ 5,4,3,2,1）
    (0x30, '0'), (0x39, '9'), (0x38, '8'), (0x37, '7'), (0x36, '6'),
    (0x35, '5'), (0x34, '4'), (0x33, '3'), (0x32, '2'), (0x31, '1'),
    // 上段（か行 ＝ p,o,i,u,y ／ な行 ＝ t,r,e,w,q）
    (0x50, 'p'), (0x4F, 'o'), (0x49, 'i'), (0x55, 'u'), (0x59, 'y'),
    (0x54, 't'), (0x52, 'r'), (0x45, 'e'), (0x57, 'w'), (0x51, 'q'),
    // 中段の英字（す・せ・そ ＝ l,k,j ／ は行 ＝ g,f,d,s,a）
    (0x4C, 'l'), (0x4B, 'k'), (0x4A, 'j'),
    (0x47, 'g'), (0x46, 'f'), (0x44, 'd'), (0x53, 's'), (0x41, 'a'),
    // 後置の濁点・半濁点
    (0x42, 'b'), (0x56, 'v'),
];

/// 配列で意味の変はる 3 鍵（**走査符号**＝物理位置で引く）。
#[rustfmt::skip]
static POSITIONAL_TABLE: &[(Scan, char)] = &[
    (SC_COLON,     ':'), // さ
    (SC_SEMICOLON, ';'), // し
    (SC_CARET,     '^'), // 前置シフト（^^ で ん）
];

/// 打鍵を原器の文字へ写す。原器に無い鍵は `None`。
///
/// `scancode` は `lParam` から取り出した物理位置（0 なら未知）。
/// **位置を優先**するので、配列判定に依らず「さ・し・前置シフト」が正しく取れる。
///
/// **修飾キーは見ない。** 原器の前置シフトは `^` の**逐次打鍵**であり、
/// Shift の同時押しではない（`docs/ime/layout.md` §1）。
/// Ctrl/Alt が押されてゐるかは殻が先に判断してここへ来る前に素通しする
/// （短絡キー Ctrl+C 等を食はぬため）。
pub fn genki_char(vk: Vk, scancode: Scan) -> Option<char> {
    // ① 物理位置で決まる 3 鍵（配列に依らない）
    if let Some((_, c)) = POSITIONAL_TABLE.iter().find(|(s, _)| *s == scancode) {
        return Some(*c);
    }
    // ② 走査符号が取れない経路の予備（JIS の VK として解釈する）
    if scancode == 0 {
        if let Some(c) = jis_oem_char(vk) {
            return Some(c);
        }
    }
    // ③ 全配列共通の 30 鍵
    UNIVERSAL_TABLE
        .iter()
        .find(|(k, _)| *k == vk)
        .map(|(_, c)| *c)
}

/// JIS 配列としての OEM 3 鍵の解釈（走査符号が無いときの予備）。
pub fn jis_oem_char(vk: Vk) -> Option<char> {
    match vk {
        VK_OEM_1 => Some(':'),
        VK_OEM_PLUS => Some(';'),
        VK_OEM_7 => Some('^'),
        _ => None,
    }
}

/// 原器の文字から走査符号を引く（試験・稽古場・説明用の逆写像）。
pub fn scan_of(ch: char) -> Option<Scan> {
    POSITIONAL_TABLE
        .iter()
        .find(|(_, c)| *c == ch)
        .map(|(s, _)| *s)
}

/// 原器の文字から VK を引く（同上。3 鍵は JIS としての VK を返す）。
pub fn vk_of(ch: char) -> Option<Vk> {
    UNIVERSAL_TABLE
        .iter()
        .find(|(_, c)| *c == ch)
        .map(|(k, _)| *k)
        .or(match ch {
            ':' => Some(VK_OEM_1),
            ';' => Some(VK_OEM_PLUS),
            '^' => Some(VK_OEM_7),
            _ => None,
        })
}

/// `GetKeyboardType(0)` の値が日本語鍵盤か。
///
/// 走査符号で引く設計ゆゑ**動作には要らない**が、
/// 「US 鍵盤で原器を打たうとしてゐる」ことを利用者へ知らせるために見る
/// （黙つて違ふ字を出すより、知らせるはうが良い）。
pub fn keyboard_is_japanese(keyboard_type: u32) -> bool {
    keyboard_type == KEYBOARD_TYPE_JAPANESE
}

/// 打鍵が原器の領分か（機能キーは除く）。`OnTestKeyDown` の一次篩に使ふ。
pub fn is_genki_key(vk: Vk, scancode: Scan) -> bool {
    genki_char(vk, scancode).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use yatate_core::genki::{FIRST_PLANE, SECOND_PLANE};
    use yatate_core::session::{KeyAction, Session};

    /// 原器の文字を「実機が寄越す形」（VK と走査符号の対）へ直す試験用の写し。
    fn as_keystroke(ch: char) -> (Vk, Scan) {
        (vk_of(ch).unwrap_or(0), scan_of(ch).unwrap_or(0))
    }

    #[test]
    fn 第一面の三十鍵がすべて写せる() {
        for (key, kana) in FIRST_PLANE.iter() {
            let (vk, sc) = as_keystroke(*key);
            assert!(vk != 0 || sc != 0, "原器の鍵 '{key}'（{kana}）に写像が無い");
            assert_eq!(
                genki_char(vk, sc),
                Some(*key),
                "'{key}'（{kana}）の往復が壊れてゐる"
            );
        }
    }

    #[test]
    fn 第二面も同じ鍵を使ふゆゑ表は一つでよい() {
        // 第二面は前置シフト後に同じ鍵を打つだけ。写像に穴が無いことを確かめる。
        for (key, kana) in SECOND_PLANE.iter() {
            let (vk, sc) = as_keystroke(*key);
            assert_eq!(genki_char(vk, sc), Some(*key), "第二面 '{key}'（{kana}）");
        }
    }

    /// **この試験が本 module の存在理由である。**
    /// 配列で意味の入れ替はる 3 鍵を、物理位置で名指しして縛る。
    #[test]
    fn 配列で動く三鍵は物理位置で決まる() {
        assert_eq!(genki_char(0, SC_COLON), Some(':'), "sc 0x28 は さ");
        assert_eq!(genki_char(0, SC_SEMICOLON), Some(';'), "sc 0x27 は し");
        assert_eq!(genki_char(0, SC_CARET), Some('^'), "sc 0x0D は 前置シフト");
    }

    /// US 配列の VK が来ても、**走査符号が正しければ原器は壊れない**。
    /// （US 機で `;` を打つと VK_OEM_1=0xBA が来る。VK だけで引くと「さ」に化ける）
    #[test]
    fn us配列のvkが来ても位置で正しく引ける() {
        // US 機の「;」: VK は 0xBA（JIS では「:」の VK）だが位置は sc 0x27
        assert_eq!(
            genki_char(VK_OEM_1, SC_SEMICOLON),
            Some(';'),
            "位置が優先されるので「し」のまま"
        );
        // US 機の「'」: VK は 0xDE（JIS では「^」の VK）だが位置は sc 0x28
        assert_eq!(
            genki_char(VK_OEM_7, SC_COLON),
            Some(':'),
            "位置が優先されるので「さ」のまま"
        );
    }

    #[test]
    fn 走査符号が取れなければ_jis_の_vk_として読む() {
        assert_eq!(genki_char(VK_OEM_1, 0), Some(':'));
        assert_eq!(genki_char(VK_OEM_PLUS, 0), Some(';'));
        assert_eq!(genki_char(VK_OEM_7, 0), Some('^'));
    }

    /// **核の地図に従ふ。**
    ///
    /// web の殻（`web/`）も同じ「物理位置 → 原器」の地図を要るので、
    /// 出所を核（`yatate_core::kagi`）へ一本化した。地図を二枚持てば
    /// 放つておいて必ずずれる——このリポジトリが繰り返し学んできたことである。
    /// ここは Windows 側の写しが核から外れてゐないかを見る番人で、
    /// web 側は核の表をそのまま使ふので写し自体を持たない。
    #[test]
    fn 核の鍵の地図と食ひ違はない() {
        use yatate_core::kagi;

        // ① 配列で意味の変はる三鍵が、核と同じ物理位置を指してゐること。
        //    ここがずれると実機で黙つて違ふ字が出る（例外も警告も出ない）。
        for c in [':', ';', '^'] {
            assert_eq!(
                scan_of(c),
                kagi::scan_of(c),
                "'{c}' の走査符号が核と食ひ違ふ"
            );
        }

        // ② 鍵の数が合ふこと（片方にだけ鍵が増えるのを捕まへる）
        assert_eq!(
            kagi::KAGI.len(),
            UNIVERSAL_TABLE.len() + POSITIONAL_TABLE.len(),
            "核と Windows で原器の鍵数が違ふ"
        );

        // ③ 核が知る 33 鍵すべてが、Windows の殻でも同じ文字へ写ること
        for k in kagi::KAGI.iter() {
            let (vk, sc) = as_keystroke(k.genki);
            assert_eq!(
                genki_char(vk, sc),
                Some(k.genki),
                "核の {}（'{}'）が Windows の殻で引けない",
                k.code,
                k.genki
            );
        }
    }

    #[test]
    fn 機能キーは原器の領分でない() {
        for vk in [
            VK_BACK, VK_RETURN, VK_ESCAPE, VK_SPACE, VK_SHIFT, VK_CONTROL, VK_MENU, VK_LEFT,
            VK_UP, VK_RIGHT, VK_DOWN,
        ] {
            assert!(
                !is_genki_key(vk, 0),
                "VK {vk:#04X} を原器が食つてはいけない"
            );
        }
    }

    /// 矢印は変換の段でだけ意味を持つ。**走査符号が来ても**原器の鍵と衝突しないこと。
    ///
    /// 矢印は拡張キー（`E0` 前置）で走査符号は 0x4B・0x48・0x4D・0x50 であり、
    /// 原器が位置で引く三鍵（0x28・0x27・0x0D）とは重ならない。
    /// ここが重なると「文節を移さうとしたら さ が入る」といふ事故になる。
    #[test]
    fn 矢印の走査符号は原器の三鍵と重ならない() {
        for (vk, sc) in [
            (VK_LEFT, 0x4B),
            (VK_UP, 0x48),
            (VK_RIGHT, 0x4D),
            (VK_DOWN, 0x50),
        ] {
            assert!(
                !is_genki_key(vk, sc),
                "矢印 VK {vk:#04X} / sc {sc:#04X} を原器が食つてはいけない"
            );
        }
        for (sc, _) in POSITIONAL_TABLE.iter() {
            assert!(
                ![0x4B_u16, 0x48, 0x4D, 0x50].contains(sc),
                "原器の位置 {sc:#04X} が矢印と衝突してゐる"
            );
        }
    }

    #[test]
    fn 表に重複が無い() {
        let mut vks: Vec<Vk> = UNIVERSAL_TABLE.iter().map(|(k, _)| *k).collect();
        vks.sort_unstable();
        let n = vks.len();
        vks.dedup();
        assert_eq!(vks.len(), n, "VK が重複してゐる（同じ鍵に二つの意味）");

        let mut chars: Vec<char> = UNIVERSAL_TABLE
            .iter()
            .map(|(_, c)| *c)
            .chain(POSITIONAL_TABLE.iter().map(|(_, c)| *c))
            .collect();
        chars.sort_unstable();
        let n = chars.len();
        chars.dedup();
        assert_eq!(chars.len(), n, "原器の文字が重複してゐる");
    }

    #[test]
    fn 三十三鍵ちやうどを覆ふ() {
        // 清音 30 ＋ 前置シフト ＋ 濁点 ＋ 半濁点 ＝ 33
        assert_eq!(UNIVERSAL_TABLE.len() + POSITIONAL_TABLE.len(), 33);
    }

    #[test]
    fn 鍵盤種別の判定() {
        assert!(keyboard_is_japanese(KEYBOARD_TYPE_JAPANESE));
        assert!(!keyboard_is_japanese(KEYBOARD_TYPE_ENHANCED));
    }

    /// 打鍵列が「実機の形」（VK＋走査符号）から仮名になるところまで通す。
    #[test]
    fn 実機の形の打鍵列から文が打てる() {
        // 「けふはよきてんきなり」
        //  け(u) ふ(d) は(g) よ(^y) き(o) て(2) ん(^^) き(o) な(t) り(^4)
        let mut s = Session::new();
        for ch in "udg^yo2^^ot^4".chars() {
            let (vk, sc) = as_keystroke(ch);
            let decoded = genki_char(vk, sc).expect("原器の鍵");
            s.key(decoded);
        }
        assert_eq!(s.preedit(), "けふはよきてんきなり");

        match s.commit() {
            KeyAction::Commit(text) => assert_eq!(text, "けふはよきてんきなり"),
            other => panic!("確定にならない: {other:?}"),
        }
    }

    #[test]
    fn 濁点つきの語も実機の形から打てる() {
        let mut s = Session::new();
        for ch in "pbkb".chars() {
            let (vk, sc) = as_keystroke(ch);
            s.key(genki_char(vk, sc).expect("原器の鍵"));
        }
        assert_eq!(s.preedit(), "がぜ", "か＋濁＝が、せ＋濁＝ぜ");
    }

    /// 確定すると旧字が機械で定まる（核の仕事だが、殻から見て効いてゐるか）。
    #[test]
    fn 確定で旧字が定まる() {
        let mut s = Session::new();
        s.key('t'); // な
                    // 直接ぶち込むのではなく composer 経由で確かめたいので、
                    // 旧字の効きは核の試験に任せ、ここでは経路が繋がつてゐることだけ見る。
        match s.commit() {
            KeyAction::Commit(t) => assert_eq!(t, "な"),
            other => panic!("{other:?}"),
        }
    }
}
