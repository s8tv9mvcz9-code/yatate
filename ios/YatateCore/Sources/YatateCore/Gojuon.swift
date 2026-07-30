// 五十音の幾何学 — 行×段の地図（docs/ime/layout.md §2）。
// 梯子は常に 5 スロット固定: や行・わ行の空きは nil で残し、位置の筋肉記憶を守る。
// ゐ・ゑ はわ行の 1・3 段目の第一級市民である。

/// 逸らし（deflection）— 書き下ろしスライド中の横方向の意味。
/// 濁点は字の右肩に打たれるものだから、右への逸らしが濁点になる。
public enum Deflect: Sendable {
    case none   // 清音
    case daku   // 右: 濁点（か→が）
    case ko     // 左: 半濁点（は→ぱ）または小書き（や→ゃ、つ→っ、あ→ぁ）
}

/// 一つの行（ぎゃう）。キーの刻印と三態の梯子。
public struct Gyo: Sendable {
    public let name: String          // 行頭の仮名（キー刻印）
    public let seion: [String?]      // 5 スロット（あ〜お段）
    public let daku: [String?]?      // 右逸らし面（無い行は nil）
    public let ko: [String?]?        // 左逸らし面（無い行は nil）

    public init(_ name: String, _ seion: [String?],
                daku: [String?]? = nil, ko: [String?]? = nil) {
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
        case .ko:   return ko?[dan]
        }
    }
}

public enum Gojuon {
    public static let a  = Gyo("あ", ["あ", "い", "う", "え", "お"],
                               daku: [nil, nil, "ゔ", nil, nil],
                               ko: ["ぁ", "ぃ", "ぅ", "ぇ", "ぉ"])
    public static let ka = Gyo("か", ["か", "き", "く", "け", "こ"],
                               daku: ["が", "ぎ", "ぐ", "げ", "ご"])
    public static let sa = Gyo("さ", ["さ", "し", "す", "せ", "そ"],
                               daku: ["ざ", "じ", "ず", "ぜ", "ぞ"])
    public static let ta = Gyo("た", ["た", "ち", "つ", "て", "と"],
                               daku: ["だ", "ぢ", "づ", "で", "ど"],
                               ko: [nil, nil, "っ", nil, nil])
    public static let na = Gyo("な", ["な", "に", "ぬ", "ね", "の"])
    public static let ha = Gyo("は", ["は", "ひ", "ふ", "へ", "ほ"],
                               daku: ["ば", "び", "ぶ", "べ", "ぼ"],
                               ko: ["ぱ", "ぴ", "ぷ", "ぺ", "ぽ"])
    public static let ma = Gyo("ま", ["ま", "み", "む", "め", "も"])
    public static let ya = Gyo("や", ["や", nil, "ゆ", nil, "よ"],
                               ko: ["ゃ", nil, "ゅ", nil, "ょ"])
    public static let ra = Gyo("ら", ["ら", "り", "る", "れ", "ろ"])
    public static let wa = Gyo("わ", ["わ", "ゐ", nil, "ゑ", "を"])

    /// 二行配列の第一の行（画面右列・上→下）— 縦書きの読み順で先。
    public static let firstLine: [Gyo] = [a, ka, sa, ta, na]
    /// 第二の行（左列・上→下）。
    public static let secondLine: [Gyo] = [ha, ma, ya, ra, wa]
}
