//! 変換セッション — **OS を知らない**入力の状態機械。
//!
//! TSF の殻（[`crate::tip`]）はここへ打鍵を渡し、返つてきた指示のとほりに
//! 未確定文字列（composition）を描き、確定時に文字列を挿し込むだけでよい。
//! COM を知らないので **Linux でも試験できる**——実際 CI はここを ubuntu で回す
//! （`docs/ime/cross-platform.md` §7）。
//!
//! 配列も氣配も旧字確定も自前では持たない。すべて核（`yatate-core`）から来る。

use yatate_core::composer::Composer;
use yatate_core::genki::{Edit, Genki, DAKUTEN, HANDAKUTEN, SHIFT};
use yatate_core::kehai::{self, ActionField};

/// 一打に対する殻への指示。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum KeyAction {
    /// 未確定文字列が変はつた（殻は composition を描き直す）。
    Update,
    /// 何も起きなかつた（前置シフトが立つた等）。殻は打鍵を**呑む**。
    Swallow,
    /// 矢立の鍵ではない。殻は OS へ**素通し**する。
    Passthrough,
    /// 確定した文字列。殻はこれを挿し込み、composition を閉ぢる。
    Commit(String),
}

/// 未確定文字列を抱へる入力セッション。TIP のスレッドごとに 1 つ持つ。
#[derive(Default)]
pub struct Session {
    genki: Genki,
    composer: Composer,
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    /// いま未確定の仮名列。
    pub fn preedit(&self) -> &str {
        self.composer.text()
    }

    pub fn is_composing(&self) -> bool {
        !self.composer.is_empty()
    }

    /// 前置シフトが立つてゐるか（殻が面の表示を替へる手掛かり）。
    pub fn is_shifted(&self) -> bool {
        self.genki.is_shifted()
    }

    /// 墨の氣配 — 次の一打の分布。候補窓・鍵盤表示に使ふ（描くのは殻）。
    pub fn field(&self) -> ActionField {
        kehai::field(self.preedit().chars().last())
    }

    /// 原器の一打を食はせる。
    pub fn key(&mut self, ch: char) -> KeyAction {
        let last = self.preedit().chars().last();
        match self.genki.press(ch, last) {
            Edit::Insert(kana) => {
                self.composer.append(kana);
                KeyAction::Update
            }
            Edit::ReplaceLast(c) => {
                self.composer.delete_last();
                self.composer.append(&c.to_string());
                KeyAction::Update
            }
            // 前置シフトを立てた打鍵は**呑む**（OS へ流すと `^` が入力されてしまふ）
            Edit::None => KeyAction::Swallow,
            Edit::Unmapped => {
                // 矢立の鍵でない。未確定文字列を抱へてゐる間は取り零さぬやう呑み、
                // 何も無ければ素通しして OS 本来の働きを妨げない。
                if self.is_composing() {
                    KeyAction::Swallow
                } else {
                    KeyAction::Passthrough
                }
            }
        }
    }

    /// この鍵を矢立が受け取るべきか（TSF の `OnTestKeyDown` が先に尋ねてくる）。
    ///
    /// 未確定文字列がある間は、原器に無い鍵も**一旦は受ける**
    /// （確定・取り消しの機会を殻に残すため）。
    pub fn wants_key(&self, ch: char) -> bool {
        self.is_composing()
            || self.genki.is_shifted()
            || ch == SHIFT
            || ch == DAKUTEN
            || ch == HANDAKUTEN
            || yatate_core::genki::FIRST_PLANE
                .iter()
                .any(|(k, _)| *k == ch)
    }

    /// 一字消す。未確定文字列が空になつたら `false`（殻は composition を閉ぢる）。
    pub fn backspace(&mut self) -> bool {
        self.composer.delete_last();
        self.is_composing()
    }

    /// 確定 — **旧字体は核が機械で確定させる**。
    pub fn commit(&mut self) -> KeyAction {
        self.genki.reset();
        KeyAction::Commit(self.composer.commit())
    }

    /// 取り消し（Esc）。未確定文字列を捨てる。
    pub fn cancel(&mut self) {
        self.genki.reset();
        self.composer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(keys: &str) -> Session {
        let mut s = Session::new();
        for c in keys.chars() {
            s.key(c);
        }
        s
    }

    #[test]
    fn 原器で打つた仮名が未確定文字列に積まれる() {
        let s = typed("udg^yo2^^ot^4");
        assert_eq!(s.preedit(), "けふはよきてんきなり");
        assert!(s.is_composing());
    }

    #[test]
    fn 前置シフトの打鍵は呑む() {
        let mut s = Session::new();
        assert_eq!(s.key('^'), KeyAction::Swallow);
        assert!(s.is_shifted());
        assert_eq!(s.key('r'), KeyAction::Update); // ゐ
        assert_eq!(s.preedit(), "ゐ");
    }

    #[test]
    fn 濁点は直前の一字を差し替へる() {
        let s = typed("pb");
        assert_eq!(s.preedit(), "が");
    }

    #[test]
    fn 未確定が無いとき原器外の鍵は素通しする() {
        let mut s = Session::new();
        assert_eq!(s.key('z'), KeyAction::Passthrough);
        assert!(!s.wants_key('z'));
    }

    #[test]
    fn 未確定を抱へてゐる間は原器外の鍵も受ける() {
        let mut s = typed("0");
        assert!(s.wants_key('z'));
        assert_eq!(s.key('z'), KeyAction::Swallow);
        assert_eq!(s.preedit(), "あ", "呑んでも文字は壊さない");
    }

    #[test]
    fn 確定で旧字が定まり未確定が空く() {
        let mut s = typed("0p");
        assert_eq!(s.commit(), KeyAction::Commit("あか".to_string()));
        assert!(!s.is_composing());
    }

    #[test]
    fn 取り消しで未確定が消える() {
        let mut s = typed("0p9");
        s.cancel();
        assert_eq!(s.preedit(), "");
        assert!(!s.is_shifted());
    }

    #[test]
    fn 一字消しは空で止まる() {
        let mut s = typed("0p");
        assert!(s.backspace());
        assert!(!s.backspace());
        assert_eq!(s.preedit(), "");
    }

    #[test]
    fn 氣配は未確定の末尾から立つ() {
        let s = typed("t"); // な
        let f = s.field();
        assert!(!f.is_empty(), "「な」の後には次の一打の分布がある");
    }
}
