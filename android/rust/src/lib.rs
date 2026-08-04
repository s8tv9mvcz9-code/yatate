//! 矢立の Android 殻の橋 — Kotlin から核（`yatate_core`）を呼ぶ JNI。
//!
//! **ここに頭脳は無い。** 打鍵の意味も変換も旧字も、決めるのは核である。
//! この crate が引き受けるのは JNI の作法だけ——Kotlin と Rust の間で
//! 文字列と整数をやり取りする、それだけ。
//!
//! ## なぜ Android だけ核を手で写さないのか
//!
//! iOS / macOS の殻は Swift なので、核の一部（`Kagi`・`Genki`・`Session`）を
//! 写して黄金ベクトルで縛つてゐる。表が小さく、機械で完全に縛れるからである。
//!
//! Android は `InputMethodService` が Kotlin だが、**共有ライブラリを読める**。
//! ゆゑに核をそのまま呼べる。すると `henkan`・`bunsetsu`・`jisho`（約 900 行、
//! 格子探索と費用計算を含む）が初日から丸ごと載る——これらは表と違ひ、
//! 手で写せば必ずずれる性質のものである（`docs/ime/cross-platform.md` §5 案 A）。
//!
//! ## 受け渡しの形
//!
//! ```text
//!   Kotlin → Rust : 打鍵は **Unicode の符号位置**（int）一つ
//!   Rust → Kotlin : 指示は **int の合図**、文字列は別に問ひ合はせる
//! ```
//!
//! 文字列を返り値に混ぜないのは、`Act::Commit(String)` のやうな
//! 「合図＋文字列」を JNI の一つの返り値で表せないからである。
//! 合図だけ返し、中身は [`Java_jp_yatate_ime_Core_nativeCommitted`] で取りに来させる。

use yatate_core::gojuon;
use yatate_core::henkan::{Act, Henkan};

/// Kotlin と共有する合図。**この数は `Core.kt` の `ACT_*` と一致してゐなければならない。**
pub const ACT_UPDATE: i32 = 0;
pub const ACT_SWALLOW: i32 = 1;
pub const ACT_PASSTHROUGH: i32 = 2;
pub const ACT_COMMIT: i32 = 3;
pub const ACT_COMMIT_THEN_UPDATE: i32 = 4;
pub const ACT_CANCEL: i32 = 5;

/// 変換の状態機械と、直前に確定した文字列。
///
/// 確定文字列を持つのは、JNI の返り値が一つしか無いためである
/// （合図を返してから Kotlin が取りに来る）。
#[derive(Default)]
pub struct Bridge {
    henkan: Henkan,
    committed: String,
}

impl Bridge {
    pub fn new() -> Self {
        Self::default()
    }

    /// 核の返した指示を合図へ落とし、確定文字列を控へる。
    pub fn absorb(&mut self, act: Act) -> i32 {
        match act {
            Act::Update => ACT_UPDATE,
            Act::Swallow => ACT_SWALLOW,
            Act::Passthrough => ACT_PASSTHROUGH,
            Act::Commit(text) => {
                self.committed = text;
                ACT_COMMIT
            }
            Act::CommitThenUpdate(text) => {
                self.committed = text;
                ACT_COMMIT_THEN_UPDATE
            }
            Act::Cancel => ACT_CANCEL,
        }
    }

    pub fn preedit(&self) -> String {
        self.henkan.preedit()
    }

    pub fn committed(&self) -> &str {
        &self.committed
    }

    pub fn is_composing(&self) -> bool {
        self.henkan.is_composing()
    }

    pub fn focus(&self) -> usize {
        self.henkan.focus()
    }

    pub fn chosen(&self) -> usize {
        self.henkan.chosen()
    }

    /// 注目してゐる文節の候補を改行区切りで。候補窓を描く殻が使ふ。
    pub fn candidates(&self) -> String {
        self.henkan
            .candidates()
            .iter()
            .map(|c| c.surface.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 注目文節の範囲（未確定文字列の何文字目から何文字か）を `start\tlen` で。
    pub fn focus_range(&self) -> String {
        let (start, len) = self.henkan.focus_range();
        format!("{start}\t{len}")
    }

    pub fn henkan_mut(&mut self) -> &mut Henkan {
        &mut self.henkan
    }
}

/// 五十音の地図を TSV で吐く。**鍵盤の図は核の表を描くだけ**にするためである。
///
/// 殻が配列表を持たなければ「図と実際の打鍵がずれる」事故は起きやうが無い
/// （web の殻が `yatate_layout()` を描くだけなのと同じ）。
///
/// ```text
///   行の名 \t 面 \t 段0 \t 段1 \t 段2 \t 段3 \t 段4
///   面 = sei | daku | ko、空きスロットは空文字
/// ```
pub fn gojuon_tsv() -> String {
    let mut out = String::new();
    for gyo in gojuon::all() {
        for (plane, slots) in [
            ("sei", Some(gyo.seion)),
            ("daku", gyo.daku),
            ("ko", gyo.ko),
        ] {
            let Some(slots) = slots else { continue };
            out.push_str(gyo.name);
            out.push('\t');
            out.push_str(plane);
            for s in slots.iter() {
                out.push('\t');
                out.push_str(s.unwrap_or(""));
            }
            out.push('\n');
        }
    }
    out
}

/// 行の並び（第一の行・第二の行）。鍵盤の列を核の順で並べるために使ふ。
pub fn lines_tsv() -> String {
    let first: Vec<&str> = gojuon::FIRST_LINE.iter().map(|g| g.name).collect();
    let second: Vec<&str> = gojuon::SECOND_LINE.iter().map(|g| g.name).collect();
    format!("first\t{}\nsecond\t{}\n", first.join("\t"), second.join("\t"))
}

// ── ここから下は Android でしか意味を持たない ─────────────────
#[cfg(target_os = "android")]
mod jni_bridge {
    use super::*;
    use jni::objects::JClass;
    use jni::sys::{jboolean, jint, jlong, jstring};
    use jni::JNIEnv;

    /// # Safety
    /// `ptr` は [`Java_jp_yatate_ime_Core_nativeNew`] が返したもので、
    /// まだ [`Java_jp_yatate_ime_Core_nativeFree`] へ渡してゐないこと。
    unsafe fn bridge<'a>(ptr: jlong) -> &'a mut Bridge {
        &mut *(ptr as *mut Bridge)
    }

    fn out(env: &mut JNIEnv, s: &str) -> jstring {
        match env.new_string(s) {
            Ok(j) => j.into_raw(),
            // 文字列が作れないのは記憶が尽きたとき。空を返して落とさない。
            Err(_) => std::ptr::null_mut(),
        }
    }

    #[no_mangle]
    pub extern "system" fn Java_jp_yatate_ime_Core_nativeNew(
        _env: JNIEnv,
        _class: JClass,
    ) -> jlong {
        Box::into_raw(Box::new(Bridge::new())) as jlong
    }

    /// # Safety
    /// 同じ `ptr` を二度渡してはならない。
    #[no_mangle]
    pub unsafe extern "system" fn Java_jp_yatate_ime_Core_nativeFree(
        _env: JNIEnv,
        _class: JClass,
        ptr: jlong,
    ) {
        if ptr != 0 {
            drop(Box::from_raw(ptr as *mut Bridge));
        }
    }

    /// 原器の一打。`cp` は Unicode の符号位置。
    ///
    /// # Safety
    /// `ptr` が生きてゐること。
    #[no_mangle]
    pub unsafe extern "system" fn Java_jp_yatate_ime_Core_nativeKey(
        _env: JNIEnv,
        _class: JClass,
        ptr: jlong,
        cp: jint,
    ) -> jint {
        let b = bridge(ptr);
        let Some(ch) = char::from_u32(cp as u32) else {
            return ACT_PASSTHROUGH;
        };
        let act = b.henkan_mut().key(ch);
        b.absorb(act)
    }

    /// 硝子の鍵盤から仮名を直に入れる（二行配列は原器の写像を経ない）。
    ///
    /// # Safety
    /// `ptr` が生きてゐること。
    #[no_mangle]
    pub unsafe extern "system" fn Java_jp_yatate_ime_Core_nativeKana(
        mut env: JNIEnv,
        _class: JClass,
        ptr: jlong,
        kana: jni::objects::JString,
    ) -> jint {
        let b = bridge(ptr);
        let Ok(s) = env.get_string(&kana) else {
            return ACT_SWALLOW;
        };
        let s: String = s.into();
        let act = b.henkan_mut().insert_kana(&s);
        b.absorb(act)
    }

    /// 引数を取らぬ操作をまとめて呼ぶ（合図の番号で選ぶ）。
    ///
    /// JNI の関数を十個並べるより、殻と橋の食ひ違ひが起きにくい。
    /// 番号は `Core.kt` の `OP_*` と一致してゐなければならない。
    ///
    /// # Safety
    /// `ptr` が生きてゐること。
    #[no_mangle]
    pub unsafe extern "system" fn Java_jp_yatate_ime_Core_nativeOp(
        _env: JNIEnv,
        _class: JClass,
        ptr: jlong,
        op: jint,
    ) -> jint {
        let b = bridge(ptr);
        let act = {
            let h = b.henkan_mut();
            match op {
                0 => h.convert(),
                1 => h.commit(),
                2 => h.cancel(),
                3 => h.backspace(),
                4 => h.focus_prev(),
                5 => h.focus_next(),
                6 => h.shrink_focus(),
                7 => h.grow_focus(),
                8 => h.prev_candidate(),
                9 => h.next_candidate(),
                10 => h.unconvert(),
                _ => Act::Swallow,
            }
        };
        b.absorb(act)
    }

    /// 候補を番号で選ぶ。
    ///
    /// # Safety
    /// `ptr` が生きてゐること。
    #[no_mangle]
    pub unsafe extern "system" fn Java_jp_yatate_ime_Core_nativeChoose(
        _env: JNIEnv,
        _class: JClass,
        ptr: jlong,
        index: jint,
    ) -> jint {
        let b = bridge(ptr);
        let act = b.henkan_mut().choose(index.max(0) as usize);
        b.absorb(act)
    }

    /// 入力欄が替はつた等で全部捨てる。
    ///
    /// # Safety
    /// `ptr` が生きてゐること。
    #[no_mangle]
    pub unsafe extern "system" fn Java_jp_yatate_ime_Core_nativeReset(
        _env: JNIEnv,
        _class: JClass,
        ptr: jlong,
    ) {
        bridge(ptr).henkan_mut().reset();
    }

    /// # Safety
    /// `ptr` が生きてゐること。
    #[no_mangle]
    pub unsafe extern "system" fn Java_jp_yatate_ime_Core_nativePreedit(
        mut env: JNIEnv,
        _class: JClass,
        ptr: jlong,
    ) -> jstring {
        let s = bridge(ptr).preedit();
        out(&mut env, &s)
    }

    /// # Safety
    /// `ptr` が生きてゐること。
    #[no_mangle]
    pub unsafe extern "system" fn Java_jp_yatate_ime_Core_nativeCommitted(
        mut env: JNIEnv,
        _class: JClass,
        ptr: jlong,
    ) -> jstring {
        let s = bridge(ptr).committed().to_string();
        out(&mut env, &s)
    }

    /// # Safety
    /// `ptr` が生きてゐること。
    #[no_mangle]
    pub unsafe extern "system" fn Java_jp_yatate_ime_Core_nativeCandidates(
        mut env: JNIEnv,
        _class: JClass,
        ptr: jlong,
    ) -> jstring {
        let s = bridge(ptr).candidates();
        out(&mut env, &s)
    }

    /// # Safety
    /// `ptr` が生きてゐること。
    #[no_mangle]
    pub unsafe extern "system" fn Java_jp_yatate_ime_Core_nativeFocusRange(
        mut env: JNIEnv,
        _class: JClass,
        ptr: jlong,
    ) -> jstring {
        let s = bridge(ptr).focus_range();
        out(&mut env, &s)
    }

    /// # Safety
    /// `ptr` が生きてゐること。
    #[no_mangle]
    pub unsafe extern "system" fn Java_jp_yatate_ime_Core_nativeIsComposing(
        _env: JNIEnv,
        _class: JClass,
        ptr: jlong,
    ) -> jboolean {
        u8::from(bridge(ptr).is_composing())
    }

    /// # Safety
    /// `ptr` が生きてゐること。
    #[no_mangle]
    pub unsafe extern "system" fn Java_jp_yatate_ime_Core_nativeChosen(
        _env: JNIEnv,
        _class: JClass,
        ptr: jlong,
    ) -> jint {
        bridge(ptr).chosen() as jint
    }

    #[no_mangle]
    pub extern "system" fn Java_jp_yatate_ime_Core_nativeGojuon(
        mut env: JNIEnv,
        _class: JClass,
    ) -> jstring {
        out(&mut env, &gojuon_tsv())
    }

    #[no_mangle]
    pub extern "system" fn Java_jp_yatate_ime_Core_nativeLines(
        mut env: JNIEnv,
        _class: JClass,
    ) -> jstring {
        out(&mut env, &lines_tsv())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 橋は合図を落とすだけで、判断は一つもしないこと。
    #[test]
    fn 合図の番号が重複してゐない() {
        let all = [
            ACT_UPDATE,
            ACT_SWALLOW,
            ACT_PASSTHROUGH,
            ACT_COMMIT,
            ACT_COMMIT_THEN_UPDATE,
            ACT_CANCEL,
        ];
        let mut seen = all;
        seen.sort_unstable();
        let n = seen.len();
        let mut dedup = seen.to_vec();
        dedup.dedup();
        assert_eq!(dedup.len(), n, "合図の番号が衝突してゐる");
    }

    fn typed(keys: &str) -> Bridge {
        let mut b = Bridge::new();
        for c in keys.chars() {
            let act = b.henkan_mut().key(c);
            b.absorb(act);
        }
        b
    }

    #[test]
    fn 打鍵から確定まで橋を通る() {
        let mut b = typed("udg^yo2^^ot^4");
        assert_eq!(b.preedit(), "けふはよきてんきなり");
        assert!(b.is_composing());

        // Space（op 0）で変換
        let act = b.henkan_mut().convert();
        assert_eq!(b.absorb(act), ACT_UPDATE);
        assert!(!b.candidates().is_empty(), "候補が取れる");

        // Enter（op 1）で確定 — **旧字はここで定まる**
        let act = b.henkan_mut().commit();
        assert_eq!(b.absorb(act), ACT_COMMIT);
        assert_eq!(b.committed(), "今日は良き天氣なり");
        assert!(!b.is_composing());
    }

    #[test]
    fn 確定文字列は合図の後に取りに来られる() {
        // JNI の返り値は一つなので、合図と文字列を分けてゐる。
        // 合図を返した後に取りに来ても中身が残つてゐること。
        let mut b = typed("0");
        let act = b.henkan_mut().commit();
        assert_eq!(b.absorb(act), ACT_COMMIT);
        assert_eq!(b.committed(), "あ");
        assert_eq!(b.committed(), "あ", "二度読んでも消えない");
    }

    #[test]
    fn 候補は改行区切りで並ぶ() {
        let mut b = typed("udg^yo2^^ot^4");
        let act = b.henkan_mut().convert();
        b.absorb(act);
        let joined = b.candidates();
        let cands: Vec<&str> = joined.split('\n').collect();
        assert!(cands.len() > 1, "控へを含めて二つ以上ある");
        assert!(
            cands.contains(&"けふは"),
            "全部仮名の控へが残つてゐる: {cands:?}"
        );
    }

    #[test]
    fn 注目文節の範囲が取れる() {
        let mut b = typed("udg^yo2^^ot^4");
        let act = b.henkan_mut().convert();
        b.absorb(act);
        let r = b.focus_range();
        let cols: Vec<&str> = r.split('\t').collect();
        assert_eq!(cols.len(), 2, "start\\tlen の形");
        assert_eq!(cols[0], "0", "注目は先頭から始まる");
        assert!(cols[1].parse::<usize>().unwrap() > 0);
    }

    /// **鍵盤の図は核の表を描くだけ**にするための出口。
    #[test]
    fn 五十音の地図が吐ける() {
        let tsv = gojuon_tsv();
        assert!(tsv.contains("あ\tsei\tあ\tい\tう\tえ\tお"), "{tsv}");
        assert!(tsv.contains("か\tdaku\tが\tぎ\tぐ\tげ\tご"), "{tsv}");
        assert!(tsv.contains("は\tko\t"), "は行だけが半濁点を持つ");
        // ゐ・ゑ は第一級（わ行の 1・3 段目）
        assert!(tsv.contains("わ\tsei\tわ\tゐ\t\tゑ\tを"), "{tsv}");
        // 全 10 行が清音の面を持つ
        assert_eq!(tsv.lines().filter(|l| l.contains("\tsei\t")).count(), 10);
    }

    #[test]
    fn 行の並びが核の順である() {
        let t = lines_tsv();
        assert!(t.contains("first\tあ\tか\tさ\tた\tな"), "{t}");
        assert!(t.contains("second\tは\tま\tや\tら\tわ"), "{t}");
    }
}
