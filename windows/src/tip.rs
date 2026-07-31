//! COM の入口 — `ITfTextInputProcessor` の実装（TSF が矢立を起こす場所）。
//!
//! **ここは薄い。** 打鍵の意味を決めるのは [`crate::session`]（OS を知らない層）で、
//! この module は TSF の作法に翻訳するだけである。
//!
//! ## TSF の生き死に
//!
//! ```text
//! TSF が DLL を読む
//!   → DllGetClassObject でクラスファクトリを得る（**未実装**）
//!   → CoCreateInstance で Tip を作る
//!   → Activate(thread_mgr, client_id)   ← ここで鍵盤の目（KeyEventSink）を刺す
//!   → 打鍵ごとに OnTestKeyDown / OnKeyDown が来る
//!   → Deactivate() で目を外す
//! ```
//!
//! ## まだ Windows 機が要ること（印）
//!
//! - `DllMain` / `DllGetClassObject` / `DllCanUnloadNow` / `DllRegisterServer`
//! - `ITfKeyEventSink` を挿し、仮想キーコード → 文字への写像を実機で詰める
//!   （原器は JIS 前提。`;` `:` の位置は配列によつて動く）
//! - `ITfComposition` で未確定文字列を描き、確定時に差し替へる
//! - 候補窓（TSF は出してくれない。DPI・マルチモニタ・UI-less mode を自前で見る）
//!
//! いづれも実機の上でしか正しさを確かめられないので、**ここでは書かない**。
//! 書けば「動くやうに見えて動かないコード」が残るだけである。

use windows::Win32::UI::TextServices::{
    ITfTextInputProcessor, ITfTextInputProcessor_Impl, ITfThreadMgr,
};
use windows_core::{Ref, Result};

use yatate_core::session::Session;

/// テキストサービス本体。TSF がスレッドごとに 1 つ作る。
#[windows_implement::implement(ITfTextInputProcessor)]
pub struct Tip {
    /// 打鍵の状態機械（OS 非依存）。実機配線では内部可変性が要る。
    _session: Session,
    /// `Activate` で受け取るクライアント ID（sink の登録・解除に使ふ）。
    _client_id: std::cell::Cell<u32>,
}

impl Default for Tip {
    fn default() -> Self {
        Self::new()
    }
}

impl Tip {
    pub fn new() -> Self {
        Self {
            _session: Session::new(),
            _client_id: std::cell::Cell::new(0),
        }
    }
}

impl ITfTextInputProcessor_Impl for Tip_Impl {
    /// TSF が矢立を起こす。ここで鍵盤の目（`ITfKeyEventSink`）を刺す。
    fn Activate(&self, _thread_mgr: Ref<'_, ITfThreadMgr>, client_id: u32) -> Result<()> {
        self.this._client_id.set(client_id);
        // TODO(Windows 機): ITfKeyEventSink を AdviseKeyEventSink で登録する。
        // 打鍵は OnTestKeyDown → Session::wants_key、OnKeyDown → Session::key へ渡す。
        Ok(())
    }

    /// 片付け。刺した目を抜く。
    fn Deactivate(&self) -> Result<()> {
        // TODO(Windows 機): UnadviseKeyEventSink と composition の後始末。
        self.this._client_id.set(0);
        Ok(())
    }
}
