#!/usr/bin/env python3
"""
gen_bigram_tables.py — 仮名連 bigram の言語中立データから、Swift と Rust の表を起こす。

    青空文庫 --(gen_yatate_ngram.py・要ネットワーク)--> core/data/kana_bigram.txt
                                                            |
                                    +-----------------------+-----------------------+
                                    v                                               v
      ios/.../Generated/KanaBigram.swift                        core/src/generated/kana_bigram.rs

**なぜ間にデータファイルを挟むか**（docs/ime/cross-platform.md §6）:
Swift 版と Rust 版を別々に収穫すると、青空文庫のカタログは時とともに変はるので
**二つの表が静かにずれる**。ずれた瞬間、同じ入力に対して iOS と Windows で
「墨の氣配」が違ふ絵を描くことになり、黄金ベクトルでは捕まらない
（ベクトルは論理の一致を見る道具で、データの一致は見ないため）。
唯一の出所を一つのファイルに固定し、両言語の表を**そこから機械で起こす**。

このスクリプトは**ネットワークを使はない**。だから CI の鮮度ゲートで回せる
（収穫の側 gen_yatate_ngram.py は青空文庫へ通信するので CI では回さない）。
"""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA = os.path.join(ROOT, "core", "data", "kana_bigram.txt")
OUT_SWIFT = os.path.join(ROOT, "ios", "YatateCore", "Sources", "YatateCore",
                         "Generated", "KanaBigram.swift")
OUT_RUST = os.path.join(ROOT, "core", "src", "generated", "kana_bigram.rs")


def load(path: str = DATA):
    """データファイルを読み、(packed 本体, meta 辞書) を返す。"""
    meta, rows = {}, []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if line.startswith("#"):
                m = re.search(r"meta:\s*(.*)$", line)
                if m:
                    for kv in m.group(1).split():
                        k, _, v = kv.partition("=")
                        meta[k] = int(v) if v.isdigit() else v
                continue
            if line:
                rows.append(line)
    if not rows:
        raise SystemExit(f"{path} に行が無い（データが空ではゲートが沈黙して死ぬ）")
    return "\n".join(rows), meta


def emit_swift(packed: str, meta: dict) -> str:
    return f'''// 自動生成 — 手で編集しないこと。
// SSOT: core/data/kana_bigram.txt（青空文庫 旧字旧仮名 {meta.get("books", "?")} 作品の仮名連）
// 総文字数 {meta.get("chars", 0):,}・行形式「前字>次字:度数,…」・`^` は連なりの開始、、。 は終端遷移。
// 生成: scripts/gen_bigram_tables.py（Rust 版と同じデータから起こす）

public enum KanaBigram {{
    static let packed = """
{packed}
"""

    /// 前字 → [(次字, 度数)]（度数降順）。`"^"` は仮名連の開始分布。
    public static let table: [String: [(next: Character, count: Int)]] = {{
        var t: [String: [(Character, Int)]] = [:]
        for line in packed.split(separator: "\\n") {{
            guard let sep = line.firstIndex(of: ">") else {{ continue }}
            let prev = String(line[..<sep])
            var items: [(Character, Int)] = []
            for pair in line[line.index(after: sep)...].split(separator: ",") {{
                guard let colon = pair.firstIndex(of: ":"),
                      let ch = pair[..<colon].first,
                      let n = Int(pair[pair.index(after: colon)...]) else {{ continue }}
                items.append((ch, n))
            }}
            t[prev] = items
        }}
        return t
    }}()
}}
'''


def emit_rust(packed: str, meta: dict) -> str:
    """Rust 版は表を**静的配列**として持つ（実行時パースをしない）。

    キーボード拡張・TSF の in-proc DLL では起動時の仕事を減らしたい。
    Swift 版は既存の実装を尊重して packed 文字列のままにしてある（挙動は同じで、
    黄金ベクトルが両者の一致を縛る）。
    """
    lines = [
        "// 自動生成 — 手で編集しないこと。",
        "// SSOT: core/data/kana_bigram.txt"
        "（青空文庫 旧字旧仮名 %s 作品の仮名連）" % meta.get("books", "?"),
        "// 総文字数 %s・生成: scripts/gen_bigram_tables.py（Swift 版と同じデータから起こす）"
        % f"{meta.get('chars', 0):,}",
        "",
        "/// 前字 → [(次字, 度数)]（度数降順）。`\"^\"` は仮名連の開始分布。",
        "///",
        "/// 実行時パースを避けて静的配列で持つ（拡張・in-proc DLL の起動を軽くするため）。",
        "///",
        "/// rustfmt を止めてゐる — 1 行 1 前字の形は元データ（core/data/kana_bigram.txt）と",
        "/// 行が一対一で対応してをり、崩すと差分が読めなくなる（生成物なので整形の益も無い）。",
        "#[rustfmt::skip]",
        "pub static KANA_BIGRAM: [(&str, &[(char, u32)]); %d] = ["
        % len(packed.split("\n")),
    ]
    for row in packed.split("\n"):
        prev, _, items = row.partition(">")
        pairs = []
        for pair in items.split(","):
            ch, _, n = pair.partition(":")
            pairs.append("('%s', %s)" % (ch, n))
        lines.append('    ("%s", &[%s]),' % (prev, ", ".join(pairs)))
    lines += ["];", ""]
    return "\n".join(lines)


def main() -> None:
    packed, meta = load()
    for path, text in ((OUT_SWIFT, emit_swift(packed, meta)),
                       (OUT_RUST, emit_rust(packed, meta))):
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as f:
            f.write(text)
        print("wrote %s" % os.path.relpath(path, ROOT))
    print("（前字 %d 種・%s 作品・%s 字）"
          % (len(packed.split("\n")), meta.get("books", "?"),
             f"{meta.get('chars', 0):,}"))


if __name__ == "__main__":
    sys.exit(main())
