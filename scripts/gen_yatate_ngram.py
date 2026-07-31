#!/usr/bin/env python3
"""
gen_yatate_ngram.py — 青空文庫の仮名連 bigram から「墨の気配」テーブルを機械生成する。

docs/ime/vla.md の A0。旧字旧仮名テキストの本文からルビ・注記を除いた後に残る
**ひらがなの連なり**（助詞・助動詞・送り仮名——文語の呼吸）を取り出し、
仮名 bigram を数へ、**言語中立のデータ**（core/data/kana_bigram.txt）に書き出す。
Swift と Rust の表はそこから scripts/gen_bigram_tables.py が起こす
（別々に収穫すると青空文庫のカタログの変化で二つの表が静かにずれるため——
docs/ime/cross-platform.md §6）。

- 行頭記号 `^` は仮名連の開始（作業帯が空のとき・句読点の直後の分布）。
- 遷移先には句読点 `、` `。` も含む（「なり」の後に文の終はる気配が見えるやうに）。
- 決定的・訓練なし。ネットワークは青空文庫のカタログと本文取得のみ
  （Azure には一切触れない）。

カタログ取得・本文クリーニングは scripts/build_index.py の load_catalog / fetch_text と
同じ規約（同スクリプトは import 時に Azure 資格情報を要求するため、取得部のみ複製。
build_index.py 側の規約を変へたらここも揃へること）。
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

CATALOG_URL = "https://www.aozora.gr.jp/index_pages/list_person_all_extended_utf8.zip"
TARGET_STYLES = {"旧字旧仮名"}          # 気配は旧仮名の呼吸に限る（旧字新仮名は混ぜない）
SLEEP_SEC = 0.5

AOZORA_NOISE = re.compile(
    r"(?:《[^》]*》|［＃[^］]*］|｜|〔[^〕]*〕)", re.UNICODE)
HEADER_FOOTER = re.compile(r"-{5,}.*?-{5,}", re.DOTALL)

# ひらがなの連なり（ゝゞー等の記号で区切る——繰り返し記号の展開はしない）
KANA_RUN = re.compile(r"[ぁ-ゖ]+")
START = "^"
PUNCT = "、。"

OUT = "core/data/kana_bigram.txt"


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
        raw = AOZORA_NOISE.sub("", raw)
        return raw
    except Exception as e:  # noqa: BLE001
        print(f"  [warn] 取得失敗: {book.get('作品名')} — {e}")
        return None


def count_bigrams(text: str, counts: dict[str, Counter]) -> None:
    for m in KANA_RUN.finditer(text):
        run = m.group()
        counts[START][run[0]] += 1
        for a, b in zip(run, run[1:]):
            counts[a][b] += 1
        # 連なりの直後の句読点への遷移（文の終はる気配）
        end = m.end()
        if end < len(text) and text[end] in PUNCT:
            counts[run[-1]][text[end]] += 1


def emit_data(counts: dict[str, Counter], books: int, chars: int,
              top: int, min_count: int) -> str:
    """言語中立のデータファイル本文（# 見出し ＋ 「前字>次字:度数,…」の行）。"""
    rows = []
    for prev in sorted(counts.keys()):
        items = [(k, v) for k, v in counts[prev].most_common(top) if v >= min_count]
        if not items:
            continue
        rows.append(prev + ">" + ",".join(f"{k}:{v}" for k, v in items))
    head = (
        "# 仮名連 bigram — 「墨の氣配」の元データ（言語中立）\n"
        "#\n"
        "# 青空文庫の旧字旧仮名作品から数へた仮名の連なり（docs/ime/vla.md A0）。\n"
        "# ここが**唯一の出所**で、Swift と Rust の表は scripts/gen_bigram_tables.py が\n"
        "# この一つのファイルから起こす（別々に収穫すると二つの表がずれる）。\n"
        "#\n"
        "# 形式: 「前字>次字:度数,…」度数降順。`^` は仮名連の開始、、。 は終端遷移。\n"
        f"# meta: books={books} chars={chars} top={top} min_count={min_count}\n"
    )
    return head + "\n".join(rows) + "\n"


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--books", type=int, default=24)
    ap.add_argument("--top", type=int, default=16, help="前字ごとに残す遷移数")
    ap.add_argument("--min-count", type=int, default=3)
    args = ap.parse_args()

    counts: dict[str, Counter] = defaultdict(Counter)
    total_chars = 0
    fetched = 0
    for book in load_catalog(args.books):
        text = fetch_text(book)
        if not text:
            continue
        fetched += 1
        total_chars += len(text)
        count_bigrams(text, counts)
        print(f"  [{fetched}] {book.get('作品名')}（{len(text):,} 字）")
        time.sleep(SLEEP_SEC)

    data = emit_data(counts, fetched, total_chars, args.top, args.min_count)
    with open(OUT, "w", encoding="utf-8") as f:
        f.write(data)
    n_rows = sum(1 for k in counts if counts[k])
    print(f"[out] {OUT}（前字 {n_rows} 種・{fetched} 作品・{total_chars:,} 字）")

    # 同じ収穫から Swift と Rust の表を起こす（ここを分けると二つの表がずれる）
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import gen_bigram_tables  # noqa: E402
    gen_bigram_tables.main()


if __name__ == "__main__":
    main()
