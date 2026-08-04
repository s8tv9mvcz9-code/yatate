#!/usr/bin/env python3
"""
check_wasm.py — 組み上がつた wasm を検める（**通信なし・依存ゼロ**）。

Windows 殻の CI が実 DLL のエクスポート表を検べてゐるのと同じ役回りである
（「読み込まれない DLL」を防ぐ関門）。web では二つを見る:

1. **出口が揃つてゐるか** — JS が呼ぶ関数が本当に居るか。
   `#[no_mangle]` を書き忘れると、頁は「関数が undefined」で黙つて死ぬ。

2. **入口が一つも無いか** — これが本命である。
   矢立は「通信するコードを持たない」を公開の約束にしてゐる（docs/privacy.md）。
   wasm に import section が無いといふことは、**このモジュールは外の世界の関数を
   一つも呼べない**といふことで、通信も保存も出来ない。約束が作文ではなく
   バイナリの性質として立つ。ここが破れたら（依存を足して std の I/O が入る等）、
   この関門が落ちる。

wasm の節（section）を素朴に読むだけなので、外部の道具は要らない。
"""
import sys

EXPECTED = [
    "yatate_new", "yatate_free", "yatate_alloc", "yatate_dealloc",
    "yatate_out_ptr", "yatate_out_len",
    "yatate_press", "yatate_backspace", "yatate_cancel",
    "yatate_preedit", "yatate_is_shifted", "yatate_commit",
    "yatate_kyuji", "yatate_suggest", "yatate_convert",
    "yatate_layout", "yatate_kehai",
    "memory",
]

SECTION_IMPORT = 2
SECTION_EXPORT = 7


def uleb(data: bytes, i: int) -> tuple[int, int]:
    result = shift = 0
    while True:
        b = data[i]
        i += 1
        result |= (b & 0x7F) << shift
        if not b & 0x80:
            return result, i
        shift += 7


def sections(data: bytes) -> dict[int, tuple[int, int]]:
    if data[:4] != b"\0asm":
        raise SystemExit("wasm の magic が違ふ（組み上がつてゐない可能性）")
    out: dict[int, tuple[int, int]] = {}
    i = 8
    while i < len(data):
        sid = data[i]
        i += 1
        size, i = uleb(data, i)
        out.setdefault(sid, (i, size))
        i += size
    return out


def export_names(data: bytes, start: int) -> list[str]:
    count, i = uleb(data, start)
    names = []
    for _ in range(count):
        n, i = uleb(data, i)
        names.append(data[i:i + n].decode("utf-8"))
        i += n
        i += 1                    # kind
        _, i = uleb(data, i)      # index
    return names


def main(path: str) -> int:
    data = open(path, "rb").read()
    secs = sections(data)
    bad = 0

    # ① 出口
    if SECTION_EXPORT not in secs:
        print("[NG] export section が無い", file=sys.stderr)
        return 1
    names = set(export_names(data, secs[SECTION_EXPORT][0]))
    for want in EXPECTED:
        if want not in names:
            print(f"[NG] 出口が無い: {want}（#[no_mangle] の書き忘れ？）", file=sys.stderr)
            bad += 1

    # ② 入口 — 一つも無いことが約束の実体である
    if SECTION_IMPORT in secs:
        count, _ = uleb(data, secs[SECTION_IMPORT][0])
        print(
            f"[NG] import が {count} 件ある。\n"
            f"     矢立の wasm は外の世界の関数を一つも呼ばない（docs/privacy.md）。\n"
            f"     依存を足したか、std の I/O が紛れ込んでゐる。",
            file=sys.stderr,
        )
        bad += 1

    if bad:
        print(f"[NG] {bad} 件", file=sys.stderr)
        return 1
    print(f"[OK] 出口 {len(EXPECTED)} 件・import 無し（{len(data):,} バイト）")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1
                  else "target/wasm32-unknown-unknown/release/yatate_web.wasm"))
