// 原器 — 縦組五十音配列（docs/ime/layout.md §1）。**物理鍵盤の座標系**である。
//
// これは核（core/src/genki.rs・Rust）の写しであり、SSOT はあちらである。
// ずれてゐないことは ParityTests が core/vectors/genki.json を読んで機械で検める。
//
// ## 紙面の見立て
//
//       左半面（第二の行）              右半面（第一の行）
//   ［1］［2］［3］［4］［5］    ［6］［7］［8］［9］［0］   ［^］
//    と   て   つ   ち   た      お   え   う   い   あ     前置シフト
//   ［q］［w］［e］［r］［t］    ［y］［u］［i］［o］［p］
//    の   ね   ぬ   に   な      こ   け   く   き   か
//   ［a］［s］［d］［f］［g］    ［j］［k］［l］［;］［:］
//    ほ   へ   ふ   ひ   は      そ   せ   す   し   さ
//
// 第二面（`^` 前置後）は ま・や・ら・わ行。ん は行にも段にも属さぬ唯一の仮名として、
// シフトそのものの重ね打ち `^^` に住む。
//
// ## ここに来るのは「文字」でなく「位置」である
//
// 表の鍵が `'0'` や `':'` といふ文字なのは原器の定義がさうだからで、
// 殻は **KagiTable で物理位置から引いた文字**をここへ渡す。
// 出る字（`NSEvent.characters` / `UIKey.characters`）から引いてはならない——
// 英字配列の機で「し」が黙つて「さ」になる。

/// 原器の特別な三鍵。
public enum GenkiKey {
    /// 前置シフト。これ自身を重ね打つと「ん」になる。
    public static let shift: Character = "^"
    /// 濁点（後置打鍵）。
    public static let dakuten: Character = "b"
    /// 半濁点（後置打鍵）。
    public static let handakuten: Character = "v"
}

/// 打鍵が作業帯へ及ぼす変化。**核は文を持たない**——殻が持つ文への指示だけを返す。
public enum Edit: Equatable, Sendable {
    /// 仮名を足す。
    case insert(String)
    /// 直前の一字を差し替へる（濁点・半濁点の後置打鍵）。
    case replaceLast(Character)
    /// 目に見える変化なし（前置シフトが立つた等）。
    case none
    /// この鍵は原器に無い（記号・機能キーは殻の領分）。
    case unmapped
}

/// 第一面（無シフト）— 清音 30 字。
public let firstPlane: [(Character, String)] = [
    ("0", "あ"), ("9", "い"), ("8", "う"), ("7", "え"), ("6", "お"),
    ("p", "か"), ("o", "き"), ("i", "く"), ("u", "け"), ("y", "こ"),
    (":", "さ"), (";", "し"), ("l", "す"), ("k", "せ"), ("j", "そ"),
    // ここで紙面を折り返して左半面へ
    ("5", "た"), ("4", "ち"), ("3", "つ"), ("2", "て"), ("1", "と"),
    ("t", "な"), ("r", "に"), ("e", "ぬ"), ("w", "ね"), ("q", "の"),
    ("g", "は"), ("f", "ひ"), ("d", "ふ"), ("s", "へ"), ("a", "ほ"),
]

/// 第二面（`^` 前置後）— ま・や・ら・わ行。`^^` は「ん」（表には入れない）。
///
/// や行に い・え、わ行に う を重複配置する——地図の完全性を欠けより優先する。
/// ゐ・ゑ は第一級の鍵を持つ（`^r`・`^w`）。
public let secondPlane: [(Character, String)] = [
    ("0", "ま"), ("9", "み"), ("8", "む"), ("7", "め"), ("6", "も"),
    ("p", "や"), ("o", "い"), ("i", "ゆ"), ("u", "え"), ("y", "よ"),
    ("5", "ら"), ("4", "り"), ("3", "る"), ("2", "れ"), ("1", "ろ"),
    ("t", "わ"), ("r", "ゐ"), ("e", "う"), ("w", "ゑ"), ("q", "を"),
]

private let firstPlaneMap: [Character: String] = Dictionary(
    uniqueKeysWithValues: firstPlane.map { ($0.0, $0.1) })
private let secondPlaneMap: [Character: String] = Dictionary(
    uniqueKeysWithValues: secondPlane.map { ($0.0, $0.1) })

/// 全 10 行（あ〜な ＋ は〜わ）。
private let allGyo: [Gyo] = Gojuon.firstLine + Gojuon.secondLine

/// 仮名から（行の名, 段）を引く。
private func reverseLookup(_ kana: Character) -> (String, Int)? {
    let s = String(kana)
    for gyo in allGyo {
        for (dan, slot) in gyo.seion.enumerated() where slot == s {
            return (gyo.name, dan)
        }
    }
    return nil
}

private func gyoNamed(_ name: String) -> Gyo? {
    allGyo.first { $0.name == name }
}

/// 濁点を打つた結果。濁れない字なら `nil`。
public func dakuten(_ kana: Character) -> Character? {
    guard let (name, dan) = reverseLookup(kana),
        let gyo = gyoNamed(name),
        let s = gyo.kana(dan: dan, deflect: .daku)
    else { return nil }
    return s.first
}

/// 半濁点を打つた結果。**は行だけ**が半濁点を持つ。
///
/// 小書き（ゃゅょっ）は原器では未定なので、や行・た行には及ぼさない
/// （二行配列では同じ左逸らし面に小書きが同居するが、それは硝子の話である）。
public func handakuten(_ kana: Character) -> Character? {
    guard let (name, dan) = reverseLookup(kana), name == "は",
        let gyo = gyoNamed(name),
        let s = gyo.kana(dan: dan, deflect: .ko)
    else { return nil }
    return s.first
}

/// 原器の状態機械。前置シフトは**逐次打鍵**（同時押しでない）ゆゑ状態が要る。
public struct Genki: Sendable {
    /// 前置シフトが立つてゐるか（殻が面の表示を切り替へるために見る）。
    public private(set) var shifted = false

    public init() {}

    /// 一打を食はせる。`last` は作業帯の末尾の一字（濁点の後置打鍵に要る）。
    public mutating func press(_ key: Character, last: Character?) -> Edit {
        if key == GenkiKey.shift {
            // ん は行にも段にも属さぬ唯一の仮名——シフトの重ね打ちに住む
            if shifted {
                shifted = false
                return .insert("ん")
            }
            shifted = true
            return .none
        }

        if shifted {
            shifted = false
            if let kana = secondPlaneMap[key] { return .insert(kana) }
            return .unmapped
        }

        switch key {
        case GenkiKey.dakuten:
            if let l = last, let c = dakuten(l) { return .replaceLast(c) }
            return .unmapped
        case GenkiKey.handakuten:
            if let l = last, let c = handakuten(l) { return .replaceLast(c) }
            return .unmapped
        default:
            if let kana = firstPlaneMap[key] { return .insert(kana) }
            return .unmapped
        }
    }

    /// 前置シフトを取り消す（別の入力欄へ移る等、殻の都合で状態を捨てるとき）。
    public mutating func reset() {
        shifted = false
    }
}

/// 打鍵の列を仮名の列へ（試験と稽古場のための便宜。殻は `Genki.press` を使ふ）。
public func typeKeys(_ keys: String) -> String {
    var g = Genki()
    var out = ""
    for key in keys {
        switch g.press(key, last: out.last) {
        case .insert(let kana): out += kana
        case .replaceLast(let c):
            out.removeLast()
            out.append(c)
        case .none, .unmapped: break
        }
    }
    return out
}
