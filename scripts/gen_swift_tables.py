#!/usr/bin/env python3
"""
gen_swift_tables.py — Python SSOT から Swift テーブルを機械生成する（docs/ime/protocol.md §5）。

生成物: ios/YatateCore/Sources/YatateCore/Generated/KyujiTable.swift
SSOT:   ssot/kyuji.py の KYUJI_MAP（新字体→旧字体 1:1 写像）と AMBIGUOUS_SHINJI

手書き禁止 — このスクリプトの再実行で常に上書きされる。CI は再生成して
`git diff --exit-code` することで生成物の鮮度を守る（roadmap M1）。
"""
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "ssot"))
from kyuji import KYUJI_MAP, AMBIGUOUS_SHINJI  # noqa: E402

OUT = os.path.join(ROOT, "ios", "YatateCore", "Sources", "YatateCore",
                   "Generated", "KyujiTable.swift")


def main() -> None:
    pairs = sorted(KYUJI_MAP.items())
    lines = [
        "// 自動生成 — 手で編集しないこと。",
        "// SSOT: app/kyuji.py（scripts/gen_swift_tables.py が生成）",
        "",
        "/// 新字体→旧字体の 1:1・文脈非依存写像（%d 字）。" % len(pairs),
        "public enum KyujiTable {",
        "    public static let map: [Character: Character] = [",
    ]
    for k, v in pairs:
        lines.append('        "%s": "%s",' % (k, v))
    lines += [
        "    ]",
        "",
        "    /// 一対多で危険なため写像から除外されてゐる新字体。",
        '    public static let ambiguous: Set<Character> = Set("%s")'
        % "".join(sorted(AMBIGUOUS_SHINJI)),
        "}",
        "",
        "/// 新字体を旧字体へ機械的に確定する（app/kyuji.py: to_kyuji のパリティ実装）。",
        "public func toKyuji(_ text: String) -> String {",
        "    String(text.map { KyujiTable.map[$0] ?? $0 })",
        "}",
        "",
    ]
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))
    print("wrote %s (%d entries)" % (OUT, len(pairs)))


if __name__ == "__main__":
    main()
