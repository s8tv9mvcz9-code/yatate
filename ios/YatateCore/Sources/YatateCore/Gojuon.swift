// 五十音の幾何学 — 行×段の地図（docs/ime/layout.md §2）。
// 梯子は常に 5 スロット固定: や行・わ行の空きは nil で残し、位置の筋肉記憶を守る。
// ゐ・ゑ はわ行の 1・3 段目の第一級市民である。
//
// **地図の実体は核（core/src/gojuon.rs）に在る。** ここはそれを一度だけ読んで
// Swift の値型へ移すだけで、仮名を一つも書いてゐない——書けば二枚目の地図になる。

import YatateFFI

/// 逸らし（deflection）— 書き下ろしスライド中の横方向の意味。
/// 濁点は字の右肩に打たれるものだから、右への逸らしが濁点になる。
public enum Deflect: String, Sendable {
    case none  // 清音
    case daku  // 右: 濁点（か→が）
    case ko  // 左: 半濁点（は→ぱ）または小書き（や→ゃ、つ→っ、あ→ぁ）
}

/// 一つの行（ぎゃう）。キーの刻印と三態の梯子。
public struct Gyo: Sendable {
    public let name: String  // 行頭の仮名（キー刻印）
    public let seion: [String?]  // 5 スロット（あ〜お段）
    public let daku: [String?]?  // 右逸らし面（無い行は nil）
    public let ko: [String?]?  // 左逸らし面（無い行は nil）

    public init(
        _ name: String, _ seion: [String?],
        daku: [String?]? = nil, ko: [String?]? = nil
    ) {
        precondition(seion.count == 5)
        self.name = name
        self.seion = seion
        self.daku = daku
        self.ko = ko
    }

    /// 段と逸らしから仮名を引く。空きスロットは nil（何も入力しない）。
    public func kana(dan: Int, deflect: Deflect = .none) -> String? {
        guard (0..<5).contains(dan) else { return nil }
        switch deflect {
        case .none: return seion[dan]
        case .daku: return daku?[dan]
        case .ko: return ko?[dan]
        }
    }
}

public enum Gojuon {
    /// 二行配列の第一の行（画面右列・上→下）— 縦書きの読み順で先。
    public static let firstLine: [Gyo] = lines.first

    /// 第二の行（左列・上→下）。
    public static let secondLine: [Gyo] = lines.second

    /// 読み順（あ→か→…→わ）の全 10 行。核の並びそのままである。
    public static let all: [Gyo] = lines.first + lines.second

    /// 名で行を引く。
    public static func gyo(named name: String) -> Gyo? {
        all.first { $0.name == name }
    }

    // MARK: - 核から起こす

    private static let lines: (first: [Gyo], second: [Gyo]) = {
        // 枡（行・段・逸らし・仮名）。実在する枡だけが来るので、空きは nil のまま残る。
        var slots: [String: [Deflect: [String?]]] = [:]
        for row in coreRows(coreText(yatate_gojuon_table())) where row.count == 4 {
            guard let dan = Int(row[1]), let deflect = Deflect(rawValue: row[2]) else { continue }
            var planes = slots[row[0]] ?? [:]
            var plane = planes[deflect] ?? [nil, nil, nil, nil, nil]
            plane[dan] = row[3]
            planes[deflect] = plane
            slots[row[0]] = planes
        }

        func build(_ name: String) -> Gyo? {
            guard let planes = slots[name], let seion = planes[.none] else { return nil }
            return Gyo(name, seion, daku: planes[.daku], ko: planes[.ko])
        }

        var first: [Gyo] = []
        var second: [Gyo] = []
        for row in coreRows(coreText(yatate_gojuon_lines())) where row.count == 2 {
            guard let gyo = build(row[1]) else { continue }
            if row[0] == "1" { first.append(gyo) } else { second.append(gyo) }
        }
        return (first, second)
    }()
}
