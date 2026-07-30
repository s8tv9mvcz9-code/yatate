// 作業帯の中身 — 確定前の假名バッファ（M0 は素朴な文字列。M2 で文節列に育つ）。

public struct Composer: Sendable {
    public private(set) var text: String = ""

    public init() {}

    public var isEmpty: Bool { text.isEmpty }

    public mutating func append(_ kana: String) { text += kana }

    public mutating func deleteLast() {
        guard !text.isEmpty else { return }
        text.removeLast()
    }

    /// 確定 — 旧字体を機械で確定させた文を返し、バッファを空にする。
    public mutating func commit() -> String {
        let out = toKyuji(text)
        text = ""
        return out
    }
}
