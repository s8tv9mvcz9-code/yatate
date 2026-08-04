//! 鍵の物理位置 — 原器の文字 ↔ 指の置き場。**殻をまたぐ唯一の出所**。
//!
//! 原器は「指の位置の地図」であり、字面ではなく鍵の位置に意味がある
//! （`docs/ime/layout.md` §1「五十音の幾何学」）。ゆゑに殻は**位置で引く**。
//! 位置の呼び名は環境ごとに違ふが、指してゐるものは同じ一つである:
//!
//! | | 呼び名 | 例（さ） |
//! |---|---|---|
//! | Windows（TSF） | 走査符号 set 1 | `0x28` |
//! | ブラウザ | `KeyboardEvent.code` | `"Quote"` |
//! | macOS（IMKit） | Carbon の `kVK_ANSI_*` | `0x27` |
//! | iOS（ハード鍵盤） | `UIKeyboardHIDUsage`（USB HID） | `0x34` |
//!
//! ## なぜ核に置くのか
//!
//! 同じ地図を殻ごとに書けば、放つておけば必ずずれる（このリポジトリが繰り返し
//! 学んできたことである）。ここを唯一の出所にして、
//!
//! - **web の殻はこの表をそのまま使ふ**（写しを持たない＝ずれやうが無い）
//! - **Windows の殻は自分の表をここに照合する**（VK は Windows 固有なので殻に残る）
//! - **macOS・iOS の殻はこの表をそのまま使ふ**（`kVK_*` も HID usage も位置なので写しが要らない）
//!
//! 表そのものは走査符号も `code` も**ただの整数と文字列**なので、依存は増えない。
//!
//! ## 配列で意味の変はる三鍵
//!
//! ```text
//!        位置(win)  code        kVK    HID    JIS の刻印   US の刻印
//!   さ   0x28       Quote       0x27   0x34   :            '
//!   し   0x27       Semicolon   0x29   0x33   ;            ;      ← 刻印が同じ
//!   ^    0x0D       Equal       0x18   0x2E   ^            =
//! ```
//!
//! 「し」が最も危い。**刻印も物理位置も同じなのに意味だけが違ふ**ので、
//! `KeyboardEvent.key`（出る文字）で引くと、US 配列の機で黙つて「さ」になる。
//! 例外も警告も出ない。位置で引けばこの誤爆はそもそも生まれない
//! ——Windows の殻が `ToUnicodeEx` を使はないのと**同じ一手**である。
//!
//! ## 英字配列でも原器は成立する
//!
//! macOS の殻は**英字（US）配列のハード鍵盤**を前提にするが、原器の 33 鍵が要る
//! 物理位置は US にもすべて在る（`'` `;` `=` の三つが JIS の `:` `;` `^` に当たる）。
//! 刻印は違ふが**指の置き場は同じ**なので、原器はそのまま打てる。
//! これは「位置で引く」設計の当然の帰結であり、配列ごとの分岐は要らない。

use crate::genki::{DAKUTEN, HANDAKUTEN, SHIFT};

/// 走査符号（set 1）。Windows は `lParam` の 16〜23 bit で受け取る。
pub type Scan = u16;
/// Carbon の仮想キーコード（`kVK_ANSI_*`）。**位置**であり刻印ではない。
/// macOS は `NSEvent.keyCode` で受け取る。
pub type Mac = u16;
/// USB HID Keyboard/Keypad ページ（0x07）の usage。
/// iOS は `UIKey.keyCode`（`UIKeyboardHIDUsage`）で受け取る。
pub type Hid = u16;

/// 一つの鍵 — 原器の文字と、その物理位置の四つの呼び名。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Kagi {
    /// 原器の文字（`core/src/genki.rs` の鍵）。
    pub genki: char,
    /// `KeyboardEvent.code`（W3C UI Events）。
    pub code: &'static str,
    /// 走査符号 set 1（Windows）。
    pub scan: Scan,
    /// Carbon の `kVK_ANSI_*`（macOS）。
    pub mac: Mac,
    /// USB HID usage（iOS の `UIKeyboardHIDUsage`）。
    pub hid: Hid,
}

const fn k(genki: char, code: &'static str, scan: Scan, mac: Mac, hid: Hid) -> Kagi {
    Kagi {
        genki,
        code,
        scan,
        mac,
        hid,
    }
}

/// 原器が使ふ 33 鍵。清音 30 ＋ 前置シフト ＋ 濁点 ＋ 半濁点。
///
/// 並びは原器の紙面の順（`genki::FIRST_PLANE`）に合はせてある——
/// 表を眺めたときに配列の形が見えるやうに。
#[rustfmt::skip]
pub static KAGI: [Kagi; 33] = [
    // 右半面・数字列（あ〜お）
    k('0', "Digit0", 0x0B, 0x1D, 0x27), k('9', "Digit9", 0x0A, 0x19, 0x26),
    k('8', "Digit8", 0x09, 0x1C, 0x25), k('7', "Digit7", 0x08, 0x1A, 0x24),
    k('6', "Digit6", 0x07, 0x16, 0x23),
    // 右半面・上段（か行）
    k('p', "KeyP", 0x19, 0x23, 0x13), k('o', "KeyO", 0x18, 0x1F, 0x12),
    k('i', "KeyI", 0x17, 0x22, 0x0C), k('u', "KeyU", 0x16, 0x20, 0x18),
    k('y', "KeyY", 0x15, 0x10, 0x1C),
    // 右半面・中段（さ行）— 先頭の二鍵が配列で意味の変はる鍵
    k(':', "Quote",     0x28, 0x27, 0x34),
    k(';', "Semicolon", 0x27, 0x29, 0x33),
    k('l', "KeyL", 0x26, 0x25, 0x0F), k('k', "KeyK", 0x25, 0x28, 0x0E),
    k('j', "KeyJ", 0x24, 0x26, 0x0D),
    // 左半面・数字列（た行）
    k('5', "Digit5", 0x06, 0x17, 0x22), k('4', "Digit4", 0x05, 0x15, 0x21),
    k('3', "Digit3", 0x04, 0x14, 0x20), k('2', "Digit2", 0x03, 0x13, 0x1F),
    k('1', "Digit1", 0x02, 0x12, 0x1E),
    // 左半面・上段（な行）
    k('t', "KeyT", 0x14, 0x11, 0x17), k('r', "KeyR", 0x13, 0x0F, 0x15),
    k('e', "KeyE", 0x12, 0x0E, 0x08), k('w', "KeyW", 0x11, 0x0D, 0x1A),
    k('q', "KeyQ", 0x10, 0x0C, 0x14),
    // 左半面・中段（は行）
    k('g', "KeyG", 0x22, 0x05, 0x0A), k('f', "KeyF", 0x21, 0x03, 0x09),
    k('d', "KeyD", 0x20, 0x02, 0x07), k('s', "KeyS", 0x1F, 0x01, 0x16),
    k('a', "KeyA", 0x1E, 0x00, 0x04),
    // 面と逸らし
    k(SHIFT,      "Equal", 0x0D, 0x18, 0x2E),
    k(DAKUTEN,    "KeyB",  0x30, 0x0B, 0x05),
    k(HANDAKUTEN, "KeyV",  0x2F, 0x09, 0x19),
];

/// `KeyboardEvent.code` から原器の文字へ。原器に無い鍵は `None`。
///
/// **`key` ではなく `code` で引くこと。** `key` は配列に依つて変はるので、
/// US 配列の機で「し」が黙つて「さ」になる。
pub fn genki_of_code(code: &str) -> Option<char> {
    KAGI.iter().find(|k| k.code == code).map(|k| k.genki)
}

/// 走査符号から原器の文字へ（Windows）。
pub fn genki_of_scan(scan: Scan) -> Option<char> {
    KAGI.iter().find(|k| k.scan == scan).map(|k| k.genki)
}

/// `kVK_ANSI_*` から原器の文字へ（macOS）。
pub fn genki_of_mac(mac: Mac) -> Option<char> {
    KAGI.iter().find(|k| k.mac == mac).map(|k| k.genki)
}

/// HID usage から原器の文字へ（iOS のハード鍵盤）。
pub fn genki_of_hid(hid: Hid) -> Option<char> {
    KAGI.iter().find(|k| k.hid == hid).map(|k| k.genki)
}

/// 原器の文字から `KeyboardEvent.code` へ（稽古場の図・試験用の逆引き）。
pub fn code_of(genki: char) -> Option<&'static str> {
    KAGI.iter().find(|k| k.genki == genki).map(|k| k.code)
}

/// 原器の文字から走査符号へ。
pub fn scan_of(genki: char) -> Option<Scan> {
    KAGI.iter().find(|k| k.genki == genki).map(|k| k.scan)
}

/// 原器の文字から `kVK_ANSI_*` へ。
pub fn mac_of(genki: char) -> Option<Mac> {
    KAGI.iter().find(|k| k.genki == genki).map(|k| k.mac)
}

/// 原器の文字から HID usage へ。
pub fn hid_of(genki: char) -> Option<Hid> {
    KAGI.iter().find(|k| k.genki == genki).map(|k| k.hid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genki::{FIRST_PLANE, SECOND_PLANE};

    #[test]
    fn 三十三鍵ちやうどを覆ふ() {
        // 清音 30 ＋ 前置シフト ＋ 濁点 ＋ 半濁点
        assert_eq!(KAGI.len(), 33);
    }

    #[test]
    fn 原器の鍵がすべて位置を持つ() {
        for (key, kana) in FIRST_PLANE.iter() {
            assert!(
                code_of(*key).is_some(),
                "第一面 '{key}'（{kana}）に code が無い"
            );
            assert!(
                scan_of(*key).is_some(),
                "第一面 '{key}'（{kana}）に走査符号が無い"
            );
            assert!(
                mac_of(*key).is_some(),
                "第一面 '{key}'（{kana}）に kVK が無い"
            );
            assert!(
                hid_of(*key).is_some(),
                "第一面 '{key}'（{kana}）に HID が無い"
            );
        }
        // 第二面は同じ鍵を前置シフト後に打つだけなので、表は一つで足りる
        for (key, kana) in SECOND_PLANE.iter() {
            assert!(code_of(*key).is_some(), "第二面 '{key}'（{kana}）");
        }
        for c in [SHIFT, DAKUTEN, HANDAKUTEN] {
            assert!(code_of(c).is_some(), "'{c}' に code が無い");
            assert!(mac_of(c).is_some(), "'{c}' に kVK が無い");
            assert!(hid_of(c).is_some(), "'{c}' に HID が無い");
        }
    }

    #[test]
    fn 往復する() {
        for k in KAGI.iter() {
            assert_eq!(genki_of_code(k.code), Some(k.genki), "{} の往復", k.code);
            assert_eq!(genki_of_scan(k.scan), Some(k.genki), "{:#04X}", k.scan);
            assert_eq!(genki_of_mac(k.mac), Some(k.genki), "kVK {:#04X}", k.mac);
            assert_eq!(genki_of_hid(k.hid), Some(k.genki), "HID {:#04X}", k.hid);
        }
        assert_eq!(genki_of_code("KeyZ"), None, "原器に無い鍵");
        assert_eq!(genki_of_code(""), None);
    }

    #[test]
    fn 重複が無い() {
        // 同じ位置に二つの意味があると、どちらかが黙つて死ぬ
        for (i, a) in KAGI.iter().enumerate() {
            for b in KAGI.iter().skip(i + 1) {
                assert_ne!(a.genki, b.genki, "原器の文字 '{}' が重複", a.genki);
                assert_ne!(a.code, b.code, "code {} が重複", a.code);
                assert_ne!(a.scan, b.scan, "走査符号 {:#04X} が重複", a.scan);
                assert_ne!(a.mac, b.mac, "kVK {:#04X} が重複", a.mac);
                assert_ne!(a.hid, b.hid, "HID usage {:#04X} が重複", a.hid);
            }
        }
    }

    /// **この表の存在理由。** 配列で意味の入れ替はる三鍵を、物理位置で名指しして縛る。
    #[test]
    fn 配列で動く三鍵は位置で決まる() {
        assert_eq!(genki_of_code("Quote"), Some(':'), "Quote は さ");
        assert_eq!(genki_of_code("Semicolon"), Some(';'), "Semicolon は し");
        assert_eq!(genki_of_code("Equal"), Some('^'), "Equal は 前置シフト");

        // Windows 側の走査符号と同じ位置を指してゐること
        assert_eq!(scan_of(':'), Some(0x28));
        assert_eq!(scan_of(';'), Some(0x27));
        assert_eq!(scan_of('^'), Some(0x0D));
    }

    /// **英字配列の実機で三鍵がずれない。**
    ///
    /// macOS は `kVK_ANSI_Quote` / `kVK_ANSI_Semicolon` / `kVK_ANSI_Equal`、
    /// iOS は HID の `'"` / `;:` / `=+`。いづれも US 刻印の名だが、
    /// 指してゐるのは JIS の `:` `;` `^` と**同じ物理位置**である。
    /// ここが狂ふと「し」が黙つて「さ」になる。
    #[test]
    fn 英字配列でも三鍵は同じ指の置き場を指す() {
        // macOS: Carbon の kVK_ANSI_*
        assert_eq!(mac_of(':'), Some(0x27), "kVK_ANSI_Quote");
        assert_eq!(mac_of(';'), Some(0x29), "kVK_ANSI_Semicolon");
        assert_eq!(mac_of('^'), Some(0x18), "kVK_ANSI_Equal");

        // iOS: USB HID usage
        assert_eq!(hid_of(':'), Some(0x34), "HID Quote '\"");
        assert_eq!(hid_of(';'), Some(0x33), "HID Semicolon ;:");
        assert_eq!(hid_of('^'), Some(0x2E), "HID Equal =+");

        // 四つの呼び名が同じ鍵に集まつてゐること
        for c in [':', ';', '^'] {
            let k = KAGI.iter().find(|k| k.genki == c).expect("三鍵は表に在る");
            assert_eq!(genki_of_scan(k.scan), Some(c));
            assert_eq!(genki_of_mac(k.mac), Some(c));
            assert_eq!(genki_of_hid(k.hid), Some(c));
            assert_eq!(genki_of_code(k.code), Some(c));
        }
    }

    /// HID usage の英字は A=0x04 から並ぶ。数字は 1=0x1E で 0 が 0x27 と**最後に来る**
    /// ——原器は数字列を多用するので、ここを取り違へると あ〜お が丸ごとずれる。
    #[test]
    fn hid_の数字列は一から始まり零で終はる() {
        assert_eq!(hid_of('1'), Some(0x1E));
        assert_eq!(hid_of('9'), Some(0x26));
        assert_eq!(hid_of('0'), Some(0x27), "0 は 9 の次であり 1 の前ではない");
        assert_eq!(hid_of('a'), Some(0x04), "英字は A から");
    }

    /// macOS の kVK は英字が刻印順でも位置順でもない散らばつた値である
    /// （`kVK_ANSI_A` = 0x00、`kVK_ANSI_B` = 0x0B）。写し間違ひが起きやすいので、
    /// 原器が実際に使ふ鍵を名指しで縛つておく。
    #[test]
    fn mac_の_kvk_は名指しで縛る() {
        assert_eq!(mac_of('a'), Some(0x00), "kVK_ANSI_A");
        assert_eq!(mac_of('s'), Some(0x01), "kVK_ANSI_S");
        assert_eq!(mac_of('b'), Some(0x0B), "kVK_ANSI_B（濁点）");
        assert_eq!(mac_of('v'), Some(0x09), "kVK_ANSI_V（半濁点）");
        assert_eq!(mac_of('0'), Some(0x1D), "kVK_ANSI_0");
        assert_eq!(mac_of('6'), Some(0x16), "kVK_ANSI_6 は 5 より小さい");
        assert_eq!(mac_of('5'), Some(0x17), "kVK_ANSI_5");
    }

    #[test]
    fn 機能キーは原器の領分でない() {
        for code in [
            "Space",
            "Enter",
            "Backspace",
            "Escape",
            "Tab",
            "ShiftLeft",
            "ControlLeft",
        ] {
            assert_eq!(genki_of_code(code), None, "{code} を原器が食つてはいけない");
        }
        // HID の Enter(0x28)・Space(0x2C)・Esc(0x29)・Tab(0x2B) も食はない
        for hid in [0x28, 0x2C, 0x29, 0x2B] {
            assert_eq!(genki_of_hid(hid), None, "HID {hid:#04X}");
        }
    }
}
