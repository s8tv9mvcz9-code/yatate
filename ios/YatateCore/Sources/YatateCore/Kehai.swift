// 墨の気配 — 言語の期待を行為空間（どの鍵か）へ射影する（docs/ime/vla.md A0）。
// KanaBigram（青空文庫の仮名連 bigram・機械生成）を、二行配列の鍵ごとの
// 墨の濃淡（0...1）に落とす。決定的・訓練なし・ネットワーク不要。

/// 鍵の同一性。行キーは行名で、体系外の字は字そのもので指す。
public enum KeyID: Hashable, Sendable {
    case gyo(String)     // "あ" "か" … "わ"
    case moji(String)    // "ん" "、" "。"
}

/// 行為空間に描かれた場。ink は鍵の墨（最大値 1 に正規化）、dan は梯子内の段の墨。
public struct ActionField: Sendable {
    public let ink: [KeyID: Double]
    public let dan: [String: [Double]]
    public let peak: KeyID?                 // 筆脈の先（最有力の一手）

    public static let empty = ActionField(ink: [:], dan: [:], peak: nil)
}

public enum Kehai {
    /// 弱信号は無地（実用性ガード — vla.md §3）。観測がこの回数に満たねば描かない。
    static let minEvidence = 20

    /// 仮名 → (行名, 段)。濁点・半濁点・小書きも基底の鍵へ畳む。
    static let reverseMap: [Character: (gyo: String, dan: Int)] = {
        var m: [Character: (String, Int)] = [:]
        for gyo in Gojuon.firstLine + Gojuon.secondLine {
            for dan in 0..<5 {
                for plane in [gyo.seion, gyo.daku ?? [], gyo.ko ?? []] {
                    if dan < plane.count, let kana = plane[dan], let ch = kana.first {
                        m[ch] = (gyo.name, dan)
                    }
                }
            }
        }
        return m
    }()

    /// 直前の一字（なければ連なりの開始 `^`）から、鍵盤に滲む気配を出す。
    public static func field(after prev: Character?) -> ActionField {
        let key = prev.map(String.init) ?? "^"
        // 句読点・表に無い字の後は連なりの開始分布へ退避する
        let rows = KanaBigram.table[key] ?? KanaBigram.table["^"] ?? []
        var inkCount: [KeyID: Int] = [:]
        var danCount: [String: [Int]] = [:]
        var total = 0
        for (next, count) in rows {
            total += count
            if next == "ん" || next == "、" || next == "。" {
                inkCount[.moji(String(next)), default: 0] += count
            } else if let (gyo, dan) = reverseMap[next] {
                inkCount[.gyo(gyo), default: 0] += count
                var row = danCount[gyo] ?? [0, 0, 0, 0, 0]
                row[dan] += count
                danCount[gyo] = row
            }
        }
        guard total >= minEvidence, let maxInk = inkCount.values.max(), maxInk > 0 else {
            return .empty
        }
        let ink = inkCount.mapValues { Double($0) / Double(maxInk) }
        let dan = danCount.mapValues { row -> [Double] in
            let m = row.max() ?? 0
            return m > 0 ? row.map { Double($0) / Double(m) } : [0, 0, 0, 0, 0]
        }
        let peak = inkCount.max { $0.value < $1.value }?.key
        return ActionField(ink: ink, dan: dan, peak: peak)
    }
}
