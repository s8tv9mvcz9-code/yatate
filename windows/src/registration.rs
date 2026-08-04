//! TSF への登録（`docs/ime/cross-platform.md` §4）。
//!
//! テキストサービスは二段で名乗る。
//!
//! 1. **COM サーバとして** — `HKCR\CLSID\{CLSID}\InprocServer32` に DLL の道を書く
//! 2. **入力プロファイルとして** — `ITfInputProcessorProfiles::Register` ＋
//!    `AddLanguageProfile`、さらに `ITfCategoryMgr::RegisterCategory` で
//!    「これは鍵盤である・候補窓は自前で描く」と宣言する
//!
//! 値（GUID・言語 ID・名）は OS を知らないので **Linux でも試験できる**。
//! 実際に書き込む手続きだけが `cfg(windows)` に閉ぢてゐる。
//!
//! ## 署名について
//!
//! Microsoft はテキストサービスのバイナリに**デジタル署名**を求めてゐる。
//! 手元での検証は未署名でも通るが、配布の前提としては署名を用意する。
//! OSS は SignPath Foundation の無償証明書といふ道があり、
//! 先行例（`windows-chewing-tsf`）が実際にそれで配布してゐる。
//! **x64 と ARM64 の両方**を出すこと——ARM64 機のネイティブ処理
//! （メモ帳・Edge 等）に x64 の DLL は読み込まれない。

/// テキストサービスの CLSID。**一度配つたら変へられない**（利用者の登録が壊れる）。
///
/// 文字列形（レジストリの道に使ふ）と数値形（COM に渡す）の**両方**を持つ。
/// 二つが食ひ違ふと「登録はされたのに COM が見つけられない」といふ
/// 症状の見え難い壊れ方をするので、[`tests`] で一致を縛つてある。
pub const CLSID_YATATE_TEXT_SERVICE: &str = "{3F3A263D-9E15-42A5-BFCF-EE776BCD5EE9}";
/// 同上の数値形。
pub const CLSID_YATATE_TEXT_SERVICE_U128: u128 = 0x3F3A263D_9E15_42A5_BFCF_EE776BCD5EE9;

/// 入力プロファイルの GUID（言語プロファイルの識別子）。
pub const GUID_YATATE_PROFILE: &str = "{453BE216-4DC7-49A6-B630-096AFED51D69}";
/// 同上の数値形。
pub const GUID_YATATE_PROFILE_U128: u128 = 0x453BE216_4DC7_49A6_B630_096AFED51D69;

/// 未確定文字列の表示属性（下線・色）の GUID。
///
/// 矢立はここに**墨の氣配**と**共感覚（情調 → 伝統色）**を載せる余地を持つ
/// （`docs/ime/vla.md`）。TSF は表示属性を GUID で識別するので、
/// 属性を増やすときはこの隣に GUID を足す。
pub const GUID_YATATE_DISPLAY_ATTRIBUTE_INPUT: &str = "{18051C39-4219-4ECF-9B33-73AF945A8B46}";

/// 言語 ID（日本語・日本）。TSF のプロファイルはこの言語に属す。
pub const LANGID_JA_JP: u16 = 0x0411;

/// 入力方式一覧に出る名。
pub const PROFILE_DESCRIPTION: &str = "矢立（文語 IME）";

/// COM の apartment 種別。TIP は STA（各スレッドに 1 つ）で動く。
pub const THREADING_MODEL: &str = "Apartment";

/// TSF のカテゴリ登録に渡すもの。
///
/// - `GUID_TFCAT_TIP_KEYBOARD` … 鍵盤としてのテキストサービス
/// - `GUID_TFCAT_TIPCAP_UIELEMENTENABLED` … 候補窓を**自前で描く**宣言。
///   TSF は候補窓を出してくれないので、これは飾りでなく実務上の必須事項である。
/// - `GUID_TFCAT_TIPCAP_SECUREMODE` … 保護された入力欄（UAC・ログオン画面）でも動かす
pub const REQUIRED_CATEGORIES: [&str; 3] = [
    "GUID_TFCAT_TIP_KEYBOARD",
    "GUID_TFCAT_TIPCAP_UIELEMENTENABLED",
    "GUID_TFCAT_TIPCAP_SECUREMODE",
];

/// `HKCR\CLSID\{CLSID}` の下に作る鍵の道（`DllRegisterServer` が書く）。
pub fn clsid_key_path() -> String {
    format!("CLSID\\{CLSID_YATATE_TEXT_SERVICE}")
}

/// 同 `InprocServer32`（DLL の実体を指す）。
pub fn inproc_key_path() -> String {
    format!("CLSID\\{CLSID_YATATE_TEXT_SERVICE}\\InprocServer32")
}

/// UTF-16 の緩衝を **NUL 終端付き**で作る。
///
/// `ITfInputProcessorProfiles::AddLanguageProfile` は文字列と長さを別々に受け取るが、
/// **実機の TSF はアイコンの道を `wcslen` で読む**。終端の無い緩衝を渡すと隣の
/// 解放済みヒープまで読み進み、その中身が機械全体の
/// `HKLM\SOFTWARE\Microsoft\CTF\TIP\{CLSID}\LanguageProfile\…\IconFile` へ
/// 書き出される。
///
/// 2026-08-04 に ARM64 実機で観測した実害:
///
/// ```text
///   期待 42 字: C:\Program Files\Yatate\yatate_windows.dll
///   実際 69 字: C:\Program Files\Yatate\yatate_windows.dllCF-EE776BCD5EE9}\Inpr…
///                                                         ^^^ 直前に作つた
///                                                         InprocServer32 鍵の道の残骸
/// ```
///
/// 同じ呼びの `Description` は長さちやうどで正しかつたので、長さを守る引数と
/// 守らない引数が混在してゐる。**両方に正しく見える形**（終端を持たせ、
/// 長さは終端を含めない）で渡すのが唯一安全な渡し方である。
pub fn wide_nul(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// [`wide_nul`] の緩衝から「終端を含まない長さ」の切片を取る。
///
/// 長さを守る実装にはこの切片の長さが、`wcslen` する実装には切片の直後に在る
/// NUL が効く。
pub fn wide_body(buf: &[u16]) -> &[u16] {
    &buf[..buf.len().saturating_sub(1)]
}

// ── ここから下は Windows でしか意味を持たない ─────────────────
#[cfg(windows)]
mod imp {
    use super::*;
    use windows::core::{GUID, HSTRING, PCWSTR};
    use windows::Win32::Foundation::{HMODULE, MAX_PATH};
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
    use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_CLASSES_ROOT,
        KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
    };
    use windows::Win32::UI::TextServices::{
        CLSID_TF_CategoryMgr, CLSID_TF_InputProcessorProfiles, ITfCategoryMgr,
        ITfInputProcessorProfiles, GUID_TFCAT_TIPCAP_SECUREMODE,
        GUID_TFCAT_TIPCAP_UIELEMENTENABLED, GUID_TFCAT_TIP_KEYBOARD,
    };

    /// 矢立の CLSID（COM に渡す数値形）。
    pub const fn clsid() -> GUID {
        GUID::from_u128(CLSID_YATATE_TEXT_SERVICE_U128)
    }

    /// 入力プロファイルの GUID（同上）。
    pub const fn profile_guid() -> GUID {
        GUID::from_u128(GUID_YATATE_PROFILE_U128)
    }

    /// このモジュール（DLL）の実体の道。`InprocServer32` に書く値。
    pub fn module_path(hinstance: isize) -> windows::core::Result<String> {
        let mut buf = [0u16; MAX_PATH as usize];
        // SAFETY: buf は MAX_PATH 分あり、戻り値で実長を受ける。
        let len = unsafe { GetModuleFileNameW(Some(HMODULE(hinstance as *mut _)), &mut buf) };
        if len == 0 {
            // 実体の道が引けないなら登録しても壊れた値が残るだけなので、ここで止める
            return Err(windows::Win32::Foundation::E_FAIL.into());
        }
        Ok(String::from_utf16_lossy(&buf[..len as usize]))
    }

    fn write_string_value(
        key_path: &str,
        value_name: Option<&str>,
        data: &str,
    ) -> windows::core::Result<()> {
        let mut hkey = HKEY::default();
        let path = HSTRING::from(key_path);
        // SAFETY: 引数は全て有効な参照で、成功時のみ hkey を使ひ最後に閉ぢる。
        unsafe {
            RegCreateKeyExW(
                HKEY_CLASSES_ROOT,
                PCWSTR(path.as_ptr()),
                None,
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                None,
                &mut hkey,
                None,
            )
            .ok()?;

            let wide: Vec<u16> = data.encode_utf16().chain(std::iter::once(0)).collect();
            let bytes = std::slice::from_raw_parts(
                wide.as_ptr() as *const u8,
                std::mem::size_of_val(&wide[..]),
            );
            let name = value_name.map(HSTRING::from);
            let name_ptr = name
                .as_ref()
                .map(|n| PCWSTR(n.as_ptr()))
                .unwrap_or(PCWSTR::null());
            let r = RegSetValueExW(hkey, name_ptr, None, REG_SZ, Some(bytes));
            let _ = RegCloseKey(hkey);
            r.ok()?;
        }
        Ok(())
    }

    /// COM サーバとしての登録（レジストリ）＋ TSF への名乗り。
    ///
    /// `DllRegisterServer` から呼ぶ。**管理者権限が要る**（HKCR へ書くため）。
    pub fn register_server(hinstance: isize) -> windows::core::Result<()> {
        let dll = module_path(hinstance)?;

        // ① COM サーバ: HKCR\CLSID\{CLSID}
        write_string_value(&clsid_key_path(), None, PROFILE_DESCRIPTION)?;
        write_string_value(&inproc_key_path(), None, &dll)?;
        write_string_value(&inproc_key_path(), Some("ThreadingModel"), THREADING_MODEL)?;

        // ② 入力プロファイル: TSF へ「日本語の鍵盤がここに居る」と名乗る
        let clsid = clsid();
        let profile = profile_guid();
        // **NUL 終端を持たせる**（理由は `wide_nul` の説明を見よ——
        // 終端が無いと解放済みヒープが機械全体のレジストリへ漏れる）。
        let desc = wide_nul(PROFILE_DESCRIPTION);
        let dll_wide = wide_nul(&dll);

        // SAFETY: CoCreateInstance は TSF の標準オブジェクトを作る。以降の呼びは
        // すべて有効な参照・長さを渡してゐる。
        unsafe {
            let profiles: ITfInputProcessorProfiles =
                CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?;
            profiles.Register(&clsid)?;
            profiles.AddLanguageProfile(
                &clsid,
                LANGID_JA_JP,
                &profile,
                wide_body(&desc),
                wide_body(&dll_wide),
                0, // アイコンの索引（DLL にアイコンを持たせたら差し替へる）
            )?;

            // ③ カテゴリ: 鍵盤である・候補窓は自前・保護された入力欄でも動く
            let categories: ITfCategoryMgr =
                CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)?;
            for cat in [
                &GUID_TFCAT_TIP_KEYBOARD,
                &GUID_TFCAT_TIPCAP_UIELEMENTENABLED,
                &GUID_TFCAT_TIPCAP_SECUREMODE,
            ] {
                categories.RegisterCategory(&clsid, cat, &clsid)?;
            }
        }
        Ok(())
    }

    /// 登録の取り消し（`DllUnregisterServer`）。**入れた順の逆で外す。**
    ///
    /// 既に消えてゐる場合があるので、個々の失敗では止まらない。
    pub fn unregister_server() -> windows::core::Result<()> {
        let clsid = clsid();

        // SAFETY: 取得に失敗しても片付けを続ける。
        unsafe {
            if let Ok(categories) = CoCreateInstance::<_, ITfCategoryMgr>(
                &CLSID_TF_CategoryMgr,
                None,
                CLSCTX_INPROC_SERVER,
            ) {
                for cat in [
                    &GUID_TFCAT_TIP_KEYBOARD,
                    &GUID_TFCAT_TIPCAP_UIELEMENTENABLED,
                    &GUID_TFCAT_TIPCAP_SECUREMODE,
                ] {
                    let _ = categories.UnregisterCategory(&clsid, cat, &clsid);
                }
            }
            if let Ok(profiles) = CoCreateInstance::<_, ITfInputProcessorProfiles>(
                &CLSID_TF_InputProcessorProfiles,
                None,
                CLSCTX_INPROC_SERVER,
            ) {
                let profile = profile_guid();
                let _ = profiles.RemoveLanguageProfile(&clsid, LANGID_JA_JP, &profile);
                let _ = profiles.Unregister(&clsid);
            }

            let path = HSTRING::from(clsid_key_path());
            let _ = RegDeleteTreeW(HKEY_CLASSES_ROOT, PCWSTR(path.as_ptr()));
        }
        Ok(())
    }

    /// `GetKeyboardType(0)` の値（配列の種別）。原器は JIS 前提なので、
    /// 違ふ鍵盤で使はれてゐることを知らせるために見る（動作には要らない）。
    pub fn keyboard_type() -> u32 {
        // SAFETY: 引数なしの問合せ。失敗しても 0 が返るだけ。
        unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetKeyboardType(0) as u32 }
    }
}

#[cfg(windows)]
pub use imp::{clsid, keyboard_type, profile_guid, register_server, unregister_server};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guid_の形をしてゐる() {
        for g in [
            CLSID_YATATE_TEXT_SERVICE,
            GUID_YATATE_PROFILE,
            GUID_YATATE_DISPLAY_ATTRIBUTE_INPUT,
        ] {
            assert!(g.starts_with('{') && g.ends_with('}'), "{g}");
            assert_eq!(g.len(), 38, "{g}");
            for c in g[1..37].chars() {
                assert!(
                    c.is_ascii_hexdigit() || c == '-',
                    "GUID に不正な文字 '{c}': {g}"
                );
            }
        }
    }

    #[test]
    fn 三つの_guid_は互ひに異なる() {
        let gs = [
            CLSID_YATATE_TEXT_SERVICE,
            GUID_YATATE_PROFILE,
            GUID_YATATE_DISPLAY_ATTRIBUTE_INPUT,
        ];
        for (i, a) in gs.iter().enumerate() {
            for b in gs.iter().skip(i + 1) {
                assert_ne!(a, b, "GUID が衝突してゐる");
            }
        }
    }

    /// 骨組みの段で置いた仮値を配つてしまふのを防ぐ関門。
    #[test]
    fn 仮の_guid_が残つてゐない() {
        for g in [CLSID_YATATE_TEXT_SERVICE, GUID_YATATE_PROFILE] {
            assert!(
                !g.contains("00000000000"),
                "仮の GUID が残つてゐる: {g}（uuidgen で採り直すこと）"
            );
        }
    }

    #[test]
    fn 日本語のプロファイルである() {
        assert_eq!(LANGID_JA_JP, 0x0411);
        assert!(!PROFILE_DESCRIPTION.is_empty());
        assert_eq!(THREADING_MODEL, "Apartment", "TIP は STA");
    }

    #[test]
    fn レジストリの道が_clsid_を含む() {
        assert!(clsid_key_path().contains(CLSID_YATATE_TEXT_SERVICE));
        assert!(inproc_key_path().ends_with("InprocServer32"));
        assert!(inproc_key_path().starts_with(&clsid_key_path()));
    }

    /// **実機で見つけた瑕の関門。**
    ///
    /// TSF へ渡す文字列は「終端を持ち、長さは終端を含めない」の両方でなければ
    /// ならない。片方でも欠けると、長さを守らない実装が解放済みヒープまで読み進み、
    /// その中身が機械全体のレジストリへ書き出される。
    #[test]
    fn tsf_へ渡す文字列は終端を持ち長さは終端を含めない() {
        for s in [PROFILE_DESCRIPTION, "C:\\Program Files\\Yatate\\yatate_windows.dll"] {
            let buf = wide_nul(s);
            assert_eq!(*buf.last().unwrap(), 0, "終端が無い: {s}");

            let body = wide_body(&buf);
            assert_eq!(
                body.len(),
                s.encode_utf16().count(),
                "渡す長さが終端を含んでゐる: {s}"
            );
            assert!(!body.contains(&0), "本体に NUL が混ざつてゐる: {s}");
            assert_eq!(
                String::from_utf16_lossy(body),
                s,
                "往復しない（文字が落ちてゐる）: {s}"
            );
            // 本体の直後が NUL であること＝`wcslen` する実装にも正しく見える
            assert_eq!(buf[body.len()], 0, "本体の直後が終端でない: {s}");
        }
    }

    #[test]
    fn 空文字列でも壊れない() {
        let buf = wide_nul("");
        assert_eq!(buf, vec![0]);
        assert!(wide_body(&buf).is_empty());
        // 万一空の緩衝を渡されても添字で落ちない
        assert!(wide_body(&[]).is_empty());
    }

    #[test]
    fn 三つのカテゴリを名乗る() {
        assert_eq!(REQUIRED_CATEGORIES.len(), 3);
        assert!(REQUIRED_CATEGORIES.contains(&"GUID_TFCAT_TIP_KEYBOARD"));
        // 候補窓を自前で描く宣言は必須（TSF は出してくれない）
        assert!(REQUIRED_CATEGORIES.contains(&"GUID_TFCAT_TIPCAP_UIELEMENTENABLED"));
    }
}
