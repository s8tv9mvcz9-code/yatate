// 変換のパリティ検査 — Swift から駆動した変換が、核（core/src/henkan.rs・
// core/src/bunsetsu.rs）と同じ文節・同じ候補・同じ確定文を出すか。
//
// **ここに期待値は一行も書かない。** 出所は core/vectors/bunsetsu.json で、
// `cargo run --bin gen-vectors` が書く（ParityTests.swift と同じ規律）。
//
// この関門が守るもの: 殻が変換の頭脳を持たないこと。もし Swift 側に写しが
// 生まれれば、候補の順や費用が核とずれた瞬間にここが落ちる。

import XCTest

@testable import YatateCore

private let repoRootForHenkan: URL = {
    var url = URL(fileURLWithPath: #filePath)
    for _ in 0..<5 { url.deleteLastPathComponent() }  // …/ios/YatateCore/Tests/YatateCoreTests/
    return url
}()

private func loadHenkanVector() throws -> [String: Any] {
    let url = repoRootForHenkan.appendingPathComponent("core/vectors/bunsetsu.json")
    guard let data = try? Data(contentsOf: url) else {
        XCTFail("黄金ベクトルが読めない: \(url.path) — cd core && cargo run --bin gen-vectors")
        return [:]
    }
    return try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
}

final class HenkanParityTests: XCTestCase {

    /// 変換 → 文節・候補・確定文が核と一致する。
    func testQueriesMatchVectors() throws {
        let v = try loadHenkanVector()
        let queries = try XCTUnwrap(v["queries"] as? [[String: Any]])
        XCTAssertFalse(queries.isEmpty, "queries が空（ゲートが無効）")

        for row in queries {
            let query = try XCTUnwrap(row["query"] as? String)
            let h = Henkan()
            h.insertKana(query)
            XCTAssertEqual(h.yomi, query, "積んだ仮名が読みとして残る")

            let yomiList = try XCTUnwrap(row["yomi_list"] as? [String])
            guard !yomiList.isEmpty else {
                // 分けられない（空の入力）ものは**変換しない**。呑んで仮名の段に留まる。
                XCTAssertEqual(h.convert(), .swallow, "分けられないのに変換してゐる")
                XCTAssertEqual(h.phase, .kana)
                continue
            }

            XCTAssertEqual(h.convert(), .update, "変換できるはず: \(query)")
            XCTAssertEqual(h.phase, .henkan, query)

            // 文節の切れ目
            XCTAssertEqual(h.segments.map(\.yomi), yomiList, "文節の切れ目: \(query)")

            // 未確定に出る表記（**新字体のまま**）
            XCTAssertEqual(h.preedit, row["compose"] as? String, "未確定の表記: \(query)")

            // 文節ごとの候補（順序・費用・辞書由来かまで）
            let segs = try XCTUnwrap(row["segments"] as? [[String: Any]])
            XCTAssertEqual(h.segments.count, segs.count, query)
            for (i, seg) in segs.enumerated() {
                // 注目を当てないと候補は読めない（殻が候補窓を出すのと同じ道）
                while h.focus < i { XCTAssertEqual(h.focusNext(), .update, query) }
                XCTAssertEqual(h.focus, i, query)
                XCTAssertEqual(h.chosen, seg["chosen"] as? Int, "\(query) の \(i) 文節の既定")

                let want = try XCTUnwrap(seg["candidates"] as? [[String: Any]])
                let got = h.candidates
                XCTAssertEqual(got.count, want.count, "\(query) の \(i) 文節の候補数")
                for (j, c) in want.enumerated() where j < got.count {
                    XCTAssertEqual(got[j].surface, c["surface"] as? String, "\(query) \(i)-\(j)")
                    XCTAssertEqual(got[j].cost, c["cost"] as? Int, "\(query) \(i)-\(j) の費用")
                    XCTAssertEqual(got[j].inJisho, c["in_jisho"] as? Bool, "\(query) \(i)-\(j)")
                }
            }

            // 確定 —— **旧字体はここで初めて定まる**
            XCTAssertEqual(h.commit().committed, row["commit"] as? String, "確定文: \(query)")
            XCTAssertFalse(h.isComposing, query)
        }
    }

    /// **仮名のままへ必ず戻れる。**
    ///
    /// どの文節にも「読みそのままの表記」が候補として必ず居ること。
    /// これが欠けると、変換した途端に**打つた通りの仮名へ戻る道が消える**
    /// ——歴史的仮名遣ひを打つ道具として、それは致命である。
    ///
    /// なほ、その控へが候補の**末尾**に来るとは限らない。読みそのものが辞書に
    /// 在る語（例: なり）なら費用の順で先頭寄りに立つ。位置ではなく**在ること**を見る。
    func testEveryBunsetsuKeepsTheKanaForm() throws {
        let v = try loadHenkanVector()
        for row in try XCTUnwrap(v["queries"] as? [[String: Any]]) {
            let query = try XCTUnwrap(row["query"] as? String)
            let h = Henkan()
            h.insertKana(query)
            h.convert()
            for (i, seg) in h.segments.enumerated() {
                while h.focus < i { h.focusNext() }
                XCTAssertTrue(
                    h.candidates.contains { $0.surface == seg.yomi },
                    "\(query): 文節「\(seg.yomi)」に仮名のままの候補が無い")
            }
        }
    }

    /// 読みを被覆できないなら変換しない（核の `coverage_error`）。
    ///
    /// 文節の読みを繋げたものが打つた仮名と一致しないなら、それは
    /// 「打つてゐない字が画面に出る」といふことなので、変換しない方がまし。
    func testConversionNeverInventsCharacters() throws {
        let v = try loadHenkanVector()
        for row in try XCTUnwrap(v["queries"] as? [[String: Any]]) {
            let query = try XCTUnwrap(row["query"] as? String)
            let h = Henkan()
            h.insertKana(query)
            h.convert()
            guard h.phase == .henkan else { continue }
            XCTAssertEqual(
                h.segments.map(\.yomi).joined(), query,
                "文節を繋げても打つた仮名に戻らない: \(query)")
        }
    }

    /// 原器で打つてから変換する道（macOS・Windows の殻がやること）。
    func testTypingThenConverting() {
        let h = Henkan()
        // 「けふはよきてんきなり」= u d g ^y o 2 ^^ o t ^4
        for ch in "udg^yo2^^ot^4" { _ = h.key(ch) }
        XCTAssertEqual(h.preedit, "けふはよきてんきなり")

        h.convert()
        XCTAssertEqual(h.preedit, "今日は良き天気なり", "候補を並べる間は新字体のまま")

        // 変換を解いても読みは失はれない
        h.unconvert()
        XCTAssertEqual(h.phase, .kana)
        XCTAssertEqual(h.preedit, "けふはよきてんきなり")

        h.convert()
        XCTAssertEqual(h.commit().committed, "今日は良き天氣なり", "確定で旧字が定まる")
    }

    /// 注目文節の範囲 — 下線を引き分けるために殻が使ふ。
    func testFocusRangeTracksSegments() {
        let h = Henkan()
        h.insertKana("けふはよきてんきなり")
        h.convert()

        var start = 0
        for (i, seg) in h.segments.enumerated() {
            while h.focus < i { h.focusNext() }
            let range = h.focusRange
            XCTAssertEqual(range.start, start, "\(i) 文節の始まり")
            XCTAssertEqual(range.length, seg.surface.count, "\(i) 文節の長さ")
            start += seg.surface.count
        }
        XCTAssertEqual(start, h.preedit.count, "文節を繋げると未確定文字列に戻る")
    }

    /// **核はスカラで、Apple は UTF-16 で数へる。**
    ///
    /// 黄金ベクトルの仮名も常用の漢字も一字＝UTF-16 一単位なので、
    /// `testFocusRangeTracksSegments` はこの換算に一度も触れない。
    /// **触れないまま壊れてゐても誰も気づかない**——下線が一文字ぶんずれて
    /// 「どの文節を直してゐるか」を嘘で示す、といふ形で出る。
    /// だから基本多言語面の外の字（𠮷）を手で置いて縛る。
    func testScalarRangeConvertsToUTF16() {
        // 「𠮷」は UTF-16 で 2 単位・スカラで 1 つ。「野」は両方 1。
        let text = "𠮷野家"
        XCTAssertEqual(text.unicodeScalars.count, 3)
        XCTAssertEqual(text.utf16.count, 4, "面外の字が UTF-16 で二単位を占める")

        // 先頭の一字（𠮷）
        var r = utf16Range(scalarStart: 0, scalarLength: 1, in: text)
        XCTAssertEqual(r.location, 0)
        XCTAssertEqual(r.length, 2, "𠮷 は UTF-16 で 2")

        // 二字目（野）— **ここが要**。素朴に数へると location が 1 になり一文字ずれる
        r = utf16Range(scalarStart: 1, scalarLength: 1, in: text)
        XCTAssertEqual(r.location, 2, "面外の字の後ろは 2 から始まる")
        XCTAssertEqual(r.length, 1)

        // 全体
        r = utf16Range(scalarStart: 0, scalarLength: 3, in: text)
        XCTAssertEqual(r.location, 0)
        XCTAssertEqual(r.length, 4)

        // 面内だけの文（普段の道）は素朴な数へ方と一致する
        r = utf16Range(scalarStart: 3, scalarLength: 2, in: "今日は良き天氣なり")
        XCTAssertEqual(r.location, 3)
        XCTAssertEqual(r.length, 2)

        // 範囲外を渡されても壊れない（NSRange に負や飛び出しを渡すと落ちる）
        r = utf16Range(scalarStart: 99, scalarLength: 5, in: text)
        XCTAssertEqual(r.length, 0)
        XCTAssertLessThanOrEqual(r.location, text.utf16.count)
        r = utf16Range(scalarStart: -1, scalarLength: -1, in: text)
        XCTAssertEqual(r.location, 0)
        XCTAssertEqual(r.length, 0)
    }

    /// 殻が使ふ口（`focusRangeUTF16`）が、素の換算と同じ答へを出すこと。
    func testFocusRangeUTF16MatchesTheConversion() {
        let h = Henkan()
        h.insertKana("けふはよきてんきなり")
        h.convert()
        for i in 0..<h.segments.count {
            while h.focus < i { h.focusNext() }
            let (start, length) = h.focusRange
            let want = utf16Range(scalarStart: start, scalarLength: length, in: h.preedit)
            let got = h.focusRangeUTF16
            XCTAssertEqual(got.location, want.location, "\(i) 文節")
            XCTAssertEqual(got.length, want.length, "\(i) 文節")
        }
    }

    /// 変換中に原器の鍵が来たら、いま選んでゐる形で確定してから積み直す。
    func testKeyDuringHenkanCommitsThenStartsAnew() {
        let h = Henkan()
        h.insertKana("けふ")
        h.convert()
        // "0" は あ（第一面）
        let act = h.key("0")
        XCTAssertEqual(act, .commitThenUpdate("今日"))
        XCTAssertEqual(h.phase, .kana)
        XCTAssertEqual(h.preedit, "あ", "打つた字を捨てない")
    }
}
