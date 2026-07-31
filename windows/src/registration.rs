//! TSF への登録に要る値（`docs/ime/cross-platform.md` §4）。
//!
//! テキストサービスは COM サーバとして登録され、さらに TSF へ
//! `ITfInputProcessorProfiles::Register` で「入力プロファイル」として名乗る。
//! ここではその**値だけ**を持つ（実際に書き込む手続きは Windows 機で詰める）。
//!
//! ## 署名について
//!
//! Microsoft はテキストサービスのバイナリに**デジタル署名**を求めてゐる。
//! 未署名でも動く場面はあるが、配布の前提としては署名を用意する。
//! OSS は SignPath Foundation の無償証明書といふ道があり、
//! 先行例（`windows-chewing-tsf`）が実際にそれで配布してゐる。
//! **x64 と ARM64 の両方**を出すこと（ARM64 機で x64 の DLL は読み込まれない）。

/// テキストサービスの CLSID。
///
/// **仮の値である。** 実機で登録する前に `uuidgen` で採り直し、
/// インストーラと合はせて固定すること（一度配つたら変へられない）。
pub const CLSID_YATATE_TEXT_SERVICE: &str = "{6A3E1B2C-7D40-4F58-9A21-000000000001}";

/// 入力プロファイルの GUID（同上・仮）。
pub const GUID_YATATE_PROFILE: &str = "{6A3E1B2C-7D40-4F58-9A21-000000000002}";

/// 言語 ID（日本語・日本）。TSF のプロファイルはこの言語に属す。
pub const LANGID_JA_JP: u16 = 0x0411;

/// 入力方式一覧に出る名。
pub const PROFILE_DESCRIPTION: &str = "矢立（文語 IME）";

/// TSF のカテゴリ登録に必要なもの（`ITfCategoryMgr::RegisterCategory` へ渡す）。
///
/// - `GUID_TFCAT_TIP_KEYBOARD` … 鍵盤としてのテキストサービス
/// - `GUID_TFCAT_TIPCAP_UIELEMENTENABLED` … 候補窓を**自前で描く**宣言。
///   TSF は候補窓を出してくれないので、これは飾りでなく実務上の必須事項である。
/// - `GUID_TFCAT_TIPCAP_SECUREMODE` … 保護された入力欄でも動かす場合
pub const REQUIRED_CATEGORIES: [&str; 3] = [
    "GUID_TFCAT_TIP_KEYBOARD",
    "GUID_TFCAT_TIPCAP_UIELEMENTENABLED",
    "GUID_TFCAT_TIPCAP_SECUREMODE",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guid_の形をしてゐる() {
        for g in [CLSID_YATATE_TEXT_SERVICE, GUID_YATATE_PROFILE] {
            assert!(g.starts_with('{') && g.ends_with('}'), "{g}");
            assert_eq!(g.len(), 38, "{g}");
        }
        assert_ne!(CLSID_YATATE_TEXT_SERVICE, GUID_YATATE_PROFILE);
    }

    #[test]
    fn 日本語のプロファイルである() {
        assert_eq!(LANGID_JA_JP, 0x0411);
        assert!(!PROFILE_DESCRIPTION.is_empty());
    }
}
