// 入力セッション — **どの OS でも同じ**打鍵の状態機械（core/src/session.rs）。
//
// 殻はここへ打鍵を渡し、返つてきた指示のとほりに未確定文字列を描き、
// 確定時に文字列を挿し込むだけでよい。TSF の composition も IMKit の marked text も
// Android の composing text も、**呼び名が違ふだけで同じもの**である。
//
// 変換（Space で漢字へ）まで要るなら [`Henkan`] を使ふ。ここは仮名だけの道で、
// 二行配列（硝子）のやうに変換を持たない場面のための細い口である。

import YatateFFI

/// 一打に対する殻への指示。
public enum KeyAction: Equatable, Sendable {
    /// 未確定文字列が変はつた（殻は marked text を描き直す）。
    case update
    /// 何も起きなかつた（前置シフトが立つた等）。殻は打鍵を**呑む**。
    case swallow
    /// 矢立の鍵ではない。殻は OS へ**素通し**する。
    case passthrough
    /// 確定した文字列。殻はこれを挿し込み、marked text を閉ぢる。
    case commit(String)
}

/// 未確定文字列を抱へる入力セッション。入力欄ごとに 1 つ持つ。
///
/// 核の状態を握るので参照型である（値型にすると写した瞬間に状態が分裂する）。
public final class Session {
    private let handle = yatate_session_new()

    public init() {}

    deinit { yatate_session_free(handle) }

    /// いま未確定の仮名列。
    public var preedit: String { coreText(yatate_session_preedit(handle)) }

    public var isComposing: Bool { yatate_session_is_composing(handle) != 0 }

    /// 前置シフトが立つてゐるか（殻が面の表示を替へる手掛かり）。
    public var isShifted: Bool { yatate_session_is_shifted(handle) != 0 }

    /// 墨の氣配 — 次の一打の分布。候補窓・鍵盤表示に使ふ（描くのは殻）。
    public var field: ActionField { Kehai.parse(coreText(yatate_session_kehai(handle))) }

    /// 原器の一打を食はせる。
    public func key(_ ch: Character) -> KeyAction {
        action(yatate_session_key(handle, coreScalar(ch)))
    }

    /// 仮名を**直に**積む（硝子の鍵盤用。二行配列は原器の写像を経ない）。
    @discardableResult
    public func insertKana(_ kana: String) -> KeyAction {
        action(kana.withCore { yatate_session_insert_kana(handle, $0) })
    }

    /// この鍵を矢立が受け取るべきか（IMKit の `handle` が先に判断する）。
    ///
    /// 未確定文字列がある間は、原器に無い鍵も**一旦は受ける**
    /// （確定・取り消しの機会を殻に残すため）。**副作用は無い。**
    public func wantsKey(_ ch: Character) -> Bool {
        yatate_session_wants_key(handle, coreScalar(ch)) != 0
    }

    /// 一字消す。未確定文字列が空になつたら `false`（殻は marked text を閉ぢる）。
    @discardableResult
    public func backspace() -> Bool {
        yatate_session_backspace(handle) != 0
    }

    /// 確定 — **旧字体は核が機械で確定させる**。
    public func commit() -> KeyAction {
        action(yatate_session_commit(handle))
    }

    /// 取り消し（Esc）。未確定文字列を捨てる。
    public func cancel() {
        yatate_session_cancel(handle)
    }

    private func action(_ code: Int32) -> KeyAction {
        switch code {
        case YATATE_ACT_UPDATE: return .update
        case YATATE_ACT_SWALLOW: return .swallow
        case YATATE_ACT_COMMIT: return .commit(coreText(yatate_session_take_commit(handle)))
        default: return .passthrough
        }
    }
}
