//! 矢立の web 殻 — 核を wasm へ載せ、**素の C ABI** で JS へ出す。
//!
//! ## wasm-bindgen を使はない
//!
//! 核の入出力は「文字を入れる／文字列を貰ふ」だけである。
//! それだけなら `extern "C"` で足り、道具立ては **cargo だけ**で済む
//! （`wasm-pack` も `npm` も要らない）。CI は ubuntu（課金 1 倍）のまま増えない。
//! 殻がまた一つ増えても核は一つ、といふ `docs/ime/cross-platform.md` の骨格も崩れない。
//!
//! ## 文字列の渡し方
//!
//! wasm の線形メモリを JS と共有する。手順は二段だけである:
//!
//! ```text
//!   JS → wasm : yatate_alloc(len) で場所を貰ひ、UTF-8 を書き込んで渡す
//!   wasm → JS : 関数は長さを返す。中身は yatate_out_ptr() の先に UTF-8 で置いてある
//! ```
//!
//! 戻り値の文字列は**次の呼び出しまで**有効である（殻が持つ一枚の帯を使ひ回す）。
//!
//! ## 鍵は `code` で取る
//!
//! `KeyboardEvent` には `key`（出る文字）と `code`（物理位置）があり、**`code` を使ふ**。
//! 原器は指の位置の地図だからで、`key` を見ると JIS と US で意味の入れ替はる三鍵
//! （さ・し・前置シフト）が壊れる。とりわけ「し」は刻印も物理位置も同じなので、
//! 気づかぬまま「さ」が出る——例外も警告も出ない。
//!
//! 位置の地図は**核が持つ**（`yatate_core::kagi`）。Windows の殻も同じ地図に従ふので、
//! 一手で二つの殻の誤爆を同時に殺せる。ここに写しは置かない。

use std::slice;
use std::str;

use yatate_core::bunsetsu;
use yatate_core::genki::{FIRST_PLANE, SECOND_PLANE, SHIFT};
use yatate_core::gojuon::{self, Deflect};
use yatate_core::jisho;
use yatate_core::kagi;
use yatate_core::kehai::KeyId;
use yatate_core::kyuji::to_kyuji;
use yatate_core::session::{KeyAction, Session};

/// 一つの入力欄ぶんの状態。JS は不透明な手綱として持つ。
pub struct Shell {
    session: Session,
    /// 直前の呼び出しが書いた文字列。JS が `out_ptr`/`out_len` で読む。
    out: String,
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

impl Shell {
    pub fn new() -> Self {
        Self {
            session: Session::new(),
            out: String::new(),
        }
    }

    fn put(&mut self, s: String) -> usize {
        self.out = s;
        self.out.len()
    }
}

// ── 手綱と記憶 ──────────────────────────────────────────────

/// 入力欄を一つ起こす。使ひ終へたら [`yatate_free`] へ渡すこと。
#[no_mangle]
pub extern "C" fn yatate_new() -> *mut Shell {
    Box::into_raw(Box::new(Shell::new()))
}

/// 入力欄を畳む。
///
/// # Safety
/// `h` は [`yatate_new`] が返した手綱で、まだ畳んでゐないものであること。
#[no_mangle]
pub unsafe extern "C" fn yatate_free(h: *mut Shell) {
    if !h.is_null() {
        drop(unsafe { Box::from_raw(h) });
    }
}

/// JS が文字列を書き込むための場所を借りる。
#[no_mangle]
pub extern "C" fn yatate_alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// 借りた場所を返す。
///
/// # Safety
/// `ptr` と `len` は [`yatate_alloc`] へ渡した組と同じであること。
#[no_mangle]
pub unsafe extern "C" fn yatate_dealloc(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        drop(unsafe { Vec::from_raw_parts(ptr, 0, len) });
    }
}

/// 直前の呼び出しが書いた文字列の先頭（UTF-8）。
///
/// # Safety
/// `h` は生きてゐる手綱であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_out_ptr(h: *mut Shell) -> *const u8 {
    match unsafe { h.as_ref() } {
        Some(s) => s.out.as_ptr(),
        None => std::ptr::null(),
    }
}

/// 同、長さ（バイト）。
///
/// # Safety
/// `h` は生きてゐる手綱であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_out_len(h: *mut Shell) -> usize {
    match unsafe { h.as_ref() } {
        Some(s) => s.out.len(),
        None => 0,
    }
}

/// # Safety
/// `ptr`/`len` は生きてゐる UTF-8 の並びを指すこと。
unsafe fn as_str<'a>(ptr: *const u8, len: usize) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    str::from_utf8(unsafe { slice::from_raw_parts(ptr, len) }).ok()
}

// ── 打鍵 ────────────────────────────────────────────────────

/// [`yatate_press`] の戻り値。
pub const PRESS_PASSTHROUGH: i32 = 0;
/// 未確定文字列が変はつた（殻は描き直す）。
pub const PRESS_UPDATE: i32 = 1;
/// 呑んだ（前置シフトが立つた等）。殻は既定の動作を止めるが、描き直しは要らない。
pub const PRESS_SWALLOW: i32 = 2;

/// `KeyboardEvent.code` を一つ食はせる。
///
/// 原器の鍵でなければ [`PRESS_PASSTHROUGH`] を返し、**状態に触れない**
/// （Ctrl+C や Tab を食つてはいけない——判断は殻が先にする）。
///
/// # Safety
/// `h` は生きてゐる手綱、`code_ptr`/`code_len` は UTF-8 の並びであること。
#[no_mangle]
pub unsafe extern "C" fn yatate_press(h: *mut Shell, code_ptr: *const u8, code_len: usize) -> i32 {
    let Some(shell) = (unsafe { h.as_mut() }) else {
        return PRESS_PASSTHROUGH;
    };
    let Some(code) = (unsafe { as_str(code_ptr, code_len) }) else {
        return PRESS_PASSTHROUGH;
    };
    // 位置の地図は核が持つ。ここに写しを置かない（置けば必ずずれる）。
    let Some(ch) = kagi::genki_of_code(code) else {
        return PRESS_PASSTHROUGH;
    };
    match shell.session.key(ch) {
        KeyAction::Update => PRESS_UPDATE,
        KeyAction::Swallow => PRESS_SWALLOW,
        // 原器の鍵しか渡してゐないので、ここへは来ない
        KeyAction::Passthrough | KeyAction::Commit(_) => PRESS_PASSTHROUGH,
    }
}

/// 一字消す。まだ未確定文字列が残つてゐれば 1、空になつたら 0。
///
/// # Safety
/// `h` は生きてゐる手綱であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_backspace(h: *mut Shell) -> i32 {
    match unsafe { h.as_mut() } {
        Some(s) => i32::from(s.session.backspace()),
        None => 0,
    }
}

/// 取り消し（Esc）。未確定文字列を捨てる。
///
/// # Safety
/// `h` は生きてゐる手綱であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_cancel(h: *mut Shell) {
    if let Some(s) = unsafe { h.as_mut() } {
        s.session.cancel();
    }
}

/// いま未確定の仮名を `out` へ書き、長さを返す。
///
/// # Safety
/// `h` は生きてゐる手綱であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_preedit(h: *mut Shell) -> usize {
    match unsafe { h.as_mut() } {
        Some(s) => {
            let t = s.session.preedit().to_string();
            s.put(t)
        }
        None => 0,
    }
}

/// 前置シフトが立つてゐるか（殻が面の表示を替へる手掛かり）。
///
/// # Safety
/// `h` は生きてゐる手綱であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_is_shifted(h: *mut Shell) -> i32 {
    match unsafe { h.as_ref() } {
        Some(s) => i32::from(s.session.is_shifted()),
        None => 0,
    }
}

/// 確定 — 未確定の仮名を `out` へ書き出し、作業帯を空ける。
///
/// **旧字体はここでは定めない。** web 殻は確定した仮名を「読みを持つた塊」として
/// 編輯欄へ置き、使ひ手の手修正を読みごと覚える（`docs/ime/web.md` §4）。
/// 読みを旧字へ潰してしまふと、覚えるべきものが失はれる。
/// 表記を差し込むときは [`yatate_kyuji`] を通す。
///
/// # Safety
/// `h` は生きてゐる手綱であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_commit(h: *mut Shell) -> usize {
    match unsafe { h.as_mut() } {
        Some(s) => match s.session.commit() {
            KeyAction::Commit(text) => s.put(text),
            _ => s.put(String::new()),
        },
        None => 0,
    }
}

// ── 変換と学習の助け ────────────────────────────────────────

/// 新字体を旧字体へ（確定のときに機械で定まる部分）。
///
/// 覚えた表記に新字体が混じつてゐても、差し込むときにここを通せば旧字体へ直る。
/// **使ひ手が旧字を覚える必要は無い。**
///
/// # Safety
/// `h` は生きてゐる手綱、`ptr`/`len` は UTF-8 の並びであること。
#[no_mangle]
pub unsafe extern "C" fn yatate_kyuji(h: *mut Shell, ptr: *const u8, len: usize) -> usize {
    let Some(shell) = (unsafe { h.as_mut() }) else {
        return 0;
    };
    match unsafe { as_str(ptr, len) } {
        Some(text) => {
            let out = to_kyuji(text);
            shell.put(out)
        }
        None => shell.put(String::new()),
    }
}

/// 読みで辞書を引く。**打つてゐる最中の提案**に使ふ。
///
/// 一行一候補の TSV（`表記\t度数`）。当たらなければ空を返す
/// ——覚えが無ければ何も出さない（空の候補窓を出さない）。
///
/// # Safety
/// `h` は生きてゐる手綱、`ptr`/`len` は UTF-8 の並びであること。
#[no_mangle]
pub unsafe extern "C" fn yatate_suggest(h: *mut Shell, ptr: *const u8, len: usize) -> usize {
    let Some(shell) = (unsafe { h.as_mut() }) else {
        return 0;
    };
    let Some(yomi) = (unsafe { as_str(ptr, len) }) else {
        return shell.put(String::new());
    };
    let mut out = String::new();
    for w in jisho::lookup(yomi) {
        out.push_str(w.surface);
        out.push('\t');
        out.push_str(&w.freq.to_string());
        out.push('\n');
    }
    shell.put(out)
}

/// 読みの列を文節へ分ける（変換）。
///
/// 一行一文節の TSV: `読み\t既定の候補番号\t候補1\t候補2\t…`。
/// 候補は費用の昇順で、最後に必ず仮名のままの控へが入る。
///
/// # Safety
/// `h` は生きてゐる手綱、`ptr`/`len` は UTF-8 の並びであること。
#[no_mangle]
pub unsafe extern "C" fn yatate_convert(h: *mut Shell, ptr: *const u8, len: usize) -> usize {
    let Some(shell) = (unsafe { h.as_mut() }) else {
        return 0;
    };
    let Some(yomi) = (unsafe { as_str(ptr, len) }) else {
        return shell.put(String::new());
    };
    let mut out = String::new();
    for seg in bunsetsu::segment(yomi) {
        out.push_str(&seg.yomi);
        out.push('\t');
        out.push_str(&seg.chosen.to_string());
        for c in &seg.candidates {
            out.push('\t');
            out.push_str(&c.surface);
        }
        out.push('\n');
    }
    shell.put(out)
}

/// 原器の図を描くための表。一行一鍵の TSV: `code\t第一面\t第二面`。
///
/// **頁は配列表を持たない。** 稽古の図はここが返すものを描くだけなので、
/// 「頁の図と実際の打鍵がずれる」といふ事故がそもそも起きない。
/// 面に無い鍵（前置シフト・濁点・半濁点）は仮名の欄が空になる。
///
/// # Safety
/// `h` は生きてゐる手綱であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_layout(h: *mut Shell) -> usize {
    let Some(shell) = (unsafe { h.as_mut() }) else {
        return 0;
    };
    let mut out = String::new();
    for k in kagi::KAGI.iter() {
        let find = |plane: &[(char, &'static str)]| {
            plane
                .iter()
                .find(|(key, _)| *key == k.genki)
                .map(|(_, kana)| *kana)
                .unwrap_or("")
        };
        out.push_str(k.code);
        out.push('\t');
        out.push_str(find(&FIRST_PLANE[..]));
        out.push('\t');
        out.push_str(find(&SECOND_PLANE[..]));
        out.push('\n');
    }
    shell.put(out)
}

/// 墨の氣配 — 次の一打の濃淡。一行一鍵の TSV（`code\t墨`）。
///
/// 核の氣配（`core/src/kehai.rs`）は**行の墨**と**行の中の段の墨**で来る
/// ——二行配列（硝子）では行が一つの鍵だからである。原器では一鍵が一字なので、
/// **行の墨 × 段の墨**を掛けて鍵へ落とす。掛けるだけで、新しい重みは発明しない。
///
/// 面の扱ひ:
///
/// - いまの面で打てる字は、その鍵へ
/// - 前置シフトの先にある字（ま・や・ら・わ行）は、**前置シフトの鍵**へ寄せる
///   ——「次は `^` らしい」が濃淡で見えるやうに
/// - 「ん」は行にも段にも属さず `^^` に住むので、同じく前置シフトの鍵へ
/// - 句読点は原器で置き場が**未定**なので、ここでも決めない（発明しない）
///
/// # Safety
/// `h` は生きてゐる手綱であること。
#[no_mangle]
pub unsafe extern "C" fn yatate_kehai(h: *mut Shell) -> usize {
    let Some(shell) = (unsafe { h.as_mut() }) else {
        return 0;
    };
    let field = shell.session.field();
    let shifted = shell.session.is_shifted();

    // 挿入順のまま持つ（核の氣配が正準順序で来るので、出力も決定的になる）
    let mut acc: Vec<(&'static str, f64)> = Vec::new();
    for (id, gyo_ink) in field.ink.iter() {
        match id {
            KeyId::Moji(c) => {
                if *c == 'ん' {
                    if let Some(code) = kagi::code_of(SHIFT) {
                        soak(&mut acc, code, *gyo_ink);
                    }
                }
            }
            KeyId::Gyo(gyo) => {
                let Some(g) = gojuon::gyo_named(gyo) else {
                    continue;
                };
                for (dan, dan_ink) in field.dan_of(gyo).iter().enumerate() {
                    let ink = gyo_ink * dan_ink;
                    if ink <= 0.0 {
                        continue;
                    }
                    let Some(kana) = g.kana(dan, Deflect::None).and_then(|s| s.chars().next())
                    else {
                        continue;
                    };
                    match (key_of_kana(kana, false), key_of_kana(kana, true)) {
                        // いまの面で直に打てる
                        (Some(k), _) if !shifted => stroke(&mut acc, k, ink),
                        (_, Some(k)) if shifted => stroke(&mut acc, k, ink),
                        // 前置シフトの先にある — シフトの鍵を濃くする
                        (None, Some(_)) => {
                            if let Some(code) = kagi::code_of(SHIFT) {
                                soak(&mut acc, code, ink);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    let mut out = String::new();
    for (code, ink) in acc {
        out.push_str(code);
        out.push('\t');
        // 小数は 3 桁で切る（言語ごとの既定書式で揺れないやうに）
        out.push_str(&format!("{ink:.3}"));
        out.push('\n');
    }
    shell.put(out)
}

/// 鍵へ墨を置く（同じ鍵が二度来たら濃い方を採る）。
fn soak(acc: &mut Vec<(&'static str, f64)>, code: &'static str, ink: f64) {
    match acc.iter_mut().find(|(c, _)| *c == code) {
        Some((_, cur)) => {
            if ink > *cur {
                *cur = ink;
            }
        }
        None => acc.push((code, ink)),
    }
}

/// 原器の鍵を `code` へ直してから置く。
fn stroke(acc: &mut Vec<(&'static str, f64)>, key: char, ink: f64) {
    if let Some(code) = kagi::code_of(key) {
        soak(acc, code, ink);
    }
}

/// 仮名から、その面でそれを出す原器の鍵を引く。
///
/// 表は核（`genki`）のものをそのまま舐める。ここに写しは作らない。
fn key_of_kana(kana: char, shifted: bool) -> Option<char> {
    let plane: &[(char, &str)] = if shifted {
        &SECOND_PLANE[..]
    } else {
        &FIRST_PLANE[..]
    };
    let mut buf = [0u8; 4];
    let s: &str = kana.encode_utf8(&mut buf);
    plane.iter().find(|(_, k)| *k == s).map(|(key, _)| *key)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// C ABI を JS から呼ぶのと同じ形でなぞる小道具。
    fn press(h: *mut Shell, code: &str) -> i32 {
        unsafe { yatate_press(h, code.as_ptr(), code.len()) }
    }

    fn out(h: *mut Shell) -> String {
        unsafe {
            let ptr = yatate_out_ptr(h);
            let len = yatate_out_len(h);
            if ptr.is_null() || len == 0 {
                return String::new();
            }
            str::from_utf8(slice::from_raw_parts(ptr, len))
                .unwrap()
                .to_string()
        }
    }

    fn shell() -> *mut Shell {
        yatate_new()
    }

    #[test]
    fn 物理位置で原器が打てる() {
        let h = shell();
        // 「けふはよきてんきなり」= u d g ^y o 2 ^^ o t ^4
        for code in [
            "KeyU", "KeyD", "KeyG", "Equal", "KeyY", "KeyO", "Digit2", "Equal", "Equal", "KeyO",
            "KeyT", "Equal", "Digit4",
        ] {
            press(h, code);
        }
        unsafe { yatate_preedit(h) };
        assert_eq!(out(h), "けふはよきてんきなり");
        unsafe { yatate_free(h) };
    }

    #[test]
    fn 配列で意味の変はる三鍵が位置で決まる() {
        // 「し」は刻印も物理位置も同じなのに配列で別物になる鍵。
        // key で引いてゐたら、ここが「さ」になる。
        for (code, want) in [("Quote", "さ"), ("Semicolon", "し")] {
            let h = shell();
            press(h, code);
            unsafe { yatate_preedit(h) };
            assert_eq!(out(h), want, "{code}");
            unsafe { yatate_free(h) };
        }
        // Equal は前置シフト。単体では字を出さず、呑む
        let h = shell();
        assert_eq!(press(h, "Equal"), PRESS_SWALLOW);
        assert_eq!(unsafe { yatate_is_shifted(h) }, 1);
        unsafe { yatate_free(h) };
    }

    #[test]
    fn 原器に無い鍵は素通しする() {
        let h = shell();
        for code in ["Space", "Enter", "Tab", "KeyZ", "ControlLeft"] {
            assert_eq!(press(h, code), PRESS_PASSTHROUGH, "{code}");
        }
        unsafe { yatate_preedit(h) };
        assert_eq!(out(h), "", "素通しは状態に触れない");
        unsafe { yatate_free(h) };
    }

    #[test]
    fn 確定は仮名のまま返る() {
        // 読みを持つた塊として編輯欄へ置くので、ここで旧字へ潰してはいけない
        let h = shell();
        for code in ["KeyU", "KeyD"] {
            press(h, code);
        }
        unsafe { yatate_commit(h) };
        assert_eq!(out(h), "けふ");
        unsafe { yatate_preedit(h) };
        assert_eq!(out(h), "", "確定で作業帯が空く");
        unsafe { yatate_free(h) };
    }

    #[test]
    fn 旧字は差し込むときに定まる() {
        let h = shell();
        let s = "今日は良き天気なり";
        unsafe { yatate_kyuji(h, s.as_ptr(), s.len()) };
        assert_eq!(out(h), "今日は良き天氣なり");
        unsafe { yatate_free(h) };
    }

    #[test]
    fn 一字消せる() {
        let h = shell();
        press(h, "KeyU");
        press(h, "KeyD");
        assert_eq!(unsafe { yatate_backspace(h) }, 1);
        assert_eq!(unsafe { yatate_backspace(h) }, 0, "空になつた");
        unsafe { yatate_free(h) };
    }

    #[test]
    fn 取り消せる() {
        let h = shell();
        press(h, "KeyU");
        press(h, "Equal");
        unsafe { yatate_cancel(h) };
        unsafe { yatate_preedit(h) };
        assert_eq!(out(h), "");
        assert_eq!(unsafe { yatate_is_shifted(h) }, 0);
        unsafe { yatate_free(h) };
    }

    #[test]
    fn 辞書を引ける() {
        let h = shell();
        let y = "けふ";
        unsafe { yatate_suggest(h, y.as_ptr(), y.len()) };
        assert_eq!(out(h), "今日\t900\n");

        // 覚えが無ければ何も出さない（空の候補窓を出さない）
        let y = "ぷりん";
        unsafe { yatate_suggest(h, y.as_ptr(), y.len()) };
        assert_eq!(out(h), "");
        unsafe { yatate_free(h) };
    }

    #[test]
    fn 変換が文節ごとに返る() {
        let h = shell();
        let y = "けふはよきてんきなり";
        unsafe { yatate_convert(h, y.as_ptr(), y.len()) };
        let text = out(h);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("けふは\t0\t今日は\t"), "{}", lines[0]);
        // 最後は必ず仮名のままの控へ
        for line in &lines {
            let cols: Vec<&str> = line.split('\t').collect();
            assert_eq!(*cols.last().unwrap(), cols[0], "{line}");
        }
        unsafe { yatate_free(h) };
    }

    #[test]
    fn 氣配が鍵の位置で返る() {
        let h = shell();
        press(h, "KeyT"); // な
        unsafe { yatate_kehai(h) };
        let text = out(h);
        assert!(!text.is_empty(), "「な」の後には次の一打の分布がある");
        for line in text.lines() {
            let (code, ink) = line.split_once('\t').expect("code\\tink");
            assert!(
                kagi::genki_of_code(code).is_some(),
                "原器に無い鍵へ墨を流してゐる: {code}"
            );
            assert!(ink.parse::<f64>().is_ok(), "{ink}");
        }
        unsafe { yatate_free(h) };
    }

    #[test]
    fn 配列の図が核から出る() {
        let h = shell();
        unsafe { yatate_layout(h) };
        let text = out(h);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), kagi::KAGI.len(), "33 鍵ちやうど");

        let mut first = 0;
        let mut second = 0;
        for line in &lines {
            let cols: Vec<&str> = line.split('\t').collect();
            assert_eq!(cols.len(), 3, "{line}");
            assert!(kagi::genki_of_code(cols[0]).is_some(), "{}", cols[0]);
            if !cols[1].is_empty() {
                first += 1;
            }
            if !cols[2].is_empty() {
                second += 1;
            }
        }
        assert_eq!(first, FIRST_PLANE.len(), "第一面 30 字");
        assert_eq!(second, SECOND_PLANE.len(), "第二面 20 字");

        // 図が実際の打鍵と食ひ違はないこと（頁が写しを持たない理由がここ）
        assert!(text.contains("Quote\tさ\t"), "Quote は さ");
        assert!(text.contains("Semicolon\tし\t"), "Semicolon は し");
        assert!(text.contains("Equal\t\t\n"), "前置シフトは面に字を持たない");
        unsafe { yatate_free(h) };
    }

    #[test]
    fn 手綱が無くても落ちない() {
        // JS 側の取り回しを誤つても wasm を落とさない（頁ごと死ぬのを防ぐ）
        let null: *mut Shell = std::ptr::null_mut();
        assert_eq!(press(null, "KeyA"), PRESS_PASSTHROUGH);
        assert_eq!(unsafe { yatate_preedit(null) }, 0);
        assert_eq!(unsafe { yatate_out_len(null) }, 0);
        assert_eq!(unsafe { yatate_backspace(null) }, 0);
        unsafe { yatate_cancel(null) };
        unsafe { yatate_free(null) };
    }

    #[test]
    fn 壊れた並びを渡されても落ちない() {
        let h = shell();
        let bad: [u8; 2] = [0xff, 0xfe];
        assert_eq!(
            unsafe { yatate_press(h, bad.as_ptr(), bad.len()) },
            PRESS_PASSTHROUGH
        );
        assert_eq!(unsafe { yatate_kyuji(h, bad.as_ptr(), bad.len()) }, 0);
        assert_eq!(unsafe { yatate_suggest(h, bad.as_ptr(), bad.len()) }, 0);
        unsafe { yatate_free(h) };
    }
}
