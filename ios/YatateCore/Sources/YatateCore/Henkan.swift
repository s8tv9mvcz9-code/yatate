// 変換の状態機械 — 仮名を積む段と、文節を選び直す段（core/src/henkan.rs）。
//
// ```text
//   Kana 段                          Henkan 段
//   ───────                          ─────────
//   原器の打鍵 → 仮名が積まれる       Space   → 次の候補へ
//   Space  → 変換して Henkan へ ───→  ←/→    → 注目する文節を移す
//   Enter  → 仮名のまま確定           Shift+←/→ → 注目文節を縮める/伸ばす
//   Esc    → 捨てる                   Enter   → 選んだ形で確定（旧字が定まる）
//                                    Esc/BS  → 変換を解いて Kana 段へ戻る
//                          ←───────  原器の鍵 → 確定して、続けて新しい未確定を立てる
// ```
//
// **旧字体が定まるのは確定の瞬間だけ**である。辞書は新字体で持つてゐるので、
// 候補を並べてゐる間の表記も新字体のままでよい（同じ知識を二か所に置かない）。
//
// ## 四枚目の手写しを作らない
//
// 段の管理・格子探索・費用計算は核に約 1,500 行ある。これを Swift へ写せば
// web・Windows・macOS・iOS で四枚の地図を持つことになり、このリポジトリが
// 繰り返し学んだ事故をそのまま踏む。ゆゑに殻は核を**駆動するだけ**である。

import YatateFFI

/// いまどの段に居るか。
public enum Phase: Sendable {
    /// 仮名を積んでゐる（未変換）。
    case kana
    /// 変換して文節を選び直してゐる。
    case henkan
}

/// 殻への指示。[`KeyAction`] を変換の段まで広げたもの。
public enum Act: Equatable, Sendable {
    /// 未確定文字列が変はつた（殻は描き直す）。
    case update
    /// 食つたが見た目は変はらない。
    case swallow
    /// 矢立の鍵ではない。殻は OS へ素通しする。
    case passthrough
    /// 確定した文字列。殻はこれを挿し込み、未確定を閉ぢる。
    case commit(String)
    /// 確定した上で、**続けて新しい未確定が立つてゐる**（変換中に原器の鍵が来た）。
    case commitThenUpdate(String)
    /// 捨てて閉ぢる。
    case cancel

    /// 確定した文字列があれば取り出す（殻が挿し込むもの）。
    public var committed: String? {
        switch self {
        case .commit(let t), .commitThenUpdate(let t): return t
        default: return nil
        }
    }
}

/// 一つの候補（この文節をどう書くか）。
public struct Candidate: Equatable, Sendable {
    /// 表記。**新字体のまま**であり、旧字体は確定のとき核が定める。
    public let surface: String
    /// 費用（小さいほど確からしい）。
    public let cost: Int
    /// 辞書に有る語から出た候補か（仮名のままの控へは false）。
    public let inJisho: Bool
}

/// 一つの文節。
public struct Segment: Equatable, Sendable {
    /// この文節の読み（仮名）。
    public let yomi: String
    /// いま選ばれてゐる候補の番号。
    public let chosen: Int
    /// いま選ばれてゐる表記。
    public let surface: String
}

/// 仮名と変換を通した入力の状態機械。入力欄ごとに 1 つ持つ。
public final class Henkan {
    private let handle = yatate_henkan_new()

    public init() {}

    deinit { yatate_henkan_free(handle) }

    // MARK: - いまの姿

    public var phase: Phase {
        yatate_henkan_phase(handle) == YATATE_PHASE_HENKAN ? .henkan : .kana
    }

    /// 未確定文字列を抱へてゐるか。
    public var isComposing: Bool { yatate_henkan_is_composing(handle) != 0 }

    /// 前置シフトが立つてゐるか（殻が面の表示を替へる手掛かり）。
    public var isShifted: Bool { yatate_henkan_is_shifted(handle) != 0 }

    /// 画面に出す未確定文字列。Kana 段では仮名、Henkan 段では**新字体の表記**。
    public var preedit: String { coreText(yatate_henkan_preedit(handle)) }

    /// いま抱へてゐる読み（段に依らず仮名）。
    public var yomi: String { coreText(yatate_henkan_yomi(handle)) }

    /// 墨の氣配 — 次の一打の分布。変換中は立たない。
    public var field: ActionField { Kehai.parse(coreText(yatate_henkan_kehai(handle))) }

    /// 注目してゐる文節の番号。
    public var focus: Int { Int(yatate_henkan_focus(handle)) }

    /// 注目してゐる文節で選ばれてゐる候補の番号。
    public var chosen: Int { Int(yatate_henkan_chosen(handle)) }

    /// 注目してゐる文節の候補（Kana 段では空）。
    public var candidates: [Candidate] {
        coreRows(coreText(yatate_henkan_candidates(handle))).compactMap { row in
            guard row.count == 3, let cost = Int(row[1]) else { return nil }
            return Candidate(surface: row[0], cost: cost, inJisho: row[2] == "1")
        }
    }

    /// 変換中の文節（Kana 段では空）。
    public var segments: [Segment] {
        coreRows(coreText(yatate_henkan_segments(handle))).compactMap { row in
            guard row.count == 3, let chosen = Int(row[1]) else { return nil }
            return Segment(yomi: row[0], chosen: chosen, surface: row[2])
        }
    }

    /// 注目文節が未確定文字列の何**文字目**（Unicode スカラ）から何文字か。
    /// **下線を引き分ける**ために殻が使ふ（IMKit の marked text・TSF の表示属性）。
    public var focusRange: (start: Int, length: Int) {
        var start = 0
        var length = 0
        yatate_henkan_focus_range(handle, &start, &length)
        return (start, length)
    }

    /// 同じものを **UTF-16** で。`NSRange` を要る殻（IMKit）はこちらを使ふ。
    ///
    /// 換算を殻ごとに書けば四度書くことになり、四度目に間違へる（[`utf16Range`]）。
    public var focusRangeUTF16: (location: Int, length: Int) {
        let (start, length) = focusRange
        return utf16Range(scalarStart: start, scalarLength: length, in: preedit)
    }

    /// この鍵を矢立が受け取るべきか。**副作用禁止**の問合せである。
    public func wantsKey(_ ch: Character) -> Bool {
        yatate_henkan_wants_key(handle, coreScalar(ch)) != 0
    }

    // MARK: - 打つ

    /// 原器の一打（位置から引いた文字を渡す）。
    public func key(_ ch: Character) -> Act {
        act(yatate_henkan_key(handle, coreScalar(ch)))
    }

    /// 仮名を**直に**積む（硝子の鍵盤用。二行配列は原器の写像を経ない）。
    @discardableResult
    public func insertKana(_ kana: String) -> Act {
        act(kana.withCore { yatate_henkan_insert_kana(handle, $0) })
    }

    /// Space —— 未変換なら**変換し**、変換中なら**次の候補へ**。
    @discardableResult
    public func convert() -> Act { act(yatate_henkan_convert(handle)) }

    @discardableResult
    public func nextCandidate() -> Act { act(yatate_henkan_next_candidate(handle)) }

    @discardableResult
    public func prevCandidate() -> Act { act(yatate_henkan_prev_candidate(handle)) }

    /// 候補を番号で選ぶ（候補窓を持つ殻が使ふ）。
    @discardableResult
    public func choose(_ index: Int) -> Act {
        act(yatate_henkan_choose(handle, index))
    }

    @discardableResult
    public func focusNext() -> Act { act(yatate_henkan_focus_next(handle)) }

    @discardableResult
    public func focusPrev() -> Act { act(yatate_henkan_focus_prev(handle)) }

    /// 注目文節を一字**伸ばす**（Shift+→）。
    @discardableResult
    public func growFocus() -> Act { act(yatate_henkan_grow_focus(handle)) }

    /// 注目文節を一字**縮める**（Shift+←）。
    @discardableResult
    public func shrinkFocus() -> Act { act(yatate_henkan_shrink_focus(handle)) }

    /// 一字消す。**変換中は消さず、変換を解いて仮名へ戻す**。
    @discardableResult
    public func backspace() -> Act { act(yatate_henkan_backspace(handle)) }

    /// 変換を解いて仮名の段へ戻す（読みは失はない）。
    @discardableResult
    public func unconvert() -> Act { act(yatate_henkan_unconvert(handle)) }

    /// Esc —— 変換中なら一段戻し、仮名の段なら捨てる。
    @discardableResult
    public func cancel() -> Act { act(yatate_henkan_cancel(handle)) }

    /// Enter —— 確定。**旧字体はここで機械が定まる。**
    @discardableResult
    public func commit() -> Act { act(yatate_henkan_commit(handle)) }

    /// 入力欄が替はつた等で全部捨てる。
    public func reset() { yatate_henkan_reset(handle) }

    private func act(_ code: Int32) -> Act {
        switch code {
        case YATATE_ACT_UPDATE: return .update
        case YATATE_ACT_SWALLOW: return .swallow
        case YATATE_ACT_CANCEL: return .cancel
        case YATATE_ACT_COMMIT:
            return .commit(coreText(yatate_henkan_take_commit(handle)))
        case YATATE_ACT_COMMIT_THEN_UPDATE:
            return .commitThenUpdate(coreText(yatate_henkan_take_commit(handle)))
        default: return .passthrough
        }
    }
}
