//! 新字體 → 舊字體の決定的トランスデューサ（`ssot/kyuji.py` のパリティ実装）。
//!
//! 旧字旧仮名変換のうち **新字体→旧字体の字形置換は 1:1 の写像で決定的**に定まる。
//! 語の同定も文脈も要らないので、モデルにも人にも任せず機械に焼く。
//!
//! 字が 1:1 対応であることから **ストリーミング中でもそのまま適用できる**（先読み不要）。
//! 唯一の例外が末尾解説【ポイント】——そこでは「国→國」のやうに新字体を*言及*する
//! ため、置換すると「國→國」となつて意味を失ふ。[`KyujiStream`] は解説マーカーを
//! 検出したら以降を素通しに切り替へ、マーカーがトークン境界に跨いでも拾へるやう
//! **マーカー長 -1 文字までを保留**する。

use crate::generated::kyuji_table::{AMBIGUOUS_SHINJI, KYUJI_PAIRS};

/// 末尾解説の見出し。これ以降は解説なので置換しない。
pub const POINT_MARKER: &str = "【ポイント】";

/// 一字の写像を引く。写像に無い字（新旧同形・曖昧字）は `None`。
#[inline]
pub fn kyuji_of(c: char) -> Option<char> {
    KYUJI_PAIRS
        .binary_search_by_key(&c, |&(k, _)| k)
        .ok()
        .map(|i| KYUJI_PAIRS[i].1)
}

/// 一対多で危険なため写像から除外されてゐる新字体か（弁・予・余・欠・芸）。
#[inline]
pub fn is_ambiguous(c: char) -> bool {
    AMBIGUOUS_SHINJI.binary_search(&c).is_ok()
}

/// 新字体を旧字体へ置換する（1:1・文脈非依存）。
pub fn to_kyuji(text: &str) -> String {
    text.chars().map(|c| kyuji_of(c).unwrap_or(c)).collect()
}

/// 完成テキスト用。【ポイント】以降は解説なので置換しない。
pub fn to_kyuji_body(text: &str) -> String {
    match text.find(POINT_MARKER) {
        None => to_kyuji(text),
        Some(i) => {
            let mut out = to_kyuji(&text[..i]);
            out.push_str(&text[i..]);
            out
        }
    }
}

/// `buf` の末尾が `marker` の真の接頭辞になつてゐる最長の**文字数**。
///
/// ここだけは文字単位で見る必要がある（UTF-8 のバイト境界で切ると
/// 「【ポイ」の途中で分断され、マーカー検出が壊れる）。
fn hold_len(buf: &str, marker: &str) -> usize {
    let m: Vec<char> = marker.chars().collect();
    let b: Vec<char> = buf.chars().collect();
    let max_n = m.len().saturating_sub(1).min(b.len());
    for n in (1..=max_n).rev() {
        if m.starts_with(&b[b.len() - n..]) {
            return n;
        }
    }
    0
}

/// ストリーミング用の状態付きトランスデューサ。
///
/// `feed()` をトークン順に呼び、最後に `flush()` を呼ぶ。
/// 分割位置に依らず [`to_kyuji_body`] と同じ結果になることが不変条件で、
/// 黄金ベクトル（`core/vectors/kyuji.json`）がそれを検査する。
pub struct KyujiStream {
    marker: String,
    buf: String,
    passthrough: bool,
}

impl Default for KyujiStream {
    fn default() -> Self {
        Self::new()
    }
}

impl KyujiStream {
    pub fn new() -> Self {
        Self::with_marker(POINT_MARKER)
    }

    pub fn with_marker(marker: &str) -> Self {
        Self {
            marker: marker.to_string(),
            buf: String::new(),
            passthrough: false,
        }
    }

    /// トークンを 1 つ食はせ、確定した分だけ返す（保留分は内部に残る）。
    pub fn feed(&mut self, chunk: &str) -> String {
        if self.passthrough {
            return chunk.to_string();
        }
        self.buf.push_str(chunk);

        if let Some(i) = self.buf.find(&self.marker) {
            // 解説に到達 → 以降は素通し
            let mut out = to_kyuji(&self.buf[..i]);
            out.push_str(&self.buf[i..]);
            self.buf.clear();
            self.passthrough = true;
            return out;
        }

        // マーカーの途中かもしれない末尾を保留する
        let hold = hold_len(&self.buf, &self.marker);
        let total = self.buf.chars().count();
        let split = self
            .buf
            .char_indices()
            .nth(total - hold)
            .map(|(b, _)| b)
            .unwrap_or(self.buf.len());
        let rest = self.buf[split..].to_string();
        let emit = to_kyuji(&self.buf[..split]);
        self.buf = rest;
        emit
    }

    /// 保留分を吐き出して終はる。**呼び忘れると末尾が落ちる。**
    pub fn flush(&mut self) -> String {
        let rest = std::mem::take(&mut self.buf);
        if self.passthrough {
            rest
        } else {
            to_kyuji(&rest)
        }
    }
}

/// トークン列を旧字体化しながら流し直す（空文字列は落とす）。
pub fn kyuji_stream<I, S>(tokens: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut st = KyujiStream::new();
    let mut out: Vec<String> = Vec::new();
    for tok in tokens {
        let s = st.feed(tok.as_ref());
        if !s.is_empty() {
            out.push(s);
        }
    }
    let tail = st.flush();
    if !tail.is_empty() {
        out.push(tail);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 写像は昇順で二分探索が効く() {
        assert!(KYUJI_PAIRS.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(AMBIGUOUS_SHINJI.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn 曖昧字は写像に混入してゐない() {
        for &c in AMBIGUOUS_SHINJI.iter() {
            assert!(kyuji_of(c).is_none(), "曖昧字 {c} が写像に混入");
            assert_eq!(to_kyuji(&c.to_string()), c.to_string());
        }
    }

    #[test]
    fn 冪等かつ同形不変() {
        assert_eq!(to_kyuji("国の学問"), "國の學問");
        assert_eq!(to_kyuji("國の學問"), "國の學問");
        assert_eq!(to_kyuji("山の花"), "山の花");
        assert_eq!(to_kyuji(""), "");
    }

    #[test]
    fn マーカーが境界に跨いでも検出できる() {
        let mut st = KyujiStream::new();
        let got = st.feed("学【ポイ") + &st.feed("ント】学") + &st.flush();
        assert_eq!(got, "學【ポイント】学");
    }

    #[test]
    fn 接頭辞で終はる文が保留のまま消えない() {
        let mut st = KyujiStream::new();
        let got = st.feed("國なり【ポイ") + &st.flush();
        assert_eq!(got, "國なり【ポイ");
    }
}
