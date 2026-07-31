#!/usr/bin/env python3
"""
gen_parity_vectors.py — 黄金ベクトル（parity vectors）を SSOT から生成する。

生成物: core/vectors/kyuji.json
SSOT:   ssot/kyuji.py（to_kyuji / to_kyuji_body / KyujiStream を**実際に呼ぶ**）

なぜ在るか（docs/ime/cross-platform.md §6）:
    殻が OS ごとに増えると、核の写しも増える。「気をつける」では同期は保てないので、
    **入力→期待出力の対を SSOT から生成し、全ての束縛で流す**。
    Rust は `cargo test`、Swift は `swift test`、将来の Kotlin / C++ も同じ表を食ふ。

規律（`eval/test_client_parity.py` と同じ）:
    **期待値を手で書かない。** 書いた瞬間に古くなり、ゲートが嘘をつき始める。
    ここでは Python SSOT を実行した結果をそのまま期待値として書き出す。
"""
import json
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "ssot"))
from kyuji import (  # noqa: E402
    AMBIGUOUS_SHINJI, KYUJI_MAP, POINT_MARKER, KyujiStream, kyuji_stream,
    to_kyuji, to_kyuji_body,
)

OUT = os.path.join(ROOT, "core", "vectors", "kyuji.json")

# 変換の素材。表の全字は機械的に足すので、ここには**境界と実文**だけ置く。
BODY_CASES = [
    "",
    "国の学問",
    "國の學問",                                   # 冪等
    "山の花",                                     # 新旧同形は不変
    "これは国の学問の発展に関する実験です。",
    "けふは麗しき天氣なり。",
    "弁予余欠芸",                                 # 曖昧字は触らない
    f"國の學問なり。{POINT_MARKER}①国→國 ②学→學",  # 解説は素通し
    f"{POINT_MARKER}国",                          # 冒頭が解説
    "國なり【ポイ",                               # マーカーの接頭辞で終はる
]

# ストリーミング: 分割位置に依らず to_kyuji_body と一致することを検査する。
STREAM_TEXTS = [
    f"これは国の学問です。{POINT_MARKER}①国→國",
    "国" * 8,
    f"学{POINT_MARKER}学",
]
STREAM_SIZES = [1, 2, 3, 5, 7, 13]

# 明示的なトークン列（境界の跨ぎ・空トークン）。
STREAM_CHUNKS = [
    ["学【ポイ", "ント】学"],
    ["", "国", "", "学"],
    ["國なり【ポイ"],
    ["【ポイント】", "国"],
]


def _chunked(text: str, size: int):
    return [text[i:i + size] for i in range(0, len(text), size)]


def main() -> None:
    vectors = {
        "_comment": (
            "自動生成（scripts/gen_parity_vectors.py）— 手で編集しないこと。"
            "期待値は ssot/kyuji.py を実行した結果である。"
        ),
        "source": "ssot/kyuji.py",
        "point_marker": POINT_MARKER,
        "map_size": len(KYUJI_MAP),
        "ambiguous": sorted(AMBIGUOUS_SHINJI),
        # 表の全字（1 字ずつ）— 写しが 1 字でも欠ければここで落ちる
        "map": [{"in": k, "out": v} for k, v in sorted(KYUJI_MAP.items())],
        "to_kyuji": [{"in": s, "out": to_kyuji(s)} for s in BODY_CASES],
        "to_kyuji_body": [{"in": s, "out": to_kyuji_body(s)} for s in BODY_CASES],
        "stream": [],
    }

    for text in STREAM_TEXTS:
        for size in STREAM_SIZES:
            chunks = _chunked(text, size)
            vectors["stream"].append({
                "chunks": chunks,
                "out": "".join(kyuji_stream(chunks)),
                # 分割に依らず完成文の変換と一致する、が不変条件
                "equals_body": to_kyuji_body(text),
            })

    for chunks in STREAM_CHUNKS:
        st = KyujiStream()
        pieces = [st.feed(c) for c in chunks] + [st.flush()]
        vectors["stream"].append({
            "chunks": chunks,
            "out": "".join(pieces),
            "equals_body": to_kyuji_body("".join(chunks)),
        })

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as f:
        json.dump(vectors, f, ensure_ascii=False, indent=1, sort_keys=False)
        f.write("\n")
    print("wrote %s (map=%d, to_kyuji=%d, body=%d, stream=%d)"
          % (OUT, len(vectors["map"]), len(vectors["to_kyuji"]),
             len(vectors["to_kyuji_body"]), len(vectors["stream"])))


if __name__ == "__main__":
    main()
