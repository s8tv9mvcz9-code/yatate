// 鍵の物理位置 — 原器の文字 ↔ 指の置き場。
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
// ## 引くのは核である
//
// 表は核（core/src/kagi.rs）に一枚だけ在り、ここはそれを呼ぶ。M5-b2 より前は
// Swift にも同じ 33 行が写してあつたが、写しを置く理由が消えた——
// 四つの殻（web・Windows・macOS・iOS）が同じ一枚を見る。
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

import YatateFFI

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
/// 並びは核が持つ原器の紙面の順そのままである——表を眺めたときに配列の形が見えるやうに。
public enum KagiTable {
    public static let keys: [Kagi] = {
        coreRows(coreText(yatate_kagi_table())).compactMap { row in
            guard row.count == 5, let genki = row[0].first,
                let scan = UInt16(row[2]), let mac = UInt16(row[3]), let hid = UInt16(row[4])
            else { return nil }
            return Kagi(genki, row[1], scan, mac, hid)
        }
    }()
}

/// `kVK_ANSI_*` から原器の文字へ（macOS）。原器に無い鍵は `nil`。
///
/// **`NSEvent.characters` で引いてはならない。** 出る字は配列に依るので、
/// US 配列の機で「し」が黙つて「さ」になる。加へて かな入力の状態でも壊れる。
public func genki(mac keyCode: UInt16) -> Character? {
    coreChar(yatate_genki_of_mac(keyCode))
}

/// HID usage から原器の文字へ（iOS のハード鍵盤）。
///
/// `UIKey.keyCode`（`UIKeyboardHIDUsage`）の生値をそのまま渡す。
/// **`UIKey.characters` で引いてはならない**（理由は上と同じ）。
public func genki(hid usage: UInt16) -> Character? {
    coreChar(yatate_genki_of_hid(usage))
}

/// `KeyboardEvent.code` から原器の文字へ（web の殻と同じ引き方）。
public func genki(code: String) -> Character? {
    coreChar(code.withCore { yatate_genki_of_code($0) })
}

/// 走査符号 set 1 から原器の文字へ（Windows の殻と同じ引き方）。
public func genki(scan: UInt16) -> Character? {
    coreChar(yatate_genki_of_scan(scan))
}

/// 原器の文字から `kVK_ANSI_*` へ（図の描画・試験用の逆引き）。
public func macKeyCode(of genki: Character) -> UInt16? {
    KagiTable.keys.first { $0.genki == genki }?.mac
}

/// 原器の文字から HID usage へ。
public func hidUsage(of genki: Character) -> UInt16? {
    KagiTable.keys.first { $0.genki == genki }?.hid
}
