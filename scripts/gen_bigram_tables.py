#!/usr/bin/env python3
"""
gen_bigram_tables.py — 仮名連 bigram の言語中立データから、核（Rust）の表を起こす。

    青空文庫 --(gen_yatate_ngram.py・要ネットワーク)--> core/data/kana_bigram.txt
                                                            |
                                                            v
                                              core/src/generated/kana_bigram.rs

**なぜ間にデータファイルを挟むか**（docs/ime/cross-platform.md §6）:
収穫（通信あり・非決定的）と表を起こす仕事（通信なし・決定的）を分けるためである。
分けてあるので、表の鮮度は CI の $0 の段で守れる。

**Swift 版はもう無い。** M5-b2 で iOS/macOS の殻が核（Rust）を静的ライブラリとして
呼ぶやうになつたので、同じ表を Swift へも起こす理由が消えた——表は一枚だけになつた。

このスクリプトは**ネットワークを使はない**。だから CI の鮮度ゲートで回せる
（収穫の側 gen_yatate_ngram.py は青空文庫へ通信するので CI では回さない）。
"""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA = os.path.join(ROOT, "core", "data", "kana_bigram.txt")
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


def emit_rust(packed: str, meta: dict) -> str:
    """表は**静的配列**として持つ（実行時パースをしない）。

    キーボード拡張・TSF の in-proc DLL では起動時の仕事を減らしたい。
    """
    lines = [
        "// 自動生成 — 手で編集しないこと。",
        "// SSOT: core/data/kana_bigram.txt"
        "（青空文庫 旧字旧仮名 %s 作品の仮名連）" % meta.get("books", "?"),
        "// 総文字数 %s・生成: scripts/gen_bigram_tables.py"
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
    os.makedirs(os.path.dirname(OUT_RUST), exist_ok=True)
    with open(OUT_RUST, "w", encoding="utf-8") as f:
        f.write(emit_rust(packed, meta))
    print("wrote %s" % os.path.relpath(OUT_RUST, ROOT))
    print("（前字 %d 種・%s 作品・%s 字）"
          % (len(packed.split("\n")), meta.get("books", "?"),
             f"{meta.get('chars', 0):,}"))


if __name__ == "__main__":
    sys.exit(main())
