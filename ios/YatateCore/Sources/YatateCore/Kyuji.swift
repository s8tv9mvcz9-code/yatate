// 旧字体 — 新字体からの 1:1・文脈非依存の写像（248 字）。
//
// **表は核（core/src/generated/kyuji_table.rs）に在る。** その出所は Python の
// `ssot/kyuji.py` で、`scripts/gen_rust_tables.py` が写す。
// かつては同じ表が Swift にも生成されてゐたが（`Generated/KyujiTable.swift`）、
// M5-b2 で核へ一本化した——同じ 248 字を二か所に置く理由がもう無い。

import YatateFFI

/// 新字体を旧字体へ。**確定の瞬間にだけ掛ける**（辞書は新字体で持つてゐる）。
public func toKyuji(_ text: String) -> String {
    text.withCore { coreText(yatate_to_kyuji($0)) }
}

/// 旧字の写像そのもの（図の描画・検査のために覗く窓）。
///
/// 打鍵ごとに引く道ではない——変換は [`toKyuji`] を通す。
public enum KyujiTable {
    /// 新字体 → 旧字体。
    public static let map: [Character: Character] = loaded.map

    /// 一対多で危険なため写像から除外されてゐる新字体。
    public static let ambiguous: Set<Character> = loaded.ambiguous

    private static let loaded: (map: [Character: Character], ambiguous: Set<Character>) = {
        var map: [Character: Character] = [:]
        var ambiguous: Set<Character> = []
        for row in coreRows(coreText(yatate_kyuji_table())) where row.count == 2 {
            // 曖昧字は第一欄が空の一行で来る（写像を持たない字だから）
            if row[0].isEmpty {
                ambiguous = Set(row[1])
            } else if let a = row[0].first, let b = row[1].first {
                map[a] = b
            }
        }
        return (map, ambiguous)
    }()
}
