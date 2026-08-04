#!/usr/bin/env python3
"""
check_artifacts.py — 焼き付け成果物の目録を検査する（**通信なし**）。

    python3 scripts/check_artifacts.py            # 検査（CI が回す）
    python3 scripts/check_artifacts.py --update   # 目録を書き直す（収穫した人が回す）

## なぜハッシュの目録が要るのか（docs/ime/artifacts.md §3）

焼き付け成果物には二種類ある。

| | 例 | git diff | 守り方 |
|---|---|---|---|
| テキスト | `jisho.tsv`・`kana_bigram.txt` | **読める** | 差分を人が見る ＋ 目録 |
| 非テキスト・大型 | 埋め込み・索引 | 読めない（全行が変はる） | **目録だけ** |

テキストの成果物は差分が読めるので、そちらを第一とする。だが差分が読めることと
**壊れてゐないこと**は別である。生成物の鮮度ゲート（再生成して `git diff --exit-code`）は
「元を変へたのに写しが古い」を捕まへるが、**元そのもの**（`kana_bigram.txt` のやうな
収穫の成果物）は CI で再生成できない——通信するからである。そこには今まで何の関門も
無かつた。目録はその穴を塞ぐ。

そして埋め込みが入つたとき、**同じ関門がそのまま効く**。バイナリの差分は人には
読めないが、ハッシュの一行なら読める。「埋め込みモデルを差し替へた」は目録の
一行の変化として現れ、レビューで見える（docs/ime/artifacts.md §5）。

## 目録に無い成果物は撥ねる

新しい成果物が関門を持たずに紛れ込むのを防ぐため、`core/data/` の下にあつて
目録に載つてゐないファイルは**失敗**にする。「気をつける」で守らない
——このリポジトリが繰り返し学んできたことである。
"""
import argparse
import hashlib
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MANIFEST = os.path.join(ROOT, "core", "data", "MANIFEST.sha256")

# 目録が覆ふ範囲。ここを広げるときは docs/ime/artifacts.md も直すこと。
WATCHED_DIRS = [os.path.join("core", "data")]

HEADER = """\
# 焼き付け成果物の目録 — scripts/check_artifacts.py が縛る
#
# 形式: <sha256>  <リポジトリ相対path>
#
# 収穫（通信あり・オフラインで一回）した人が --update で書き直し、
# CI は検査だけを回す。目録に無いファイルが core/data/ に居ると失敗する
# ——関門を持たない成果物を作らせないためである（docs/ime/artifacts.md §3）。
#
# 埋め込み・索引のやうな非テキストの成果物が入つたときも、守り方はこの一枚で足りる。
# バイナリの差分は読めないが、ハッシュの一行なら読める。
"""


def sha256_of(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 16), b""):
            h.update(chunk)
    return h.hexdigest()


def watched_files() -> list[str]:
    """目録が覆ふべきファイル（リポジトリ相対・昇順）。目録自身は除く。"""
    out = []
    for d in WATCHED_DIRS:
        base = os.path.join(ROOT, d)
        for dirpath, _, names in os.walk(base):
            for name in names:
                full = os.path.join(dirpath, name)
                rel = os.path.relpath(full, ROOT)
                if os.path.abspath(full) == os.path.abspath(MANIFEST):
                    continue
                out.append(rel)
    return sorted(out)


def load_manifest(path: str = MANIFEST) -> dict[str, str]:
    entries: dict[str, str] = {}
    if not os.path.exists(path):
        return entries
    with open(path, encoding="utf-8") as f:
        for lineno, line in enumerate(f, 1):
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue
            digest, _, rel = line.partition("  ")
            if not digest or not rel:
                raise SystemExit(f"{path}:{lineno} 形式が「<sha256>  <path>」でない: {line!r}")
            entries[rel] = digest
    return entries


def write_manifest(path: str = MANIFEST) -> dict[str, str]:
    files = watched_files()
    if not files:
        raise SystemExit("覆ふべき成果物が一つも無い（空の目録はゲートが沈黙して死ぬ）")
    entries = {rel: sha256_of(os.path.join(ROOT, rel)) for rel in files}
    with open(path, "w", encoding="utf-8") as f:
        f.write(HEADER)
        for rel in files:
            f.write(f"{entries[rel]}  {rel}\n")
    return entries


def check() -> int:
    entries = load_manifest()
    if not entries:
        print("[NG] 目録が空か存在しない（--update で作ること）", file=sys.stderr)
        return 1

    bad = 0
    listed = set(entries)
    present = set(watched_files())

    for rel in sorted(listed):
        full = os.path.join(ROOT, rel)
        if not os.path.exists(full):
            print(f"[NG] 目録にあるが実体が無い: {rel}", file=sys.stderr)
            bad += 1
            continue
        got = sha256_of(full)
        if got != entries[rel]:
            print(
                f"[NG] 中身が目録と違ふ: {rel}\n"
                f"     目録 {entries[rel]}\n"
                f"     実体 {got}\n"
                f"     収穫し直したなら `python3 scripts/check_artifacts.py --update`",
                file=sys.stderr,
            )
            bad += 1

    for rel in sorted(present - listed):
        print(
            f"[NG] 目録に無い成果物: {rel}\n"
            f"     関門を持たない成果物は置かない（--update で目録へ入れること）",
            file=sys.stderr,
        )
        bad += 1

    if bad:
        print(f"[NG] {bad} 件", file=sys.stderr)
        return 1
    print(f"[OK] 焼き付け成果物 {len(entries)} 件が目録と一致する")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--update", action="store_true", help="目録を書き直す")
    args = ap.parse_args()
    if args.update:
        entries = write_manifest()
        print(f"wrote {os.path.relpath(MANIFEST, ROOT)}（{len(entries)} 件）")
        return 0
    return check()


if __name__ == "__main__":
    sys.exit(main())
