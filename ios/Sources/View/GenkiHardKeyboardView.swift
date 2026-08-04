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
// 成立する——そして何より、**核（Session・Kagi）が iOS でも同じ答へを出すこと**を
// 実機で確かめる場になる。
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
    /// 物理位置（HID usage）をそのまま上へ渡す。判断は核がする。
    var onKey: ((UIKeyboardHIDUsage) -> Bool)?

    override var canBecomeFirstResponder: Bool { true }

    override func pressesBegan(_ presses: Set<UIPress>, with event: UIPressesEvent?) {
        var unhandled = Set<UIPress>()
        for press in presses {
            guard let key = press.key else {
                unhandled.insert(press)
                continue
            }
            // 修飾キーが押されてゐれば手を出さない（短絡キーを奪はない）。
            // Shift は見ない——原器の前置シフトは `^` の逐次打鍵である。
            let mods = key.modifierFlags.intersection([.command, .control, .alternate])
            if !mods.isEmpty || onKey?(key.keyCode) != true {
                unhandled.insert(press)
            }
        }
        if !unhandled.isEmpty {
            super.pressesBegan(unhandled, with: event)
        }
    }
}

struct HardKeyPickup: UIViewRepresentable {
    let onKey: (UIKeyboardHIDUsage) -> Bool

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

/// 打鍵の状態を持つ器。**核の Session をそのまま駆動する**（写しを持たない）。
@MainActor
final class GenkiKeyboardModel: ObservableObject {
    @Published private(set) var preedit = ""
    @Published private(set) var committed = ""
    @Published private(set) var shifted = false
    /// 直前に受けた物理位置（実機で「どの鍵が来たか」を目で確かめるため）。
    @Published private(set) var lastUsage: UInt16?
    @Published private(set) var lastGenki: Character?

    private var session = Session()

    /// 返り値は「食つたか」。食はなかつた鍵は OS へ返す。
    func handle(_ usage: UIKeyboardHIDUsage) -> Bool {
        lastUsage = UInt16(usage.rawValue)

        // ── 機能キー（未確定を抱へてゐるときだけ意味を持つ）──
        if session.isComposing {
            switch usage {
            case .keyboardReturnOrEnter, .keypadEnter, .keyboardSpacebar:
                commit()
                return true
            case .keyboardEscape:
                session.cancel()
                sync()
                return true
            case .keyboardDeleteOrBackspace:
                _ = session.backspace()
                sync()
                return true
            default:
                break
            }
        }

        guard let ch = genki(hid: UInt16(usage.rawValue)) else {
            lastGenki = nil
            return false
        }
        lastGenki = ch
        switch session.key(ch) {
        case .update, .swallow:
            sync()
            return true
        case .passthrough:
            return false
        case .commit(let text):
            committed += text
            sync()
            return true
        }
    }

    func commit() {
        if case .commit(let text) = session.commit() {
            committed += text
        }
        sync()
    }

    func clear() {
        session.cancel()
        committed = ""
        lastUsage = nil
        lastGenki = nil
        sync()
    }

    private func sync() {
        preedit = session.preedit
        shifted = session.isShifted
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

            VStack(alignment: .leading, spacing: 20) {
                header

                paperRow("確定", model.committed.isEmpty ? "—" : model.committed)
                paperRow("未確定", model.preedit.isEmpty ? "—" : model.preedit)

                HStack(spacing: 12) {
                    if model.shifted {
                        Label("前置シフト", systemImage: "arrow.up.square")
                            .foregroundStyle(Sumi.key(scheme))
                    }
                    if let u = model.lastUsage {
                        Text(lastKeyDescription(u))
                            .font(.system(.footnote, design: .monospaced))
                            .foregroundStyle(Sumi.fringe(scheme))
                    }
                }

                layoutChart

                Button("消す") { model.clear() }
                    .buttonStyle(.bordered)

                Spacer()
            }
            .padding(24)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("原器 — ハード鍵盤")
                .font(.title2.weight(.semibold))
                .foregroundStyle(Sumi.key(scheme))
            Text(
                "外付け鍵盤で打つてください。**位置**で引くので、英字配列でも刻印どほりに動きます"
                    + "（US の ' ; = が : ; ^ に当たる）。Enter か Space で確定、Esc で取消。"
            )
            .font(.footnote)
            .foregroundStyle(Sumi.fringe(scheme))
        }
    }

    private func paperRow(_ label: String, _ value: String) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label)
                .font(.caption)
                .foregroundStyle(Sumi.fringe(scheme))
            Text(value)
                .font(.system(size: 26))
                .foregroundStyle(Sumi.key(scheme))
                .textSelection(.enabled)
        }
    }

    /// **図は核の表を描くだけ。** 配列表を頁が持たないので、図と実際の打鍵はずれやうが無い。
    private var layoutChart: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("原器 \(KagiTable.keys.count) 鍵（核の表をそのまま描く）")
                .font(.caption)
                .foregroundStyle(Sumi.fringe(scheme))
            Text(
                KagiTable.keys
                    .map { "\($0.genki)\(typeKeys(String($0.genki)))" }
                    .joined(separator: "  ")
            )
            .font(.system(.caption, design: .monospaced))
            .foregroundStyle(Sumi.fringe(scheme))
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
