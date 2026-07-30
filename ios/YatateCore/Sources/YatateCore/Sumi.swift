// 墨と紙 — 明暗両様の色（docs/ime/vla.md の可視化語彙を色に落とす）。
//
// 矢立の画面は「紙に墨」といふ一つの比喩で出来てゐる。暗い場面ではこれを
// **拓本**（black を反転させた白抜き）に置き換へる: 地が濃くなり、墨は白く抜ける。
// 明度を入れ替へるだけで比喩は壊れず、濃淡の意味（＝確率の高さ）も保たれる。
//
// 鍵盤拡張（YatateKeyboard）と文机（本体アプリ）が同じ表を見る。
// 色をどちらかに書き写すと必ずずれるので、ここを唯一の出所とする。

import SwiftUI

public enum Sumi {
    /// 紙 — 画面の地。
    public static func paper(_ scheme: ColorScheme) -> Color {
        scheme == .dark ? Color(white: 0.09) : Color(white: 0.93)
    }

    /// 鍵・マスの面。
    public static func key(_ scheme: ColorScheme) -> Color {
        scheme == .dark ? Color(white: 0.17) : Color(white: 1.0)
    }

    /// 機能縁（削除・句読点など、仮名でない鍵）。
    public static func fringe(_ scheme: ColorScheme) -> Color {
        scheme == .dark ? Color(white: 0.13) : Color(white: 0.85)
    }

    /// 作業帯・書かれた文の下地。
    public static func slip(_ scheme: ColorScheme) -> Color {
        scheme == .dark ? Color(white: 0.14) : Color(white: 0.99)
    }

    /// 墨の気配 — level 0…1 の濃さ。暗い場面では白く抜ける（拓本）。
    /// 反転しても「濃い＝確率が高い」の読みは変はらない。
    public static func ink(_ scheme: ColorScheme, _ level: Double) -> Color {
        let l = min(max(level, 0), 1)
        return scheme == .dark
            ? Color(white: 1.0).opacity(0.34 * l)
            : Color(white: 0.0).opacity(0.26 * l)
    }

    /// 筆脈 — 最有力の一手へ走る一画。
    public static func hitsumyaku(_ scheme: ColorScheme) -> Color {
        scheme == .dark
            ? Color(white: 1.0).opacity(0.30)
            : Color(white: 0.0).opacity(0.20)
    }

    /// 連綿の軌跡（指が通つた跡の薄墨）。
    public static func trace(_ scheme: ColorScheme) -> Color {
        scheme == .dark
            ? Color(white: 1.0).opacity(0.16)
            : Color(white: 0.0).opacity(0.12)
    }

    /// 梯子の段の下地（段レベルの気配を敷いた面）。
    public static func ladderStep(_ scheme: ColorScheme, ink level: Double) -> Color {
        let l = min(max(level, 0), 1)
        return scheme == .dark
            ? Color(white: 0.20 + 0.16 * l)
            : Color(white: 0.97 - 0.20 * l)
    }

    /// 伝統色を地の上に置くときの最低限の可読性を確保する。
    /// 共感覚の色は明度の低いものが多く、暗い地では沈んで見えなくなるため、
    /// 暗い場面でのみ明るい側へ寄せる（色相は動かさない＝色名との対応を壊さない）。
    public static func legible(_ hex: String, on scheme: ColorScheme) -> Color {
        guard let (r, g, b) = Self.rgb(hex) else { return .secondary }
        guard scheme == .dark else { return Color(red: r, green: g, blue: b) }
        // 知覚的な明るさ（Rec.709 相当）が低ければ白へ混ぜて持ち上げる
        let y = 0.2126 * r + 0.7152 * g + 0.0722 * b
        guard y < 0.45 else { return Color(red: r, green: g, blue: b) }
        let t = (0.45 - y) / 0.45 * 0.55        // 最大 55% まで白を混ぜる
        return Color(red: r + (1 - r) * t,
                     green: g + (1 - g) * t,
                     blue: b + (1 - b) * t)
    }

    /// "#RRGGBB" → 0…1 の三成分。
    static func rgb(_ hex: String) -> (Double, Double, Double)? {
        var s = hex.trimmingCharacters(in: .whitespaces)
        if s.hasPrefix("#") { s.removeFirst() }
        guard s.count == 6, let v = UInt32(s, radix: 16) else { return nil }
        return (Double((v >> 16) & 0xFF) / 255,
                Double((v >> 8) & 0xFF) / 255,
                Double(v & 0xFF) / 255)
    }
}
