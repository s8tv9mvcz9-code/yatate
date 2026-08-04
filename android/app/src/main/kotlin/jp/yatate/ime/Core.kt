package jp.yatate.ime

// 核（Rust）への橋。**ここに頭脳は無い。**
//
// 打鍵の意味・変換・旧字確定はすべて `yatate_core` が決める。Kotlin 側は
// 描くことと Android の作法だけを引き受ける（docs/ime/cross-platform.md §4）。
//
// Android は iOS / macOS と違ひ、核を手で写さない。共有ライブラリを読めるので、
// henkan・bunsetsu・jisho が丸ごと載る——**初日から漢字変換が使へる**のは
// この一点による。

/// 核が返す指示。**番号は `android/rust/src/lib.rs` の `ACT_*` と一致する。**
object Act {
    const val UPDATE = 0            // 未確定文字列が変はつた
    const val SWALLOW = 1           // 食つたが見た目は変はらない
    const val PASSTHROUGH = 2       // 矢立の鍵でない
    const val COMMIT = 3            // 確定した
    const val COMMIT_THEN_UPDATE = 4 // 確定し、続けて新しい未確定が立つてゐる
    const val CANCEL = 5            // 捨てて閉ぢる
}

/// 引数を取らぬ操作。**番号は `nativeOp` の分岐と一致する。**
object Op {
    const val CONVERT = 0
    const val COMMIT = 1
    const val CANCEL = 2
    const val BACKSPACE = 3
    const val FOCUS_PREV = 4
    const val FOCUS_NEXT = 5
    const val SHRINK = 6
    const val GROW = 7
    const val PREV_CAND = 8
    const val NEXT_CAND = 9
    const val UNCONVERT = 10
}

/// 一つの行（ぎゃう）— 核の五十音表から来る。**殻は配列表を持たない。**
///
/// 持つてしまふと「図と実際の打鍵がずれる」事故が起きる。核が唯一の出所である。
data class Gyo(
    val name: String,
    val seion: List<String?>,
    val daku: List<String?>?,
    val ko: List<String?>?,
) {
    /// 段と逸らしから仮名を引く。空きスロットは null（何も入力しない）。
    fun kana(dan: Int, deflect: Deflect): String? {
        if (dan !in 0..4) return null
        return when (deflect) {
            Deflect.NONE -> seion[dan]
            Deflect.DAKU -> daku?.get(dan)
            Deflect.KO -> ko?.get(dan)
        }
    }
}

/// 逸らし — 書き下ろしスライド中の横方向の意味。
/// 濁点は字の右肩に打たれるものだから、右への逸らしが濁点になる。
enum class Deflect { NONE, DAKU, KO }

/// 五十音の地図（核から取る）。
class Layout(val first: List<Gyo>, val second: List<Gyo>) {
    val all: List<Gyo> get() = first + second
}

object Core {
    init {
        System.loadLibrary("yatate_android")
    }

    // ── JNI（`android/rust/src/lib.rs`）─────────────────────
    private external fun nativeNew(): Long
    private external fun nativeFree(ptr: Long)
    private external fun nativeKey(ptr: Long, codepoint: Int): Int
    private external fun nativeKana(ptr: Long, kana: String): Int
    private external fun nativeOp(ptr: Long, op: Int): Int
    private external fun nativeChoose(ptr: Long, index: Int): Int
    private external fun nativeReset(ptr: Long)
    private external fun nativePreedit(ptr: Long): String
    private external fun nativeCommitted(ptr: Long): String
    private external fun nativeCandidates(ptr: Long): String
    private external fun nativeFocusRange(ptr: Long): String
    private external fun nativeIsComposing(ptr: Long): Boolean
    private external fun nativeChosen(ptr: Long): Int
    private external fun nativeGojuon(): String
    private external fun nativeLines(): String

    /// 五十音の地図を核から組む。
    fun layout(): Layout = parseLayout(nativeGojuon(), nativeLines())

    /// 一つの入力欄ぶんの状態機械。**閉ぢ忘れると Rust 側が漏れる。**
    class Session : AutoCloseable {
        private var ptr: Long = nativeNew()

        private fun alive(): Long {
            check(ptr != 0L) { "閉ぢた Session を使つてゐる" }
            return ptr
        }

        /// 原器の一打（仮名ではなく**原器の鍵の文字**）。外付け鍵盤用。
        fun key(ch: Char): Int = nativeKey(alive(), ch.code)

        /// 仮名を直に積む（**二行配列はこちら**。原器の写像を経ない）。
        fun kana(kana: String): Int = nativeKana(alive(), kana)

        fun op(op: Int): Int = nativeOp(alive(), op)
        fun choose(index: Int): Int = nativeChoose(alive(), index)
        fun reset() = nativeReset(alive())

        val preedit: String get() = nativePreedit(alive())
        val committed: String get() = nativeCommitted(alive())
        val isComposing: Boolean get() = nativeIsComposing(alive())
        val chosen: Int get() = nativeChosen(alive())

        /// 注目してゐる文節の候補。無ければ空。
        val candidates: List<String>
            get() = nativeCandidates(alive()).let {
                if (it.isEmpty()) emptyList() else it.split('\n')
            }

        /// 注目文節が未確定文字列の何文字目から何文字か（下線を引き分けるため）。
        val focusRange: Pair<Int, Int>
            get() {
                val cols = nativeFocusRange(alive()).split('\t')
                if (cols.size != 2) return 0 to 0
                return (cols[0].toIntOrNull() ?: 0) to (cols[1].toIntOrNull() ?: 0)
            }

        override fun close() {
            if (ptr != 0L) {
                nativeFree(ptr)
                ptr = 0L
            }
        }
    }
}

// ── TSV の読み取り（**Kotlin 側で唯一の論理**なので試験で縛る）──────
//
// 核が吐く形:
//   五十音  行の名 \t 面(sei|daku|ko) \t 段0 … 段4   （空きは空文字）
//   並び    first \t あ \t か \t さ \t た \t な
//
// `object` の外に出してあるのは、`System.loadLibrary` を呼ばずに
// JVM の単体試験から叩けるやうにするためである。
internal fun parseLayout(gojuonTsv: String, linesTsv: String): Layout {
    val planes = HashMap<String, HashMap<String, List<String?>>>()
    for (line in gojuonTsv.lineSequence()) {
        if (line.isBlank()) continue
        val c = line.split('\t')
        if (c.size < 7) continue
        val slots = (2..6).map { c[it].ifEmpty { null } }
        planes.getOrPut(c[0]) { HashMap() }[c[1]] = slots
    }

    fun gyo(name: String): Gyo? {
        val p = planes[name] ?: return null
        val sei = p["sei"] ?: return null
        return Gyo(name, sei, p["daku"], p["ko"])
    }

    var first = emptyList<Gyo>()
    var second = emptyList<Gyo>()
    for (line in linesTsv.lineSequence()) {
        if (line.isBlank()) continue
        val c = line.split('\t')
        val names = c.drop(1).filter { it.isNotEmpty() }
        val gyos = names.mapNotNull { gyo(it) }
        when (c[0]) {
            "first" -> first = gyos
            "second" -> second = gyos
        }
    }
    return Layout(first, second)
}
