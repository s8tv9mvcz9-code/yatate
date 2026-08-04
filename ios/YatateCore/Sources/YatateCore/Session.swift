// 入力セッション — **どの OS でも同じ**打鍵の状態機械（core/src/session.rs の写し）。
//
// 殻はここへ打鍵を渡し、返つてきた指示のとほりに未確定文字列を描き、
// 確定時に文字列を挿し込むだけでよい。TSF の composition も IMKit の marked text も
// Android の composing text も、**呼び名が違ふだけで同じもの**である。
//
// SSOT は Rust の core/src/session.rs。ここはその写しで、
// ParityTests が core/vectors/genki.json で両者を縛る。

/// 一打に対する殻への指示。
public enum KeyAction: Equatable, Sendable {
    /// 未確定文字列が変はつた（殻は marked text を描き直す）。
    case update
    /// 何も起きなかつた（前置シフトが立つた等）。殻は打鍵を**呑む**。
    case swallow
    /// 矢立の鍵ではない。殻は OS へ**素通し**する。
    case passthrough
    /// 確定した文字列。殻はこれを挿し込み、marked text を閉ぢる。
    case commit(String)
}

/// 未確定文字列を抱へる入力セッション。入力欄ごとに 1 つ持つ。
public struct Session: Sendable {
    private var genki = Genki()
    private var composer = Composer()

    public init() {}

    /// いま未確定の仮名列。
    public var preedit: String { composer.text }

    public var isComposing: Bool { !composer.isEmpty }

    /// 前置シフトが立つてゐるか（殻が面の表示を替へる手掛かり）。
    public var isShifted: Bool { genki.shifted }

    /// 墨の氣配 — 次の一打の分布。候補窓・鍵盤表示に使ふ（描くのは殻）。
    public var field: ActionField { Kehai.field(after: composer.text.last) }

    /// 原器の一打を食はせる。
    public mutating func key(_ ch: Character) -> KeyAction {
        let last = composer.text.last
        switch genki.press(ch, last: last) {
        case .insert(let kana):
            composer.append(kana)
            return .update
        case .replaceLast(let c):
            composer.deleteLast()
            composer.append(String(c))
            return .update
        // 前置シフトを立てた打鍵は**呑む**（OS へ流すと `^` が入力されてしまふ）
        case .none:
            return .swallow
        case .unmapped:
            // 矢立の鍵でない。未確定文字列を抱へてゐる間は取り零さぬやう呑み、
            // 何も無ければ素通しして OS 本来の働きを妨げない。
            return isComposing ? .swallow : .passthrough
        }
    }

    /// この鍵を矢立が受け取るべきか（IMKit の `handle` が先に判断する）。
    ///
    /// 未確定文字列がある間は、原器に無い鍵も**一旦は受ける**
    /// （確定・取り消しの機会を殻に残すため）。
    public func wantsKey(_ ch: Character) -> Bool {
        if isComposing || genki.shifted { return true }
        if ch == GenkiKey.shift || ch == GenkiKey.dakuten || ch == GenkiKey.handakuten {
            return true
        }
        return firstPlane.contains { $0.0 == ch }
    }

    /// 一字消す。未確定文字列が空になつたら `false`（殻は marked text を閉ぢる）。
    @discardableResult
    public mutating func backspace() -> Bool {
        composer.deleteLast()
        return isComposing
    }

    /// 確定 — **旧字体は核が機械で確定させる**。
    public mutating func commit() -> KeyAction {
        genki.reset()
        return .commit(composer.commit())
    }

    /// 取り消し（Esc）。未確定文字列を捨てる。
    public mutating func cancel() {
        genki.reset()
        composer = Composer()
    }
}
