#!/usr/bin/env python3
"""
harvest_jisho.py — 青空文庫のルビから「読み → 表記」を収穫する（**通信あり**）。

    青空文庫 --(本スクリプト・要ネットワーク)--> core/data/jisho.tsv
                                                       |
                                (scripts/gen_jisho_tables.py・通信なし)
                                                       v
                                     core/src/generated/jisho_table.rs

**収穫と焼き付けを分けるのが本スクリプトの存在理由である**（docs/ime/artifacts.md）。

収穫は ①通信する ②青空文庫のカタログが時とともに変はるので非決定的 ③オフラインで
一回だけ回す、の三拍子が揃つてゐる。焼き付け（TSV → Rust の表）は通信せず決定的なので、
CI の鮮度ゲートで回せる。gen_yatate_ngram.py（収穫）と gen_bigram_tables.py（焼き付け）が
既に同じ形をしてをり、本スクリプトはその流儀を辞書へ広げたものである。

ルビは「漢字列《よみ》」の形で本文に埋まつてゐる。旧字旧仮名の作品に限るので、
取れる読みは**歴史的仮名遣ひ**である——矢立が要るのはまさにそれで、
現代仮名遣ひの辞書からは作れない（「けふ→今日」は新仮名の辞書に無い）。

## 付属語は収穫できない

ルビは自立語（漢字を含む語）にしか振られないので、助詞・助動詞は一語も取れない。
付属語は既存の TSV から**引き継ぐ**（`--keep-fuzoku`・既定で有効）。
そこは人が育てる部分で、青空文庫から機械で起こせる部分ではない。

カタログ取得・本文クリーニングは gen_yatate_ngram.py と同じ規約である
（同スクリプトの規約を変へたらここも揃へること）。
"""
from __future__ import annotations

import argparse
import csv
import io
import os
import re
import sys
import time
import zipfile
from collections import Counter, defaultdict

import requests

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "core", "data", "jisho.tsv")

CATALOG_URL = "https://www.aozora.gr.jp/index_pages/list_person_all_extended_utf8.zip"
TARGET_STYLES = {"旧字旧仮名"}          # 読みは旧仮名に限る（新仮名を混ぜると辞書が壊れる）
SLEEP_SEC = 0.5

HEADER_FOOTER = re.compile(r"-{5,}.*?-{5,}", re.DOTALL)
NOTE = re.compile(r"［＃[^］]*］")

# ルビ — 「｜任意の語《よみ》」と「漢字列《よみ》」の二形。読みはひらがなのみ採る
# （カタカナ・ローマ字のルビは語の読みではなく注記であることが多い）。
RUBY = re.compile(r"｜([^《｜\n]{1,16})《([ぁ-ゖ]{1,16})》|([一-龥々ヶ]{1,16})《([ぁ-ゖ]{1,16})》")

# 収穫した語の外側にある送り仮名までは取らない。表記に仮名が混じるのは
# ｜付きルビ（｜思ひ出《おもひで》）の場合だけである。
JIRITSU, FUZOKU = "自", "付"


def load_catalog(max_books: int) -> list[dict]:
    print(f"[catalog] {CATALOG_URL}")
    resp = requests.get(CATALOG_URL, timeout=60)
    resp.raise_for_status()
    with zipfile.ZipFile(io.BytesIO(resp.content)) as zf:
        csv_name = next(n for n in zf.namelist() if n.endswith(".csv"))
        raw_csv = zf.read(csv_name).decode("utf-8-sig")
    books, seen = [], set()
    for row in csv.DictReader(io.StringIO(raw_csv)):
        if row.get("文字遣い種別") not in TARGET_STYLES:
            continue
        if row.get("作品著作権フラグ") != "なし" or row.get("人物著作権フラグ") != "なし":
            continue
        if not row.get("テキストファイルURL"):
            continue
        bid = row.get("作品ID", "")
        if bid in seen:
            continue
        seen.add(bid)
        books.append(row)
        if len(books) >= max_books:
            break
    print(f"[catalog] 対象 {len(books)} 作品")
    return books


def fetch_text(book: dict) -> str | None:
    url = book.get("テキストファイルURL", "")
    try:
        resp = requests.get(url, timeout=30)
        resp.raise_for_status()
        with zipfile.ZipFile(io.BytesIO(resp.content)) as zf:
            name = next((n for n in zf.namelist() if n.endswith(".txt")), None)
            if not name:
                return None
            raw = zf.read(name).decode("shift_jis", errors="replace")
        raw = HEADER_FOOTER.sub("", raw)
        return NOTE.sub("", raw)
    except Exception as e:  # noqa: BLE001
        print(f"  [warn] 取得失敗: {book.get('作品名')} — {e}")
        return None


def count_ruby(text: str, counts: dict[str, Counter]) -> int:
    """本文からルビを拾ひ、読み → 表記 の度数を数へる。戻り値は拾つた数。"""
    n = 0
    for m in RUBY.finditer(text):
        surface = m.group(1) or m.group(3)
        yomi = m.group(2) or m.group(4)
        if not surface or not yomi:
            continue
        counts[yomi][surface] += 1
        n += 1
    return n


def load_existing(path: str) -> list[tuple[str, str, int, str]]:
    """既存の TSV を読む（付属語の引き継ぎに使ふ）。無ければ空。"""
    rows: list[tuple[str, str, int, str]] = []
    if not os.path.exists(path):
        return rows
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue
            parts = line.split("\t")
            if len(parts) != 4:
                continue
            yomi, surface, freq, pos = parts
            rows.append((yomi, surface, int(freq), pos))
    return rows


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--books", type=int, default=60)
    ap.add_argument("--min-count", type=int, default=2,
                    help="この度数に満たない対は捨てる（誤ルビ・一回きりの当て字を除く）")
    ap.add_argument("--top", type=int, default=8, help="読みごとに残す表記の数")
    ap.add_argument("--no-keep-fuzoku", action="store_true",
                    help="既存 TSV の付属語を引き継がない（既定は引き継ぐ）")
    args = ap.parse_args()

    counts: dict[str, Counter] = defaultdict(Counter)
    fetched = harvested = 0
    for book in load_catalog(args.books):
        text = fetch_text(book)
        if not text:
            continue
        fetched += 1
        n = count_ruby(text, counts)
        harvested += n
        print(f"  [{fetched}] {book.get('作品名')}（ルビ {n:,}）")
        time.sleep(SLEEP_SEC)

    rows: list[tuple[str, str, int, str]] = []
    for yomi, surfaces in counts.items():
        for surface, freq in surfaces.most_common(args.top):
            if freq < args.min_count:
                continue
            rows.append((yomi, surface, freq, JIRITSU))

    if not args.no_keep_fuzoku:
        kept = [r for r in load_existing(OUT) if r[3] == FUZOKU]
        print(f"[keep] 付属語 {len(kept)} 語を既存の TSV から引き継ぐ（ルビからは取れない）")
        rows.extend(kept)

    if not rows:
        raise SystemExit("一語も収穫できなかつた（空の辞書を書くとゲートが沈黙して死ぬ）")

    # 焼き付け側に正規化と書き出しを任せる（並べ替への規則を二か所に置かない）
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import gen_jisho_tables  # noqa: E402

    gen_jisho_tables.write_tsv(rows, source="aozora", books=fetched)
    print(f"[out] {os.path.relpath(OUT, ROOT)}"
          f"（{len(rows)} 語・{fetched} 作品・ルビ {harvested:,}）")
    gen_jisho_tables.main()


if __name__ == "__main__":
    main()
