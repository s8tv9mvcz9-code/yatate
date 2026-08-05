// 墨の気配 — 言語の期待を行為空間（どの鍵か）へ射影する（docs/ime/vla.md A0）。
// 青空文庫の仮名連 bigram を、二行配列の鍵ごとの墨の濃淡（0…1）に落とす。
// 決定的・訓練なし・ネットワーク不要。
//
// **数へるのは核（core/src/kehai.rs）である。** ここは受け取つた濃淡を Swift の
// 形へ移すだけで、bigram の表も正規化の式も持たない。
//
// M5-b2 で一つ直つたことがある: 以前の Swift 版は峰（peak）を
// `Dictionary.max{…}` で選んでゐたので、**同点のとき結果が辞書の内部順序に
// 依存して実行ごとに変はり得た**。核は同点を読み順（あ→か→…→わ）の先着で割るので、
// 核へ載せた今、その不安定さは消えてゐる。

import YatateFFI

/// 鍵の同一性。行キーは行名で、体系外の字は字そのもので指す。
public enum KeyID: Hashable, Sendable {
    case gyo(String)  // "あ" "か" … "わ"
    case moji(String)  // "ん" "、" "。"
}

/// 行為空間に描かれた場。ink は鍵の墨（最大値 1 に正規化）、dan は梯子内の段の墨。
public struct ActionField: Sendable {
    public let ink: [KeyID: Double]
    public let dan: [String: [Double]]
    public let peak: KeyID?  // 筆脈の先（最有力の一手）

    public static let empty = ActionField(ink: [:], dan: [:], peak: nil)
}

public enum Kehai {
    /// 弱信号は無地（実用性ガード — vla.md §3）。観測がこの回数に満たねば描かない。
    static let minEvidence = Int(yatate_kehai_min_evidence())

    /// 仮名 → (行名, 段)。濁点・半濁点・小書きも基底の鍵へ畳む。
    static let reverseMap: [Character: (gyo: String, dan: Int)] = {
        var m: [Character: (String, Int)] = [:]
        for row in coreRows(coreText(yatate_gojuon_reverse())) where row.count == 3 {
            guard let kana = row[0].first, let dan = Int(row[2]) else { continue }
            m[kana] = (row[1], dan)
        }
        return m
    }()

    /// 直前の一字（なければ連なりの開始 `^`）から、鍵盤に滲む気配を出す。
    public static func field(after prev: Character?) -> ActionField {
        let text = (prev.map(String.init) ?? "").withCore { coreText(yatate_kehai_field($0)) }
        return parse(text)
    }

    /// 核が返す TSV を場へ。行頭の札で種類が分かれてゐる。
    ///
    ///     peak\t<gyo|moji>\t<名>
    ///     ink\t<gyo|moji>\t<名>\t<墨>
    ///     dan\t<行>\t<段0>…<段4>
    static func parse(_ text: String) -> ActionField {
        var ink: [KeyID: Double] = [:]
        var dan: [String: [Double]] = [:]
        var peak: KeyID?

        for row in coreRows(text) {
            switch row.first {
            case "peak" where row.count == 3:
                peak = keyID(kind: row[1], name: row[2])
            case "ink" where row.count == 4:
                guard let value = Double(row[3]) else { continue }
                ink[keyID(kind: row[1], name: row[2])] = value
            case "dan" where row.count == 7:
                dan[row[1]] = row[2...].compactMap(Double.init)
            default:
                continue
            }
        }
        return ActionField(ink: ink, dan: dan, peak: peak)
    }

    private static func keyID(kind: String, name: String) -> KeyID {
        kind == "gyo" ? .gyo(name) : .moji(name)
    }
}
