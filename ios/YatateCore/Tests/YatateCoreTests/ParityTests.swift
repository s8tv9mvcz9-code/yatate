// 黄金ベクトルによるパリティ検査 — Swift の核が、共有核（core/・Rust）と同じ答へを出すか。
//
// ベクトルの出所は二つある（docs/ime/cross-platform.md §6）:
//
//   表（旧字 248 字）のロジック   … ssot/kyuji.py が SSOT   → scripts/gen_parity_vectors.py が書く
//   五十音の幾何・墨の氣配        … core/（Rust）が SSOT     → cargo run --bin gen-vectors が書く
//
// **ここに期待値は一行も書かない。** 書いた瞬間に古くなり、ゲートが嘘をつき始める
// （bungo-rag の eval/test_client_parity.py が学んだ規律の横展開）。
// Swift・Rust・将来の Kotlin/C++ が、同じ一つのファイルに従ふ。

import XCTest

@testable import YatateCore

/// リポジトリ根（このファイルの位置から辿る）。
/// swift test はソース木の中で走るので、これで確実に核のベクトルへ届く。
private let repoRoot: URL = {
    var url = URL(fileURLWithPath: #filePath)
    for _ in 0..<5 { url.deleteLastPathComponent() }  // …/ios/YatateCore/Tests/YatateCoreTests/
    return url
}()

private func loadVector(_ name: String) throws -> [String: Any] {
    let url = repoRoot.appendingPathComponent("core/vectors/\(name)")
    guard let data = try? Data(contentsOf: url) else {
        XCTFail("""
            黄金ベクトルが読めない: \(url.path)
            `python3 scripts/gen_parity_vectors.py` と `cd core && cargo run --bin gen-vectors` で生成すること
            """)
        return [:]
    }
    return try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
}

/// 空のベクトルは「何でも通る」ゲートになる＝沈黙して死ぬので、非空を要求する。
private func requireNonEmpty(_ rows: [Any], _ what: String) {
    XCTAssertFalse(rows.isEmpty, "\(what) が空（ゲートが無効化されてゐる）")
}

final class ParityTests: XCTestCase {

    // MARK: - 旧字（SSOT は Python）

    func testKyujiMapMatchesVectors() throws {
        let v = try loadVector("kyuji.json")
        let map = try XCTUnwrap(v["map"] as? [[String: String]])
        requireNonEmpty(map, "kyuji.map")

        XCTAssertEqual(map.count, v["map_size"] as? Int, "ベクトル自身の整合が壊れてゐる")
        XCTAssertEqual(KyujiTable.map.count, map.count, "写しの字数が SSOT と違ふ")

        for row in map {
            let shinji = try XCTUnwrap(row["in"])
            let kyuji = try XCTUnwrap(row["out"])
            XCTAssertEqual(toKyuji(shinji), kyuji, "\(shinji) → \(kyuji)")
        }

        let ambiguous = try XCTUnwrap(v["ambiguous"] as? [String])
        requireNonEmpty(ambiguous, "kyuji.ambiguous")
        for a in ambiguous {
            let c = try XCTUnwrap(a.first)
            XCTAssertTrue(KyujiTable.ambiguous.contains(c), "\(c) が曖昧字集合に無い")
            XCTAssertNil(KyujiTable.map[c], "曖昧字 \(c) が写像に混入してゐる")
        }
    }

    /// `to_kyuji`（1:1 置換）の境界例。
    ///
    /// なほ Swift 版は `to_kyuji_body`（【ポイント】以降の素通し）と
    /// ストリーム変換を**まだ持たない** — 鍵盤は確定のたびに全文を変換するので要らない。
    /// ベクトルにはその段が在るが、ここでは 1:1 置換の段だけを見る。
    func testToKyujiMatchesVectors() throws {
        let v = try loadVector("kyuji.json")
        let cases = try XCTUnwrap(v["to_kyuji"] as? [[String: String]])
        requireNonEmpty(cases, "kyuji.to_kyuji")
        for row in cases {
            let input = try XCTUnwrap(row["in"])
            XCTAssertEqual(toKyuji(input), try XCTUnwrap(row["out"]), "toKyuji(\(input))")
        }
    }

    // MARK: - 五十音の幾何（SSOT は核）

    func testGojuonMatchesVectors() throws {
        let v = try loadVector("gojuon.json")
        let kana = try XCTUnwrap(v["kana"] as? [[String: Any]])
        requireNonEmpty(kana, "gojuon.kana")
        XCTAssertEqual(kana.count, 150, "10 行 × 5 段 × 3 面 が揃つてゐない")

        let byName = Dictionary(
            uniqueKeysWithValues: (Gojuon.firstLine + Gojuon.secondLine).map { ($0.name, $0) })

        // 読み順（あ→か→…→わ）も核と揃つてゐること
        let order = try XCTUnwrap(v["reading_order"] as? [String])
        XCTAssertEqual((Gojuon.firstLine + Gojuon.secondLine).map(\.name), order)

        for row in kana {
            let gyoName = try XCTUnwrap(row["gyo"] as? String)
            let dan = try XCTUnwrap(row["dan"] as? Int)
            let deflectName = try XCTUnwrap(row["deflect"] as? String)
            var deflect: Deflect = .none
            if deflectName == "daku" { deflect = .daku }
            if deflectName == "ko" { deflect = .ko }
            let gyo = try XCTUnwrap(byName[gyoName], "行 \(gyoName) が無い")
            XCTAssertEqual(
                gyo.kana(dan: dan, deflect: deflect), row["kana"] as? String,
                "\(gyoName) \(dan) 段 \(row["deflect"] ?? "")")
        }
    }

    /// 逆引き（濁点・半濁点・小書きが基底の鍵へ畳まれること）。
    func testReverseLookupMatchesVectors() throws {
        let v = try loadVector("gojuon.json")
        let reverse = try XCTUnwrap(v["reverse"] as? [[String: Any]])
        requireNonEmpty(reverse, "gojuon.reverse")

        for row in reverse {
            let c = try XCTUnwrap((row["kana"] as? String)?.first)
            let got = Kehai.reverseMap[c]
            if let gyo = row["gyo"] as? String {
                XCTAssertEqual(got?.gyo, gyo, "\(c) の逆引き行")
                XCTAssertEqual(got?.dan, row["dan"] as? Int, "\(c) の逆引き段")
            } else {
                XCTAssertNil(got, "\(c) は体系外のはず")
            }
        }
    }

    // MARK: - 墨の氣配（SSOT は核）

    func testKehaiMatchesVectors() throws {
        let v = try loadVector("kehai.json")
        XCTAssertEqual(Kehai.minEvidence, v["min_evidence"] as? Int)

        let cases = try XCTUnwrap(v["cases"] as? [[String: Any]])
        requireNonEmpty(cases, "kehai.cases")

        for row in cases {
            let prev = (row["prev"] as? String)?.first
            let field = Kehai.field(after: prev)
            let label = prev.map(String.init) ?? "^"

            let expectEmpty = try XCTUnwrap(row["empty"] as? Bool)
            XCTAssertEqual(field.ink.isEmpty, expectEmpty, "\(label): 無地かどうか")

            // 峰（筆脈の先）
            if let peak = row["peak"] as? [String: String] {
                XCTAssertEqual(field.peak, try keyID(peak), "\(label): 峰")
            } else {
                XCTAssertNil(field.peak, "\(label): 峰は無いはず")
            }

            // 鍵の墨（件数も一致させる＝余計な鍵に墨を差してゐないこと）
            let inks = try XCTUnwrap(row["ink"] as? [[String: Any]])
            XCTAssertEqual(field.ink.count, inks.count, "\(label): 墨を差した鍵の数")
            for entry in inks {
                let key = try keyID(XCTUnwrap(entry["key"] as? [String: String]))
                let want = try XCTUnwrap(entry["ink"] as? Double)
                let got = field.ink[key] ?? 0
                XCTAssertEqual(got, want, accuracy: 1e-6, "\(label): \(key) の墨")
            }

            // 段の墨（行ごとに正規化された 5 スロット）
            let dans = try XCTUnwrap(row["dan"] as? [[String: Any]])
            XCTAssertEqual(field.dan.count, dans.count, "\(label): 段の墨を持つ行の数")
            for entry in dans {
                let gyo = try XCTUnwrap(entry["gyo"] as? String)
                let want = try XCTUnwrap(entry["dan"] as? [Double])
                let got = try XCTUnwrap(field.dan[gyo], "\(label): \(gyo) の段が無い")
                XCTAssertEqual(got.count, want.count, "\(label): \(gyo) の段数")
                for (i, w) in want.enumerated() {
                    XCTAssertEqual(got[i], w, accuracy: 1e-6, "\(label): \(gyo) の \(i) 段目")
                }
            }
        }
    }

    private func keyID(_ dict: [String: String]) throws -> KeyID {
        let id = try XCTUnwrap(dict["id"])
        let kind = try XCTUnwrap(dict["kind"])
        return kind == "gyo" ? .gyo(id) : .moji(id)
    }
}
