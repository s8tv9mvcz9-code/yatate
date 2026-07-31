#!/usr/bin/env python3
"""
gen_rust_tables.py — Python SSOT から Rust テーブルを機械生成する。

生成物: core/src/generated/kyuji_table.rs
SSOT:   ssot/kyuji.py の KYUJI_MAP（新字体→旧字体 1:1 写像）と AMBIGUOUS_SHINJI

`scripts/gen_swift_tables.py` の Rust 版で、**同じ SSOT から二つの写しを作る**
（docs/ime/cross-platform.md §6）。手書き禁止 — 再実行で常に上書きされる。
CI は再生成して `git diff --exit-code` することで生成物の鮮度を守る。

出力は char 昇順に並べる（Rust 側が binary_search で引くため。順序が崩れると
探索が静かに誤る＝ここは性能でなく正しさの要件である）。
"""
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "ssot"))
from kyuji import KYUJI_MAP, AMBIGUOUS_SHINJI  # noqa: E402

OUT = os.path.join(ROOT, "core", "src", "generated", "kyuji_table.rs")


def main() -> None:
    pairs = sorted(KYUJI_MAP.items())
    ambiguous = sorted(AMBIGUOUS_SHINJI)

    lines = [
        "// 自動生成 — 手で編集しないこと。",
        "// SSOT: ssot/kyuji.py（scripts/gen_rust_tables.py が生成）",
        "",
        "/// 新字体→旧字体の 1:1・文脈非依存写像（%d 字）。" % len(pairs),
        "///",
        "/// **char 昇順**。`binary_search_by_key` で引くので順序が不変条件である。",
        "pub static KYUJI_PAIRS: [(char, char); %d] = [" % len(pairs),
    ]
    lines += ["    ('%s', '%s')," % (k, v) for k, v in pairs]
    lines += [
        "];",
        "",
        "/// 一対多で危険なため写像から除外されてゐる新字体（弁・予・余・欠・芸）。",
        "///",
        "/// **char 昇順**（`binary_search` で引く）。KYUJI_PAIRS との共通部分が無いことは",
        "/// 不変条件で、`cargo test` と黄金ベクトルの双方が検査する。",
        "pub static AMBIGUOUS_SHINJI: [char; %d] = [%s];"
        % (len(ambiguous), ", ".join("'%s'" % c for c in ambiguous)),
        "",
    ]

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))
    print("wrote %s (%d entries, %d ambiguous)"
          % (OUT, len(pairs), len(ambiguous)))


if __name__ == "__main__":
    main()
