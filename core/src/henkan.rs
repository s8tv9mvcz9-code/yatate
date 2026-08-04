//! 変換の状態機械 — 仮名を積む段と、文節を選び直す段。**どの OS でも同じ**。
//!
//! [`Session`] が「打鍵 → 仮名」を、[`bunsetsu`] が「仮名 → 文節と候補」を持つてゐたが、
//! **その二つを繋ぐ段の管理**はどこにも無く、web の殻が JS で持つてゐた。
//! 同じ物を Windows・macOS・iOS の殻へ書けば四枚目まで増える——
//! このリポジトリが繰り返し学んできた「地図を二枚持つと必ずずれる」の型である。
//! ゆゑに核へ置く。
//!
//! ```text
//!   Kana 段                          Henkan 段
//!   ───────                          ─────────
//!   原器の打鍵 → 仮名が積まれる       Space   → 次の候補へ
//!   Space  → 変換して Henkan へ ───→  ←/→    → 注目する文節を移す
//!   Enter  → 仮名のまま確定           Shift+←/→ → 注目文節を縮める/伸ばす
//!   Esc    → 捨てる                   Enter   → 選んだ形で確定（旧字が定まる）
//!                                    Esc/BS  → 変換を解いて Kana 段へ戻る
//!                          ←───────  原器の鍵 → 確定して、続けて新しい未確定を立てる
//! ```
//!
//! ## 旧字はどこで定まるか
//!
//! **確定の瞬間だけ**である（[`bunsetsu::commit`] ＝ [`crate::kyuji::to_kyuji`]）。
//! 辞書は新字体で持つてゐるので、候補を並べてゐる間の表記も新字体のままでよい。
//! 同じ知識を二か所に置かないためである。
//!
//! ## 読みの完全被覆を破らない
//!
//! 変換は [`bunsetsu::coverage_error`] が無いときにだけ成立させる。
//! 文節の読みを繋げたものが打つた仮名と一致しないなら、それは
//! 「打つてゐない字が画面に出る」といふことなので、**変換しない方がまし**である。

use crate::bunsetsu::{self, Bunsetsu, Cand};
use crate::kehai::ActionField;
use crate::session::{KeyAction, Session};

/// いまどの段に居るか。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Phase {
    /// 仮名を積んでゐる（未変換）。
    #[default]
    Kana,
    /// 変換して文節を選び直してゐる。
    Henkan,
}

/// 殻への指示。[`crate::session::KeyAction`] を変換の段まで広げたもの。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Act {
    /// 未確定文字列が変はつた（殻は描き直す）。
    Update,
    /// 食つたが見た目は変はらない。
    Swallow,
    /// 矢立の鍵ではない。殻は OS へ素通しする。
    Passthrough,
    /// 確定した文字列。殻はこれを挿し込み、未確定を閉ぢる。
    Commit(String),
    /// 確定した上で、**続けて新しい未確定が立つてゐる**
    /// （変換中に原器の鍵が来たとき）。殻は挿し込んでから
    /// [`Henkan::preedit`] を描き直す。
    CommitThenUpdate(String),
    /// 捨てて閉ぢる。
    Cancel,
}

/// 仮名と変換を通した入力の状態機械。TIP のスレッド／入力欄ごとに 1 つ持つ。
#[derive(Default)]
pub struct Henkan {
    /// 打鍵 → 仮名。変換中も**読みを保つたまま**残す（Esc で戻れるやうに）。
    session: Session,
    /// 変換中の文節列。Kana 段では空。
    segs: Vec<Bunsetsu>,
    /// 注目してゐる文節の番号。
    focus: usize,
    phase: Phase,
}

impl Henkan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// 未確定文字列を抱へてゐるか。
    pub fn is_composing(&self) -> bool {
        match self.phase {
            Phase::Kana => self.session.is_composing(),
            Phase::Henkan => !self.segs.is_empty(),
        }
    }

    /// 前置シフトが立つてゐるか（殻が面の表示を替へる手掛かり）。
    pub fn is_shifted(&self) -> bool {
        self.session.is_shifted()
    }

    /// 墨の氣配 — 次の一打の分布。変換中は立たない。
    pub fn field(&self) -> ActionField {
        self.session.field()
    }

    /// 画面に出す未確定文字列。Kana 段では仮名、Henkan 段では**新字体の表記**。
    pub fn preedit(&self) -> String {
        match self.phase {
            Phase::Kana => self.session.preedit().to_string(),
            Phase::Henkan => bunsetsu::compose(&self.segs),
        }
    }

    /// いま抱へてゐる読み（段に依らず仮名）。
    pub fn yomi(&self) -> &str {
        self.session.preedit()
    }

    /// 変換中の文節（Kana 段では空）。
    pub fn segments(&self) -> &[Bunsetsu] {
        &self.segs
    }

    pub fn focus(&self) -> usize {
        self.focus
    }

    /// 注目してゐる文節の候補（Kana 段では空）。候補窓を描く殻が使ふ。
    pub fn candidates(&self) -> &[Cand] {
        match self.segs.get(self.focus) {
            Some(b) => &b.candidates,
            None => &[],
        }
    }

    /// 注目してゐる文節で選ばれてゐる候補の番号。
    pub fn chosen(&self) -> usize {
        self.segs.get(self.focus).map(|b| b.chosen).unwrap_or(0)
    }

    /// 注目文節が未確定文字列の何文字目から何文字か。
    /// **下線を引き分ける**ために殻が使ふ（TSF の表示属性・IMKit の marked text）。
    pub fn focus_range(&self) -> (usize, usize) {
        let upto = self.focus.min(self.segs.len());
        let start: usize = self.segs[..upto]
            .iter()
            .map(|b| b.surface().chars().count())
            .sum();
        let len = self
            .segs
            .get(self.focus)
            .map(|b| b.surface().chars().count())
            .unwrap_or(0);
        (start, len)
    }

    /// この鍵を矢立が受け取るべきか（`OnTestKeyDown` 等が先に尋ねてくる）。
    ///
    /// **副作用禁止**の問合せなので、状態は一切触らない。
    pub fn wants_key(&self, ch: char) -> bool {
        self.is_composing() || self.session.wants_key(ch)
    }

    /// 原器の一打。
    pub fn key(&mut self, ch: char) -> Act {
        if self.phase == Phase::Henkan {
            // 変換中に原器の鍵が来た。世間の IME と同じく**いま選んでゐる形で確定**し、
            // その打鍵から新しい未確定を立てる（打つた字を捨てない）。
            let text = self.take_commit();
            return match self.session.key(ch) {
                KeyAction::Update => Act::CommitThenUpdate(text),
                // 前置シフトが立つただけ等。確定だけ済ませる
                _ => Act::Commit(text),
            };
        }
        match self.session.key(ch) {
            KeyAction::Update => Act::Update,
            KeyAction::Swallow => Act::Swallow,
            KeyAction::Passthrough => Act::Passthrough,
            KeyAction::Commit(t) => Act::Commit(t),
        }
    }

    /// 仮名を**直に**積む（硝子の鍵盤用。二行配列は原器の写像を経ない）。
    ///
    /// 物理鍵盤の [`Self::key`] と同じく、変換中なら先に確定させてから積む。
    pub fn insert_kana(&mut self, kana: &str) -> Act {
        if kana.is_empty() {
            return Act::Swallow;
        }
        if self.phase == Phase::Henkan {
            let text = self.take_commit();
            self.session.insert_kana(kana);
            return Act::CommitThenUpdate(text);
        }
        match self.session.insert_kana(kana) {
            KeyAction::Update => Act::Update,
            _ => Act::Swallow,
        }
    }

    /// Space —— 未変換なら**変換し**、変換中なら**次の候補へ**。
    pub fn convert(&mut self) -> Act {
        match self.phase {
            Phase::Kana => {
                let yomi = self.session.preedit().to_string();
                if yomi.is_empty() {
                    return Act::Swallow;
                }
                let segs = bunsetsu::segment(&yomi);
                if segs.is_empty() {
                    return Act::Swallow;
                }
                // **打つてゐない字を画面に出さない。** 読みが被覆できないなら変換しない。
                if bunsetsu::coverage_error(&yomi, &segs).is_some() {
                    return Act::Swallow;
                }
                self.segs = segs;
                self.focus = 0;
                self.phase = Phase::Henkan;
                Act::Update
            }
            Phase::Henkan => self.next_candidate(),
        }
    }

    /// 次の候補へ（一巡する）。
    pub fn next_candidate(&mut self) -> Act {
        self.step_candidate(1)
    }

    /// 前の候補へ（一巡する）。
    pub fn prev_candidate(&mut self) -> Act {
        self.step_candidate(-1)
    }

    fn step_candidate(&mut self, d: isize) -> Act {
        let Some(seg) = self.segs.get_mut(self.focus) else {
            return Act::Swallow;
        };
        let n = seg.candidates.len();
        if n <= 1 {
            return Act::Swallow;
        }
        let next = (seg.chosen as isize + d).rem_euclid(n as isize) as usize;
        seg.choose(next);
        Act::Update
    }

    /// 候補を番号で選ぶ（候補窓を持つ殻が使ふ）。
    pub fn choose(&mut self, index: usize) -> Act {
        // パターンガードの中では可変借用が取れないので、素直に分岐する。
        match self.segs.get_mut(self.focus) {
            Some(seg) if seg.candidates.len() > index => {
                seg.choose(index);
                Act::Update
            }
            _ => Act::Swallow,
        }
    }

    /// 注目する文節を右へ。
    pub fn focus_next(&mut self) -> Act {
        if self.phase != Phase::Henkan || self.focus + 1 >= self.segs.len() {
            return Act::Swallow;
        }
        self.focus += 1;
        Act::Update
    }

    /// 注目する文節を左へ。
    pub fn focus_prev(&mut self) -> Act {
        if self.phase != Phase::Henkan || self.focus == 0 {
            return Act::Swallow;
        }
        self.focus -= 1;
        Act::Update
    }

    /// 注目文節を一字**伸ばす**（次の文節から一字貰ふ）。Shift+→。
    pub fn grow_focus(&mut self) -> Act {
        if self.phase != Phase::Henkan {
            return Act::Swallow;
        }
        let Some(seg) = self.segs.get(self.focus) else {
            return Act::Swallow;
        };
        let want = seg.yomi.chars().count() + 1;
        // 次と繋げてから望みの長さで割り直す。割れないなら繋がつたまま（＝最後まで伸びた）。
        if !bunsetsu::merge(&mut self.segs, self.focus) {
            return Act::Swallow;
        }
        bunsetsu::split(&mut self.segs, self.focus, want);
        Act::Update
    }

    /// 注目文節を一字**縮める**（溢れた一字は次の文節へ返す）。Shift+←。
    pub fn shrink_focus(&mut self) -> Act {
        if self.phase != Phase::Henkan {
            return Act::Swallow;
        }
        let Some(seg) = self.segs.get(self.focus) else {
            return Act::Swallow;
        };
        let want = seg.yomi.chars().count().saturating_sub(1);
        if want == 0 {
            return Act::Swallow; // 空の文節は作らない
        }
        let had_next = self.focus + 1 < self.segs.len();
        if !bunsetsu::split(&mut self.segs, self.focus, want) {
            return Act::Swallow;
        }
        if had_next {
            // 溢れた一字を、もともと次に在つた文節へ返す
            bunsetsu::merge(&mut self.segs, self.focus + 1);
        }
        Act::Update
    }

    /// 一字消す。**変換中は消さず、変換を解いて仮名へ戻す**
    /// ——選び直しの途中で一字失ふ事故を作らないため。
    pub fn backspace(&mut self) -> Act {
        match self.phase {
            Phase::Kana => {
                if self.session.backspace() {
                    Act::Update
                } else {
                    Act::Cancel
                }
            }
            Phase::Henkan => self.unconvert(),
        }
    }

    /// 変換を解いて仮名の段へ戻す。読みは [`Session`] が保つてゐるので消えない。
    pub fn unconvert(&mut self) -> Act {
        if self.phase != Phase::Henkan {
            return Act::Swallow;
        }
        self.segs.clear();
        self.focus = 0;
        self.phase = Phase::Kana;
        Act::Update
    }

    /// Esc —— 変換中なら**一段戻す**、仮名の段なら捨てる。
    pub fn cancel(&mut self) -> Act {
        if self.phase == Phase::Henkan {
            return self.unconvert();
        }
        self.session.cancel();
        Act::Cancel
    }

    /// Enter —— 確定。**旧字体はここで機械が定める。**
    pub fn commit(&mut self) -> Act {
        match self.phase {
            Phase::Kana => match self.session.commit() {
                KeyAction::Commit(t) => Act::Commit(t),
                _ => Act::Cancel,
            },
            Phase::Henkan => Act::Commit(self.take_commit()),
        }
    }

    /// 入力欄が替はつた等で全部捨てる。
    pub fn reset(&mut self) {
        self.segs.clear();
        self.focus = 0;
        self.phase = Phase::Kana;
        self.session.cancel();
    }

    /// 変換中の文節を確定文へ畳み、Kana 段へ戻す。
    fn take_commit(&mut self) -> String {
        let text = bunsetsu::commit(&self.segs);
        self.segs.clear();
        self.focus = 0;
        self.phase = Phase::Kana;
        self.session.cancel();
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 原器で打つ（`docs/ime/layout.md` §1 の打鍵列）。
    fn typed(keys: &str) -> Henkan {
        let mut h = Henkan::new();
        for c in keys.chars() {
            h.key(c);
        }
        h
    }

    /// けふはよきてんきなり
    const HARE: &str = "udg^yo2^^ot^4";

    #[test]
    fn 仮名の段では従来どほり仮名が積まれる() {
        let h = typed(HARE);
        assert_eq!(h.phase(), Phase::Kana);
        assert_eq!(h.preedit(), "けふはよきてんきなり");
        assert!(h.is_composing());
        assert!(h.candidates().is_empty(), "未変換に候補は無い");
    }

    #[test]
    fn 空のときの変換は何も起こさない() {
        let mut h = Henkan::new();
        assert_eq!(h.convert(), Act::Swallow);
        assert_eq!(h.phase(), Phase::Kana);
    }

    #[test]
    fn 変換すると文節に分かれる() {
        let mut h = typed(HARE);
        assert_eq!(h.convert(), Act::Update);
        assert_eq!(h.phase(), Phase::Henkan);
        assert_eq!(
            bunsetsu::yomi_list(h.segments()),
            ["けふは", "よき", "てんきなり"]
        );
        assert_eq!(h.focus(), 0, "注目は先頭から始まる");
    }

    /// **この module の出口条件**（roadmap M2 の決定的な側）。
    #[test]
    fn 確定で漢字と旧字が定まる() {
        let mut h = typed(HARE);
        h.convert();
        assert_eq!(h.commit(), Act::Commit("今日は良き天氣なり".to_string()));
        assert!(!h.is_composing(), "確定したら未確定は空く");
        assert_eq!(h.phase(), Phase::Kana);
    }

    #[test]
    fn 候補を並べてゐる間は新字体のまま() {
        let mut h = typed(HARE);
        h.convert();
        assert!(
            !h.preedit().contains('氣'),
            "旧字は確定の瞬間だけ。辞書は新字体で持つ"
        );
    }

    #[test]
    fn 変換中の空白は次の候補へ送る() {
        let mut h = typed(HARE);
        h.convert();
        let first = h.preedit();
        assert!(h.candidates().len() > 1, "「けふは」には控へがある");
        assert_eq!(h.convert(), Act::Update);
        assert_ne!(h.preedit(), first, "候補が送られてゐる");
        assert_eq!(h.chosen(), 1);
    }

    #[test]
    fn 候補は一巡する() {
        let mut h = typed(HARE);
        h.convert();
        let n = h.candidates().len();
        let first = h.preedit();
        for _ in 0..n {
            h.next_candidate();
        }
        assert_eq!(h.preedit(), first, "一巡して戻る");
        h.prev_candidate();
        h.next_candidate();
        assert_eq!(h.preedit(), first, "前後は打ち消し合ふ");
    }

    #[test]
    fn 仮名のままの控へが必ず在る() {
        let mut h = typed(HARE);
        h.convert();
        assert!(
            h.candidates().iter().any(|c| c.surface == "けふは"),
            "全部仮名で書く道が残つてゐること"
        );
    }

    #[test]
    fn 取り消しは一段だけ戻し読みを失はない() {
        let mut h = typed(HARE);
        h.convert();
        assert_eq!(h.cancel(), Act::Update);
        assert_eq!(h.phase(), Phase::Kana);
        assert_eq!(h.preedit(), "けふはよきてんきなり", "読みは残つてゐる");
        // もう一度で本当に捨てる
        assert_eq!(h.cancel(), Act::Cancel);
        assert!(!h.is_composing());
    }

    #[test]
    fn 変換中の一字消しは消さずに解く() {
        let mut h = typed(HARE);
        h.convert();
        assert_eq!(h.backspace(), Act::Update);
        assert_eq!(h.phase(), Phase::Kana);
        assert_eq!(
            h.preedit(),
            "けふはよきてんきなり",
            "選び直しの途中で一字失はない"
        );
    }

    #[test]
    fn 注目文節を移せる() {
        let mut h = typed(HARE);
        h.convert();
        assert_eq!(h.focus_prev(), Act::Swallow, "先頭より左は無い");
        assert_eq!(h.focus_next(), Act::Update);
        assert_eq!(h.focus(), 1);
        h.focus_next();
        assert_eq!(h.focus(), 2);
        assert_eq!(h.focus_next(), Act::Swallow, "末尾より右は無い");
    }

    #[test]
    fn 注目文節の候補だけが替はる() {
        let mut h = typed(HARE);
        h.convert();
        h.focus_next(); // 「よき」
        let head = h.segments()[0].surface().to_string();
        h.next_candidate();
        assert_eq!(h.segments()[0].surface(), head, "先頭の文節は動かない");
    }

    #[test]
    fn 区切りを縮めても読みは失はれない() {
        let mut h = typed(HARE);
        h.convert();
        assert_eq!(h.shrink_focus(), Act::Update);
        assert_eq!(h.segments()[0].yomi.chars().count(), 2, "けふは → けふ");
        assert_eq!(
            bunsetsu::yomi_list(h.segments()).concat(),
            "けふはよきてんきなり",
            "読みの完全被覆は区切り修正でも崩れない"
        );
    }

    #[test]
    fn 区切りを伸ばしても読みは失はれない() {
        let mut h = typed(HARE);
        h.convert();
        assert_eq!(h.grow_focus(), Act::Update);
        assert_eq!(h.segments()[0].yomi.chars().count(), 4, "けふは → けふはよ");
        assert_eq!(
            bunsetsu::yomi_list(h.segments()).concat(),
            "けふはよきてんきなり"
        );
    }

    #[test]
    fn 末尾の文節は伸ばせない() {
        let mut h = typed(HARE);
        h.convert();
        h.focus_next();
        h.focus_next();
        assert_eq!(h.grow_focus(), Act::Swallow, "貰ふ先が無い");
    }

    #[test]
    fn 変換中の打鍵は確定してから次を積む() {
        let mut h = typed(HARE);
        h.convert();
        // 「あ」を打つ
        let act = h.key('0');
        assert_eq!(act, Act::CommitThenUpdate("今日は良き天氣なり".to_string()));
        assert_eq!(h.phase(), Phase::Kana);
        assert_eq!(h.preedit(), "あ", "打つた字は捨てない");
    }

    #[test]
    fn 注目文節の範囲が取れる() {
        let mut h = typed(HARE);
        h.convert();
        let (start, len) = h.focus_range();
        assert_eq!(start, 0);
        assert_eq!(len, h.segments()[0].surface().chars().count());
        h.focus_next();
        let (start2, _) = h.focus_range();
        assert_eq!(start2, h.segments()[0].surface().chars().count());
    }

    #[test]
    fn 未確定が無いとき原器外の鍵は素通しする() {
        let mut h = Henkan::new();
        assert!(!h.wants_key('z'));
        assert_eq!(h.key('z'), Act::Passthrough);
    }

    #[test]
    fn 変換中は原器外の鍵も受ける() {
        let mut h = typed(HARE);
        h.convert();
        assert!(h.wants_key('z'), "確定・取消の機会を殻に残す");
    }

    #[test]
    fn 仮名のまま確定すれば旧字だけが掛かる() {
        let mut h = typed(HARE);
        assert_eq!(h.commit(), Act::Commit("けふはよきてんきなり".to_string()));
    }

    /// 硝子の鍵盤は仮名を直に積む（原器の写像を経ない）。
    #[test]
    fn 硝子から仮名を直に積める() {
        let mut h = Henkan::new();
        for kana in ["け", "ふ", "は", "よ", "き", "て", "ん", "き", "な", "り"] {
            assert_eq!(h.insert_kana(kana), Act::Update);
        }
        assert_eq!(h.preedit(), "けふはよきてんきなり");
        h.convert();
        assert_eq!(h.commit(), Act::Commit("今日は良き天氣なり".to_string()));
    }

    #[test]
    fn 硝子の仮名も変換中なら先に確定させる() {
        let mut h = typed(HARE);
        h.convert();
        let act = h.insert_kana("あ");
        assert_eq!(act, Act::CommitThenUpdate("今日は良き天氣なり".to_string()));
        assert_eq!(h.preedit(), "あ");
    }

    #[test]
    fn 空の仮名は何も起こさない() {
        let mut h = Henkan::new();
        assert_eq!(h.insert_kana(""), Act::Swallow);
        assert!(!h.is_composing());
    }

    /// 硝子には前置シフトが無い。物理鍵盤で立てたシフトを持ち越さない。
    #[test]
    fn 硝子の仮名は前置シフトを降ろす() {
        let mut h = Henkan::new();
        h.key('^');
        assert!(h.is_shifted());
        h.insert_kana("さ");
        assert!(!h.is_shifted(), "硝子へ移つたらシフトは降りる");
        assert_eq!(h.preedit(), "さ");
    }

    #[test]
    fn 片付けで段も文節も戻る() {
        let mut h = typed(HARE);
        h.convert();
        h.reset();
        assert_eq!(h.phase(), Phase::Kana);
        assert!(!h.is_composing());
        assert!(h.segments().is_empty());
    }
}
