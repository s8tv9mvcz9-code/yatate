// 矢立 macOS — IMKit のテキストサービス。**ハード鍵盤で原器を打つ殻**である。
//
// iOS のキーボード拡張はハードキーを受け取れないので、原器（縦組五十音配列）が
// そのまま活きる場は macOS と Windows しかない（docs/ime/cross-platform.md §4）。
// これはその macOS 側で、Windows の TSF 殻（windows/src/tip.rs）と対になる。
//
// ## ここは薄い
//
// 配列（原器）・旧字確定は**一つも持たない**。すべて核（YatateCore）から来る。
// ここが受け持つのは macOS の作法だけ —— IMKit の契約と、鍵盤の翻訳。
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
// ## まだ無いもの（正直に）
//
// **候補窓と漢字変換は無い。** 確定は仮名のまま（旧字は核が機械で直す）で、
// Windows 殻の M7-b と同じ地点に居る。変換は Rust の核（core/src/henkan.rs）が
// 既に持つてゐるが、Swift 側にはまだ無い——四枚目の手写しを作るより
// Rust の核を静的ライブラリとして繋ぐ方が筋である（macos/YatateIME/README.md）。

import Cocoa
import InputMethodKit
import YatateCore

@objc(YatateInputController)
final class YatateInputController: IMKInputController {

    /// 打鍵の状態機械（**OS 非依存**）。矢立の頭脳はここに居る。
    private var session = Session()

    // MARK: - macOS の仮想キーコード（機能キー）
    //
    // 原器の 33 鍵は核の表から引くので、殻が名前で持つのは機能キーだけでよい。
    private enum FnKey {
        static let returnKey: UInt16 = 0x24  // kVK_Return
        static let tab: UInt16 = 0x30  // kVK_Tab
        static let space: UInt16 = 0x31  // kVK_Space
        static let delete: UInt16 = 0x33  // kVK_Delete（前方削除でなく BackSpace）
        static let escape: UInt16 = 0x35  // kVK_Escape
        static let keypadEnter: UInt16 = 0x4C  // kVK_ANSI_KeypadEnter
    }

    // MARK: - IMKit の入口

    override func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event, event.type == .keyDown else { return false }
        guard let client = sender as? IMKTextInput else { return false }

        // Command / Control / Option が押されてゐれば手を出さない。
        // 短絡キー（⌘C 等）を食ふとアプリの働きを奪つてしまふ。
        // **Shift は見ない**——原器の前置シフトは `^` の逐次打鍵であつて
        // 同時押しではないので、Shift を修飾として扱ふ必要が無い。
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        if !flags.isDisjoint(with: [.command, .control, .option]) {
            return false
        }

        // ── 機能キー（未確定を抱へてゐるときだけ意味を持つ）──
        if session.isComposing {
            switch event.keyCode {
            case FnKey.returnKey, FnKey.keypadEnter, FnKey.space:
                // 候補窓を持たないので確定＝仮名のまま（旧字は核が定める）
                return commit(client)
            case FnKey.escape:
                session.cancel()
                clearMarked(client)
                return true
            case FnKey.delete:
                let still = session.backspace()
                if still {
                    showMarked(client)
                } else {
                    clearMarked(client)
                }
                return true
            case FnKey.tab:
                // Tab は確定して素通しする（欄移動を奪はない）
                _ = commit(client)
                return false
            default:
                break
            }
        }

        // ── 原器の鍵 ──
        guard let ch = genki(mac: event.keyCode) else { return false }
        switch session.key(ch) {
        case .update:
            showMarked(client)
            return true
        // 前置シフトが立つた等。**食ふ**（流すと `=` が入力されてしまふ）
        case .swallow:
            return true
        case .passthrough:
            return false
        case .commit(let text):
            insert(text, client)
            return true
        }
    }

    /// 入力欄が替はつた。抱へてゐた未確定文字列は捨てる（別の欄へ持ち越さない）。
    override func deactivateServer(_ sender: Any!) {
        if let client = sender as? IMKTextInput, session.isComposing {
            // 打つた仮名は捨てない——選んでもゐない形で勝手に確定させもしない、
            // といふ折り合ひで、仮名のまま置く（web 殻の blur と同じ判断）。
            _ = commit(client)
        }
        session.cancel()
        super.deactivateServer(sender)
    }

    /// アプリ側から確定を促された（別の場所を触つた等）。
    override func commitComposition(_ sender: Any!) {
        guard let client = sender as? IMKTextInput else { return }
        _ = commit(client)
    }

    override func composedString(_ sender: Any!) -> Any {
        session.preedit
    }

    override func originalString(_ sender: Any!) -> NSAttributedString {
        NSAttributedString(string: session.preedit)
    }

    // MARK: - 画面へ移す

    /// 未確定文字列を marked text として置く。
    ///
    /// TSF の composition・IMKit の marked text・Android の composing text は
    /// **呼び名が違ふだけで同じもの**である（core/src/session.rs）。
    private func showMarked(_ client: IMKTextInput) {
        let text = session.preedit
        let attributed = NSAttributedString(
            string: text,
            attributes: mark(forStyle: kTSMHiliteSelectedRawText, at: NSRange(location: 0, length: text.utf16.count))
                as? [NSAttributedString.Key: Any] ?? [:]
        )
        client.setMarkedText(
            attributed,
            selectionRange: NSRange(location: text.utf16.count, length: 0),
            replacementRange: NSRange(location: NSNotFound, length: 0)
        )
    }

    private func clearMarked(_ client: IMKTextInput) {
        client.setMarkedText(
            NSAttributedString(string: ""),
            selectionRange: NSRange(location: 0, length: 0),
            replacementRange: NSRange(location: NSNotFound, length: 0)
        )
    }

    private func insert(_ text: String, _ client: IMKTextInput) {
        // 空の確定で marked text だけ残らぬやう、先に消してから挿す
        clearMarked(client)
        guard !text.isEmpty else { return }
        client.insertText(text, replacementRange: NSRange(location: NSNotFound, length: 0))
    }

    /// 確定 — **旧字体は核が機械で定める**。
    @discardableResult
    private func commit(_ client: IMKTextInput) -> Bool {
        guard case .commit(let text) = session.commit() else {
            clearMarked(client)
            return true
        }
        insert(text, client)
        return true
    }
}
