// 文机（ふづくえ）— 紙面ステートマシンの草稿（docs/ime/fuzukue.md §5）。
// 画面全体を五十音の紙面とし、マスが状態・指の運びが遷移・持ち上げが切れ。
// 触れたまま次のマスへ進めば連綿（一筆で複数字）。マスには墨の気配が滲み、
// 最有力の次のマスへ筆脈が走る。macOS 版（IMKit）の先行像を iOS で試す。
// 未実装（設計のみ）: 濁点の鉤・行掠め・A2 軌跡復号（fuzukue.md 参照）。

import SwiftUI
import YatateCore

struct FuzukueView: View {
    @State private var composed = ""
    @State private var field: ActionField = Kehai.field(after: nil)
    @State private var stroke: [CGPoint] = []
    @State private var currentCell: Cell? = nil
    @State private var lastCell: Cell? = nil     // 筆脈の起点（持ち上げ後も残る）
    @State private var pending: Cell? = nil      // 滞留を計つてゐる最中のマス
    @State private var pendingSince: Date? = nil // そのマスに入つた時刻
    @State private var strokeBegan = false       // 一筆が始まつてゐるか
    @Environment(\.colorScheme) private var scheme

    /// 滞留の閾値。**回数でなく時間**で測る。
    /// 当初はサンプル回数（同じマスを 2 回見たら確定）で代用してゐたが、これは指を
    /// 前提にした代理指標で、ポインタでは逆転する——押下は 1 回しかイベントが出ないので
    /// 始点が落ち、素早く通過したマスは 2 回出て書かれてしまふ（macOS 版で実測）。
    /// 時間なら指でもポインタでも同じ意味になる。
    private let dwell: TimeInterval = 0.09

    // 紙面の列: 右から左へ 十行 ＋ 左端の機能縁
    private static let gyos: [Gyo] = Gojuon.firstLine + Gojuon.secondLine
    private static let fringe = ["ん", "、", "。", "　", "⌫"]

    struct Cell: Equatable {
        let col: Int      // 0 = 最右（あ行）… 9 = わ行, 10 = 機能縁
        let row: Int      // 0...4
    }

    var body: some View {
        VStack(spacing: 6) {
            header
            GeometryReader { geo in
                paper(size: geo.size)
            }
        }
        .padding(10)
        .background(Sumi.paper(scheme))
    }

    private var header: some View {
        HStack {
            Text("文机 ─ 連綿の草稿")
                .font(.headline).foregroundStyle(.secondary)
            Spacer()
            Button("清める") {
                composed = ""
                stroke = []
                currentCell = nil
                lastCell = nil
                pending = nil
                pendingSince = nil
                strokeBegan = false
                field = Kehai.field(after: nil)
            }
            .font(.subheadline)
        }
    }

    // ── 紙面の幾何（描画とヒットテストで同じ算術を使ふ）─────────
    private func cellRect(_ cell: Cell, in size: CGSize) -> CGRect {
        let textW: CGFloat = 40                       // 右端: 書かれた文（縦書き）
        let gridW = size.width - textW - 8
        let colW = gridW / 11                          // 十行＋機能縁
        let rowH = size.height / 5
        // col 0 = あ行 = 紙面の最右（縦書きの読み順）。機能縁(10)は最左。
        let x = cell.col == 10 ? 0 : gridW - colW * CGFloat(cell.col + 1)
        return CGRect(x: x, y: rowH * CGFloat(cell.row), width: colW, height: rowH)
            .insetBy(dx: 2, dy: 2)
    }

    private func cellAt(_ p: CGPoint, in size: CGSize) -> Cell? {
        for col in 0...10 {
            for row in 0..<5 where cellRect(Cell(col: col, row: row), in: size).contains(p) {
                return Cell(col: col, row: row)
            }
        }
        return nil
    }

    private func kana(of cell: Cell) -> String? {
        if cell.col == 10 { return Self.fringe[cell.row] }
        return Self.gyos[cell.col].kana(dan: cell.row)
    }

    private func ink(of cell: Cell) -> Double {
        if cell.col == 10 {
            guard cell.row < 3 else { return 0 }
            return field.ink[.moji(Self.fringe[cell.row])] ?? 0
        }
        let gyo = Self.gyos[cell.col]
        let gyoInk = field.ink[.gyo(gyo.name)] ?? 0
        let danInk = field.dan[gyo.name]?[cell.row] ?? 0
        return gyoInk * danInk
    }

    // 筆脈の先: 最有力の鍵を、紙面のマスへ落とす
    private var peakCell: Cell? {
        switch field.peak {
        case .gyo(let name):
            guard let col = Self.gyos.firstIndex(where: { $0.name == name }) else { return nil }
            let row = field.dan[name]?.enumerated().max { $0.element < $1.element }?.offset ?? 0
            return Cell(col: col, row: row)
        case .moji(let m):
            guard let row = Self.fringe.firstIndex(of: m) else { return nil }
            return Cell(col: 10, row: row)
        case nil:
            return nil
        }
    }

    // ── 紙面 ─────────────────────────────────────────
    private func paper(size: CGSize) -> some View {
        ZStack(alignment: .topTrailing) {
            Canvas { ctx, sz in
                draw(in: &ctx, size: sz)
            }
            .gesture(
                DragGesture(minimumDistance: 0)
                    .onChanged { v in
                        let first = !strokeBegan
                        if first {
                            strokeBegan = true
                            stroke = [v.location]      // 前の一筆と繋げない
                        } else {
                            stroke.append(v.location)
                            if stroke.count > 240 { stroke.removeFirst() }
                        }
                        advance(to: cellAt(v.location, in: size), at: v.time, first: first)
                    }
                    .onEnded { _ in                    // 持ち上げ＝切れ
                        currentCell = nil
                        pending = nil
                        pendingSince = nil
                        strokeBegan = false
                    }
            )
            verticalText
        }
    }

    /// 進入確定の状態機械（fuzukue.md §2）。
    ///
    /// ・**一筆の最初の一点は即座に書く** — そこに置いた指／ポインタに迷ひは無い。
    /// ・**以後のマスは滞留してはじめて書く** — 通り過ぎただけのマスは拾はない。
    private func advance(to cell: Cell?, at time: Date, first: Bool) {
        guard let cell, cell != currentCell else { return }
        if first {
            commit(cell)
            return
        }
        guard cell == pending else {                  // 別のマスに入つた: 計測を始める
            pending = cell
            pendingSince = time
            return
        }
        guard let since = pendingSince,
              time.timeIntervalSince(since) >= dwell else { return }
        commit(cell)
    }

    private func commit(_ cell: Cell) {
        pending = nil
        pendingSince = nil
        currentCell = cell
        lastCell = cell
        guard let k = kana(of: cell) else { return }
        if k == "⌫" {
            if !composed.isEmpty { composed.removeLast() }
        } else {
            composed.append(k)
        }
        field = Kehai.field(after: composed.last)
    }

    private func draw(in ctx: inout GraphicsContext, size: CGSize) {
        for col in 0...10 {
            for row in 0..<5 {
                let cell = Cell(col: col, row: row)
                guard let k = kana(of: cell) else {
                    // 空きスロット（や行・わ行）は灰点で残す（位置の筋肉記憶）
                    let r = cellRect(cell, in: size)
                    ctx.draw(Text("・").foregroundColor(.gray.opacity(0.4)),
                             at: CGPoint(x: r.midX, y: r.midY))
                    continue
                }
                let r = cellRect(cell, in: size)
                let base = cell.col == 10 ? Sumi.fringe(scheme) : Sumi.key(scheme)
                ctx.fill(Path(roundedRect: r, cornerRadius: 6), with: .color(base))
                let inkLevel = ink(of: cell)
                if inkLevel > 0.02 {                   // 墨の気配（マス単位）
                    ctx.fill(Path(roundedRect: r, cornerRadius: 6),
                             with: .color(Sumi.ink(scheme, inkLevel)))
                }
                if cell == currentCell {
                    ctx.stroke(Path(roundedRect: r, cornerRadius: 6),
                               with: .color(.accentColor), lineWidth: 2)
                }
                ctx.draw(Text(k == "　" ? "空" : k)
                    .font(.system(size: min(22, cellRect(cell, in: size).width * 0.5)))
                    .foregroundColor(.primary),
                    at: CGPoint(x: r.midX, y: r.midY))
            }
        }
        // 筆脈 — 最後に書いたマスから最有力の次のマスへ（持ち上げ後も気脈は残る）
        if let from = lastCell, let to = peakCell, from != to {
            let a = cellRect(from, in: size), b = cellRect(to, in: size)
            var p = Path()
            p.move(to: CGPoint(x: a.midX, y: a.midY))
            let c = CGPoint(x: (a.midX + b.midX) / 2 + (a.midY - b.midY) * 0.1,
                            y: (a.midY + b.midY) / 2 + (b.midX - a.midX) * 0.1)
            p.addQuadCurve(to: CGPoint(x: b.midX, y: b.midY), control: c)
            ctx.stroke(p, with: .color(Sumi.hitsumyaku(scheme)),
                       style: StrokeStyle(lineWidth: 2, lineCap: .round))
        }
        // 連綿の軌跡（薄墨）
        if stroke.count > 1 {
            var p = Path()
            p.move(to: stroke[0])
            for pt in stroke.dropFirst() { p.addLine(to: pt) }
            ctx.stroke(p, with: .color(Sumi.trace(scheme)),
                       style: StrokeStyle(lineWidth: 5, lineCap: .round, lineJoin: .round))
        }
    }

    // 書かれた文 — 紙面の右端に縦書きで立つ（確定表示は旧字化）
    private var verticalText: some View {
        VStack(spacing: 0) {
            ForEach(Array(toKyuji(composed).suffix(14).enumerated()), id: \.offset) { _, ch in
                Text(String(ch)).font(.system(size: 22))
            }
            Spacer(minLength: 0)
        }
        .frame(width: 36)
        .padding(.vertical, 4)
        .background(Sumi.slip(scheme), in: RoundedRectangle(cornerRadius: 6))
        .allowsHitTesting(false)
    }
}

#Preview {
    FuzukueView()
}
