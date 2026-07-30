// 二行（ふたくだり）配列 — docs/ime/layout.md §2 の M0 実装（楷のみ）＋ VLA 層 A0。
// 右列＝第一の行（あかさたな）、左列＝第二の行（はまやらわ）。
// 段は「書き下ろし」: 押へて下へ滑らせると梯子が降り、右肩への逸らしが濁点になる。
// 鍵には「墨の気配」（次の一打の分布）が滲み、最有力の一手へ「筆脈」が走る
// （docs/ime/vla.md — コーパス bigram の決定的射影。訓練なし・端末内）。

import SwiftUI
import YatateCore

final class KeyboardState: ObservableObject {
    @Published var composer = Composer()
    @Published var needsGlobe = false
    @Published var field: ActionField = Kehai.field(after: nil)
    @Published var lastKey: KeyID? = nil
    var onInsert: (String) -> Void = { _ in }
    var onDeleteBackward: () -> Void = {}
    var onGlobe: () -> Void = {}

    func tap(_ kana: String, from key: KeyID) {
        composer.append(kana)
        lastKey = key
        refresh()
    }

    func delete() {
        if composer.isEmpty { onDeleteBackward() } else { composer.deleteLast() }
        refresh()
    }

    func commit() {
        guard !composer.isEmpty else { return }
        onInsert(composer.commit())
        lastKey = nil
        refresh()
    }

    private func refresh() {
        field = Kehai.field(after: composer.text.last)
    }
}

// 鍵の位置を集める（筆脈の描画用）
private struct KeyFramePreference: PreferenceKey {
    static var defaultValue: [KeyID: CGRect] = [:]
    static func reduce(value: inout [KeyID: CGRect], nextValue: () -> [KeyID: CGRect]) {
        value.merge(nextValue()) { $1 }
    }
}

private extension View {
    func trackFrame(_ id: KeyID) -> some View {
        background(GeometryReader { g in
            Color.clear.preference(key: KeyFramePreference.self,
                                   value: [id: g.frame(in: .named("yatate"))])
        })
    }
}

struct FutakudariView: View {
    @ObservedObject var state: KeyboardState
    @State private var frames: [KeyID: CGRect] = [:]
    @Environment(\.colorScheme) private var scheme

    var body: some View {
        VStack(spacing: 4) {
            workingStrip
            ZStack {
                HStack(spacing: 5) {
                    functionColumn
                    gyoColumn(Gojuon.secondLine)   // 第二の行（は ま や ら わ）
                    gyoColumn(Gojuon.firstLine)    // 第一の行（あ か さ た な）— 画面右端
                }
                hitsumyaku                          // 筆脈 — 最有力の一手への淡い一画
            }
            .coordinateSpace(name: "yatate")
            .onPreferenceChange(KeyFramePreference.self) { frames = $0 }
        }
        .padding(6)
        .background(Sumi.paper(scheme).opacity(0.6))
    }

    // 作業帯 — 確定前の仮名。確定で旧字体が機械確定される（T0）。
    private var workingStrip: some View {
        HStack {
            Text(state.composer.isEmpty ? "矢立 ─ 楷" : state.composer.text)
                .font(.system(size: 18))
                .foregroundStyle(state.composer.isEmpty ? .secondary : .primary)
                .lineLimit(1)
                .frame(maxWidth: .infinity, alignment: .leading)
            Button(action: state.commit) {
                Text("確定")
                    .font(.system(size: 15, weight: .semibold))
                    .padding(.horizontal, 14).padding(.vertical, 7)
                    .background(Color.accentColor.opacity(0.15))
                    .clipShape(RoundedRectangle(cornerRadius: 7))
            }
        }
        .padding(.horizontal, 8)
        .frame(height: 40)
        .background(Sumi.slip(scheme), in: RoundedRectangle(cornerRadius: 8))
    }

    private func gyoColumn(_ line: [Gyo]) -> some View {
        VStack(spacing: 5) {
            ForEach(line, id: \.name) { gyo in
                KanaKeyView(
                    gyo: gyo,
                    ink: state.field.ink[.gyo(gyo.name)] ?? 0,
                    danInk: state.field.dan[gyo.name] ?? [0, 0, 0, 0, 0],
                    scheme: scheme
                ) { state.tap($0, from: .gyo(gyo.name)) }
                .trackFrame(.gyo(gyo.name))
            }
        }
    }

    private var functionColumn: some View {
        VStack(spacing: 5) {
            functionKey("⌫", id: nil) { state.delete() }
            functionKey("ん", id: .moji("ん")) { state.tap("ん", from: .moji("ん")) }
            functionKey("、", id: .moji("、")) { state.tap("、", from: .moji("、")) }
            functionKey("。", id: .moji("。")) { state.tap("。", from: .moji("。")) }
            if state.needsGlobe {
                functionKey("🌐", id: nil) { state.onGlobe() }
            } else {
                functionKey("空白", id: nil) { state.tap("　", from: .moji("　")) }
            }
        }
        .frame(width: 64)
    }

    private func functionKey(_ label: String, id: KeyID?,
                             action: @escaping () -> Void) -> some View {
        let ink = id.flatMap { state.field.ink[$0] } ?? 0
        return Button(action: action) {
            Text(label)
                .font(.system(size: 16))
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(Sumi.fringe(scheme), in: RoundedRectangle(cornerRadius: 8))
                .overlay(RoundedRectangle(cornerRadius: 8)
                    .fill(Sumi.ink(scheme, ink)))             // 墨の気配（暗所は白抜き）
                .foregroundStyle(.primary)
        }
        .modifier(TrackIfPresent(id: id))
    }

    // 筆脈 — 最後に触れた鍵から、最有力の次の鍵への気脈（vla.md §2）
    @ViewBuilder private var hitsumyaku: some View {
        if let from = state.lastKey, let to = state.field.peak, from != to,
           let f = frames[from], let t = frames[to] {
            Path { p in
                let a = CGPoint(x: f.midX, y: f.midY)
                let b = CGPoint(x: t.midX, y: t.midY)
                // わづかに撓ませる（直線は機械の線、筆脈は生きた線）
                let c = CGPoint(x: (a.x + b.x) / 2 + (a.y - b.y) * 0.12,
                                y: (a.y + b.y) / 2 + (b.x - a.x) * 0.12)
                p.move(to: a)
                p.addQuadCurve(to: b, control: c)
            }
            .stroke(Sumi.hitsumyaku(scheme),
                    style: StrokeStyle(lineWidth: 2, lineCap: .round))
            .allowsHitTesting(false)
        }
    }
}

private struct TrackIfPresent: ViewModifier {
    let id: KeyID?
    func body(content: Content) -> some View {
        if let id { content.trackFrame(id) } else { content }
    }
}

// 行キー — 押下で梯子（五段の縦列）が現れ、下へ滑つて段を選ぶ。
// 右への逸らし＝濁点、左＝半濁・小書き。タップはあ段（楷の既定）。
// 鍵の墨（ink）は次の一打の気配、梯子の段にも段レベルの気配が差す。
struct KanaKeyView: View {
    let gyo: Gyo
    let ink: Double
    let danInk: [Double]
    let scheme: ColorScheme
    let onCommit: (String) -> Void

    @State private var translation: CGSize? = nil

    private let stepH: CGFloat = 34          // 梯子一段の高さ
    private let deflectW: CGFloat = 26       // 逸らしの閾値

    private var currentDan: Int {
        guard let t = translation else { return 0 }
        return min(4, max(0, Int(t.height / stepH)))
    }

    private var currentDeflect: Deflect {
        guard let t = translation else { return .none }
        if t.width > deflectW { return .daku }
        if t.width < -deflectW { return .ko }
        return .none
    }

    var body: some View {
        Text(gyo.name)
            .font(.system(size: 22))
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(Sumi.key(scheme), in: RoundedRectangle(cornerRadius: 8))
            .overlay(RoundedRectangle(cornerRadius: 8)
                .fill(Sumi.ink(scheme, ink)))                 // 墨の気配（暗所は白抜き）
            .overlay(alignment: .topLeading) { ladder }
            .contentShape(Rectangle())
            .gesture(
                DragGesture(minimumDistance: 0)
                    .onChanged { translation = $0.translation }
                    .onEnded { _ in
                        if let kana = gyo.kana(dan: currentDan, deflect: currentDeflect) {
                            onCommit(kana)
                        }
                        translation = nil
                    }
            )
    }

    // 梯子 — その行の五段（逸らし面に追随）。空きスロットは灰点で残す。
    // 段の下地にも気配が差す（か行を押えた瞬間、け の段が既に濃い — vla.md §2）。
    @ViewBuilder private var ladder: some View {
        if translation != nil {
            VStack(spacing: 2) {
                ForEach(0..<5, id: \.self) { dan in
                    Text(gyo.kana(dan: dan, deflect: currentDeflect) ?? "・")
                        .font(.system(size: 19))
                        .frame(width: 36, height: 30)
                        .background(
                            dan == currentDan ? Color.accentColor.opacity(0.30)
                                              : Sumi.ladderStep(scheme, ink: danInk[dan]),
                            in: RoundedRectangle(cornerRadius: 5))
                }
            }
            .padding(3)
            .background(.thinMaterial, in: RoundedRectangle(cornerRadius: 7))
            .shadow(radius: 3)
            .offset(x: -46, y: -4)
            .allowsHitTesting(false)
            .zIndex(10)
        }
    }
}
