// 鍵の物理位置 — 原器の文字 ↔ 指の置き場。**核（core/src/kagi.rs）の写しである。**
//
// 原器は「指の位置の地図」であり、字面ではなく鍵の位置に意味がある
// （docs/ime/layout.md §1「五十音の幾何学」）。ゆゑに殻は**位置で引く**。
// 位置の呼び名は環境ごとに違ふが、指してゐるものは同じ一つである:
//
//   Windows（TSF）   走査符号 set 1            さ = 0x28
//   ブラウザ          KeyboardEvent.code        さ = "Quote"
//   macOS（IMKit）   Carbon の kVK_ANSI_*      さ = 0x27
//   iOS（ハード鍵盤） UIKeyboardHIDUsage        さ = 0x34
//
// ## これは写しである
//
// SSOT は Rust の core/src/kagi.rs で、ここはその写し。ずれてゐないことは
// ParityTests が core/vectors/kagi.json を読んで機械で検める——
// 期待値をここへ書けばその瞬間に古くなり、ゲートが嘘をつき始めるからである。
//
// ## 英字（US）配列でも原器は成立する
//
// macOS の殻は英字配列のハード鍵盤を前提にするが、原器の 33 鍵が要る物理位置は
// US にもすべて在る。刻印だけが違ふ:
//
//        位置(kVK)  JIS の刻印   US の刻印
//   さ   0x27       :            '
//   し   0x29       ;            ;          ← 刻印が同じ
//   ^    0x18       ^            =
//
// 「し」が最も危い。**刻印も物理位置も同じなのに意味だけが違ふ**ので、
// 「打つと何の字が出るか」で引く実装は US 配列の機で黙つて「さ」になる。
// 例外も警告も出ない。位置で引けばこの誤爆はそもそも生まれず、
// 「JIS か US か」を実行時に当てる必要も消える。

import Foundation

/// 一つの鍵 — 原器の文字と、その物理位置の四つの呼び名。
public struct Kagi: Equatable, Sendable {
    /// 原器の文字（Gojuon の鍵）。
    public let genki: Character
    /// `KeyboardEvent.code`（W3C UI Events）。
    public let code: String
    /// 走査符号 set 1（Windows）。
    public let scan: UInt16
    /// Carbon の `kVK_ANSI_*`（macOS の `NSEvent.keyCode`）。
    public let mac: UInt16
    /// USB HID usage（iOS の `UIKey.keyCode`）。
    public let hid: UInt16

    public init(_ genki: Character, _ code: String, _ scan: UInt16, _ mac: UInt16, _ hid: UInt16) {
        self.genki = genki
        self.code = code
        self.scan = scan
        self.mac = mac
        self.hid = hid
    }
}

/// 原器が使ふ 33 鍵。清音 30 ＋ 前置シフト ＋ 濁点 ＋ 半濁点。
///
/// 並びは原器の紙面の順に合はせてある——表を眺めたときに配列の形が見えるやうに。
public enum KagiTable {
    public static let keys: [Kagi] = [
        // 右半面・数字列（あ〜お）
        Kagi("0", "Digit0", 0x0B, 0x1D, 0x27), Kagi("9", "Digit9", 0x0A, 0x19, 0x26),
        Kagi("8", "Digit8", 0x09, 0x1C, 0x25), Kagi("7", "Digit7", 0x08, 0x1A, 0x24),
        Kagi("6", "Digit6", 0x07, 0x16, 0x23),
        // 右半面・上段（か行）
        Kagi("p", "KeyP", 0x19, 0x23, 0x13), Kagi("o", "KeyO", 0x18, 0x1F, 0x12),
        Kagi("i", "KeyI", 0x17, 0x22, 0x0C), Kagi("u", "KeyU", 0x16, 0x20, 0x18),
        Kagi("y", "KeyY", 0x15, 0x10, 0x1C),
        // 右半面・中段（さ行）— 先頭の二鍵が配列で意味の変はる鍵
        Kagi(":", "Quote", 0x28, 0x27, 0x34),
        Kagi(";", "Semicolon", 0x27, 0x29, 0x33),
        Kagi("l", "KeyL", 0x26, 0x25, 0x0F), Kagi("k", "KeyK", 0x25, 0x28, 0x0E),
        Kagi("j", "KeyJ", 0x24, 0x26, 0x0D),
        // 左半面・数字列（た行）
        Kagi("5", "Digit5", 0x06, 0x17, 0x22), Kagi("4", "Digit4", 0x05, 0x15, 0x21),
        Kagi("3", "Digit3", 0x04, 0x14, 0x20), Kagi("2", "Digit2", 0x03, 0x13, 0x1F),
        Kagi("1", "Digit1", 0x02, 0x12, 0x1E),
        // 左半面・上段（な行）
        Kagi("t", "KeyT", 0x14, 0x11, 0x17), Kagi("r", "KeyR", 0x13, 0x0F, 0x15),
        Kagi("e", "KeyE", 0x12, 0x0E, 0x08), Kagi("w", "KeyW", 0x11, 0x0D, 0x1A),
        Kagi("q", "KeyQ", 0x10, 0x0C, 0x14),
        // 左半面・中段（は行）
        Kagi("g", "KeyG", 0x22, 0x05, 0x0A), Kagi("f", "KeyF", 0x21, 0x03, 0x09),
        Kagi("d", "KeyD", 0x20, 0x02, 0x07), Kagi("s", "KeyS", 0x1F, 0x01, 0x16),
        Kagi("a", "KeyA", 0x1E, 0x00, 0x04),
        // 面と逸らし
        Kagi("^", "Equal", 0x0D, 0x18, 0x2E),
        Kagi("b", "KeyB", 0x30, 0x0B, 0x05),
        Kagi("v", "KeyV", 0x2F, 0x09, 0x19),
    ]

    /// `kVK_ANSI_*` → 原器（macOS）。引けるやうに辞書で持つ（打鍵ごとに舐めない）。
    static let byMac: [UInt16: Character] = {
        var m = [UInt16: Character]()
        for k in keys { m[k.mac] = k.genki }
        return m
    }()

    /// HID usage → 原器（iOS のハード鍵盤）。
    static let byHid: [UInt16: Character] = {
        var m = [UInt16: Character]()
        for k in keys { m[k.hid] = k.genki }
        return m
    }()

    static let byCode: [String: Character] = {
        var m = [String: Character]()
        for k in keys { m[k.code] = k.genki }
        return m
    }()
}

/// `kVK_ANSI_*` から原器の文字へ（macOS）。原器に無い鍵は `nil`。
///
/// **`NSEvent.characters` で引いてはならない。** 出る字は配列に依るので、
/// US 配列の機で「し」が黙つて「さ」になる。加へて かな入力の状態でも壊れる。
public func genki(mac keyCode: UInt16) -> Character? {
    KagiTable.byMac[keyCode]
}

/// HID usage から原器の文字へ（iOS のハード鍵盤）。
///
/// `UIKey.keyCode`（`UIKeyboardHIDUsage`）の生値をそのまま渡す。
/// **`UIKey.characters` で引いてはならない**（理由は上と同じ）。
public func genki(hid usage: UInt16) -> Character? {
    KagiTable.byHid[usage]
}

/// `KeyboardEvent.code` から原器の文字へ（web の殻と同じ引き方）。
public func genki(code: String) -> Character? {
    KagiTable.byCode[code]
}

/// 原器の文字から `kVK_ANSI_*` へ（図の描画・試験用の逆引き）。
public func macKeyCode(of genki: Character) -> UInt16? {
    KagiTable.keys.first { $0.genki == genki }?.mac
}

/// 原器の文字から HID usage へ。
public func hidUsage(of genki: Character) -> UInt16? {
    KagiTable.keys.first { $0.genki == genki }?.hid
}
