"""
test_kyuji.py — app/kyuji.py（新字體→舊字體の決定的トランスデューサ）のテスト。
依存ゼロ・LLM不使用。

実行:
    python eval/test_kyuji.py          # 直接実行（成功で exit 0）
    python -m pytest eval/             # pytest でも可
"""
import os
import sys

_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, _ROOT)
sys.path.insert(0, os.path.join(_ROOT, "ssot"))

from kyuji import (  # noqa: E402
    KYUJI_MAP, AMBIGUOUS_SHINJI, POINT_MARKER,
    KyujiStream, kyuji_stream, to_kyuji, to_kyuji_body,
)


def _r1(text):
    """新字体の残留を数へる（旧リポのリンタ R1 と同値の判定を、依存なしで行ふ）。

    KYUJI_MAP の鍵＝変換対象の新字体なので、変換後にそれが残つてゐれば違反。
    """
    return [c for c in text if c in KYUJI_MAP]


def run():
    # ── 基本の 1:1 置換 ─────────────────────────────
    assert to_kyuji("国の学問") == "國の學問"
    assert to_kyuji("國の學問") == "國の學問"          # 冪等
    assert to_kyuji("山の花") == "山の花"              # 新旧同形は不変
    assert to_kyuji("") == ""

    # ── 一対多の危険字は触らない（弁/予/余 等）＝両集合は排他 ──────
    assert not (AMBIGUOUS_SHINJI & set(KYUJI_MAP)), "曖昧字が写像に混入"
    for ch in AMBIGUOUS_SHINJI:
        assert to_kyuji(ch) == ch, ch

    # ── 変換後は linter の R1（新字体）が消えること＝反射層の目的 ──
    dirty = "これは国の学問の発展に関する実験です。"
    assert _r1(dirty)                                   # 変換前は違反あり
    assert not _r1(to_kyuji(dirty))                     # 変換後はゼロ

    # ── 【ポイント】以降は素通し（新字体の*言及*を壊さない） ──
    body = f"國の學問なり。{POINT_MARKER}①国→國 ②学→學"
    out = to_kyuji_body(body)
    assert out.endswith("①国→國 ②学→學"), out          # 解説は無変換
    assert "國の學問なり。" in out

    # ── ストリーミング: 分割位置に依らず一括変換と一致 ──────
    full = f"これは国の学問です。{POINT_MARKER}①国→國"
    expect = to_kyuji_body(full)
    for size in (1, 2, 3, 5, 7, 13):
        chunks = [full[i:i + size] for i in range(0, len(full), size)]
        assert "".join(kyuji_stream(chunks)) == expect, size

    # ── マーカーがトークン境界に跨いでも検出できる ──────────
    st = KyujiStream()
    got = st.feed("学【ポイ") + st.feed("ント】学") + st.flush()
    assert got == "學【ポイント】学", got               # 前は変換・後は素通し

    # ── マーカー接頭辞で終わる普通の文が保留のまま消えない ──
    st2 = KyujiStream()
    got2 = st2.feed("國なり【ポイ") + st2.flush()
    assert got2 == "國なり【ポイ", got2

    # ── 空トークンを yield しない（NDJSON の無駄行を防ぐ） ──
    assert all(t for t in kyuji_stream(["", "国", "", "学"]))
    assert "".join(kyuji_stream(["", "国", "", "学"])) == "國學"

    print(f"✅ kyuji 全テスト成功（{len(KYUJI_MAP)}字・ストリーム安全・解説素通し）")
    return 0


if __name__ == "__main__":
    raise SystemExit(run())
