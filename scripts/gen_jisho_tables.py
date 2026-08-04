#!/usr/bin/env python3
"""
gen_jisho_tables.py — 辞書のテキスト成果物から Rust の表を焼き付ける（**通信なし**）。

    青空文庫 --(harvest_jisho.py・要ネットワーク)--> core/data/jisho.tsv
                                                          |
                                        (本スクリプト・通信なし・決定的)
                                                          v
                                          core/src/generated/jisho_table.rs

**このスクリプトはネットワークを使はない。** だから CI の鮮度ゲートで回せる
（収穫の側 harvest_jisho.py は青空文庫へ通信するので CI では回さない）。
gen_bigram_tables.py と同じ役回りで、辞書へ広げたものである。

## 二つの出力

1. `core/data/jisho.tsv` を**正規形へ書き直す**（読み昇順・度数降順・表記昇順、meta 行の更新）。
   並べ替への規則をここ一か所に置くことで、手で足した行が順序を崩しても
   CI が `git diff --exit-code` で拾ふ。差分が読める形（テキスト）を保つのが狙ひである。
2. `core/src/generated/jisho_table.rs` — 実行時パースをしない静的配列。
   キーボード拡張・TSF の in-proc DLL では起動時の仕事を減らしたい
   （gen_bigram_tables.py が Rust 側で同じ判断をしてゐる）。
"""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA = os.path.join(ROOT, "core", "data", "jisho.tsv")
OUT_RUST = os.path.join(ROOT, "core", "src", "generated", "jisho_table.rs")

JIRITSU, FUZOKU = "自", "付"
POS_RUST = {JIRITSU: "Pos::Jiritsu", FUZOKU: "Pos::Fuzoku"}

# 見出しはここが持つ（収穫でも焼き付けでも同じ物が出るやうに）。
HEADER = """\
# 辞書 — 読み（歴史的仮名遣ひ）→ 表記（焼き付け成果物・テキスト）
#
# ここが**唯一の出所**で、Rust の表は scripts/gen_jisho_tables.py がこの一つの
# ファイルから起こす（docs/ime/artifacts.md）。**テキストで持つ**のは、
# git diff が読めることに賭けてゐるからである——辞書は人が眺めて直す成果物で、
# 「なぜ候補が変はつたか」が差分で分かる必要がある。
#
# 形式: 読み<TAB>表記<TAB>度数<TAB>品詞。品詞は 自（自立語）／付（付属語）の二種のみ。
# 文節は「自立語 ひとつ ＋ 付属語 いくつか」であり、この二分で分割が決まる
# （core/src/bunsetsu.rs）。細かい品詞は分割に効かないので持たない（発明しない）。
#
# 表記は**新字体**で持つ。旧字体は確定のときに核が機械で定める（core/src/kyuji.rs）ので、
# ここで二重に持つと二つの表がずれる（天気 と 天氣 の両方を抱へることになる）。
#
# 並びは「読み昇順・度数降順・表記昇順」に正規化されてゐる。
# scripts/gen_jisho_tables.py がこのファイル自身を書き直すので、
# 手で足した行が順序を崩しても CI の鮮度ゲートが拾ふ。
#"""


def load(path: str = DATA):
    """TSV を読み、(行の配列, meta 辞書) を返す。"""
    meta, rows = {}, []
    with open(path, encoding="utf-8") as f:
        for lineno, line in enumerate(f, 1):
            line = line.rstrip("\n")
            if line.startswith("#"):
                m = re.search(r"meta:\s*(.*)$", line)
                if m:
                    for kv in m.group(1).split():
                        k, _, v = kv.partition("=")
                        meta[k] = int(v) if v.isdigit() else v
                continue
            if not line:
                continue
            parts = line.split("\t")
            if len(parts) != 4:
                raise SystemExit(f"{path}:{lineno} 列が 4 つでない: {line!r}")
            yomi, surface, freq, pos = parts
            if pos not in POS_RUST:
                raise SystemExit(f"{path}:{lineno} 未知の品詞 {pos!r}（自 か 付 のみ）")
            if not yomi or not surface:
                raise SystemExit(f"{path}:{lineno} 読みか表記が空: {line!r}")
            if not freq.isdigit() or int(freq) <= 0:
                raise SystemExit(f"{path}:{lineno} 度数が正の整数でない: {freq!r}")
            rows.append((yomi, surface, int(freq), pos))
    if not rows:
        raise SystemExit(f"{path} に語が無い（データが空ではゲートが沈黙して死ぬ）")
    return rows, meta


def normalize(rows):
    """重複を畳み、読み昇順・度数降順・表記昇順へ整へる。

    同じ (読み, 表記) が二度出たら**度数の大きい方**を採る（収穫を継ぎ足したときに
    小さい方で上書きされないやう）。品詞が食ひ違ふのは辞書の壊れなので落とす。
    """
    best = {}
    for yomi, surface, freq, pos in rows:
        key = (yomi, surface)
        if key in best:
            prev_freq, prev_pos = best[key]
            if prev_pos != pos:
                raise SystemExit(f"{yomi}／{surface} の品詞が {prev_pos} と {pos} で食ひ違ふ")
            freq = max(freq, prev_freq)
        best[key] = (freq, pos)
    out = [(y, s, f, p) for (y, s), (f, p) in best.items()]
    # 読みは昇順（Python の str 順＝コードポイント順＝UTF-8 のバイト順なので、
    # Rust 側の &str に対する二分探索とそのまま噛み合ふ）。
    out.sort(key=lambda r: (r[0], -r[2], r[1]))
    return out


def write_tsv(rows, source: str, books: int, path: str = DATA) -> list:
    """正規形の TSV を書き出し、正規化後の行を返す。"""
    rows = normalize(rows)
    body = "\n".join(f"{y}\t{s}\t{f}\t{p}" for y, s, f, p in rows)
    meta = f"# meta: source={source} entries={len(rows)} books={books}\n"
    with open(path, "w", encoding="utf-8") as f:
        f.write(HEADER + "\n" + meta + body + "\n")
    return rows


def emit_rust(rows, meta: dict) -> str:
    """読み単位に畳んだ静的配列。実行時パースをしない。"""
    grouped: list[tuple[str, list]] = []
    for yomi, surface, freq, pos in rows:
        if grouped and grouped[-1][0] == yomi:
            grouped[-1][1].append((surface, freq, pos))
        else:
            grouped.append((yomi, [(surface, freq, pos)]))

    max_chars = max(len(y) for y, _ in grouped)
    lines = [
        "// 自動生成 — 手で編集しないこと。",
        "// SSOT: core/data/jisho.tsv（収穫 %s・%s 作品）"
        % (meta.get("source", "?"), meta.get("books", "?")),
        "// 生成: scripts/gen_jisho_tables.py（通信しないので CI の鮮度ゲートで回る）",
        "",
        "use crate::jisho::Pos;",
        "",
        "/// 一つの表記（表記, 度数, 品詞）。",
        "pub type Entry = (&'static str, u32, Pos);",
        "",
        "/// 一つの読みとその表記の並び。",
        "pub type Row = (&'static str, &'static [Entry]);",
        "",
        "/// 読み → [(表記, 度数, 品詞)]。**読み昇順**（二分探索の前提）で、",
        "/// 同じ読みの中は度数降順・表記昇順（候補の並びがここで決まる）。",
        "///",
        "/// rustfmt を止めてゐる — 1 行 1 読みの形が元データ（core/data/jisho.tsv）と",
        "/// 対応してをり、崩すと差分が読めなくなる（生成物なので整形の益も無い）。",
        "#[rustfmt::skip]",
        "pub static JISHO: [Row; %d] = [" % len(grouped),
    ]
    for yomi, items in grouped:
        pairs = ", ".join(
            '("%s", %d, %s)' % (s, f, POS_RUST[p]) for s, f, p in items
        )
        lines.append('    ("%s", &[%s]),' % (yomi, pairs))
    lines += [
        "];",
        "",
        "/// 読みの最長（**文字数**）。格子を張るときの前方一致の上限に使ふ。",
        "pub const MAX_YOMI_CHARS: usize = %d;" % max_chars,
        "",
        "/// 語数（読み×表記の対）。空の辞書がゲートを素通りするのを防ぐ検査に使ふ。",
        "pub const ENTRIES: usize = %d;" % len(rows),
        "",
    ]
    return "\n".join(lines)


def main() -> None:
    rows, meta = load()
    # 正規形へ書き直す（並べ替への規則をここ一か所に置く）
    rows = write_tsv(rows, source=meta.get("source", "seed"),
                     books=meta.get("books", 0))
    _, meta = load()

    os.makedirs(os.path.dirname(OUT_RUST), exist_ok=True)
    with open(OUT_RUST, "w", encoding="utf-8") as f:
        f.write(emit_rust(rows, meta))
    print("wrote %s" % os.path.relpath(DATA, ROOT))
    print("wrote %s" % os.path.relpath(OUT_RUST, ROOT))
    print("（読み %d 種・語 %d・収穫 %s）"
          % (len({r[0] for r in rows}), len(rows), meta.get("source", "?")))


if __name__ == "__main__":
    sys.exit(main())
