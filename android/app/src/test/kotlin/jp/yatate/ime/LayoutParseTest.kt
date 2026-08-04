package jp.yatate.ime

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

// Kotlin 側で**唯一の論理**である TSV の読み取りを縛る。
//
// 打鍵の意味も変換も旧字も核（Rust）が決めるので、Kotlin に試験すべきものは薄い
// ——これは意図した配分である（web 殻が「試験はすべて Rust 側」と書いてゐるのと同じ）。
// ここが壊れると鍵盤の図が黙つて狂ふので、そこだけを見る。
//
// 註: JVM の単体試験なので `System.loadLibrary` は呼べない。ゆゑに
// `parseLayout` は `Core` object の外に出してある。核が実際に吐く形との一致は
// `android/rust` の `五十音の地図が吐ける` が縛る。

class LayoutParseTest {

    private val gojuon = """
        あ	sei	あ	い	う	え	お
        か	sei	か	き	く	け	こ
        か	daku	が	ぎ	ぐ	げ	ご
        は	sei	は	ひ	ふ	へ	ほ
        は	daku	ば	び	ぶ	べ	ぼ
        は	ko	ぱ	ぴ	ぷ	ぺ	ぽ
        わ	sei	わ	ゐ		ゑ	を
    """.trimIndent()

    private val lines = "first\tあ\tか\nsecond\tは\tわ\n"

    @Test
    fun `行が核の並びで取れる`() {
        val l = parseLayout(gojuon, lines)
        assertEquals(listOf("あ", "か"), l.first.map { it.name })
        assertEquals(listOf("は", "わ"), l.second.map { it.name })
    }

    @Test
    fun `清音の五段が読める`() {
        val l = parseLayout(gojuon, lines)
        val ka = l.all.first { it.name == "か" }
        assertEquals("か", ka.kana(0, Deflect.NONE))
        assertEquals("こ", ka.kana(4, Deflect.NONE))
    }

    @Test
    fun `濁点と半濁点の面を持つ行だけが返す`() {
        val l = parseLayout(gojuon, lines)
        val ka = l.all.first { it.name == "か" }
        val ha = l.all.first { it.name == "は" }
        assertEquals("が", ka.kana(0, Deflect.DAKU))
        assertNull("か行に半濁点は無い", ka.kana(0, Deflect.KO))
        assertEquals("ぱ", ha.kana(0, Deflect.KO))
    }

    /// 空きスロットは null。**梯子の位置を詰めない**——や行・わ行の空きを詰めると
    /// 指の筋肉記憶が壊れる（docs/ime/layout.md §2）。
    @Test
    fun `空きスロットは詰めずに残る`() {
        val l = parseLayout(gojuon, lines)
        val wa = l.all.first { it.name == "わ" }
        assertEquals("わ", wa.kana(0, Deflect.NONE))
        assertEquals("ゐ", wa.kana(1, Deflect.NONE))
        assertNull("う段は空き", wa.kana(2, Deflect.NONE))
        assertEquals("ゑ", wa.kana(3, Deflect.NONE))
        assertEquals("を", wa.kana(4, Deflect.NONE))
    }

    @Test
    fun `範囲外の段は何も返さない`() {
        val l = parseLayout(gojuon, lines)
        val a = l.all.first { it.name == "あ" }
        assertNull(a.kana(-1, Deflect.NONE))
        assertNull(a.kana(5, Deflect.NONE))
    }

    @Test
    fun `壊れた行は黙つて捨てる`() {
        // 列の足りない行が混ざつても、他の行を巻き込まないこと
        val broken = "あ\tsei\tあ\nか\tsei\tか\tき\tく\tけ\tこ\n"
        val l = parseLayout(broken, "first\tあ\tか\n")
        assertEquals(listOf("か"), l.first.map { it.name })
        assertTrue(l.second.isEmpty())
    }
}
