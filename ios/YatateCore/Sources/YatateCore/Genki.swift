// 原器 — 縦組五十音配列（docs/ime/layout.md §1）。**物理鍵盤の座標系**である。
//
// **実体は核（core/src/genki.rs）に在る。** ここはその入口で、面の表も
// 前置シフトの逐次性も核が持つ。ずれてゐないことは KagiGenkiParityTests が
// core/vectors/genki.json を読んで機械で検める。
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
// 殻は **Kagi で物理位置から引いた文字**をここへ渡す。
// 出る字（`NSEvent.characters` / `UIKey.characters`）から引いてはならない——
// 英字配列の機で「し」が黙つて「さ」になる。

import YatateFFI

/// 原器の特別な三鍵。**値は核から来る**（ここで決めない）。
public enum GenkiKey {
    /// 前置シフト。これ自身を重ね打つと「ん」になる。
    public static let shift: Character = special.shift
    /// 濁点（後置打鍵）。
    public static let dakuten: Character = special.dakuten
    /// 半濁点（後置打鍵）。
    public static let handakuten: Character = special.handakuten

    private static let special: (shift: Character, dakuten: Character, handakuten: Character) = {
        let row = coreRows(coreText(yatate_genki_special_keys())).first ?? []
        guard row.count == 3, let s = row[0].first, let d = row[1].first, let h = row[2].first
        else { return ("^", "b", "v") }
        return (s, d, h)
    }()
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
public let firstPlane: [(Character, String)] = planes.first

/// 第二面（`^` 前置後）— ま・や・ら・わ行。`^^` は「ん」（表には入れない）。
public let secondPlane: [(Character, String)] = planes.second

private let planes: (first: [(Character, String)], second: [(Character, String)]) = {
    var first: [(Character, String)] = []
    var second: [(Character, String)] = []
    for row in coreRows(coreText(yatate_genki_planes())) where row.count == 3 {
        guard let key = row[1].first else { continue }
        if row[0] == "1" { first.append((key, row[2])) } else { second.append((key, row[2])) }
    }
    return (first, second)
}()

/// 濁点を打つた結果。濁れない字なら `nil`。
public func dakuten(_ kana: Character) -> Character? {
    coreChar(yatate_dakuten(coreScalar(kana)))
}

/// 半濁点を打つた結果。**は行だけ**が半濁点を持つ。
///
/// 小書き（ゃゅょっ）は原器では未定なので、や行・た行には及ぼさない
/// （二行配列では同じ左逸らし面に小書きが同居するが、それは硝子の話である）。
public func handakuten(_ kana: Character) -> Character? {
    coreChar(yatate_handakuten(coreScalar(kana)))
}

/// 原器の状態機械。前置シフトは**逐次打鍵**（同時押しでない）ゆゑ状態が要る。
///
/// 核の状態を握るので値型ではなく参照型である（写せば状態が分裂する）。
public final class Genki {
    /// 核が握る状態への手綱（C からは不完全型なので Swift へは不透明な指針で来る）。
    private let handle = yatate_genki_new()

    public init() {}

    deinit { yatate_genki_free(handle) }

    /// 前置シフトが立つてゐるか（殻が面の表示を切り替へるために見る）。
    public var shifted: Bool { yatate_genki_is_shifted(handle) != 0 }

    /// 一打を食はせる。`last` は作業帯の末尾の一字（濁点の後置打鍵に要る）。
    public func press(_ key: Character, last: Character?) -> Edit {
        var text: UnsafeMutablePointer<CChar>?
        let code = yatate_genki_press(handle, coreScalar(key), last.map(coreScalar) ?? 0, &text)
        let written = coreText(text)
        switch code {
        case YATATE_EDIT_INSERT: return .insert(written)
        case YATATE_EDIT_REPLACE_LAST:
            guard let c = written.first else { return .unmapped }
            return .replaceLast(c)
        case YATATE_EDIT_NONE: return .none
        default: return .unmapped
        }
    }

    /// 前置シフトを取り消す（別の入力欄へ移る等、殻の都合で状態を捨てるとき）。
    public func reset() {
        yatate_genki_reset(handle)
    }
}

/// 打鍵の列を仮名の列へ（試験と稽古場のための便宜。殻は `Genki.press` を使ふ）。
public func typeKeys(_ keys: String) -> String {
    keys.withCore { coreText(yatate_type_keys($0)) }
}
