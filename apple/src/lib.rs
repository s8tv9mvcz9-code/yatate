//! 矢立の Apple 束縛 — 核を**素の C ABI** で Swift へ出す（M5-b2）。
//!
//! ## なぜ uniffi ではないのか
//!
//! 設計書は当初 uniffi を想定してゐたが、実装の段でやめた。核の入出力は
//! 「文字を入れる／文字列を貰ふ」だけで、それなら `extern "C"` で足りる
//! ——`web/src/lib.rs` が wasm で既にさうしてゐる。uniffi を採ると
//!
//!   ・核（`yatate-core`）の依存ゼロ方針（`core/Cargo.toml`）に手を入れるか、
//!     さもなくば束縛用の中間クレートを結局書くことになる
//!   ・生成器と実行時ライブラリの版が道具立てとして増える
//!
//! ので、殻が四つ（web・Windows・macOS・iOS）とも同じ「素の C ABI」で核に触る形へ
//! 揃へた。**道具立ては cargo だけ**である。
//!
//! ## 約束
//!
//! | 事 | 決め |
//! |---|---|
//! | 文字列 | 返り値は UTF-8 の NUL 終端。**呼んだ側が [`yatate_string_free`] で返す** |
//! | 文字 | `u32` のスカラ値で渡す。`0` は「無い」を表す（原器に NUL は無い） |
//! | 手綱 | `NULL` を渡しても落ちない（殻の取り回しの誤りでアプリごと死なせない） |
//! | 表 | ここは表を**持たない**。すべて核から起こして TSV で渡す |
//!
//! ## ここに知識を置かない
//!
//! 原器の配列も、旧字の 248 字も、氣配の重みも、この束縛は一つも知らない。
//! `yatate_kagi_table()` などが返すのは**核の表をその場で舐めた結果**である。
//! Swift 側もそれを読むだけなので、地図は最後まで一枚のままになる
//! ——このリポジトリが繰り返し学んだ「二枚持つと必ずずれる」を踏まないため。

use std::ffi::{c_char, CStr, CString};
use std::fmt::Write as _;

use yatate_core::bunsetsu;
use yatate_core::generated::kyuji_table::{AMBIGUOUS_SHINJI, KYUJI_PAIRS};
use yatate_core::genki::{self, Edit, FIRST_PLANE, SECOND_PLANE};
use yatate_core::gojuon::{self, Deflect};
use yatate_core::henkan::{Act, Henkan, Phase};
use yatate_core::kagi;
use yatate_core::kehai::{self, ActionField, KeyId};
use yatate_core::kyuji::to_kyuji;
use yatate_core::session::{KeyAction, Session};

// ── 符号 ────────────────────────────────────────────────────
//
// Swift 側は `YatateFFI` の定数としてそのまま見る。数値を Swift へ書き写さない。

/// 矢立の鍵ではない。殻は OS へ素通しする。
pub const ACT_PASSTHROUGH: i32 = 0;
/// 未確定文字列が変はつた。殻は描き直す。
pub const ACT_UPDATE: i32 = 1;
/// 食つたが見た目は変はらない。
pub const ACT_SWALLOW: i32 = 2;
/// 確定した。殻は [`yatate_henkan_take_commit`] で文字列を取り、未確定を閉ぢる。
pub const ACT_COMMIT: i32 = 3;
/// 確定した上で、続けて新しい未確定が立つてゐる。
pub const ACT_COMMIT_THEN_UPDATE: i32 = 4;
/// 捨てて閉ぢる。
pub const ACT_CANCEL: i32 = 5;

/// 仮名を積んでゐる段。
pub const PHASE_KANA: i32 = 0;
/// 変換して文節を選び直してゐる段。
pub const PHASE_HENKAN: i32 = 1;

/// 仮名を足す。
pub const EDIT_INSERT: i32 = 0;
/// 直前の一字を差し替へる（濁点・半濁点の後置打鍵）。
pub const EDIT_REPLACE_LAST: i32 = 1;
/// 目に見える変化なし（前置シフトが立つた等）。
pub const EDIT_NONE: i32 = 2;
/// この鍵は原器に無い。
pub const EDIT_UNMAPPED: i32 = 3;

// ── 文字列の受け渡し ────────────────────────────────────────

/// Rust が作つた文字列を返す。**返り値のある関数を呼んだら必ずこれへ渡す。**
///
/// # Safety
/// `s` はこの束縛が返した先頭であり、まだ返してゐないものであること。
#[no_mangle]
pub unsafe extern "C" fn yatate_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(unsafe { CString::from_raw(s) });
    }
}

/// Rust の `String` を C の文字列へ。内部に NUL が混じつたら空を返す
/// （矢立の扱ふ文（仮名・漢字）に NUL は現れないので、実際には起こらない）。
fn out(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(c) => c.into_raw(),
        Err(_) => CString::default().into_raw(),
    }
}

/// # Safety
/// `p` は NUL 終端の UTF-8、または `NULL` であること。
unsafe fn as_str<'a>(p: *const c_char) -> Option<&'a str> {
    if p.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(p) }.to_str().ok()
}

/// `u32` のスカラ値を `char` へ。`0` と不正なスカラは「無い」。
fn as_char(scalar: u32) -> Option<char> {
    if scalar == 0 {
        None
    } else {
        char::from_u32(scalar)
    }
}

fn scalar(c: Option<char>) -> u32 {
    c.map(u32::from).unwrap_or(0)
}

// ── 卓（変換まで含む一つの入力欄） ──────────────────────────

/// 一つの入力欄ぶんの状態。Swift は不透明な手綱として持つ。
pub struct HenkanHandle {
    inner: Henkan,
    /// 直前の行為が確定させた文字列。Swift が [`yatate_henkan_take_commit`] で取る。
    ///
    /// 行為の符号（`i32`）と文字列を一度に返せないので、確定文は手綱に預けておく。
    /// **取ると空になる**ので、同じ文字列が二度差し込まれることはない。
    commit: String,
}

impl HenkanHandle {
    fn act(&mut self, act: Act) -> i32 {
        match act {
            Act::Update => ACT_UPDATE,
            Act::Swallow => ACT_SWALLOW,
            Act::Passthrough => ACT_PASSTHROUGH,
            Act::Cancel => ACT_CANCEL,
            Act::Commit(t) => {
                self.commit = t;
                ACT_COMMIT
            }
            Act::CommitThenUpdate(t) => {
                self.commit = t;
                ACT_COMMIT_THEN_UPDATE
            }
        }
    }
}

/// 入力欄を一つ起こす。使ひ終へたら [`yatate_henkan_free`] へ渡すこと。
#[no_mangle]
pub extern "C" fn yatate_henkan_new() -> *mut HenkanHandle {
    Box::into_raw(Box::new(HenkanHandle {
        inner: Henkan::new(),
        commit: String::new(),
    }))
}

/// 入力欄を畳む。
///
/// # Safety
/// `h` は [`yatate_henkan_new`] が返した手綱で、まだ畳んでゐないものであること。
#[no_mangle]
pub unsafe extern "C" fn yatate_henkan_free(h: *mut HenkanHandle) {
    if !h.is_null() {
        drop(unsafe { Box::from_raw(h) });
    }
}

/// 手綱を借りて何かする小道具。`NULL` なら既定値を返して**落ちない**。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL` であること。
unsafe fn with<T>(h: *mut HenkanHandle, d: T, f: impl FnOnce(&mut HenkanHandle) -> T) -> T {
    match unsafe { h.as_mut() } {
        Some(x) => f(x),
        None => d,
    }
}

/// 原器の一打（`genki` は原器の文字のスカラ値。位置から引いた結果を渡す）。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL` であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_henkan_key(h: *mut HenkanHandle, genki: u32) -> i32 {
    unsafe {
        with(h, ACT_PASSTHROUGH, |x| match as_char(genki) {
            Some(c) => {
                let a = x.inner.key(c);
                x.act(a)
            }
            None => ACT_PASSTHROUGH,
        })
    }
}

/// 仮名を直に積む（硝子の鍵盤用。二行配列は原器の写像を経ない）。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL`、`kana` は NUL 終端の UTF-8 であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_henkan_insert_kana(
    h: *mut HenkanHandle,
    kana: *const c_char,
) -> i32 {
    let text = unsafe { as_str(kana) }.unwrap_or("").to_string();
    unsafe {
        with(h, ACT_SWALLOW, |x| {
            let a = x.inner.insert_kana(&text);
            x.act(a)
        })
    }
}

/// 一打で一つの行為を呼ぶだけの関数を並べる。
macro_rules! act_fn {
    ($(#[$m:meta])* $name:ident => $method:ident) => {
        $(#[$m])*
        ///
        /// # Safety
        /// `h` は生きてゐる手綱か `NULL` であること。
        #[no_mangle]
        pub unsafe extern "C" fn $name(h: *mut HenkanHandle) -> i32 {
            unsafe { with(h, ACT_SWALLOW, |x| { let a = x.inner.$method(); x.act(a) }) }
        }
    };
}

act_fn!(
    /// Space —— 未変換なら変換し、変換中なら次の候補へ。
    yatate_henkan_convert => convert
);
act_fn!(
    /// 次の候補へ（一巡する）。
    yatate_henkan_next_candidate => next_candidate
);
act_fn!(
    /// 前の候補へ（一巡する）。
    yatate_henkan_prev_candidate => prev_candidate
);
act_fn!(
    /// 注目する文節を右へ。
    yatate_henkan_focus_next => focus_next
);
act_fn!(
    /// 注目する文節を左へ。
    yatate_henkan_focus_prev => focus_prev
);
act_fn!(
    /// 注目文節を一字伸ばす（Shift+→）。
    yatate_henkan_grow_focus => grow_focus
);
act_fn!(
    /// 注目文節を一字縮める（Shift+←）。
    yatate_henkan_shrink_focus => shrink_focus
);
act_fn!(
    /// 一字消す。変換中は消さず、変換を解いて仮名へ戻す。
    yatate_henkan_backspace => backspace
);
act_fn!(
    /// 変換を解いて仮名の段へ戻す（読みは失はない）。
    yatate_henkan_unconvert => unconvert
);
act_fn!(
    /// Esc —— 変換中なら一段戻し、仮名の段なら捨てる。
    yatate_henkan_cancel => cancel
);
act_fn!(
    /// Enter —— 確定。**旧字体はここで機械が定まる。**
    yatate_henkan_commit => commit
);

/// 候補を番号で選ぶ（候補窓を持つ殻が使ふ）。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL` であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_henkan_choose(h: *mut HenkanHandle, index: usize) -> i32 {
    unsafe {
        with(h, ACT_SWALLOW, |x| {
            let a = x.inner.choose(index);
            x.act(a)
        })
    }
}

/// 入力欄が替はつた等で全部捨てる。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL` であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_henkan_reset(h: *mut HenkanHandle) {
    unsafe {
        with(h, (), |x| {
            x.inner.reset();
            x.commit.clear();
        })
    }
}

/// 直前の行為が確定させた文字列を**取る**（取ると空になる）。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL` であること。返り値は [`yatate_string_free`] へ。
#[no_mangle]
pub unsafe extern "C" fn yatate_henkan_take_commit(h: *mut HenkanHandle) -> *mut c_char {
    unsafe {
        with(h, out(String::new()), |x| {
            out(std::mem::take(&mut x.commit))
        })
    }
}

/// 文字列を返すだけの問合せを並べる。
macro_rules! text_fn {
    ($(#[$m:meta])* $name:ident => |$x:ident| $body:expr) => {
        $(#[$m])*
        ///
        /// # Safety
        /// `h` は生きてゐる手綱か `NULL`。返り値は [`yatate_string_free`] へ。
        #[no_mangle]
        pub unsafe extern "C" fn $name(h: *mut HenkanHandle) -> *mut c_char {
            unsafe { with(h, out(String::new()), |$x| out($body)) }
        }
    };
}

text_fn!(
    /// 画面に出す未確定文字列（Kana 段は仮名、Henkan 段は新字体の表記）。
    yatate_henkan_preedit => |x| x.inner.preedit()
);
text_fn!(
    /// いま抱へてゐる読み（段に依らず仮名）。
    yatate_henkan_yomi => |x| x.inner.yomi().to_string()
);
text_fn!(
    /// 注目文節の候補。一行一候補の TSV: `表記\t費用\t辞書に在るか(0|1)`。
    yatate_henkan_candidates => |x| {
        let mut s = String::new();
        for c in x.inner.candidates() {
            let _ = writeln!(s, "{}\t{}\t{}", c.surface, c.cost, u8::from(c.in_jisho));
        }
        s
    }
);
text_fn!(
    /// 変換中の文節。一行一文節の TSV: `読み\t選んでゐる番号\t表記`。
    yatate_henkan_segments => |x| {
        let mut s = String::new();
        for b in x.inner.segments() {
            let _ = writeln!(s, "{}\t{}\t{}", b.yomi, b.chosen, b.surface());
        }
        s
    }
);
text_fn!(
    /// 墨の氣配 — 次の一打の分布（書式は [`yatate_kehai_field`] と同じ）。
    yatate_henkan_kehai => |x| field_tsv(&x.inner.field())
);

/// 数を返すだけの問合せを並べる。
macro_rules! int_fn {
    ($(#[$m:meta])* $name:ident -> $ty:ty, $d:expr => |$x:ident| $body:expr) => {
        $(#[$m])*
        ///
        /// # Safety
        /// `h` は生きてゐる手綱か `NULL` であること。
        #[no_mangle]
        pub unsafe extern "C" fn $name(h: *mut HenkanHandle) -> $ty {
            unsafe { with(h, $d, |$x| $body) }
        }
    };
}

int_fn!(
    /// いまどの段に居るか（[`PHASE_KANA`] / [`PHASE_HENKAN`]）。
    yatate_henkan_phase -> i32, PHASE_KANA => |x| match x.inner.phase() {
        Phase::Kana => PHASE_KANA,
        Phase::Henkan => PHASE_HENKAN,
    }
);
int_fn!(
    /// 未確定文字列を抱へてゐるか。
    yatate_henkan_is_composing -> i32, 0 => |x| i32::from(x.inner.is_composing())
);
int_fn!(
    /// 前置シフトが立つてゐるか（殻が面の表示を替へる手掛かり）。
    yatate_henkan_is_shifted -> i32, 0 => |x| i32::from(x.inner.is_shifted())
);
int_fn!(
    /// 注目してゐる文節の番号。
    yatate_henkan_focus -> usize, 0 => |x| x.inner.focus()
);
int_fn!(
    /// 注目文節で選ばれてゐる候補の番号。
    yatate_henkan_chosen -> usize, 0 => |x| x.inner.chosen()
);

/// この鍵を矢立が受け取るべきか（**副作用禁止**の問合せ）。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL` であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_henkan_wants_key(h: *mut HenkanHandle, genki: u32) -> i32 {
    unsafe {
        with(h, 0, |x| match as_char(genki) {
            Some(c) => i32::from(x.inner.wants_key(c)),
            None => 0,
        })
    }
}

/// 注目文節が未確定文字列の何**文字目**から何文字か（下線を引き分けるため）。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL`、`start`/`len` は書ける場所か `NULL` であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_henkan_focus_range(
    h: *mut HenkanHandle,
    start: *mut usize,
    len: *mut usize,
) {
    let (s, l) = unsafe { with(h, (0, 0), |x| x.inner.focus_range()) };
    unsafe {
        if let Some(p) = start.as_mut() {
            *p = s;
        }
        if let Some(p) = len.as_mut() {
            *p = l;
        }
    }
}

// ── 素の打鍵（変換を持たない殻・硝子の鍵盤が使ふ） ──────────

/// 打鍵 → 仮名の状態機械だけを持つ手綱（[`Session`] そのもの）。
pub struct SessionHandle {
    inner: Session,
    commit: String,
}

/// 打鍵の状態機械を一つ起こす。
#[no_mangle]
pub extern "C" fn yatate_session_new() -> *mut SessionHandle {
    Box::into_raw(Box::new(SessionHandle {
        inner: Session::new(),
        commit: String::new(),
    }))
}

/// 畳む。
///
/// # Safety
/// `h` は [`yatate_session_new`] が返した手綱で、まだ畳んでゐないものであること。
#[no_mangle]
pub unsafe extern "C" fn yatate_session_free(h: *mut SessionHandle) {
    if !h.is_null() {
        drop(unsafe { Box::from_raw(h) });
    }
}

/// # Safety
/// `h` は生きてゐる手綱か `NULL` であること。
unsafe fn with_session<T>(
    h: *mut SessionHandle,
    d: T,
    f: impl FnOnce(&mut SessionHandle) -> T,
) -> T {
    match unsafe { h.as_mut() } {
        Some(x) => f(x),
        None => d,
    }
}

fn key_action(x: &mut SessionHandle, a: KeyAction) -> i32 {
    match a {
        KeyAction::Update => ACT_UPDATE,
        KeyAction::Swallow => ACT_SWALLOW,
        KeyAction::Passthrough => ACT_PASSTHROUGH,
        KeyAction::Commit(t) => {
            x.commit = t;
            ACT_COMMIT
        }
    }
}

/// 原器の一打。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL` であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_session_key(h: *mut SessionHandle, genki: u32) -> i32 {
    unsafe {
        with_session(h, ACT_PASSTHROUGH, |x| match as_char(genki) {
            Some(c) => {
                let a = x.inner.key(c);
                key_action(x, a)
            }
            None => ACT_PASSTHROUGH,
        })
    }
}

/// 仮名を直に積む（硝子の鍵盤用）。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL`、`kana` は NUL 終端の UTF-8 であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_session_insert_kana(
    h: *mut SessionHandle,
    kana: *const c_char,
) -> i32 {
    let text = unsafe { as_str(kana) }.unwrap_or("").to_string();
    unsafe {
        with_session(h, ACT_SWALLOW, |x| {
            let a = x.inner.insert_kana(&text);
            key_action(x, a)
        })
    }
}

/// 確定。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL` であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_session_commit(h: *mut SessionHandle) -> i32 {
    unsafe {
        with_session(h, ACT_CANCEL, |x| {
            let a = x.inner.commit();
            key_action(x, a)
        })
    }
}

/// 直前の確定文字列を取る（取ると空になる）。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL`。返り値は [`yatate_string_free`] へ。
#[no_mangle]
pub unsafe extern "C" fn yatate_session_take_commit(h: *mut SessionHandle) -> *mut c_char {
    unsafe {
        with_session(h, out(String::new()), |x| {
            out(std::mem::take(&mut x.commit))
        })
    }
}

/// 一字消す。まだ未確定文字列が残つてゐれば 1、空になつたら 0。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL` であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_session_backspace(h: *mut SessionHandle) -> i32 {
    unsafe { with_session(h, 0, |x| i32::from(x.inner.backspace())) }
}

/// 取り消し（Esc）。未確定文字列を捨てる。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL` であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_session_cancel(h: *mut SessionHandle) {
    unsafe { with_session(h, (), |x| x.inner.cancel()) }
}

/// いま未確定の仮名。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL`。返り値は [`yatate_string_free`] へ。
#[no_mangle]
pub unsafe extern "C" fn yatate_session_preedit(h: *mut SessionHandle) -> *mut c_char {
    unsafe {
        with_session(h, out(String::new()), |x| {
            out(x.inner.preedit().to_string())
        })
    }
}

/// 未確定文字列を抱へてゐるか。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL` であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_session_is_composing(h: *mut SessionHandle) -> i32 {
    unsafe { with_session(h, 0, |x| i32::from(x.inner.is_composing())) }
}

/// 前置シフトが立つてゐるか。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL` であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_session_is_shifted(h: *mut SessionHandle) -> i32 {
    unsafe { with_session(h, 0, |x| i32::from(x.inner.is_shifted())) }
}

/// この鍵を矢立が受け取るべきか（副作用禁止）。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL` であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_session_wants_key(h: *mut SessionHandle, genki: u32) -> i32 {
    unsafe {
        with_session(h, 0, |x| match as_char(genki) {
            Some(c) => i32::from(x.inner.wants_key(c)),
            None => 0,
        })
    }
}

/// 墨の氣配（書式は [`yatate_kehai_field`] と同じ）。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL`。返り値は [`yatate_string_free`] へ。
#[no_mangle]
pub unsafe extern "C" fn yatate_session_kehai(h: *mut SessionHandle) -> *mut c_char {
    unsafe { with_session(h, out(String::new()), |x| out(field_tsv(&x.inner.field()))) }
}

// ── 原器の状態機械そのもの（前置シフトの逐次性を見るため） ──

/// 打鍵 → 編輯指示（[`yatate_genki_press`] が使ふ手綱）。
pub struct GenkiHandle(genki::Genki);

/// 起こす。
#[no_mangle]
pub extern "C" fn yatate_genki_new() -> *mut GenkiHandle {
    Box::into_raw(Box::new(GenkiHandle(genki::Genki::new())))
}

/// 畳む。
///
/// # Safety
/// `h` は [`yatate_genki_new`] が返した手綱であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_genki_free(h: *mut GenkiHandle) {
    if !h.is_null() {
        drop(unsafe { Box::from_raw(h) });
    }
}

/// 一打を食はせる。`last` は作業帯の末尾の一字（無ければ 0）。
///
/// 戻り値は `EDIT_*`。`text` が非 `NULL` なら、そこへ
/// 足す仮名（[`EDIT_INSERT`]）か差し替へる一字（[`EDIT_REPLACE_LAST`]）を書く。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL`、`text` は書ける場所か `NULL`。
/// `*text` に書かれた文字列は [`yatate_string_free`] へ渡すこと。
#[no_mangle]
pub unsafe extern "C" fn yatate_genki_press(
    h: *mut GenkiHandle,
    key: u32,
    last: u32,
    text: *mut *mut c_char,
) -> i32 {
    let mut written = String::new();
    let code = match (unsafe { h.as_mut() }, as_char(key)) {
        (Some(g), Some(k)) => match g.0.press(k, as_char(last)) {
            Edit::Insert(kana) => {
                written.push_str(kana);
                EDIT_INSERT
            }
            Edit::ReplaceLast(c) => {
                written.push(c);
                EDIT_REPLACE_LAST
            }
            Edit::None => EDIT_NONE,
            Edit::Unmapped => EDIT_UNMAPPED,
        },
        _ => EDIT_UNMAPPED,
    };
    unsafe {
        if let Some(p) = text.as_mut() {
            *p = out(written);
        }
    }
    code
}

/// 前置シフトが立つてゐるか。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL` であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_genki_is_shifted(h: *mut GenkiHandle) -> i32 {
    match unsafe { h.as_ref() } {
        Some(g) => i32::from(g.0.is_shifted()),
        None => 0,
    }
}

/// 前置シフトを取り消す。
///
/// # Safety
/// `h` は生きてゐる手綱か `NULL` であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_genki_reset(h: *mut GenkiHandle) {
    if let Some(g) = unsafe { h.as_mut() } {
        g.0.reset();
    }
}

// ── 表（すべて核から起こす。ここは何も覚えない） ────────────

/// 新字体を旧字体へ（確定のときに機械で定まる部分）。
///
/// # Safety
/// `text` は NUL 終端の UTF-8 か `NULL`。返り値は [`yatate_string_free`] へ。
#[no_mangle]
pub unsafe extern "C" fn yatate_to_kyuji(text: *const c_char) -> *mut c_char {
    out(to_kyuji(unsafe { as_str(text) }.unwrap_or("")))
}

/// 打鍵列を仮名の列へ（稽古と試験の便宜。殻は [`yatate_genki_press`] を使ふ）。
///
/// # Safety
/// `keys` は NUL 終端の UTF-8 か `NULL`。返り値は [`yatate_string_free`] へ。
#[no_mangle]
pub unsafe extern "C" fn yatate_type_keys(keys: *const c_char) -> *mut c_char {
    out(genki::type_keys(unsafe { as_str(keys) }.unwrap_or("")))
}

/// 鍵の地図。一行一鍵の TSV: `原器の文字\tcode\t走査符号\tkVK\tHID`（十進）。
///
/// # Safety
/// 返り値は [`yatate_string_free`] へ渡すこと。
#[no_mangle]
pub extern "C" fn yatate_kagi_table() -> *mut c_char {
    let mut s = String::new();
    for k in kagi::KAGI.iter() {
        let _ = writeln!(
            s,
            "{}\t{}\t{}\t{}\t{}",
            k.genki, k.code, k.scan, k.mac, k.hid
        );
    }
    out(s)
}

/// 原器の両面。一行一鍵の TSV: `面(1|2)\t鍵\t仮名`。
///
/// # Safety
/// 返り値は [`yatate_string_free`] へ渡すこと。
#[no_mangle]
pub extern "C" fn yatate_genki_planes() -> *mut c_char {
    let mut s = String::new();
    for (key, kana) in FIRST_PLANE.iter() {
        let _ = writeln!(s, "1\t{key}\t{kana}");
    }
    for (key, kana) in SECOND_PLANE.iter() {
        let _ = writeln!(s, "2\t{key}\t{kana}");
    }
    out(s)
}

/// 原器の特別な三鍵。TSV: `shift\tdakuten\thandakuten`。
///
/// # Safety
/// 返り値は [`yatate_string_free`] へ渡すこと。
#[no_mangle]
pub extern "C" fn yatate_genki_special_keys() -> *mut c_char {
    out(format!(
        "{}\t{}\t{}\n",
        genki::SHIFT,
        genki::DAKUTEN,
        genki::HANDAKUTEN
    ))
}

/// 五十音の地図。一行一枡の TSV: `行\t段\t逸らし(none|daku|ko)\t仮名`。
///
/// 空きスロットは**行を出さない**（Swift 側は無い枡を `nil` として読む）。
/// 行の並びは核の読み順（あ→か→…→わ）そのままで、二行配列の列もこれで決まる。
///
/// # Safety
/// 返り値は [`yatate_string_free`] へ渡すこと。
#[no_mangle]
pub extern "C" fn yatate_gojuon_table() -> *mut c_char {
    let mut s = String::new();
    for g in gojuon::all() {
        for dan in 0..5 {
            for (name, d) in [
                ("none", Deflect::None),
                ("daku", Deflect::Daku),
                ("ko", Deflect::Ko),
            ] {
                if let Some(kana) = g.kana(dan, d) {
                    let _ = writeln!(s, "{}\t{}\t{}\t{}", g.name, dan, name, kana);
                }
            }
        }
    }
    out(s)
}

/// 二行配列の列。TSV: `1|2\t行`（1＝第一の行〈あかさたな〉、2＝第二の行〈はまやらわ〉）。
///
/// # Safety
/// 返り値は [`yatate_string_free`] へ渡すこと。
#[no_mangle]
pub extern "C" fn yatate_gojuon_lines() -> *mut c_char {
    let mut s = String::new();
    for g in gojuon::FIRST_LINE.iter() {
        let _ = writeln!(s, "1\t{}", g.name);
    }
    for g in gojuon::SECOND_LINE.iter() {
        let _ = writeln!(s, "2\t{}", g.name);
    }
    out(s)
}

/// 仮名から（行, 段）への逆引き。TSV: `仮名\t行\t段`。
///
/// 濁点・半濁点・小書きも基底の鍵へ畳まれる（核の `reverse_lookup` そのもの）。
///
/// # Safety
/// 返り値は [`yatate_string_free`] へ渡すこと。
#[no_mangle]
pub extern "C" fn yatate_gojuon_reverse() -> *mut c_char {
    let mut s = String::new();
    for g in gojuon::all() {
        for dan in 0..5 {
            for d in [Deflect::None, Deflect::Daku, Deflect::Ko] {
                let Some(kana) = g.kana(dan, d) else { continue };
                let Some(c) = kana.chars().next() else {
                    continue;
                };
                // 核の逆引きを通す（ここで独自に畳まない＝写しを作らない）
                if let Some((gyo, dd)) = gojuon::reverse_lookup(c) {
                    let _ = writeln!(s, "{c}\t{gyo}\t{dd}");
                }
            }
        }
    }
    out(s)
}

/// 旧字の写像。TSV: `新字\t旧字`。最後に `\t曖昧字…` の一行が付く。
///
/// # Safety
/// 返り値は [`yatate_string_free`] へ渡すこと。
#[no_mangle]
pub extern "C" fn yatate_kyuji_table() -> *mut c_char {
    let mut s = String::new();
    for (shinji, kyuji) in KYUJI_PAIRS.iter() {
        let _ = writeln!(s, "{shinji}\t{kyuji}");
    }
    // 曖昧字は写像を持たない（新字のまま置く字）。空の第一欄で見分ける。
    let ambiguous: String = AMBIGUOUS_SHINJI.iter().collect();
    let _ = writeln!(s, "\t{ambiguous}");
    out(s)
}

/// 弱信号の閾値（これに満たねば氣配を描かない）。
#[no_mangle]
pub extern "C" fn yatate_kehai_min_evidence() -> u32 {
    kehai::MIN_EVIDENCE
}

/// 墨の氣配 — 直前の一字（空なら連なりの開始）からの分布。
///
/// 一行一項の TSV で、行頭の札で種類を分ける:
///
/// ```text
///   peak\t<gyo|moji>\t<名>
///   ink\t<gyo|moji>\t<名>\t<墨>
///   dan\t<行>\t<段0>\t<段1>\t<段2>\t<段3>\t<段4>
/// ```
///
/// 墨は往復可能な十進表現で書く（丸めると黄金ベクトルと食ひ違ふ）。
/// 並びは核の正準順序のままなので、殻が並べ替へる必要は無い。
///
/// # Safety
/// `prev` は NUL 終端の UTF-8 か `NULL`。返り値は [`yatate_string_free`] へ。
#[no_mangle]
pub unsafe extern "C" fn yatate_kehai_field(prev: *const c_char) -> *mut c_char {
    let prev = unsafe { as_str(prev) }.unwrap_or("").chars().next();
    out(field_tsv(&kehai::field(prev)))
}

/// [`ActionField`] を TSV へ。
fn field_tsv(f: &ActionField) -> String {
    let mut s = String::new();
    if let Some(p) = &f.peak {
        let (kind, name) = key_id_parts(p);
        let _ = writeln!(s, "peak\t{kind}\t{name}");
    }
    for (id, ink) in &f.ink {
        let (kind, name) = key_id_parts(id);
        // `{:?}` は往復可能な最短表現（丸めない＝ベクトルと 1e-6 で合ふ）
        let _ = writeln!(s, "ink\t{kind}\t{name}\t{ink:?}");
    }
    for (gyo, dan) in &f.dan {
        let _ = write!(s, "dan\t{gyo}");
        for v in dan {
            let _ = write!(s, "\t{v:?}");
        }
        s.push('\n');
    }
    s
}

fn key_id_parts(id: &KeyId) -> (&'static str, String) {
    match id {
        KeyId::Gyo(name) => ("gyo", (*name).to_string()),
        KeyId::Moji(c) => ("moji", c.to_string()),
    }
}

// ── 位置から原器の文字を引く ────────────────────────────────

/// `kVK_ANSI_*`（macOS の `NSEvent.keyCode`）から。原器に無ければ 0。
#[no_mangle]
pub extern "C" fn yatate_genki_of_mac(mac: u16) -> u32 {
    scalar(kagi::genki_of_mac(mac))
}

/// USB HID usage（iOS の `UIKey.keyCode`）から。
#[no_mangle]
pub extern "C" fn yatate_genki_of_hid(hid: u16) -> u32 {
    scalar(kagi::genki_of_hid(hid))
}

/// 走査符号 set 1（Windows）から。
#[no_mangle]
pub extern "C" fn yatate_genki_of_scan(scan: u16) -> u32 {
    scalar(kagi::genki_of_scan(scan))
}

/// `KeyboardEvent.code`（web）から。
///
/// # Safety
/// `code` は NUL 終端の UTF-8 か `NULL` であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_genki_of_code(code: *const c_char) -> u32 {
    match unsafe { as_str(code) } {
        Some(c) => scalar(kagi::genki_of_code(c)),
        None => 0,
    }
}

/// 濁点を打つた結果。濁れない字なら 0。
#[no_mangle]
pub extern "C" fn yatate_dakuten(kana: u32) -> u32 {
    scalar(as_char(kana).and_then(genki::dakuten))
}

/// 半濁点を打つた結果（**は行だけ**）。
#[no_mangle]
pub extern "C" fn yatate_handakuten(kana: u32) -> u32 {
    scalar(as_char(kana).and_then(genki::handakuten))
}

// ── 読みで辞書を引く（提案・稽古場が使ふ） ──────────────────

/// 読みの列を文節へ分ける。一行一文節の TSV: `読み\t既定の候補番号\t候補1\t候補2\t…`。
///
/// # Safety
/// `yomi` は NUL 終端の UTF-8 か `NULL`。返り値は [`yatate_string_free`] へ。
#[no_mangle]
pub unsafe extern "C" fn yatate_segment(yomi: *const c_char) -> *mut c_char {
    let yomi = unsafe { as_str(yomi) }.unwrap_or("");
    let mut s = String::new();
    for seg in bunsetsu::segment(yomi) {
        let _ = write!(s, "{}\t{}", seg.yomi, seg.chosen);
        for c in &seg.candidates {
            let _ = write!(s, "\t{}", c.surface);
        }
        s.push('\n');
    }
    out(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// C の文字列を Rust へ戻して読む（Swift がやることと同じ形でなぞる）。
    ///
    /// # Safety
    /// `p` はこの束縛が返した先頭であること。
    unsafe fn take(p: *mut c_char) -> String {
        if p.is_null() {
            return String::new();
        }
        let s = unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned();
        unsafe { yatate_string_free(p) };
        s
    }

    fn c(s: &str) -> CString {
        CString::new(s).unwrap()
    }

    /// 原器で打つ（`docs/ime/layout.md` §1 の打鍵列）。
    unsafe fn typed(keys: &str) -> *mut HenkanHandle {
        let h = yatate_henkan_new();
        for ch in keys.chars() {
            unsafe { yatate_henkan_key(h, u32::from(ch)) };
        }
        h
    }

    const HARE: &str = "udg^yo2^^ot^4"; // けふはよきてんきなり

    #[test]
    fn 打鍵が仮名になる() {
        unsafe {
            let h = typed(HARE);
            assert_eq!(take(yatate_henkan_preedit(h)), "けふはよきてんきなり");
            yatate_henkan_free(h);
        }
    }

    #[test]
    fn 変換して確定すると旧字が定まる() {
        unsafe {
            let h = typed(HARE);
            assert_eq!(yatate_henkan_convert(h), ACT_UPDATE);
            assert_eq!(yatate_henkan_phase(h), PHASE_HENKAN);
            assert_eq!(take(yatate_henkan_preedit(h)), "今日は良き天気なり");

            assert_eq!(yatate_henkan_commit(h), ACT_COMMIT);
            // 旧字は確定の瞬間だけ掛かる
            assert_eq!(take(yatate_henkan_take_commit(h)), "今日は良き天氣なり");
            // 取ると空になる（二度差し込まれない）
            assert_eq!(take(yatate_henkan_take_commit(h)), "");
            assert_eq!(yatate_henkan_is_composing(h), 0);
            yatate_henkan_free(h);
        }
    }

    #[test]
    fn 文節と候補が読める() {
        unsafe {
            let h = typed(HARE);
            yatate_henkan_convert(h);
            let segs = take(yatate_henkan_segments(h));
            let lines: Vec<&str> = segs.lines().collect();
            assert_eq!(lines.len(), 3, "{segs}");
            assert!(lines[0].starts_with("けふは\t0\t今日は"), "{}", lines[0]);

            let cands = take(yatate_henkan_candidates(h));
            assert!(!cands.is_empty(), "注目文節に候補がある");
            for line in cands.lines() {
                let cols: Vec<&str> = line.split('\t').collect();
                assert_eq!(cols.len(), 3, "{line}");
                assert!(cols[1].parse::<i32>().is_ok(), "{line}");
            }
            yatate_henkan_free(h);
        }
    }

    #[test]
    fn 注目文節の範囲が下線のために出る() {
        unsafe {
            let h = typed(HARE);
            yatate_henkan_convert(h);
            let (mut s, mut l) = (99usize, 99usize);
            yatate_henkan_focus_range(h, &mut s, &mut l);
            assert_eq!(s, 0, "はじめは第一文節");
            assert!(l > 0);

            assert_eq!(yatate_henkan_focus_next(h), ACT_UPDATE);
            let (mut s2, mut l2) = (0usize, 0usize);
            yatate_henkan_focus_range(h, &mut s2, &mut l2);
            assert_eq!(s2, l, "次の文節は前の文節の直後から");
            assert!(l2 > 0);
            yatate_henkan_free(h);
        }
    }

    #[test]
    fn 区切りを直せる() {
        unsafe {
            let h = typed(HARE);
            yatate_henkan_convert(h);
            let before = take(yatate_henkan_segments(h));
            assert_eq!(yatate_henkan_grow_focus(h), ACT_UPDATE);
            let after = take(yatate_henkan_segments(h));
            assert_ne!(before, after, "伸ばせば区切りが動く");
            yatate_henkan_free(h);
        }
    }

    #[test]
    fn 変換を解いても読みは失はれない() {
        unsafe {
            let h = typed(HARE);
            yatate_henkan_convert(h);
            assert_eq!(yatate_henkan_unconvert(h), ACT_UPDATE);
            assert_eq!(yatate_henkan_phase(h), PHASE_KANA);
            assert_eq!(take(yatate_henkan_yomi(h)), "けふはよきてんきなり");
            yatate_henkan_free(h);
        }
    }

    #[test]
    fn 硝子の鍵盤は仮名を直に積める() {
        unsafe {
            let h = yatate_henkan_new();
            for kana in ["け", "ふ"] {
                assert_eq!(yatate_henkan_insert_kana(h, c(kana).as_ptr()), ACT_UPDATE);
            }
            assert_eq!(take(yatate_henkan_preedit(h)), "けふ");
            yatate_henkan_free(h);
        }
    }

    #[test]
    fn 位置から原器が引ける() {
        // 「し」は刻印も物理位置も同じなのに配列で意味が変はる鍵。
        // 位置で引く限り、英字配列の機でも「さ」に化けない。
        assert_eq!(yatate_genki_of_mac(0x27), u32::from(':'));
        assert_eq!(yatate_genki_of_mac(0x29), u32::from(';'));
        assert_eq!(yatate_genki_of_hid(0x34), u32::from(':'));
        assert_eq!(yatate_genki_of_hid(0x33), u32::from(';'));
        unsafe {
            assert_eq!(yatate_genki_of_code(c("Quote").as_ptr()), u32::from(':'));
        }
        // 機能キーを原器が食つてはいけない
        assert_eq!(yatate_genki_of_mac(0x24), 0, "kVK_Return");
        assert_eq!(yatate_genki_of_hid(0x28), 0, "HID Enter");
    }

    #[test]
    fn 表がすべて核から出る() {
        unsafe {
            let kagi = take(yatate_kagi_table());
            assert_eq!(kagi.lines().count(), kagi::KAGI.len(), "33 鍵ちやうど");

            let planes = take(yatate_genki_planes());
            let first = planes.lines().filter(|l| l.starts_with("1\t")).count();
            let second = planes.lines().filter(|l| l.starts_with("2\t")).count();
            assert_eq!(first, FIRST_PLANE.len());
            assert_eq!(second, SECOND_PLANE.len());

            let gojuon = take(yatate_gojuon_table());
            // 10 行 × 5 段 × 3 面のうち、実在する枡だけが出る
            assert!(gojuon.lines().count() > 50);
            assert!(gojuon.contains("わ\t1\tnone\tゐ"), "ゐ は第一級");
            assert!(gojuon.contains("わ\t3\tnone\tゑ"), "ゑ は第一級");

            let lines = take(yatate_gojuon_lines());
            assert_eq!(lines.lines().count(), 10);
            assert!(lines.starts_with("1\tあ\n"));

            let kyuji = take(yatate_kyuji_table());
            assert_eq!(
                kyuji.lines().count(),
                KYUJI_PAIRS.len() + 1,
                "＋曖昧字の一行"
            );
            assert!(kyuji.contains("気\t氣"));
            assert!(
                kyuji.lines().last().unwrap().starts_with('\t'),
                "曖昧字の行"
            );
        }
    }

    #[test]
    fn 氣配が核の正準順序で出る() {
        unsafe {
            let f = take(yatate_kehai_field(c("な").as_ptr()));
            assert!(!f.is_empty());
            let mut saw_peak = false;
            for line in f.lines() {
                let cols: Vec<&str> = line.split('\t').collect();
                match cols[0] {
                    "peak" => {
                        saw_peak = true;
                        assert_eq!(cols.len(), 3, "{line}");
                    }
                    "ink" => {
                        assert_eq!(cols.len(), 4, "{line}");
                        assert!(cols[3].parse::<f64>().is_ok(), "{line}");
                    }
                    "dan" => {
                        assert_eq!(cols.len(), 7, "{line}");
                        for v in &cols[2..] {
                            assert!(v.parse::<f64>().is_ok(), "{line}");
                        }
                    }
                    other => panic!("知らない札: {other}"),
                }
            }
            assert!(saw_peak, "「な」の後には峰がある");
            assert_eq!(yatate_kehai_min_evidence(), kehai::MIN_EVIDENCE);
        }
    }

    #[test]
    fn 墨は丸めずに渡る() {
        // 1/3 のやうな値を 3 桁で切ると黄金ベクトル（1e-6）と食ひ違ふ。
        // 往復可能な表現で渡してゐることを、読み戻して確かめる。
        unsafe {
            let f = take(yatate_kehai_field(c("な").as_ptr()));
            let core = kehai::field(Some('な'));
            for line in f.lines().filter(|l| l.starts_with("ink\t")) {
                let cols: Vec<&str> = line.split('\t').collect();
                let id = if cols[1] == "gyo" {
                    KeyId::Gyo(
                        gojuon::all()
                            .find(|g| g.name == cols[2])
                            .map(|g| g.name)
                            .unwrap(),
                    )
                } else {
                    KeyId::Moji(cols[2].chars().next().unwrap())
                };
                let got: f64 = cols[3].parse().unwrap();
                assert_eq!(got, core.ink_of(&id), "{line}");
            }
        }
    }

    #[test]
    fn 原器の状態機械は前置シフトが一打で降りる() {
        unsafe {
            let g = yatate_genki_new();
            let mut text: *mut c_char = std::ptr::null_mut();
            assert_eq!(
                yatate_genki_press(g, u32::from('^'), 0, &mut text),
                EDIT_NONE
            );
            take(text);
            assert_eq!(yatate_genki_is_shifted(g), 1);

            assert_eq!(
                yatate_genki_press(g, u32::from('0'), 0, &mut text),
                EDIT_INSERT
            );
            assert_eq!(take(text), "ま");
            assert_eq!(yatate_genki_is_shifted(g), 0, "一打で降りる");

            assert_eq!(
                yatate_genki_press(g, u32::from('0'), 0, &mut text),
                EDIT_INSERT
            );
            assert_eq!(take(text), "あ");
            yatate_genki_free(g);
        }
    }

    #[test]
    fn 濁点は後置で直前の字を差し替へる() {
        unsafe {
            let g = yatate_genki_new();
            let mut text: *mut c_char = std::ptr::null_mut();
            assert_eq!(
                yatate_genki_press(g, u32::from('b'), u32::from('か'), &mut text),
                EDIT_REPLACE_LAST
            );
            assert_eq!(take(text), "が");
            yatate_genki_free(g);
        }
        // 半濁点は は行だけ（小書きは原器で未定ゆゑ発明しない）
        assert_eq!(yatate_handakuten(u32::from('は')), u32::from('ぱ'));
        assert_eq!(yatate_handakuten(u32::from('や')), 0);
        assert_eq!(yatate_handakuten(u32::from('つ')), 0);
    }

    #[test]
    fn 素の打鍵の手綱も同じ答へを返す() {
        unsafe {
            let s = yatate_session_new();
            for ch in HARE.chars() {
                yatate_session_key(s, u32::from(ch));
            }
            assert_eq!(take(yatate_session_preedit(s)), "けふはよきてんきなり");
            assert_eq!(yatate_session_commit(s), ACT_COMMIT);
            // Session の確定は仮名のまま旧字へ直る（変換を通さない道）
            assert_eq!(take(yatate_session_take_commit(s)), "けふはよきてんきなり");
            yatate_session_free(s);
        }
    }

    #[test]
    fn 手綱が無くても落ちない() {
        // Swift 側の取り回しを誤つてもアプリごと死なせない
        unsafe {
            let h: *mut HenkanHandle = std::ptr::null_mut();
            assert_eq!(yatate_henkan_key(h, u32::from('a')), ACT_PASSTHROUGH);
            assert_eq!(yatate_henkan_convert(h), ACT_SWALLOW);
            assert_eq!(take(yatate_henkan_preedit(h)), "");
            assert_eq!(yatate_henkan_phase(h), PHASE_KANA);
            let (mut a, mut b) = (9usize, 9usize);
            yatate_henkan_focus_range(h, &mut a, &mut b);
            assert_eq!((a, b), (0, 0));
            yatate_henkan_focus_range(h, std::ptr::null_mut(), std::ptr::null_mut());
            yatate_henkan_free(h);

            let s: *mut SessionHandle = std::ptr::null_mut();
            assert_eq!(yatate_session_key(s, u32::from('a')), ACT_PASSTHROUGH);
            yatate_session_free(s);

            let g: *mut GenkiHandle = std::ptr::null_mut();
            assert_eq!(
                yatate_genki_press(g, u32::from('a'), 0, std::ptr::null_mut()),
                EDIT_UNMAPPED
            );
            yatate_genki_free(g);

            yatate_string_free(std::ptr::null_mut());
        }
    }

    #[test]
    fn 空の文字列を渡されても落ちない() {
        unsafe {
            assert_eq!(take(yatate_to_kyuji(std::ptr::null())), "");
            assert_eq!(take(yatate_type_keys(std::ptr::null())), "");
            assert_eq!(yatate_genki_of_code(std::ptr::null()), 0);
            assert_eq!(take(yatate_segment(std::ptr::null())), "");
            // 空の prev は「連なりの開始」へ落ちる（無地ではない）
            assert!(!take(yatate_kehai_field(std::ptr::null())).is_empty());
        }
    }

    #[test]
    fn 旧字は差し込むときに定まる() {
        unsafe {
            let s = c("今日は良き天気なり");
            assert_eq!(take(yatate_to_kyuji(s.as_ptr())), "今日は良き天氣なり");
        }
    }

    /// 頭書（`include/yatate_ffi.h`）と実装がずれてゐないか。
    ///
    /// **これが無いと、片方だけ直したときに黙つて壊れる**——Swift は頭書を見て
    /// 呼び、リンカは実体を探すので、食ひ違ひは「実行時に見つからない」か、
    /// もつと悪く「引数の並びが違ふまま動く」形で出る。
    /// Windows 殻がエクスポート表を検めてゐるのと同じ関門である。
    mod 頭書 {
        const HEADER: &str = include_str!("../include/yatate_ffi.h");
        const IMPL: &str = include_str!("lib.rs");

        /// 実装が `#[no_mangle]` で出してゐる名を集める。
        ///
        /// 直に書いた関数と、`act_fn!` などの雛形で起こした関数の二通りがある。
        /// 雛形の定義そのもの（`fn $name(`）は名ではないので拾はない。
        fn exported() -> Vec<String> {
            let mut names = Vec::new();
            for line in IMPL.lines() {
                let line = line.trim();
                // ① 直に書いた `extern "C" fn <名>`
                if let Some(rest) = line.split("extern \"C\" fn ").nth(1) {
                    if let Some(name) = rest.split(['(', '<']).next() {
                        if name.starts_with("yatate_") {
                            names.push(name.to_string());
                        }
                    }
                    continue;
                }
                // ② 雛形の呼び出し（`yatate_… => …` / `yatate_… -> …`）
                if line.starts_with("yatate_") {
                    if let Some(name) = line.split([' ', '(']).next() {
                        names.push(name.to_string());
                    }
                }
            }
            names.sort();
            names.dedup();
            names
        }

        #[test]
        fn 実装の名がすべて頭書に在る() {
            let names = exported();
            assert!(names.len() > 40, "拾へた名が少なすぎる: {names:?}");
            for name in &names {
                assert!(
                    HEADER.contains(&format!("{name}(")),
                    "頭書に {name} が無い（Swift から呼べない）"
                );
            }
        }

        #[test]
        fn 頭書の名がすべて実装に在る() {
            let names = exported();
            for line in HEADER.lines() {
                let Some(start) = line.find("yatate_") else {
                    continue;
                };
                if line.trim_start().starts_with('*') || line.trim_start().starts_with("#define") {
                    continue;
                }
                let Some(end) = line[start..].find('(') else {
                    continue;
                };
                let name = &line[start..start + end];
                assert!(
                    names.iter().any(|n| n == name),
                    "頭書の {name} に実体が無い（リンクで落ちる）"
                );
            }
        }

        /// 符号の値も突き合はせる（数字の写し違ひは型検査に掛からない）。
        #[test]
        fn 符号の値が一致する() {
            use super::*;
            for (name, value) in [
                ("YATATE_ACT_PASSTHROUGH", ACT_PASSTHROUGH),
                ("YATATE_ACT_UPDATE", ACT_UPDATE),
                ("YATATE_ACT_SWALLOW", ACT_SWALLOW),
                ("YATATE_ACT_COMMIT", ACT_COMMIT),
                ("YATATE_ACT_COMMIT_THEN_UPDATE", ACT_COMMIT_THEN_UPDATE),
                ("YATATE_ACT_CANCEL", ACT_CANCEL),
                ("YATATE_PHASE_KANA", PHASE_KANA),
                ("YATATE_PHASE_HENKAN", PHASE_HENKAN),
                ("YATATE_EDIT_INSERT", EDIT_INSERT),
                ("YATATE_EDIT_REPLACE_LAST", EDIT_REPLACE_LAST),
                ("YATATE_EDIT_NONE", EDIT_NONE),
                ("YATATE_EDIT_UNMAPPED", EDIT_UNMAPPED),
            ] {
                let want = format!("#define {name} {value}");
                assert!(HEADER.contains(&want), "頭書に「{want}」が無い");
            }
        }
    }

    #[test]
    fn 読みを文節へ分けられる() {
        unsafe {
            let y = c("けふはよきてんきなり");
            let text = take(yatate_segment(y.as_ptr()));
            let lines: Vec<&str> = text.lines().collect();
            assert_eq!(lines.len(), 3);
            // 最後は必ず仮名のままの控へ
            for line in &lines {
                let cols: Vec<&str> = line.split('\t').collect();
                assert_eq!(*cols.last().unwrap(), cols[0], "{line}");
            }
        }
    }
}
