//! COM の入口 — TSF が矢立を起こす場所。
//!
//! **ここは薄い。** 打鍵の意味を決めるのは核（`yatate_core::Session`）で、
//! この module は TSF の作法に翻訳するだけである。
//!
//! ## TSF の生き死に
//!
//! ```text
//! TSF が DLL を読む
//!   → DllGetClassObject でクラスファクトリを得る（crate::factory）
//!   → CoCreateInstance で Tip を作る
//!   → Activate(thread_mgr, client_id)   ← ここで鍵盤の目（KeyEventSink）を刺す
//!   → 打鍵ごとに OnTestKeyDown / OnKeyDown が来る
//!   → Deactivate() で目を外す
//! ```
//!
//! ## 打鍵の二段（TSF の契約）
//!
//! | 段 | 問はれてゐること | 矢立の答へ方 | 副作用 |
//! |---|---|---|---|
//! | `OnTestKeyDown` | 「この鍵を食ふか？」 | [`Session::wants_key`] | **禁止**（状態を触らない） |
//! | `OnKeyDown` | 「では処理せよ」 | [`Session::key`] ＋ 文書の書き換へ | 可 |
//!
//! `OnTestKeyDown` で状態を変へると、TSF が同じ鍵を二度問うたときに食ひ違ふ。
//! ここを取り違へた IME は「たまに一文字余分に入る」といふ再現し難い病を持つ。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::core::{implement, IUnknownImpl, Interface, Ref, Result, BOOL};
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_MENU, VK_SHIFT};
use windows::Win32::UI::TextServices::{
    ITfComposition, ITfCompositionSink, ITfCompositionSink_Impl, ITfContext, ITfEditSession,
    ITfKeyEventSink, ITfKeyEventSink_Impl, ITfKeystrokeMgr, ITfTextInputProcessor,
    ITfTextInputProcessor_Impl, ITfThreadMgr,
};

use yatate_core::henkan::{Act, Henkan};

use crate::edit::{request, CompositionCell, CompositionEdit, EditAction};
use crate::keymap::{self, Scan, Vk};

/// テキストサービス本体。TSF がスレッドごとに 1 つ作る。
///
/// 状態は素の [`RefCell`] で持つ。TIP は STA（スレッドごとの apartment）で動き、
/// 同じスレッドからしか触られないためである。
#[implement(ITfTextInputProcessor, ITfKeyEventSink, ITfCompositionSink)]
pub struct Tip {
    /// 打鍵と変換の状態機械（**OS 非依存**）。矢立の頭脳はここに居る。
    henkan: RefCell<Henkan>,
    /// `Activate` で受け取るクライアント ID（sink の登録・解除に使ふ）。
    client_id: Cell<u32>,
    /// TSF の元締め。`Deactivate` で目を外すために持つ。
    thread_mgr: RefCell<Option<ITfThreadMgr>>,
    /// いま立つてゐる未確定文字列（編集セッションと共有する）。
    composition: CompositionCell,
}

impl Default for Tip {
    fn default() -> Self {
        Self::new()
    }
}

impl Tip {
    pub fn new() -> Self {
        Self {
            henkan: RefCell::new(Henkan::new()),
            client_id: Cell::new(0),
            thread_mgr: RefCell::new(None),
            composition: Rc::new(RefCell::new(None)),
        }
    }

    /// いま未確定文字列を抱へてゐるか。
    fn is_composing(&self) -> bool {
        self.henkan.borrow().is_composing()
    }

    /// `lParam` から走査符号（物理位置）を取り出す。16〜23 bit に入つてくる。
    fn scancode_of(lparam: LPARAM) -> Scan {
        ((lparam.0 >> 16) & 0xFF) as Scan
    }

    /// Ctrl / Alt が押されてゐるか。押されてゐれば矢立は手を出さない
    /// （Ctrl+C 等の短絡キーを食ふと、アプリの働きを奪つてしまふ）。
    fn modifier_held() -> bool {
        // SAFETY: 引数は仮想キーコードのみ。読み取り専用の問合せ。
        unsafe { (GetKeyState(VK_CONTROL.0 as i32) < 0) || (GetKeyState(VK_MENU.0 as i32) < 0) }
    }

    /// Shift が押されてゐるか。**Ctrl / Alt とは別に見る**——矢印との併用で
    /// 文節の区切りを直すのに要るからである（原器の前置シフトとは無関係で、
    /// 原器は `^` の逐次打鍵であつて同時押しを使はない）。
    fn shift_held() -> bool {
        // SAFETY: 引数は仮想キーコードのみ。読み取り専用の問合せ。
        unsafe { GetKeyState(VK_SHIFT.0 as i32) < 0 }
    }

    /// この打鍵を矢立が扱ふか（**問はれてゐるだけ。状態は触らない**）。
    fn wants(&self, vk: Vk, scan: Scan) -> bool {
        if Self::modifier_held() {
            return false;
        }
        // 未確定を抱へてゐる間は、機能キーも受けて確定・取り消し・変換の機会を残す
        if self.is_composing()
            && matches!(
                vk,
                keymap::VK_BACK
                    | keymap::VK_RETURN
                    | keymap::VK_ESCAPE
                    | keymap::VK_SPACE
                    | keymap::VK_LEFT
                    | keymap::VK_RIGHT
                    | keymap::VK_UP
                    | keymap::VK_DOWN
            )
        {
            return true;
        }
        match keymap::genki_char(vk, scan) {
            Some(ch) => self.henkan.borrow().wants_key(ch),
            None => false,
        }
    }

    /// 核が返した指示を文書へ移す。返り値は「食つたか」。
    fn apply_act(&self, context: &ITfContext, sink: &ITfCompositionSink, act: Act) -> bool {
        match act {
            Act::Update => {
                let text = self.henkan.borrow().preedit();
                self.apply(context, sink, EditAction::Update(text));
                true
            }
            // 前置シフトが立つた・範囲外の操作だつた等。**食ふ**
            //（流すと `^` や矢印がアプリへ抜けて未確定が壊れる）
            Act::Swallow => true,
            Act::Passthrough => false,
            Act::Commit(text) => {
                self.apply(context, sink, EditAction::Commit(text));
                true
            }
            // 変換中に原器の鍵が来た場合。**次の未確定を先に読んでおく**——
            // 確定で composition を閉ぢると `OnCompositionTerminated` が来て
            // 核を片付け得るので、その後に読むと新しい打鍵が消える。
            Act::CommitThenUpdate(text) => {
                let next = self.henkan.borrow().preedit();
                self.apply(context, sink, EditAction::Commit(text));
                self.apply(context, sink, EditAction::Update(next));
                true
            }
            Act::Cancel => {
                self.apply(context, sink, EditAction::Cancel);
                true
            }
        }
    }

    /// 文書へ指示を出す（編集セッションを願ひ出る）。
    fn apply(&self, context: &ITfContext, sink: &ITfCompositionSink, action: EditAction) {
        let edit: ITfEditSession = CompositionEdit::new(
            context.clone(),
            Rc::clone(&self.composition),
            sink.clone(),
            action,
        )
        .into();
        // 失敗しても打鍵は食つたままにする（半端に OS へ流すと二重入力になる）。
        let _ = request(context, self.client_id.get(), edit);
    }

    /// 打鍵を実際に処理する。返り値は「食つたか」。
    fn handle_key(
        &self,
        context: &ITfContext,
        sink: &ITfCompositionSink,
        vk: Vk,
        scan: Scan,
    ) -> bool {
        if Self::modifier_held() {
            return false;
        }

        // ── 機能キー（未確定を抱へてゐるときだけ意味を持つ）──
        //
        // Space は**確定ではなく変換**である（世間の IME と同じ作法）。
        // 仮名のまま入れたいときは Enter で確定する。
        if self.is_composing() {
            let shift = Self::shift_held();
            let act = match vk {
                keymap::VK_SPACE => Some(self.henkan.borrow_mut().convert()),
                keymap::VK_RETURN => Some(self.henkan.borrow_mut().commit()),
                keymap::VK_ESCAPE => Some(self.henkan.borrow_mut().cancel()),
                keymap::VK_BACK => Some(self.henkan.borrow_mut().backspace()),
                // Shift 併用は**区切りの修正**、単独は注目文節の移動
                keymap::VK_LEFT if shift => Some(self.henkan.borrow_mut().shrink_focus()),
                keymap::VK_RIGHT if shift => Some(self.henkan.borrow_mut().grow_focus()),
                keymap::VK_LEFT => Some(self.henkan.borrow_mut().focus_prev()),
                keymap::VK_RIGHT => Some(self.henkan.borrow_mut().focus_next()),
                keymap::VK_DOWN => Some(self.henkan.borrow_mut().next_candidate()),
                keymap::VK_UP => Some(self.henkan.borrow_mut().prev_candidate()),
                _ => None,
            };
            if let Some(act) = act {
                return self.apply_act(context, sink, act);
            }
        }

        // ── 原器の鍵 ──
        let Some(ch) = keymap::genki_char(vk, scan) else {
            return false;
        };
        let act = self.henkan.borrow_mut().key(ch);
        self.apply_act(context, sink, act)
    }
}

impl ITfTextInputProcessor_Impl for Tip_Impl {
    /// TSF が矢立を起こす。ここで鍵盤の目（`ITfKeyEventSink`）を刺す。
    fn Activate(&self, ptim: Ref<'_, ITfThreadMgr>, tid: u32) -> Result<()> {
        let me = &self.this;
        me.client_id.set(tid);

        let Some(thread_mgr) = ptim.as_ref() else {
            return Ok(());
        };
        *me.thread_mgr.borrow_mut() = Some(thread_mgr.clone());

        // 自分自身を鍵盤の目として差し出す
        let sink: ITfKeyEventSink = self.to_interface();
        // SAFETY: thread_mgr は TSF が寄越した有効な参照。sink は自分自身。
        unsafe {
            let keystroke: ITfKeystrokeMgr = thread_mgr.cast()?;
            keystroke.AdviseKeyEventSink(tid, &sink, true)?;
        }
        Ok(())
    }

    /// 片付け。刺した目を抜く。
    fn Deactivate(&self) -> Result<()> {
        let me = &self.this;
        if let Some(thread_mgr) = me.thread_mgr.borrow_mut().take() {
            // SAFETY: 有効な参照。失敗しても片付けは続ける。
            unsafe {
                if let Ok(keystroke) = thread_mgr.cast::<ITfKeystrokeMgr>() {
                    let _ = keystroke.UnadviseKeyEventSink(me.client_id.get());
                }
            }
        }
        me.henkan.borrow_mut().reset();
        *me.composition.borrow_mut() = None;
        me.client_id.set(0);
        Ok(())
    }
}

impl ITfKeyEventSink_Impl for Tip_Impl {
    /// 入力欄が替はつた。抱へてゐた未確定文字列は捨てる（別の欄へ持ち越さない）。
    fn OnSetFocus(&self, _fforeground: BOOL) -> Result<()> {
        let me = &self.this;
        me.henkan.borrow_mut().reset();
        *me.composition.borrow_mut() = None;
        Ok(())
    }

    /// **問はれてゐるだけ。状態を触つてはならない。**
    fn OnTestKeyDown(
        &self,
        _pic: Ref<'_, ITfContext>,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Result<BOOL> {
        let me = &self.this;
        let vk = wparam.0 as Vk;
        let scan = Tip::scancode_of(lparam);
        Ok(me.wants(vk, scan).into())
    }

    fn OnTestKeyUp(
        &self,
        _pic: Ref<'_, ITfContext>,
        _wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Result<BOOL> {
        // 離鍵は使はない（原器は押下だけで決まる）
        Ok(false.into())
    }

    /// 実際に処理する段。
    fn OnKeyDown(&self, pic: Ref<'_, ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        let me = &self.this;
        let Some(context) = pic.as_ref() else {
            return Ok(false.into());
        };
        let sink: ITfCompositionSink = self.to_interface();
        let vk = wparam.0 as Vk;
        let scan = Tip::scancode_of(lparam);
        Ok(me.handle_key(context, &sink, vk, scan).into())
    }

    fn OnKeyUp(&self, _pic: Ref<'_, ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        Ok(false.into())
    }

    fn OnPreservedKey(
        &self,
        _pic: Ref<'_, ITfContext>,
        _rguid: *const windows::core::GUID,
    ) -> Result<BOOL> {
        // 予約キー（IME の切替等）は今は持たない
        Ok(false.into())
    }
}

impl ITfCompositionSink_Impl for Tip_Impl {
    /// アプリ側の都合で未確定文字列が打ち切られた（別の場所を触つた等）。
    /// **核の状態も一緒に捨てる**——ここを忘れると画面と頭脳が食ひ違ふ。
    fn OnCompositionTerminated(
        &self,
        _ecwrite: u32,
        _pcomposition: Ref<'_, ITfComposition>,
    ) -> Result<()> {
        let me = &self.this;
        me.henkan.borrow_mut().reset();
        *me.composition.borrow_mut() = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 走査符号の取り出し（`lParam` の 16〜23 bit）。実機の値で確かめる。
    #[test]
    fn 走査符号を_lparam_から取り出せる() {
        // 「:」= sc 0x28。lParam は 0x00280001 のやうな形で来る。
        assert_eq!(Tip::scancode_of(LPARAM(0x0028_0001)), 0x28);
        // 「;」= sc 0x27
        assert_eq!(Tip::scancode_of(LPARAM(0x0027_0001)), 0x27);
        // 「^」= sc 0x0D
        assert_eq!(Tip::scancode_of(LPARAM(0x000D_0001)), 0x0D);
        // 拡張ビット等が立つてゐても 8bit だけを見る
        assert_eq!(Tip::scancode_of(LPARAM(0x0128_0001u32 as isize)), 0x28);
    }
}
