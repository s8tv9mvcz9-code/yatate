// 原器と鍵の地図のパリティ検査 — Swift の写しが核（core/・Rust）とずれてゐないか。
//
// **ここに期待値は一行も書かない。** 出所は core/vectors/ で、
// 書いた瞬間に古くなりゲートが嘘をつき始める（ParityTests.swift と同じ規律）。
//
// ここが守るもの:
//
//   genki.json … 打鍵 → 仮名（第一面・第二面・濁点の後置・打鍵列）
//   kagi.json  … 原器の文字 → 物理位置（code・走査符号・kVK・HID）
//
// とりわけ kagi は、macOS（kVK）と iOS（HID）の列が Windows・web と
// **同じ鍵を指してゐる**ことを縛る。ここがずれると英字配列の実機で
// 「し」が黙つて「さ」になる——例外も警告も出ない類の事故である。

import XCTest

@testable import YatateCore

private let repoRootForKagi: URL = {
    var url = URL(fileURLWithPath: #filePath)
    for _ in 0..<5 { url.deleteLastPathComponent() }  // …/ios/YatateCore/Tests/YatateCoreTests/
    return url
}()

private func loadKagiVector(_ name: String) throws -> [String: Any] {
    let url = repoRootForKagi.appendingPathComponent("core/vectors/\(name)")
    guard let data = try? Data(contentsOf: url) else {
        XCTFail("黄金ベクトルが読めない: \(url.path) — cd core && cargo run --bin gen-vectors")
        return [:]
    }
    return try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
}

final class GenkiParityTests: XCTestCase {

    func testSpecialKeysMatchVectors() throws {
        let v = try loadKagiVector("genki.json")
        XCTAssertEqual(String(GenkiKey.shift), v["shift"] as? String)
        XCTAssertEqual(String(GenkiKey.dakuten), v["dakuten"] as? String)
        XCTAssertEqual(String(GenkiKey.handakuten), v["handakuten"] as? String)
    }

    func testFirstPlaneMatchesVectors() throws {
        let v = try loadKagiVector("genki.json")
        let rows = try XCTUnwrap(v["first_plane"] as? [[String: String]])
        XCTAssertFalse(rows.isEmpty, "first_plane が空（ゲートが無効）")
        XCTAssertEqual(firstPlane.count, rows.count, "第一面の鍵数が SSOT と違ふ")
        for row in rows {
            let key = try XCTUnwrap(row["key"]?.first)
            let kana = try XCTUnwrap(row["kana"])
            XCTAssertEqual(typeKeys(String(key)), kana, "第一面 '\(key)' → \(kana)")
        }
    }

    func testSecondPlaneMatchesVectors() throws {
        let v = try loadKagiVector("genki.json")
        let rows = try XCTUnwrap(v["second_plane"] as? [[String: String]])
        XCTAssertFalse(rows.isEmpty, "second_plane が空（ゲートが無効）")
        XCTAssertEqual(secondPlane.count, rows.count, "第二面の鍵数が SSOT と違ふ")
        for row in rows {
            let key = try XCTUnwrap(row["key"])
            let kana = try XCTUnwrap(row["kana"])
            // 前置シフトを置いてから打つ
            XCTAssertEqual(typeKeys("^" + key), kana, "第二面 '^\(key)' → \(kana)")
        }
    }

    /// 打鍵列（`^^` の「ん」・濁点の後置・原器に無い鍵の黙殺まで）。
    func testSequencesMatchVectors() throws {
        let v = try loadKagiVector("genki.json")
        let rows = try XCTUnwrap(v["sequences"] as? [[String: String]])
        XCTAssertFalse(rows.isEmpty, "sequences が空（ゲートが無効）")
        for row in rows {
            let keys = try XCTUnwrap(row["keys"])
            let kana = try XCTUnwrap(row["kana"])
            XCTAssertEqual(typeKeys(keys), kana, "打鍵列 \(keys)")
        }
    }

    /// 前置シフトは**逐次打鍵**であり、一打だけ効く。
    func testPrefixShiftLastsOneKey() {
        var g = Genki()
        XCTAssertEqual(g.press("^", last: nil), .none)
        XCTAssertTrue(g.shifted)
        XCTAssertEqual(g.press("0", last: nil), .insert("ま"))
        XCTAssertFalse(g.shifted, "一打で降りる")
        XCTAssertEqual(g.press("0", last: nil), .insert("あ"))
    }

    /// 小書き（ゃゅょっ）は原器では未定ゆゑ**発明しない**。
    func testNoInventedSmallKana() {
        XCTAssertNil(handakuten("や"))
        XCTAssertNil(handakuten("つ"))
        XCTAssertNil(handakuten("か"), "半濁点は は行だけ")
    }
}

final class KagiParityTests: XCTestCase {

    private func rows() throws -> [[String: Any]] {
        let v = try loadKagiVector("kagi.json")
        let keys = try XCTUnwrap(v["keys"] as? [[String: Any]])
        XCTAssertFalse(keys.isEmpty, "kagi.keys が空（ゲートが無効）")
        return keys
    }

    func testTableMatchesVectorsExactly() throws {
        let keys = try rows()
        XCTAssertEqual(KagiTable.keys.count, keys.count, "鍵の数が SSOT と違ふ")

        for row in keys {
            let genkiCh = try XCTUnwrap((row["genki"] as? String)?.first)
            guard let mine = KagiTable.keys.first(where: { $0.genki == genkiCh }) else {
                XCTFail("原器の '\(genkiCh)' が Swift の表に無い")
                continue
            }
            XCTAssertEqual(mine.code, row["code"] as? String, "'\(genkiCh)' の code")
            XCTAssertEqual(Int(mine.scan), row["scan"] as? Int, "'\(genkiCh)' の走査符号")
            XCTAssertEqual(Int(mine.mac), row["mac"] as? Int, "'\(genkiCh)' の kVK")
            XCTAssertEqual(Int(mine.hid), row["hid"] as? Int, "'\(genkiCh)' の HID usage")
        }
    }

    /// 四つの呼び名すべてから同じ文字へ引けること。
    func testLookupsRoundTrip() throws {
        for row in try rows() {
            let genkiCh = try XCTUnwrap((row["genki"] as? String)?.first)
            let mac = UInt16(try XCTUnwrap(row["mac"] as? Int))
            let hid = UInt16(try XCTUnwrap(row["hid"] as? Int))
            let code = try XCTUnwrap(row["code"] as? String)

            XCTAssertEqual(genki(mac: mac), genkiCh, "kVK \(mac)")
            XCTAssertEqual(genki(hid: hid), genkiCh, "HID \(hid)")
            XCTAssertEqual(genki(code: code), genkiCh, code)
            XCTAssertEqual(macKeyCode(of: genkiCh), mac)
            XCTAssertEqual(hidUsage(of: genkiCh), hid)
        }
    }

    /// **この表の存在理由。** 原器の 33 鍵すべてが、位置から引いて打てること。
    ///
    /// 位置 → 原器の文字 → 仮名、が最後まで繋がるかを見る。
    /// どこか一つでも欠ければ「その鍵だけ黙つて効かない」になる。
    func testEveryKeyIsTypableFromPhysicalPosition() throws {
        for row in try rows() {
            let mac = UInt16(try XCTUnwrap(row["mac"] as? Int))
            let hid = UInt16(try XCTUnwrap(row["hid"] as? Int))
            let fromMac = try XCTUnwrap(genki(mac: mac), "kVK \(mac) から原器が引けない")
            let fromHid = try XCTUnwrap(genki(hid: hid), "HID \(hid) から原器が引けない")
            XCTAssertEqual(fromMac, fromHid, "macOS と iOS が違ふ鍵を指してゐる")

            var s = Session()
            XCTAssertTrue(s.wantsKey(fromMac), "'\(fromMac)' を殻が受けない")
        }
    }

    /// 配列で意味の変はる三鍵。**英字配列でも同じ指の置き場**であること。
    ///
    /// US 刻印では `'` `;` `=` に当たるが、原器は位置で引くので意味は動かない。
    func testThreeShiftingKeysStayPositional() throws {
        let keys = try rows()
        func vector(_ ch: String) throws -> [String: Any] {
            try XCTUnwrap(keys.first { ($0["genki"] as? String) == ch }, "\(ch) が無い")
        }

        for (ch, kana) in [(":", "さ"), (";", "し"), ("^", "")] {
            let row = try vector(ch)
            let mac = UInt16(try XCTUnwrap(row["mac"] as? Int))
            let hid = UInt16(try XCTUnwrap(row["hid"] as? Int))
            XCTAssertEqual(genki(mac: mac), Character(ch), "kVK から \(ch)")
            XCTAssertEqual(genki(hid: hid), Character(ch), "HID から \(ch)")
            if !kana.isEmpty {
                XCTAssertEqual(typeKeys(ch), kana, "\(ch) は \(kana)")
            }
        }

        // 「し」が「さ」に化けてゐないこと（この一行がこの試験の主目的である）
        let shi = try vector(";")
        let sa = try vector(":")
        XCTAssertNotEqual(shi["mac"] as? Int, sa["mac"] as? Int)
        XCTAssertNotEqual(shi["hid"] as? Int, sa["hid"] as? Int)
        XCTAssertEqual(typeKeys(";"), "し")
        XCTAssertEqual(typeKeys(":"), "さ")
    }

    /// 機能キーを原器が食つてはいけない（HID の Enter・Space・Esc・Tab）。
    func testFunctionKeysAreNotGenki() {
        for usage: UInt16 in [0x28, 0x2C, 0x29, 0x2B] {
            XCTAssertNil(genki(hid: usage), "HID \(usage) を原器が食つてはいけない")
        }
        // macOS: kVK_Return 0x24・kVK_Tab 0x30・kVK_Space 0x31・kVK_Escape 0x35
        for code: UInt16 in [0x24, 0x30, 0x31, 0x35] {
            XCTAssertNil(genki(mac: code), "kVK \(code) を原器が食つてはいけない")
        }
    }
}
