// 矢立 macOS — IMKit のテキストサービス。**ハード鍵盤で原器を打つ殻**である。
//
// iOS のキーボード拡張はハードキーを受け取れないので、原器（縦組五十音配列）が
// そのまま活きる場は macOS と Windows しかない（docs/ime/cross-platform.md §4）。
// これはその macOS 側で、Windows の TSF 殻（windows/src/tip.rs）と対になる。
//
// ## ここは薄い
//
// 配列（原器）も、旧字確定も、文節分割も、候補の並べ方も**一つも持たない**。
// すべて核（YatateCore → core/・Rust）から来る。ここが受け持つのは macOS の
// 作法だけ —— IMKit の契約と、鍵盤の翻訳と、板を出す位置である。
//
// ## 鍵は kVK で引く（**英字配列を前提とする**）
//
// `NSEvent.characters`（出る字）ではなく `NSEvent.keyCode`（Carbon の
// `kVK_ANSI_*` ＝ **物理位置**）で引く。理由は Windows 殻が `ToUnicodeEx` を
// 使はないのと同じである:
//
//   ・出る字は配列に依つて動く。US 配列の機で「し」が黙つて「さ」になる
//     （`;` と `:` は刻印も物理位置も同じで、意味だけが入れ替はる）
//   ・かな入力の状態では `characters` が半角カタカナを返して表が全面的に破綻する
//
// 位置で引けば、この誤爆はそもそも生まれない。英字配列でも原器の 33 鍵が要る
// 物理位置はすべて在るので（US の ' ; = が JIS の : ; ^ に当たる）、
// 配列ごとの分岐は要らない。
//
// ## 運指は Windows 殻と同じ
//
//   Space      未変換なら変換／変換中は次の候補へ
//   Enter      確定（**ここで旧字が定まる**）
//   Esc・BS    変換を解いて仮名へ戻す（読みは失はない）
//   ←→        注目する文節を移す
//   Shift+←→  注目文節を縮める／伸ばす
//   ↑↓        前の／次の候補へ
//
// **数字で候補を選ばせない。** 1〜0 は原器の第一の段（と て つ ち た／お え う い あ）
// であり、そこに候補選択を重ねると「た を打たうとして候補が飛ぶ」といふ
// 無音の事故になる。候補窓に番号を出さないのもそのためである。

import Cocoa
import InputMethodKit
import YatateCore

@objc(YatateInputController)
final class YatateInputController: IMKInputController {

    /// 変換まで含む状態機械（**OS 非依存**）。矢立の頭脳はここに居る。
    private let henkan = Henkan()
    private let candidates = CandidateWindow()

    // MARK: - macOS の仮想キーコード（機能キー）
    //
    // 原器の 33 鍵は核の表から引くので、殻が名前で持つのは機能キーだけでよい。
    // どれも原器の kVK と重ならないことは YatateCoreTests が縛つてゐる
    // ——重なれば「文節を移さうとしたら と が入る」といふ無音の事故になる。
    private enum FnKey {
        static let returnKey: UInt16 = 0x24  // kVK_Return
        static let tab: UInt16 = 0x30  // kVK_Tab
        static let space: UInt16 = 0x31  // kVK_Space
        static let delete: UInt16 = 0x33  // kVK_Delete（前方削除でなく BackSpace）
        static let escape: UInt16 = 0x35  // kVK_Escape
        static let keypadEnter: UInt16 = 0x4C  // kVK_ANSI_KeypadEnter
        static let left: UInt16 = 0x7B  // kVK_LeftArrow
        static let right: UInt16 = 0x7C  // kVK_RightArrow
        static let down: UInt16 = 0x7D  // kVK_DownArrow
        static let up: UInt16 = 0x7E  // kVK_UpArrow
    }

    // MARK: - IMKit の入口

    override func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event, event.type == .keyDown else { return false }
        guard let client = sender as? IMKTextInput else { return false }

        // Command / Control / Option が押されてゐれば手を出さない。
        // 短絡キー（⌘C 等）を食ふとアプリの働きを奪つてしまふ。
        // **Shift だけは見る**——原器の前置シフトは `^` の逐次打鍵であつて
        // 同時押しではないので、Shift+矢印を区切り修正に使つても衝突しない。
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        if !flags.isDisjoint(with: [.command, .control, .option]) {
            return false
        }
        let shift = flags.contains(.shift)

        // ── 機能キー（未確定を抱へてゐるときだけ意味を持つ）──
        if henkan.isComposing {
            switch event.keyCode {
            case FnKey.space:
                return apply(henkan.convert(), client)
            case FnKey.returnKey, FnKey.keypadEnter:
                return apply(henkan.commit(), client)
            case FnKey.escape:
                return apply(henkan.cancel(), client)
            case FnKey.delete:
                return apply(henkan.backspace(), client)
            case FnKey.left:
                return apply(shift ? henkan.shrinkFocus() : henkan.focusPrev(), client)
            case FnKey.right:
                return apply(shift ? henkan.growFocus() : henkan.focusNext(), client)
            case FnKey.down:
                return apply(henkan.nextCandidate(), client)
            case FnKey.up:
                return apply(henkan.prevCandidate(), client)
            case FnKey.tab:
                // Tab は確定して素通しする（欄移動を奪はない）
                _ = apply(henkan.commit(), client)
                return false
            default:
                break
            }
        }

        // ── 原器の鍵 ──
        guard let ch = genki(mac: event.keyCode) else { return false }
        return apply(henkan.key(ch), client)
    }

    /// 入力欄が替はつた。抱へてゐた未確定文字列は捨てない。
    override func deactivateServer(_ sender: Any!) {
        if let client = sender as? IMKTextInput, henkan.isComposing {
            // 打つた仮名は捨てない——選んでもゐない形で勝手に確定させもしない、
            // といふ折り合ひで、いま選んでゐる形のまま置く（web 殻の blur と同じ判断）。
            _ = apply(henkan.commit(), client)
        }
        henkan.reset()
        candidates.hide()
        super.deactivateServer(sender)
    }

    /// アプリ側から確定を促された（別の場所を触つた等）。
    override func commitComposition(_ sender: Any!) {
        guard let client = sender as? IMKTextInput else { return }
        _ = apply(henkan.commit(), client)
    }

    override func composedString(_ sender: Any!) -> Any {
        henkan.preedit
    }

    override func originalString(_ sender: Any!) -> NSAttributedString {
        NSAttributedString(string: henkan.yomi)
    }

    // MARK: - 核の指示を画面へ移す

    /// 核が返した指示のとほりに描く。**判断はここでしない。**
    @discardableResult
    private func apply(_ act: Act, _ client: IMKTextInput) -> Bool {
        switch act {
        case .update:
            showMarked(client)
        case .swallow:
            break
        case .passthrough:
            return false
        case .cancel:
            clearMarked(client)
            candidates.hide()
        case .commit(let text):
            insert(text, client)
            candidates.hide()
        case .commitThenUpdate(let text):
            // 確定した上で、続けて新しい未確定が立つてゐる
            insert(text, client)
            showMarked(client)
        }
        return true
    }

    /// 未確定文字列を marked text として置き、注目文節に濃い下線を引く。
    ///
    /// TSF の composition・IMKit の marked text・Android の composing text は
    /// **呼び名が違ふだけで同じもの**である（core/src/session.rs）。
    private func showMarked(_ client: IMKTextInput) {
        let text = henkan.preedit
        guard !text.isEmpty else {
            clearMarked(client)
            candidates.hide()
            return
        }

        let full = NSRange(location: 0, length: text.utf16.count)
        let attributed = NSMutableAttributedString(string: text)

        if henkan.phase == .henkan {
            // 変換中は「いま直せるのはここ」を下線で示す。文節が一つでも引き分ける
            // ——注目の在り処が見えないと Shift+←→ が手探りになる。
            attributed.addAttributes(attrs(kTSMHiliteConvertedText), range: full)
            // 核はスカラで、IMKit は UTF-16 で数へる。換算は核の側が持つ
            // （殻ごとに書けば四度書くことになる）。
            let focus = henkan.focusRangeUTF16
            if focus.length > 0 {
                attributed.addAttributes(
                    attrs(kTSMHiliteSelectedConvertedText),
                    range: NSRange(location: focus.location, length: focus.length))
            }
        } else {
            attributed.addAttributes(attrs(kTSMHiliteSelectedRawText), range: full)
        }

        client.setMarkedText(
            attributed,
            selectionRange: NSRange(location: text.utf16.count, length: 0),
            replacementRange: NSRange(location: NSNotFound, length: 0))

        showCandidates(client)
    }

    /// 変換中だけ候補窓を出す。候補が一つしか無ければ出さない（選ぶ余地が無い）。
    private func showCandidates(_ client: IMKTextInput) {
        let list = henkan.candidates.map(\.surface)
        guard henkan.phase == .henkan, list.count > 1 else {
            candidates.hide()
            return
        }
        var cursor = NSRect.zero
        // カーソルの画面上の位置を訊く。答へない相手も居るので、その時は出さない
        // （画面の隅に迷子の板を置くより、出さない方がまし）。
        _ = client.attributes(forCharacterIndex: 0, lineHeightRectangle: &cursor)
        guard cursor.width > 0 || cursor.height > 0 else {
            candidates.hide()
            return
        }
        candidates.show(candidates: list, chosen: henkan.chosen, at: cursor)
    }

    private func attrs(_ style: Int) -> [NSAttributedString.Key: Any] {
        mark(forStyle: style, at: NSRange(location: NSNotFound, length: 0))
            as? [NSAttributedString.Key: Any] ?? [:]
    }

    private func clearMarked(_ client: IMKTextInput) {
        client.setMarkedText(
            NSAttributedString(string: ""),
            selectionRange: NSRange(location: 0, length: 0),
            replacementRange: NSRange(location: NSNotFound, length: 0))
    }

    private func insert(_ text: String, _ client: IMKTextInput) {
        // 空の確定で marked text だけ残らぬやう、先に消してから挿す
        clearMarked(client)
        guard !text.isEmpty else { return }
        client.insertText(text, replacementRange: NSRange(location: NSNotFound, length: 0))
    }

}
