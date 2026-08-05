// 原器の稽古場（ハード鍵盤）— iPad / Mac に外付け鍵盤を繋いだときの入口。
//
// ## なぜ本体アプリに置くのか（拡張ではなく）
//
// **iOS のキーボード拡張はハードキーを受け取れない。** これは矢立が繰り返し
// 突き当たつてきた前提で（docs/ime/cross-platform.md §4）、だから原器がそのまま
// 活きる場は macOS と Windows しかない、と設計に書いてある。
//
// ただし**本体アプリ**は `pressesBegan(_:with:)` でハードキーを受け取れる。
// システム全体の IME にはならないが、原器を iPad の外付け鍵盤で打つ稽古場としては
// 成立する——そして何より、**核（Henkan・Kagi）が iOS でも同じ答へを出すこと**を
// 実機で確かめる場になる。運指は macOS 殻・Windows 殻と一字一句同じである。
//
// ## 鍵は HID usage で引く
//
// `UIKey.characters`（出る字）ではなく `UIKey.keyCode`（`UIKeyboardHIDUsage`
// ＝ **物理位置**）で引く。英字配列でも原器の 33 鍵の位置はすべて在り、
// 出る字で引くと「し」が黙つて「さ」になる（US の `;` と `:` は刻印も位置も同じで
// 意味だけが入れ替はる）。macOS 殻が `NSEvent.characters` を使はないのと同じ一手。

import SwiftUI
import UIKit
import YatateCore

// MARK: - ハードキーを拾ふ UIView

/// `pressesBegan` を拾ふためだけの UIView。**ここに頭脳は無い。**
final class HardKeyPickupView: UIView {
    /// 打鍵をそのまま上へ渡す。判断は核がする。
    var onKey: ((UIKey) -> Bool)?

    override var canBecomeFirstResponder: Bool { true }

    override func pressesBegan(_ presses: Set<UIPress>, with event: UIPressesEvent?) {
        var unhandled = Set<UIPress>()
        for press in presses {
            guard let key = press.key else {
                unhandled.insert(press)
                continue
            }
            // 修飾キーが押されてゐれば手を出さない（短絡キーを奪はない）。
            // **Shift だけは通す**——原器の前置シフトは `^` の逐次打鍵なので
            // 同時押しの Shift は空いてをり、区切り修正に使へる。
            let mods = key.modifierFlags.intersection([.command, .control, .alternate])
            if !mods.isEmpty || onKey?(key) != true {
                unhandled.insert(press)
            }
        }
        if !unhandled.isEmpty {
            super.pressesBegan(unhandled, with: event)
        }
    }
}

struct HardKeyPickup: UIViewRepresentable {
    let onKey: (UIKey) -> Bool

    func makeUIView(context: Context) -> HardKeyPickupView {
        let v = HardKeyPickupView()
        v.onKey = onKey
        DispatchQueue.main.async { v.becomeFirstResponder() }
        return v
    }

    func updateUIView(_ v: HardKeyPickupView, context: Context) {
        v.onKey = onKey
    }
}

// MARK: - 稽古場

/// 打鍵の状態を持つ器。**核の Henkan をそのまま駆動する**（写しを持たない）。
@MainActor
final class GenkiKeyboardModel: ObservableObject {
    @Published private(set) var preedit = ""
    @Published private(set) var committed = ""
    @Published private(set) var shifted = false
    @Published private(set) var converting = false
    @Published private(set) var candidates: [String] = []
    @Published private(set) var chosen = 0
    /// 直前に受けた物理位置（実機で「どの鍵が来たか」を目で確かめるため）。
    @Published private(set) var lastUsage: UInt16?
    @Published private(set) var lastGenki: Character?

    private let henkan = Henkan()

    /// 返り値は「食つたか」。食はなかつた鍵は OS へ返す。
    func handle(_ key: UIKey) -> Bool {
        let usage = key.keyCode
        lastUsage = UInt16(usage.rawValue)
        // **位置と、その位置が指す原器の字は、常に同時に更新する。**
        // 機能キーで早く返る道で片方だけ置いていくと、画面が
        //「HID 0x28（Enter）→ 原器 '4'」といふ嘘を平然と出す——
        // ここは「どの鍵が来たか」を目で確かめるための欄なので、嘘は致命である。
        lastGenki = genki(hid: UInt16(usage.rawValue))
        let shift = key.modifierFlags.contains(.shift)

        // ── 機能キー（未確定を抱へてゐるときだけ意味を持つ）──
        if henkan.isComposing {
            switch usage {
            case .keyboardSpacebar:
                return apply(henkan.convert())
            case .keyboardReturnOrEnter, .keypadEnter:
                return apply(henkan.commit())
            case .keyboardEscape:
                return apply(henkan.cancel())
            case .keyboardDeleteOrBackspace:
                return apply(henkan.backspace())
            case .keyboardLeftArrow:
                return apply(shift ? henkan.shrinkFocus() : henkan.focusPrev())
            case .keyboardRightArrow:
                return apply(shift ? henkan.growFocus() : henkan.focusNext())
            case .keyboardDownArrow:
                return apply(henkan.nextCandidate())
            case .keyboardUpArrow:
                return apply(henkan.prevCandidate())
            default:
                break
            }
        }

        guard let ch = lastGenki else { return false }
        return apply(henkan.key(ch))
    }

    func clear() {
        henkan.reset()
        committed = ""
        lastUsage = nil
        lastGenki = nil
        sync()
    }

    /// 核が返した指示のとほりに描く。**判断はここでしない。**
    private func apply(_ act: Act) -> Bool {
        if case .passthrough = act { return false }
        if let text = act.committed { committed += text }
        sync()
        return true
    }

    private func sync() {
        preedit = henkan.preedit
        shifted = henkan.isShifted
        converting = henkan.phase == .henkan
        candidates = henkan.candidates.map(\.surface)
        chosen = henkan.chosen
    }
}

/// 原器をハード鍵盤で打つ稽古場。
struct GenkiHardKeyboardView: View {
    @StateObject private var model = GenkiKeyboardModel()
    @Environment(\.colorScheme) private var scheme

    var body: some View {
        ZStack {
            Sumi.paper(scheme).ignoresSafeArea()

            // 鍵を拾ふだけの層（見えない）
            HardKeyPickup { model.handle($0) }
                .frame(width: 0, height: 0)

            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    header

                    paperRow("確定", model.committed.isEmpty ? "—" : model.committed)
                    paperRow(
                        model.converting ? "未確定（変換中）" : "未確定",
                        model.preedit.isEmpty ? "—" : model.preedit)

                    if model.converting && model.candidates.count > 1 {
                        candidateStrip
                    }

                    HStack(spacing: 12) {
                        if model.shifted {
                            Label("前置シフト", systemImage: "arrow.up.square")
                                .foregroundStyle(.primary)
                        }
                        if let u = model.lastUsage {
                            Text(lastKeyDescription(u))
                                .font(.system(.footnote, design: .monospaced))
                                .foregroundStyle(.secondary)
                        }
                    }

                    layoutChart

                    Button("消す") { model.clear() }
                        .buttonStyle(.bordered)
                }
                .padding(24)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("原器 — ハード鍵盤")
                .font(.title2.weight(.semibold))
                .foregroundStyle(.primary)
            Text(
                "外付け鍵盤で打つてください。**位置**で引くので、英字配列でも刻印どほりに動きます"
                    + "（US の ' ; = が : ; ^ に当たる）。"
            )
            .font(.footnote)
            .foregroundStyle(.secondary)
            Text(
                "空白＝変換／次の候補・Enter＝確定・Esc＝戻す・←→＝文節・"
                    + "Shift+←→＝区切り・↑↓＝候補。**macOS と Windows の殻と同じ運指**です。"
            )
            .font(.footnote)
            .foregroundStyle(.secondary)
        }
    }

    /// 候補の並び。**核が並べた順**をそのまま描く（費用の昇順）。
    private var candidateStrip: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("候補（\(model.chosen + 1)/\(model.candidates.count)）")
                .font(.caption)
                .foregroundStyle(.secondary)
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 8) {
                    ForEach(Array(model.candidates.enumerated()), id: \.offset) { i, surface in
                        Text(surface)
                            .font(.system(size: 20))
                            .padding(.horizontal, 12)
                            .padding(.vertical, 6)
                            .background(
                                i == model.chosen
                                    ? Color.accentColor.opacity(0.25) : Sumi.slip(scheme),
                                in: RoundedRectangle(cornerRadius: 7))
                    }
                }
            }
        }
    }

    private func paperRow(_ label: String, _ value: String) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label)
                .font(.caption)
                .foregroundStyle(.secondary)
            Text(value)
                .font(.system(size: 26))
                .foregroundStyle(.primary)
                .textSelection(.enabled)
        }
    }

    /// **図は核の表を描くだけ。** 配列表を頁が持たないので、図と実際の打鍵はずれやうが無い。
    private var layoutChart: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("原器 \(KagiTable.keys.count) 鍵（核の表をそのまま描く）")
                .font(.caption)
                .foregroundStyle(.secondary)
            Text(
                KagiTable.keys
                    .map { "\($0.genki)\(typeKeys(String($0.genki)))" }
                    .joined(separator: "  ")
            )
            .font(.system(.caption, design: .monospaced))
            .foregroundStyle(.secondary)
        }
    }

    private func lastKeyDescription(_ usage: UInt16) -> String {
        let hex = String(format: "0x%02X", usage)
        if let g = model.lastGenki {
            return "HID \(hex) → 原器 '\(g)'"
        }
        return "HID \(hex) → 原器に無い鍵"
    }
}

#Preview {
    GenkiHardKeyboardView()
}
