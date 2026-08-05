// YatateCore の検証 — 核（core/・Rust）へ載せ替へた後も、Swift から見える
// 振る舞ひが変はつてゐないこと。
//
// **表の SSOT はここには無い。** 旧字は ssot/kyuji.py、五十音と氣配は核である。
// ここが見るのは「Swift の口から同じ答へが出るか」で、
// 数の一致そのものは ParityTests が黄金ベクトルで縛る。

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

    func testGojuonLadder() throws {
        let ka = try XCTUnwrap(Gojuon.gyo(named: "か"))
        let ha = try XCTUnwrap(Gojuon.gyo(named: "は"))
        let ta = try XCTUnwrap(Gojuon.gyo(named: "た"))
        let na = try XCTUnwrap(Gojuon.gyo(named: "な"))
        let ya = try XCTUnwrap(Gojuon.gyo(named: "や"))
        let wa = try XCTUnwrap(Gojuon.gyo(named: "わ"))

        XCTAssertEqual(ka.kana(dan: 1), "き")
        XCTAssertEqual(ka.kana(dan: 1, deflect: .daku), "ぎ")
        XCTAssertEqual(ha.kana(dan: 0, deflect: .ko), "ぱ")
        XCTAssertEqual(ta.kana(dan: 2, deflect: .ko), "っ")
        // 歴史的仮名遣ひの第一級市民
        XCTAssertEqual(wa.kana(dan: 1), "ゐ")
        XCTAssertEqual(wa.kana(dan: 3), "ゑ")
        // 空きスロットは埋めない（位置の筋肉記憶を守る）
        XCTAssertNil(ya.kana(dan: 1))
        XCTAssertNil(wa.kana(dan: 2))
        XCTAssertNil(na.kana(dan: 0, deflect: .daku))
        // 二行配列の列も核の並びから来る
        XCTAssertEqual(Gojuon.firstLine.map(\.name), ["あ", "か", "さ", "た", "な"])
        XCTAssertEqual(Gojuon.secondLine.map(\.name), ["は", "ま", "や", "ら", "わ"])
    }

    func testKehai() {
        // 空の作業帯 = 連なりの開始分布（助詞たちの息づかひ）
        let start = Kehai.field(after: nil)
        XCTAssertNotNil(start.peak)
        XCTAssertFalse(start.ink.isEmpty, "bigram の表が核から届いてゐない")
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

    /// 核の手綱が入力欄ごとに独立してゐること。
    ///
    /// 参照型にしたので、写しても同じ状態を指す——**それが正しい**。
    /// 別々に起こしたものが混ざらないことを確かめる（混ざれば別の欄へ字が漏れる）。
    func testSessionsAreIndependent() {
        let a = Session()
        let b = Session()
        _ = a.key("0")  // あ
        XCTAssertEqual(a.preedit, "あ")
        XCTAssertEqual(b.preedit, "", "別の入力欄へ漏れてゐる")
        b.cancel()
        XCTAssertEqual(a.preedit, "あ", "他方の取り消しに巻き込まれてゐる")
    }
}
