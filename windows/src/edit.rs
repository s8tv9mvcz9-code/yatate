//! 未確定文字列を文書へ置く — TSF の編集セッション。
//!
//! ## なぜ「セッション」が要るのか
//!
//! TSF では**文書へ触れてよい瞬間が決まつてゐる**。アプリ（Word でも Chrome でも）は
//! 自分の都合で文書を書き換へるので、IME が勝手な時に手を出すと壊れる。
//! そこで IME は `ITfContext::RequestEditSession` で順番を願ひ出て、
//! TSF が [`ITfEditSession::DoEditSession`] を呼び返した中でだけ書ける。
//! そのとき渡される `ec`（edit cookie）が「今は触つてよい」といふ証文である。
//!
//! ```text
//!   打鍵 → Session（核）が未確定文字列を決める
//!        → RequestEditSession(…)              ← 願ひ出る
//!        → DoEditSession(ec) が呼ばれる       ← ここでだけ書ける
//!            composition が無ければ起こす
//!            範囲へ文字列を書く
//!            確定なら composition を閉ぢる
//! ```
//!
//! ## 同期を願ふ
//!
//! 打鍵の応答は速くなければならないので `TF_ES_SYNC` を付けて願ふ。
//! アプリによつては同期を断る（`TF_E_SYNCHRONOUS`）ので、そのときは
//! 非同期で願ひ直す（[`request`] がこの二段構へを持つ）。

use std::cell::RefCell;
use std::rc::Rc;

use windows::core::{implement, Interface, Result};
use windows::Win32::UI::TextServices::{
    ITfComposition, ITfCompositionSink, ITfContext, ITfContextComposition, ITfEditSession,
    ITfEditSession_Impl, ITfInsertAtSelection, TF_ES_READWRITE, TF_ES_SYNC, TF_IAS_QUERYONLY,
};

/// いま抱へてゐる未確定文字列（`None` なら composition は立つてゐない）。
/// TIP と編集セッションが共有する。
pub type CompositionCell = Rc<RefCell<Option<ITfComposition>>>;

/// 文書に対して行ふこと。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditAction {
    /// 未確定文字列を書き換へる（無ければ起こす）。
    Update(String),
    /// 確定して composition を閉ぢる。
    Commit(String),
    /// 捨てて閉ぢる（Esc）。
    Cancel,
}

/// 編集セッション本体。TSF が [`ITfEditSession_Impl::DoEditSession`] を呼び返す。
#[implement(ITfEditSession)]
pub struct CompositionEdit {
    context: ITfContext,
    composition: CompositionCell,
    sink: ITfCompositionSink,
    action: EditAction,
}

impl CompositionEdit {
    pub fn new(
        context: ITfContext,
        composition: CompositionCell,
        sink: ITfCompositionSink,
        action: EditAction,
    ) -> Self {
        Self {
            context,
            composition,
            sink,
            action,
        }
    }

    /// composition が立つてゐなければ起こす。
    ///
    /// 起こし方: いま挿入されるであらう位置を `TF_IAS_QUERYONLY`（**書かずに問ふだけ**）で
    /// 得て、その範囲を composition の初期範囲にする。
    fn ensure_composition(&self, ec: u32) -> Result<ITfComposition> {
        if let Some(c) = self.composition.borrow().as_ref() {
            return Ok(c.clone());
        }
        // SAFETY: ec は DoEditSession が寄越した有効な証文。cast は同一オブジェクトの
        // 別インタフェース取得で、失敗すれば Err になる。
        let composition = unsafe {
            let insert: ITfInsertAtSelection = self.context.cast()?;
            let range = insert.InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])?;
            let ctx_comp: ITfContextComposition = self.context.cast()?;
            ctx_comp.StartComposition(ec, &range, &self.sink)?
        };
        *self.composition.borrow_mut() = Some(composition.clone());
        Ok(composition)
    }

    /// composition の範囲へ文字列を書く。
    fn write(&self, ec: u32, composition: &ITfComposition, text: &str) -> Result<()> {
        let wide: Vec<u16> = text.encode_utf16().collect();
        // SAFETY: ec は有効な証文、wide は生存してゐる緩衝。
        unsafe {
            let range = composition.GetRange()?;
            range.SetText(ec, 0, &wide)?;
        }
        Ok(())
    }

    /// composition を閉ぢる（確定・取り消しの両方で通る道）。
    fn close(&self, ec: u32, composition: &ITfComposition) -> Result<()> {
        // SAFETY: 同上。
        unsafe { composition.EndComposition(ec)? };
        *self.composition.borrow_mut() = None;
        Ok(())
    }
}

impl ITfEditSession_Impl for CompositionEdit_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        let me = &self.this;
        match &me.action {
            EditAction::Update(text) => {
                let c = me.ensure_composition(ec)?;
                me.write(ec, &c, text)
            }
            EditAction::Commit(text) => {
                // 空の確定は composition を起こさずに閉ぢるだけ（余計な空文字を書かない）
                let c = match me.composition.borrow().clone() {
                    Some(c) => c,
                    None if text.is_empty() => return Ok(()),
                    None => me.ensure_composition(ec)?,
                };
                me.write(ec, &c, text)?;
                me.close(ec, &c)
            }
            EditAction::Cancel => {
                let c = match me.composition.borrow().clone() {
                    Some(c) => c,
                    None => return Ok(()),
                };
                // 未確定の見え方を消してから閉ぢる（残骸を残さない）
                me.write(ec, &c, "")?;
                me.close(ec, &c)
            }
        }
    }
}

/// 編集セッションを願ひ出る。**まづ同期で、断られたら非同期で。**
///
/// 打鍵の応答は速いほど良いので同期を望むが、アプリ側の事情で断られることがある
/// （`TF_E_SYNCHRONOUS`）。そのときに黙つて諦めると**打つたのに何も出ない**ので、
/// 非同期で願ひ直す。
pub fn request(context: &ITfContext, client_id: u32, session: ITfEditSession) -> Result<()> {
    // SAFETY: context・session ともに有効な COM 参照。
    unsafe {
        match context.RequestEditSession(client_id, &session, TF_ES_SYNC | TF_ES_READWRITE) {
            Ok(hr) if hr.is_ok() => Ok(()),
            // 同期を断られた（または内部で失敗した）ので非同期で願ひ直す
            _ => context
                .RequestEditSession(client_id, &session, TF_ES_READWRITE)
                .map(|_| ()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 指示の種類が取り違へられてゐないか（COM を起こさずに確かめられる部分）。
    #[test]
    fn 指示は三種で_確定と取り消しは別物() {
        let a = EditAction::Update("けふ".into());
        let b = EditAction::Commit("けふ".into());
        assert_ne!(a, b, "書き換へと確定を同一視してはならない");
        assert_ne!(b, EditAction::Cancel);
        assert_eq!(a, EditAction::Update("けふ".into()));
    }

    #[test]
    fn 確定の文字列を保つ() {
        match EditAction::Commit("けふはよきてんきなり".into()) {
            EditAction::Commit(t) => assert_eq!(t, "けふはよきてんきなり"),
            other => panic!("{other:?}"),
        }
    }
}
