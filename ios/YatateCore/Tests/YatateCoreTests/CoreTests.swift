// YatateCore の検証。旧字表は app/kyuji.py が SSOT であり、ここでは
// 生成物の性質（件数・曖昧字の除外・写像例）を検査する。
// 本格的なパリティベクトル（eval/test_kana.py の写し）は roadmap M1。

import XCTest
@testable import YatateCore

final class CoreTests: XCTestCase {
    func testKyujiMap() {
        XCTAssertEqual(KyujiTable.map.count, 248)
        XCTAssertEqual(toKyuji("万国の学"), "萬國の學")
        // 曖昧な新字体（弁予余欠芸）は写像から除外され、素通しになる
        XCTAssertEqual(toKyuji("弁"), "弁")
        XCTAssertTrue(KyujiTable.ambiguous.contains("弁"))
        XCTAssertTrue(KyujiTable.map.keys.allSatisfy { !KyujiTable.ambiguous.contains($0) })
    }

    func testGojuonLadder() {
        XCTAssertEqual(Gojuon.ka.kana(dan: 1), "き")
        XCTAssertEqual(Gojuon.ka.kana(dan: 1, deflect: .daku), "ぎ")
        XCTAssertEqual(Gojuon.ha.kana(dan: 0, deflect: .ko), "ぱ")
        XCTAssertEqual(Gojuon.ta.kana(dan: 2, deflect: .ko), "っ")
        // 歴史的仮名遣ひの第一級市民
        XCTAssertEqual(Gojuon.wa.kana(dan: 1), "ゐ")
        XCTAssertEqual(Gojuon.wa.kana(dan: 3), "ゑ")
        // 空きスロットは埋めない（位置の筋肉記憶を守る）
        XCTAssertNil(Gojuon.ya.kana(dan: 1))
        XCTAssertNil(Gojuon.wa.kana(dan: 2))
        XCTAssertNil(Gojuon.na.kana(dan: 0, deflect: .daku))
    }

    func testKehai() {
        XCTAssertFalse(KanaBigram.table.isEmpty)
        // 空の作業帯 = 連なりの開始分布（助詞たちの息づかひ）
        let start = Kehai.field(after: nil)
        XCTAssertNotNil(start.peak)
        // 「け」の後は れ・り（けれ/けり）— ら行に墨が差す
        let ke = Kehai.field(after: "け")
        XCTAssertGreaterThan(ke.ink[.gyo("ら")] ?? 0, 0)
        XCTAssertEqual(ke.dan["ら"]?.count, 5)
        // 「す」の後は文の終はる気配 — 。 の鍵が濃い
        let su = Kehai.field(after: "す")
        XCTAssertGreaterThan(su.ink[.moji("。")] ?? 0, 0.5)
        // 表に無い前字（漢字）は開始分布へ退避する
        XCTAssertNotNil(Kehai.field(after: "漢").peak)
        // 墨は 0...1 に正規化されてゐる
        XCTAssertTrue(start.ink.values.allSatisfy { $0 > 0 && $0 <= 1.0 })
    }

    func testComposer() {
        var c = Composer()
        c.append("けふは学の日")
        c.deleteLast()
        XCTAssertEqual(c.text, "けふは学の")
        XCTAssertEqual(c.commit(), "けふは學の")
        XCTAssertTrue(c.isEmpty)
    }
}
