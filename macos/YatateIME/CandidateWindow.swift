// 候補窓 — 変換中の文節に、選べる表記を並べて見せる小さな板。
//
// ## なぜ IMKCandidates ではないのか
//
// IMKit には `IMKCandidates` があるが、選択の手綱を framework 側が握るので
// 「Space で次へ・←→ で文節を移す」といふ**核が定めた運指**と二重管理になる。
// 矢立の変換の頭脳は核（core/src/henkan.rs）に一つだけ在るべきなので、
// ここは**描くだけの板**にして、何を選んでゐるかは常に核へ訊く。
//
// 見た目は矢立の比喩（紙に墨）に従ふ。明暗は OS の外観に追随する。

import AppKit

/// 変換中の候補を並べる非活性のパネル。
///
/// `NSPanel` を `.nonactivatingPanel` で建てるのが肝で、これを怠ると
/// 候補窓が前面に出た瞬間に入力先のアプリが非活性になり、打鍵が届かなくなる。
final class CandidateWindow {
    private let panel: NSPanel
    private let list: CandidateListView

    init() {
        list = CandidateListView()
        panel = NSPanel(
            contentRect: NSRect(x: 0, y: 0, width: 1, height: 1),
            styleMask: [.borderless, .nonactivatingPanel],
            backing: .buffered,
            defer: false)
        panel.isFloatingPanel = true
        panel.level = .popUpMenu
        panel.hidesOnDeactivate = false
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = true
        panel.contentView = list
        panel.ignoresMouseEvents = true  // 選ぶのは鍵盤。板は見せるだけ
    }

    /// 候補を出す。`cursor` は入力欄のカーソルの画面上の矩形。
    func show(candidates: [String], chosen: Int, at cursor: NSRect) {
        guard !candidates.isEmpty else { return hide() }
        list.update(candidates: candidates, chosen: chosen)

        let size = list.fittingSize
        // カーソルの下に置く。画面の下端に近ければ上へ返す（切れて読めなくならぬやう）。
        var origin = NSPoint(x: cursor.minX, y: cursor.minY - size.height - 4)
        if let screen = NSScreen.screens.first(where: { $0.frame.intersects(cursor) })
            ?? NSScreen.main
        {
            let visible = screen.visibleFrame
            if origin.y < visible.minY {
                origin.y = cursor.maxY + 4
            }
            origin.x = min(max(origin.x, visible.minX), visible.maxX - size.width)
        }
        panel.setFrame(NSRect(origin: origin, size: size), display: true)
        panel.orderFrontRegardless()
    }

    func hide() {
        panel.orderOut(nil)
    }
}

/// 候補を縦に並べて描くだけの view。
private final class CandidateListView: NSView {
    private var candidates: [String] = []
    private var chosen = 0

    private let font = NSFont.systemFont(ofSize: 15)
    private let padding: CGFloat = 8
    private let rowHeight: CGFloat = 22

    override var isFlipped: Bool { true }

    func update(candidates: [String], chosen: Int) {
        self.candidates = candidates
        self.chosen = chosen
        invalidateIntrinsicContentSize()
        needsDisplay = true
    }

    override var fittingSize: NSSize {
        let width =
            candidates
            .map { ($0 as NSString).size(withAttributes: [.font: font]).width }
            .max() ?? 0
        return NSSize(
            width: max(80, width + padding * 2),
            height: CGFloat(candidates.count) * rowHeight + padding * 2)
    }

    override func draw(_ dirtyRect: NSRect) {
        // 紙
        NSColor.windowBackgroundColor.setFill()
        let paper = NSBezierPath(roundedRect: bounds, xRadius: 6, yRadius: 6)
        paper.fill()
        NSColor.separatorColor.setStroke()
        paper.stroke()

        for (i, text) in candidates.enumerated() {
            let row = NSRect(
                x: padding, y: padding + CGFloat(i) * rowHeight,
                width: bounds.width - padding * 2, height: rowHeight)
            if i == chosen {
                NSColor.selectedContentBackgroundColor.setFill()
                NSBezierPath(roundedRect: row.insetBy(dx: -4, dy: 0), xRadius: 4, yRadius: 4)
                    .fill()
            }
            let color: NSColor = i == chosen ? .alternateSelectedControlTextColor : .labelColor
            (text as NSString).draw(
                in: row.offsetBy(dx: 0, dy: 2),
                withAttributes: [.font: font, .foregroundColor: color])
        }
    }
}
