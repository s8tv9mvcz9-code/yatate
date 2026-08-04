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
//!
//! ## なぜ核に置くのか
//!
//! 同じ地図を殻ごとに書けば、放つておけば必ずずれる（このリポジトリが繰り返し
//! 学んできたことである）。ここを唯一の出所にして、
//!
//! - **web の殻はこの表をそのまま使ふ**（写しを持たない＝ずれやうが無い）
//! - **Windows の殻は自分の表をここに照合する**（VK は Windows 固有なので殻に残る）
//!
//! 表そのものは走査符号も `code` も**ただの整数と文字列**なので、依存は増えない。
//!
//! ## 配列で意味の変はる三鍵
//!
//! ```text
//!        位置        JIS の刻印   US の刻印    ブラウザの code
//!   さ   0x28        :            '            Quote
//!   し   0x27        ;            ;            Semicolon      ← 刻印が同じ
//!   ^    0x0D        ^            =            Equal
//! ```
//!
//! 「し」が最も危い。**刻印も物理位置も同じなのに意味だけが違ふ**ので、
//! `KeyboardEvent.key`（出る文字）で引くと、US 配列の機で黙つて「さ」になる。
//! 例外も警告も出ない。位置で引けばこの誤爆はそもそも生まれない
//! ——Windows の殻が `ToUnicodeEx` を使はないのと**同じ一手**である。

use crate::genki::{DAKUTEN, HANDAKUTEN, SHIFT};

/// 走査符号（set 1）。Windows は `lParam` の 16〜23 bit で受け取る。
pub type Scan = u16;

/// 一つの鍵 — 原器の文字と、その物理位置の二つの呼び名。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Kagi {
    /// 原器の文字（`core/src/genki.rs` の鍵）。
    pub genki: char,
    /// `KeyboardEvent.code`（W3C UI Events）。
    pub code: &'static str,
    /// 走査符号 set 1。
    pub scan: Scan,
}

const fn k(genki: char, code: &'static str, scan: Scan) -> Kagi {
    Kagi { genki, code, scan }
}

/// 原器が使ふ 33 鍵。清音 30 ＋ 前置シフト ＋ 濁点 ＋ 半濁点。
///
/// 並びは原器の紙面の順（`genki::FIRST_PLANE`）に合はせてある——
/// 表を眺めたときに配列の形が見えるやうに。
#[rustfmt::skip]
pub static KAGI: [Kagi; 33] = [
    // 右半面・数字列（あ〜お）
    k('0', "Digit0", 0x0B), k('9', "Digit9", 0x0A), k('8', "Digit8", 0x09),
    k('7', "Digit7", 0x08), k('6', "Digit6", 0x07),
    // 右半面・上段（か行）
    k('p', "KeyP", 0x19), k('o', "KeyO", 0x18), k('i', "KeyI", 0x17),
    k('u', "KeyU", 0x16), k('y', "KeyY", 0x15),
    // 右半面・中段（さ行）— 先頭の二鍵が配列で意味の変はる鍵
    k(':', "Quote", 0x28), k(';', "Semicolon", 0x27), k('l', "KeyL", 0x26),
    k('k', "KeyK", 0x25), k('j', "KeyJ", 0x24),
    // 左半面・数字列（た行）
    k('5', "Digit5", 0x06), k('4', "Digit4", 0x05), k('3', "Digit3", 0x04),
    k('2', "Digit2", 0x03), k('1', "Digit1", 0x02),
    // 左半面・上段（な行）
    k('t', "KeyT", 0x14), k('r', "KeyR", 0x13), k('e', "KeyE", 0x12),
    k('w', "KeyW", 0x11), k('q', "KeyQ", 0x10),
    // 左半面・中段（は行）
    k('g', "KeyG", 0x22), k('f', "KeyF", 0x21), k('d', "KeyD", 0x20),
    k('s', "KeyS", 0x1F), k('a', "KeyA", 0x1E),
    // 面と逸らし
    k(SHIFT,      "Equal", 0x0D),
    k(DAKUTEN,    "KeyB",  0x30),
    k(HANDAKUTEN, "KeyV",  0x2F),
];

/// `KeyboardEvent.code` から原器の文字へ。原器に無い鍵は `None`。
///
/// **`key` ではなく `code` で引くこと。** `key` は配列に依つて変はるので、
/// US 配列の機で「し」が黙つて「さ」になる。
pub fn genki_of_code(code: &str) -> Option<char> {
    KAGI.iter().find(|k| k.code == code).map(|k| k.genki)
}

/// 走査符号から原器の文字へ。
pub fn genki_of_scan(scan: Scan) -> Option<char> {
    KAGI.iter().find(|k| k.scan == scan).map(|k| k.genki)
}

/// 原器の文字から `KeyboardEvent.code` へ（稽古場の図・試験用の逆引き）。
pub fn code_of(genki: char) -> Option<&'static str> {
    KAGI.iter().find(|k| k.genki == genki).map(|k| k.code)
}

/// 原器の文字から走査符号へ。
pub fn scan_of(genki: char) -> Option<Scan> {
    KAGI.iter().find(|k| k.genki == genki).map(|k| k.scan)
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
        }
        // 第二面は同じ鍵を前置シフト後に打つだけなので、表は一つで足りる
        for (key, kana) in SECOND_PLANE.iter() {
            assert!(code_of(*key).is_some(), "第二面 '{key}'（{kana}）");
        }
        for c in [SHIFT, DAKUTEN, HANDAKUTEN] {
            assert!(code_of(c).is_some(), "'{c}' に code が無い");
        }
    }

    #[test]
    fn 往復する() {
        for k in KAGI.iter() {
            assert_eq!(genki_of_code(k.code), Some(k.genki), "{} の往復", k.code);
            assert_eq!(genki_of_scan(k.scan), Some(k.genki), "{:#04X}", k.scan);
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
    }
}
