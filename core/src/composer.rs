//! 作業帯の中身 — 確定前の假名バッファ（`Composer.swift` のパリティ実装）。
//!
//! M0 は素朴な文字列。M2 で文節列に育つ（そのとき候補選択と区切り修正が入る）。

use crate::kyuji::to_kyuji;

#[derive(Default, Clone, Debug)]
pub struct Composer {
    text: String,
}

impl Composer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn append(&mut self, kana: &str) {
        self.text.push_str(kana);
    }

    /// 一字消す。**文字単位**で消すこと（UTF-8 のバイトで削ると壊れる）。
    pub fn delete_last(&mut self) {
        self.text.pop();
    }

    /// 捨てる（取り消し）。確定させずに空にする。
    pub fn clear(&mut self) {
        self.text.clear();
    }

    /// 確定 — 旧字体を機械で確定させた文を返し、バッファを空にする。
    pub fn commit(&mut self) -> String {
        let out = to_kyuji(&self.text);
        self.text.clear();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 確定で旧字が定まりバッファが空く() {
        let mut c = Composer::new();
        assert!(c.is_empty());
        for k in ["こ", "く", "の", "が", "く"] {
            c.append(k);
        }
        c.append("国");
        c.delete_last();
        assert_eq!(c.text(), "こくのがく");
        assert_eq!(c.commit(), "こくのがく");
        assert!(c.is_empty());
    }

    #[test]
    fn 確定時に新字体は旧字体へ() {
        let mut c = Composer::new();
        c.append("国の学問");
        assert_eq!(c.commit(), "國の學問");
    }

    #[test]
    fn 空のときに消しても壊れない() {
        let mut c = Composer::new();
        c.delete_last();
        assert_eq!(c.commit(), "");
    }
}
