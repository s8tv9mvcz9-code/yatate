// 核との継ぎ目 — C の文字列と TSV を Swift へ運ぶだけの小道具。
//
// この層より上に**核の知識を置かない**のがこのファイルの存在理由である。
// 配列も旧字の 248 字も氣配の重みも、Swift 側は一つも覚えてゐない。
// 覚えれば二枚目の地図になり、必ずずれる（docs/ime/cross-platform.md §6）。

import YatateFFI

/// 核が返した C 文字列を Swift の文字列にして、**必ず返す**。
///
/// 束縛の約束は「返り値のある関数を呼んだら `yatate_string_free` へ渡す」。
/// 呼び出し側にそれを覚えさせると必ずどこかで漏れるので、ここで一手に引き受ける。
func coreText(_ p: UnsafeMutablePointer<CChar>?) -> String {
    guard let p else { return "" }
    defer { yatate_string_free(p) }
    return String(cString: p)
}

/// TSV を行×欄に開く（空行は捨てる）。
func coreRows(_ text: String) -> [[String]] {
    text.split(separator: "\n", omittingEmptySubsequences: true).map {
        $0.components(separatedBy: "\t")
    }
}

/// 核が返す文字のスカラ値を `Character` へ。`0`（無い）は `nil`。
func coreChar(_ scalar: UInt32) -> Character? {
    guard scalar != 0, let u = Unicode.Scalar(scalar) else { return nil }
    return Character(u)
}

/// `Character` を核へ渡すスカラ値へ。
func coreScalar(_ c: Character) -> UInt32 {
    c.unicodeScalars.first?.value ?? 0
}

extension String {
    /// 核へ渡す（NUL 終端の UTF-8 として貸す）。
    func withCore<R>(_ body: (UnsafePointer<CChar>) -> R) -> R {
        withCString(body)
    }
}

/// **核は Unicode スカラで数へ、Apple の文字列 API は UTF-16 で数へる。**
///
/// 仮名と常用の漢字では両者が一致するので気づきにくいが、基本多言語面の外の字
/// （𠮷 の類）が候補に出た瞬間に一文字ぶんずれる。ずれた範囲を marked text の
/// 下線に使ふと「どの文節を直してゐるか」を**嘘で示す**ことになるので、ここで換算する。
///
/// 殻ごとに書けば四度書くことになるし、四度目に間違へる。だから核の側に置く。
public func utf16Range(
    scalarStart: Int, scalarLength: Int, in text: String
) -> (location: Int, length: Int) {
    guard scalarStart >= 0, scalarLength >= 0 else { return (0, 0) }
    var location = 0
    var length = 0
    var index = 0
    for scalar in text.unicodeScalars {
        let width = UTF16.width(scalar)
        if index < scalarStart {
            location += width
        } else if index < scalarStart + scalarLength {
            length += width
        } else {
            break
        }
        index += 1
    }
    // 範囲が文字列の外に出てゐたら、始まりだけは末尾に丸める（負や飛び出しを渡さない）
    return (min(location, text.utf16.count), length)
}
