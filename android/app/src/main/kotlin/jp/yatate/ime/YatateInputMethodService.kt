package jp.yatate.ime

import android.inputmethodservice.InputMethodService
import android.view.KeyEvent
import android.view.View
import android.view.inputmethod.EditorInfo

// 矢立 — Android の殻（`InputMethodService`）。
//
// **ここは薄い。** 打鍵の意味も変換も旧字も決めるのは核（Rust）で、
// この class が引き受けるのは Android の作法だけ——`InputConnection` へ
// 未確定文字列を置き、確定文字列を挿し込む。
//
// TSF の composition・IMKit の marked text・Android の composing text は
// **呼び名が違ふだけで同じもの**である（core/src/session.rs）。
//
// ## 外付け鍵盤も受ける
//
// Android は iOS のキーボード拡張と違ひ、**ハードキーを受け取れる**
// （`onKeyDown`）。ゆゑに硝子の二行配列と原器の物理鍵盤の両方が、
// 同じ一つの核へ入る。

class YatateInputMethodService : InputMethodService() {

    private var session: Core.Session? = null
    private var board: FutakudariView? = null

    override fun onCreate() {
        super.onCreate()
        session = Core.Session()
    }

    override fun onDestroy() {
        session?.close()
        session = null
        super.onDestroy()
    }

    override fun onCreateInputView(): View {
        val v = FutakudariView(this)
        v.onKana = { kana -> session?.let { apply(it.kana(kana)) } }
        v.onConvert = { op(Op.CONVERT) }
        v.onCommit = { op(Op.COMMIT) }
        v.onDelete = { deleteOrBackspace() }
        v.onNextCandidate = { op(Op.NEXT_CAND) }
        v.onChooseCandidate = { i -> session?.let { apply(it.choose(i)) } }
        board = v
        return v
    }

    /// 入力欄が替はつた。抱へてゐた未確定は持ち越さない。
    override fun onStartInput(info: EditorInfo?, restarting: Boolean) {
        super.onStartInput(info, restarting)
        session?.reset()
        currentInputConnection?.finishComposingText()
        redraw()
    }

    override fun onFinishInput() {
        session?.reset()
        currentInputConnection?.finishComposingText()
        super.onFinishInput()
    }

    // ── 外付け鍵盤（原器）──────────────────────────────────
    //
    // 硝子と違ひ、こちらは**原器の鍵の文字**を核へ渡す。
    // 位置で引くべきだが、Android の `KeyEvent` は配列を通した後の
    // `unicodeChar` しか素直には出さないので、まづは文字で引く
    // （物理配列の取り違への検出は M8-b へ送る。README に書いてある）。
    override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean {
        val s = session ?: return super.onKeyDown(keyCode, event)

        if (s.isComposing) {
            when (keyCode) {
                KeyEvent.KEYCODE_SPACE -> { op(Op.CONVERT); return true }
                KeyEvent.KEYCODE_ENTER -> { op(Op.COMMIT); return true }
                KeyEvent.KEYCODE_ESCAPE -> { op(Op.CANCEL); return true }
                KeyEvent.KEYCODE_DEL -> { op(Op.BACKSPACE); return true }
                KeyEvent.KEYCODE_DPAD_LEFT ->
                    { op(if (event.isShiftPressed) Op.SHRINK else Op.FOCUS_PREV); return true }
                KeyEvent.KEYCODE_DPAD_RIGHT ->
                    { op(if (event.isShiftPressed) Op.GROW else Op.FOCUS_NEXT); return true }
                KeyEvent.KEYCODE_DPAD_DOWN -> { op(Op.NEXT_CAND); return true }
                KeyEvent.KEYCODE_DPAD_UP -> { op(Op.PREV_CAND); return true }
            }
        }

        // Ctrl / Alt が押されてゐれば手を出さない（短絡キーを奪はない）
        if (event.isCtrlPressed || event.isAltPressed) {
            return super.onKeyDown(keyCode, event)
        }
        val ch = event.unicodeChar.toChar()
        if (ch.code == 0) return super.onKeyDown(keyCode, event)

        val act = s.key(ch)
        if (act == Act.PASSTHROUGH) return super.onKeyDown(keyCode, event)
        apply(act)
        return true
    }

    // ── 核の指示を画面へ移す ──────────────────────────────
    private fun op(o: Int) {
        session?.let { apply(it.op(o)) }
    }

    /// 未確定が無いときの削除は、アプリの一字消しへ返す。
    private fun deleteOrBackspace() {
        val s = session ?: return
        if (s.isComposing) {
            op(Op.BACKSPACE)
        } else {
            currentInputConnection?.deleteSurroundingText(1, 0)
        }
    }

    private fun apply(act: Int) {
        val s = session ?: return
        val ic = currentInputConnection ?: return
        when (act) {
            Act.UPDATE -> ic.setComposingText(s.preedit, 1)
            Act.SWALLOW -> Unit
            Act.PASSTHROUGH -> Unit
            Act.COMMIT -> {
                ic.setComposingText(s.committed, 1)
                ic.finishComposingText()
            }
            // 確定した上で、続けて新しい未確定が立つてゐる
            Act.COMMIT_THEN_UPDATE -> {
                ic.setComposingText(s.committed, 1)
                ic.finishComposingText()
                ic.setComposingText(s.preedit, 1)
            }
            Act.CANCEL -> {
                ic.setComposingText("", 1)
                ic.finishComposingText()
            }
        }
        redraw()
    }

    private fun redraw() {
        val s = session ?: return
        board?.let {
            it.preedit = s.preedit
            it.candidates = s.candidates
            it.chosenCandidate = s.chosen
        }
    }
}
