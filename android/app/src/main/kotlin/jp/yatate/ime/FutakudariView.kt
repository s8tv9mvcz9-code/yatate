package jp.yatate.ime

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.RectF
import android.view.MotionEvent
import android.view.View
import kotlin.math.abs
import kotlin.math.roundToInt

// 二行配列（ふたくだり）— 硝子の鍵盤。iOS の FutakudariView の写しである。
//
// 行キーは 10 個だけ置き、**段は書き下ろしのスライドで選ぶ**。
// 縦に滑らせて段（あ〜お）、横に逸らして濁点・半濁点。
// 指を離した位置の仮名が入る。
//
//   第二の行（画面左）      第一の行（画面右）
//     は ま や ら わ          あ か さ た な
//
// **この View は配列表を持たない。** 描くのは核（Core.layout()）が返した表だけで、
// 「図と実際の打鍵がずれる」事故は起きやうが無い。
//
// 原器（物理鍵盤の縦組五十音配列）とは別物である。硝子には硝子の作法があり、
// 二行配列は原器を硝子へ折り畳んだものにあたる（docs/ime/layout.md）。

class FutakudariView(context: Context) : View(context) {

    /// 仮名が決まつた（殻が核へ渡す）。
    var onKana: ((String) -> Unit)? = null
    /// 機能キー。
    var onCommit: (() -> Unit)? = null
    var onConvert: (() -> Unit)? = null
    var onDelete: (() -> Unit)? = null
    var onNextCandidate: (() -> Unit)? = null
    var onChooseCandidate: ((Int) -> Unit)? = null

    /// 殻が入れ替へる表示（核から来たものをそのまま持つ）。
    var preedit: String = ""
        set(v) { field = v; invalidate() }
    var candidates: List<String> = emptyList()
        set(v) { field = v; invalidate() }
    var chosenCandidate: Int = 0
        set(v) { field = v; invalidate() }

    private val layoutTable: Layout = Core.layout()

    // ── 触れてゐる最中の状態 ──────────────────────────────
    private var pressed: Gyo? = null
    private var pressedRect: RectF? = null
    private var startX = 0f
    private var startY = 0f
    private var curDan = 0
    private var curDeflect = Deflect.NONE

    // ── 寸法 ──────────────────────────────────────────────
    private var barH = 0f          // 作業帯（未確定＋候補）
    private var keyH = 0f
    private var colW = 0f
    private var funcH = 0f

    private val paperPaint = Paint().apply { isAntiAlias = true }
    private val keyPaint = Paint().apply { isAntiAlias = true }
    private val textPaint = Paint().apply {
        isAntiAlias = true
        textAlign = Paint.Align.CENTER
    }
    private val fringePaint = Paint().apply {
        isAntiAlias = true
        textAlign = Paint.Align.CENTER
    }

    private val paper = Color.rgb(0xF7, 0xF4, 0xEC)
    private val sumi = Color.rgb(0x25, 0x22, 0x1E)
    private val keyFace = Color.rgb(0xFF, 0xFD, 0xF8)
    private val keyEdge = Color.rgb(0xD8, 0xD1, 0xC2)
    private val accent = Color.rgb(0x8A, 0x6B, 0x3D)

    override fun onSizeChanged(w: Int, h: Int, ow: Int, oh: Int) {
        super.onSizeChanged(w, h, ow, oh)
        barH = h * 0.16f
        funcH = h * 0.14f
        keyH = (h - barH - funcH) / 5f
        colW = w / 2f
        textPaint.textSize = keyH * 0.46f
        fringePaint.textSize = barH * 0.30f
    }

    // ── 描く ──────────────────────────────────────────────
    override fun onDraw(canvas: Canvas) {
        paperPaint.color = paper
        canvas.drawRect(0f, 0f, width.toFloat(), height.toFloat(), paperPaint)

        drawBar(canvas)

        // 行キー — 左が第二の行、右が第一の行（縦書きの紙面に合はせる）
        for (row in 0 until 5) {
            layoutTable.second.getOrNull(row)?.let { drawGyo(canvas, it, 0, row) }
            layoutTable.first.getOrNull(row)?.let { drawGyo(canvas, it, 1, row) }
        }

        drawFunctionRow(canvas)
        pressed?.let { drawLadder(canvas, it) }
    }

    private fun drawBar(canvas: Canvas) {
        textPaint.color = sumi
        textPaint.textAlign = Paint.Align.LEFT
        val ts = barH * 0.42f
        textPaint.textSize = ts
        canvas.drawText(preedit.ifEmpty { "　" }, 16f, barH * 0.44f, textPaint)

        // 候補（先頭のいくつか）。触れると選べる。
        if (candidates.isNotEmpty()) {
            fringePaint.textAlign = Paint.Align.LEFT
            var x = 16f
            candidates.take(4).forEachIndexed { i, c ->
                fringePaint.color = if (i == chosenCandidate) accent else Color.rgb(0x6A, 0x63, 0x58)
                canvas.drawText(c, x, barH * 0.85f, fringePaint)
                x += fringePaint.measureText(c) + 28f
            }
        }
        textPaint.textAlign = Paint.Align.CENTER
        textPaint.textSize = keyH * 0.46f

        keyPaint.color = keyEdge
        canvas.drawRect(0f, barH - 1f, width.toFloat(), barH, keyPaint)
    }

    private fun gyoRect(col: Int, row: Int): RectF {
        val pad = 4f
        val l = col * colW + pad
        val t = barH + row * keyH + pad
        return RectF(l, t, l + colW - pad * 2, t + keyH - pad * 2)
    }

    private fun drawGyo(canvas: Canvas, gyo: Gyo, col: Int, row: Int) {
        val r = gyoRect(col, row)
        keyPaint.color = if (pressed?.name == gyo.name) Color.rgb(0xEC, 0xE3, 0xD0) else keyFace
        canvas.drawRoundRect(r, 10f, 10f, keyPaint)
        keyPaint.color = keyEdge
        canvas.drawRoundRect(r, 10f, 10f, Paint(keyPaint).apply {
            style = Paint.Style.STROKE
            strokeWidth = 1.5f
        })
        textPaint.color = sumi
        canvas.drawText(gyo.name, r.centerX(), r.centerY() + textPaint.textSize * 0.35f, textPaint)
    }

    /// 押してゐる行の梯子（5 段）を出す。**指が今どの段に居るかを見せる。**
    private fun drawLadder(canvas: Canvas, gyo: Gyo) {
        val r = pressedRect ?: return
        val w = colW * 0.9f
        val left = (r.centerX() - w / 2).coerceIn(0f, width - w)
        val stepH = keyH * 0.9f
        val top = (r.centerY() - stepH * 2.5f).coerceIn(barH, height - stepH * 5)

        for (dan in 0 until 5) {
            val box = RectF(left, top + dan * stepH, left + w, top + (dan + 1) * stepH)
            keyPaint.color = if (dan == curDan) accent else Color.rgb(0xFA, 0xF7, 0xEF)
            canvas.drawRoundRect(box, 8f, 8f, keyPaint)
            textPaint.color = if (dan == curDan) paper else sumi
            val kana = gyo.kana(dan, curDeflect) ?: "・"
            canvas.drawText(kana, box.centerX(), box.centerY() + textPaint.textSize * 0.35f, textPaint)
        }
        // 逸らしの目印
        fringePaint.color = sumi
        fringePaint.textAlign = Paint.Align.CENTER
        val mark = when (curDeflect) {
            Deflect.NONE -> "清"
            Deflect.DAKU -> "濁"
            Deflect.KO -> "半"
        }
        canvas.drawText(mark, left + w / 2, top - 8f, fringePaint)
    }

    private data class FuncKey(val label: String, val rect: RectF, val action: () -> Unit)

    private fun functionKeys(): List<FuncKey> {
        val top = height - funcH
        val w = width / 4f
        fun r(i: Int) = RectF(i * w + 4f, top + 4f, (i + 1) * w - 4f, height - 4f)
        return listOf(
            FuncKey("変換", r(0)) { onConvert?.invoke() },
            FuncKey("次候補", r(1)) { onNextCandidate?.invoke() },
            FuncKey("削除", r(2)) { onDelete?.invoke() },
            FuncKey("確定", r(3)) { onCommit?.invoke() },
        )
    }

    private fun drawFunctionRow(canvas: Canvas) {
        for (k in functionKeys()) {
            keyPaint.color = keyFace
            canvas.drawRoundRect(k.rect, 10f, 10f, keyPaint)
            keyPaint.color = keyEdge
            canvas.drawRoundRect(k.rect, 10f, 10f, Paint(keyPaint).apply {
                style = Paint.Style.STROKE
                strokeWidth = 1.5f
            })
            fringePaint.color = sumi
            fringePaint.textAlign = Paint.Align.CENTER
            canvas.drawText(
                k.label, k.rect.centerX(),
                k.rect.centerY() + fringePaint.textSize * 0.35f, fringePaint
            )
        }
    }

    // ── 触る ──────────────────────────────────────────────
    @SuppressLint("ClickableViewAccessibility")
    override fun onTouchEvent(event: MotionEvent): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                val x = event.x
                val y = event.y

                if (y >= height - funcH) {
                    functionKeys().firstOrNull { it.rect.contains(x, y) }?.action?.invoke()
                    return true
                }
                if (y < barH) {
                    tapCandidate(x, y)
                    return true
                }
                val col = if (x < colW) 0 else 1
                val row = ((y - barH) / keyH).toInt().coerceIn(0, 4)
                val line = if (col == 0) layoutTable.second else layoutTable.first
                pressed = line.getOrNull(row) ?: return true
                pressedRect = gyoRect(col, row)
                startX = x
                startY = y
                curDan = 0
                curDeflect = Deflect.NONE
                invalidate()
                return true
            }

            MotionEvent.ACTION_MOVE -> {
                if (pressed == null) return true
                // 縦の滑りが段。**下へ書き下ろすほど段が進む**（あ→い→う→え→お）。
                val dy = event.y - startY
                curDan = (dy / (keyH * 0.9f)).roundToInt().coerceIn(0, 4)
                // 横の逸らしが濁点（右）・半濁点（左）。閾値は指の幅ぶん。
                val dx = event.x - startX
                val gate = colW * 0.28f
                curDeflect = when {
                    dx > gate -> Deflect.DAKU
                    dx < -gate -> Deflect.KO
                    else -> Deflect.NONE
                }
                // 逸らした面に字が無ければ清音へ戻す（無い字を見せない）
                if (pressed?.kana(curDan, curDeflect) == null && abs(dx) > gate) {
                    curDeflect = Deflect.NONE
                }
                invalidate()
                return true
            }

            MotionEvent.ACTION_UP -> {
                val gyo = pressed
                pressed = null
                pressedRect = null
                invalidate()
                val kana = gyo?.kana(curDan, curDeflect) ?: return true
                onKana?.invoke(kana)
                return true
            }

            MotionEvent.ACTION_CANCEL -> {
                pressed = null
                pressedRect = null
                invalidate()
                return true
            }
        }
        return super.onTouchEvent(event)
    }

    private fun tapCandidate(x: Float, y: Float) {
        if (candidates.isEmpty() || y < barH * 0.55f) return
        var cur = 16f
        candidates.take(4).forEachIndexed { i, c ->
            val w = fringePaint.measureText(c)
            if (x >= cur && x <= cur + w) {
                onChooseCandidate?.invoke(i)
                return
            }
            cur += w + 28f
        }
    }
}
