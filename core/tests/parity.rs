//! 黄金ベクトル（parity vectors）— Python SSOT と Rust 核の一致を機械で検査する。
//!
//! ベクトルは `scripts/gen_parity_vectors.py` が **`ssot/kyuji.py` を実行して**書き出す。
//! ここに期待値は一行も書かない（書いた瞬間に古くなるため。
//! `eval/test_client_parity.py` が学んだ規律の横展開 — docs/ime/cross-platform.md §6）。
//!
//! 将来 Swift / Kotlin / C++ の束縛が増えたら、**同じファイルを同じやうに流す**。

use std::fs;
use std::path::PathBuf;

use serde_json::Value;
use yatate_core::kyuji::{is_ambiguous, kyuji_of};
use yatate_core::{kyuji_stream, to_kyuji, to_kyuji_body, POINT_MARKER};

fn vectors() -> Value {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "vectors", "kyuji.json"]
        .iter()
        .collect();
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "黄金ベクトルが読めない（{}）: {e}\n\
             `python3 scripts/gen_parity_vectors.py` で生成すること",
            path.display()
        )
    });
    serde_json::from_str(&raw).expect("黄金ベクトルが JSON として壊れてゐる")
}

fn cases(v: &Value, key: &str) -> Vec<(String, String)> {
    let arr = v[key]
        .as_array()
        .unwrap_or_else(|| panic!("ベクトルに {key} が無い"));
    // 空集合は「何でも通る」ゲートになる＝沈黙して死ぬので、非空を要求する
    assert!(!arr.is_empty(), "{key} が空（ゲートが無効化されてゐる）");
    arr.iter()
        .map(|c| {
            (
                c["in"].as_str().expect("in").to_string(),
                c["out"].as_str().expect("out").to_string(),
            )
        })
        .collect()
}

#[test]
fn 写像の全字が一致する() {
    let v = vectors();
    let map = cases(&v, "map");
    assert_eq!(
        map.len() as u64,
        v["map_size"].as_u64().expect("map_size"),
        "ベクトル自身の整合が壊れてゐる"
    );
    for (shinji, kyuji) in map {
        let c = shinji.chars().next().unwrap();
        assert_eq!(
            kyuji_of(c).map(|k| k.to_string()),
            Some(kyuji.clone()),
            "{shinji} → {kyuji} が写しに無い（gen_rust_tables.py の再実行を忘れてゐる可能性）"
        );
        assert_eq!(to_kyuji(&shinji), kyuji);
    }
}

#[test]
fn 曖昧字は写像から除かれてゐる() {
    let v = vectors();
    let ambiguous = v["ambiguous"].as_array().expect("ambiguous");
    assert!(!ambiguous.is_empty(), "ambiguous が空");
    for a in ambiguous {
        let c = a.as_str().unwrap().chars().next().unwrap();
        assert!(is_ambiguous(c), "{c} が曖昧字集合に無い");
        assert!(kyuji_of(c).is_none(), "曖昧字 {c} が写像に混入してゐる");
    }
}

#[test]
fn 解説マーカーが一致する() {
    let v = vectors();
    assert_eq!(
        v["point_marker"].as_str().expect("point_marker"),
        POINT_MARKER
    );
}

#[test]
fn to_kyuji_が一致する() {
    let v = vectors();
    for (input, expect) in cases(&v, "to_kyuji") {
        assert_eq!(to_kyuji(&input), expect, "to_kyuji({input:?})");
    }
}

#[test]
fn to_kyuji_body_が一致する() {
    let v = vectors();
    for (input, expect) in cases(&v, "to_kyuji_body") {
        assert_eq!(to_kyuji_body(&input), expect, "to_kyuji_body({input:?})");
    }
}

#[test]
fn ストリームが分割位置に依らない() {
    let v = vectors();
    let arr = v["stream"].as_array().expect("stream");
    assert!(!arr.is_empty(), "stream が空（ゲートが無効化されてゐる）");
    for case in arr {
        let chunks: Vec<String> = case["chunks"]
            .as_array()
            .expect("chunks")
            .iter()
            .map(|c| c.as_str().expect("chunk").to_string())
            .collect();
        let expect = case["out"].as_str().expect("out");
        let got = kyuji_stream(&chunks).concat();
        assert_eq!(got, expect, "kyuji_stream({chunks:?})");

        // 分割に依らず「完成文の変換」と一致する、が不変条件
        let body = case["equals_body"].as_str().expect("equals_body");
        assert_eq!(got, body, "分割の仕方で結果が変はつてゐる: {chunks:?}");

        // 空トークンを流さない（NDJSON の無駄行を防ぐ規約）
        assert!(
            kyuji_stream(&chunks).iter().all(|s| !s.is_empty()),
            "空トークンを流してゐる: {chunks:?}"
        );
    }
}

// ── ロジック側のベクトル（gojuon / kehai）────────────────────────
// こちらは**核が SSOT**で、ファイルは核が書いたもの（`cargo run --bin gen-vectors`）。
// ここでの検査は「committed のベクトルが古くなつてゐないか」を捕まへる番人である
// （Swift・Kotlin の殻はこの同じファイルに従ふので、古いまま配ると全員が古くなる）。

fn load(name: &str) -> Value {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "vectors", name]
        .iter()
        .collect();
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "ベクトルが読めない（{}）: {e}\n`cargo run --bin gen-vectors` で生成すること",
            path.display()
        )
    });
    serde_json::from_str(&raw).expect("ベクトルが JSON として壊れてゐる")
}

#[test]
fn 五十音のベクトルが核と一致する() {
    use yatate_core::gojuon::{self, Deflect};

    let v = load("gojuon.json");
    let kana = v["kana"].as_array().expect("kana");
    assert_eq!(kana.len(), 150, "10 行 × 5 段 × 3 面 が揃つてゐない");

    for case in kana {
        let gyo = case["gyo"].as_str().expect("gyo");
        let dan = case["dan"].as_u64().expect("dan") as usize;
        let deflect = match case["deflect"].as_str().expect("deflect") {
            "none" => Deflect::None,
            "daku" => Deflect::Daku,
            "ko" => Deflect::Ko,
            other => panic!("未知の逸らし: {other}"),
        };
        let got = gojuon::gyo_named(gyo).expect("行が無い").kana(dan, deflect);
        let want = case["kana"].as_str();
        assert_eq!(got, want, "{gyo} {dan} 段 {:?}", case["deflect"]);
    }

    for case in v["reverse"].as_array().expect("reverse") {
        let c = case["kana"].as_str().unwrap().chars().next().unwrap();
        match gojuon::reverse_lookup(c) {
            Some((gyo, dan)) => {
                assert_eq!(Some(gyo), case["gyo"].as_str(), "{c} の逆引き行");
                assert_eq!(
                    dan as u64,
                    case["dan"].as_u64().expect("dan"),
                    "{c} の逆引き段"
                );
            }
            None => assert!(case["gyo"].is_null(), "{c} は体系外のはず"),
        }
    }
}

#[test]
fn 氣配のベクトルが核と一致する() {
    use yatate_core::kehai::{self, KeyId, MIN_EVIDENCE};

    let v = load("kehai.json");
    assert_eq!(v["min_evidence"].as_u64(), Some(u64::from(MIN_EVIDENCE)));

    let cases = v["cases"].as_array().expect("cases");
    assert!(!cases.is_empty(), "cases が空（ゲートが無効化されてゐる）");

    for case in cases {
        let prev = case["prev"].as_str().map(|s| s.chars().next().unwrap());
        let f = kehai::field(prev);
        assert_eq!(
            f.is_empty(),
            case["empty"].as_bool().expect("empty"),
            "{prev:?}"
        );

        let peak = case["peak"].as_object().map(|o| {
            let id = o["id"].as_str().unwrap();
            match o["kind"].as_str().unwrap() {
                "gyo" => KeyId::Gyo(yatate_core::gojuon::gyo_named(id).expect("行が無い").name),
                _ => KeyId::Moji(id.chars().next().unwrap()),
            }
        });
        assert_eq!(f.peak, peak, "{prev:?} の峰");

        for ink in case["ink"].as_array().expect("ink") {
            let id = ink["key"]["id"].as_str().unwrap();
            let key = match ink["key"]["kind"].as_str().unwrap() {
                "gyo" => KeyId::Gyo(yatate_core::gojuon::gyo_named(id).expect("行").name),
                _ => KeyId::Moji(id.chars().next().unwrap()),
            };
            let want = ink["ink"].as_f64().expect("ink 値");
            assert!(
                (f.ink_of(&key) - want).abs() < 1e-6,
                "{prev:?} の {id} の墨: 核 {} ≠ ベクトル {want}",
                f.ink_of(&key)
            );
        }
    }
}

/// **golden query セットによる検索品質の回帰検査**（issue #4）。
///
/// 辞書（索引）が決定的なので、$0 の ubuntu ランナーで品質の物差しが持てる。
/// ここが捕まへるのは単体試験では見えない回帰である——
/// 「辞書に一語足したら無関係な文の分割が変はつた」「費用の重みを触つたら
/// 候補順が入れ替はつた」。どちらも黙つて起きるので、ベクトルに焼いて縛る。
#[test]
fn 文節のベクトルが核と一致する() {
    use yatate_core::bunsetsu;
    use yatate_core::jisho;

    let v = load("bunsetsu.json");
    assert_eq!(
        v["jisho_entries"].as_u64(),
        Some(jisho::ENTRIES as u64),
        "辞書の語数がベクトルと食ひ違ふ（gen_jisho_tables.py の再実行を忘れてゐる可能性）"
    );
    assert_eq!(
        v["max_yomi_chars"].as_u64(),
        Some(jisho::MAX_YOMI_CHARS as u64)
    );

    let queries = v["queries"].as_array().expect("queries");
    assert!(
        !queries.is_empty(),
        "queries が空（ゲートが無効化されてゐる）"
    );

    for case in queries {
        let q = case["query"].as_str().expect("query");
        let segs = bunsetsu::segment(q);

        // ① 読みの被覆 — 変換の中核の不変条件
        assert_eq!(
            bunsetsu::coverage_error(q, &segs),
            None,
            "{q:?} で読みが失はれてゐる"
        );

        // ② 区切り
        let want_yomi: Vec<&str> = case["yomi_list"]
            .as_array()
            .expect("yomi_list")
            .iter()
            .map(|y| y.as_str().expect("yomi"))
            .collect();
        assert_eq!(bunsetsu::yomi_list(&segs), want_yomi, "{q:?} の区切り");

        // ③ 合成と確定（旧字は確定のときに定まる）
        assert_eq!(
            bunsetsu::compose(&segs),
            case["compose"].as_str().expect("compose"),
            "{q:?} の合成"
        );
        assert_eq!(
            bunsetsu::commit(&segs),
            case["commit"].as_str().expect("commit"),
            "{q:?} の確定（旧字）"
        );

        // ④ 候補の中身と並び — 「品質」の実体はここである
        let want_segs = case["segments"].as_array().expect("segments");
        assert_eq!(want_segs.len(), segs.len(), "{q:?} の文節数");
        for (seg, want) in segs.iter().zip(want_segs) {
            assert_eq!(
                seg.chosen as u64,
                want["chosen"].as_u64().expect("chosen"),
                "{q:?} / {} の既定選択",
                seg.yomi
            );
            let want_cands = want["candidates"].as_array().expect("candidates");
            assert!(!want_cands.is_empty(), "候補が空の文節を作つてゐる");
            assert_eq!(
                seg.candidates.len(),
                want_cands.len(),
                "{q:?} / {} の候補数",
                seg.yomi
            );
            for (got, want) in seg.candidates.iter().zip(want_cands) {
                assert_eq!(
                    got.surface,
                    want["surface"].as_str().expect("surface"),
                    "{q:?} / {} の候補の並び",
                    seg.yomi
                );
                assert_eq!(
                    got.cost as i64,
                    want["cost"].as_i64().expect("cost"),
                    "{q:?} / {} の {} の費用",
                    seg.yomi,
                    got.surface
                );
                assert_eq!(
                    got.in_jisho,
                    want["in_jisho"].as_bool().expect("in_jisho"),
                    "{q:?} / {} の {} の出所",
                    seg.yomi,
                    got.surface
                );
            }
        }
    }
}

/// 区切り修正の往復 — 繋げても割つても**読みは失はれない**。
#[test]
fn 区切り修正のベクトルが核と一致する() {
    use yatate_core::bunsetsu;

    let v = load("bunsetsu.json");
    let edits = v["edits"].as_array().expect("edits");
    assert!(!edits.is_empty(), "edits が空（ゲートが無効化されてゐる）");

    for case in edits {
        let q = case["query"].as_str().expect("query");
        let op = case["op"].as_str().expect("op");
        let at = case["at"].as_u64().expect("at") as usize;

        let mut segs = bunsetsu::segment(q);
        let ok = match op {
            "merge" => bunsetsu::merge(&mut segs, at),
            "split" => bunsetsu::split(&mut segs, 0, at),
            other => panic!("未知の区切り修正: {other}"),
        };
        assert!(ok, "{q:?} の {op}({at}) が失敗した");

        assert_eq!(
            bunsetsu::coverage_error(q, &segs),
            None,
            "{op} で読みが失はれた"
        );
        let want_yomi: Vec<&str> = case["yomi_list"]
            .as_array()
            .expect("yomi_list")
            .iter()
            .map(|y| y.as_str().expect("yomi"))
            .collect();
        assert_eq!(bunsetsu::yomi_list(&segs), want_yomi, "{q:?} {op}({at})");
        assert_eq!(
            bunsetsu::compose(&segs),
            case["compose"].as_str().expect("compose"),
            "{q:?} {op}({at}) の合成"
        );
    }
}

/// 鍵の物理位置 — **web と Windows の二つの殻が従ふ一枚の地図**。
///
/// このベクトルが古くなると、二つの殻が違ふ位置を指しはじめる。
/// とくに配列で意味の変はる三鍵（さ・し・前置シフト）は、ずれても
/// 例外も警告も出ずに黙つて違ふ字が出るので、ここで名指しして縛る。
#[test]
fn 鍵の位置のベクトルが核と一致する() {
    use yatate_core::kagi;

    let v = load("kagi.json");
    let keys = v["keys"].as_array().expect("keys");
    assert!(!keys.is_empty(), "keys が空（ゲートが無効化されてゐる）");
    assert_eq!(keys.len(), kagi::KAGI.len(), "鍵の数");

    for case in keys {
        let genki = case["genki"]
            .as_str()
            .expect("genki")
            .chars()
            .next()
            .unwrap();
        let code = case["code"].as_str().expect("code");
        let scan = case["scan"].as_u64().expect("scan") as u16;

        assert_eq!(kagi::genki_of_code(code), Some(genki), "{code} → 原器");
        assert_eq!(kagi::genki_of_scan(scan), Some(genki), "{scan:#04X} → 原器");
        assert_eq!(kagi::code_of(genki), Some(code), "{genki} → code");
        assert_eq!(kagi::scan_of(genki), Some(scan), "{genki} → 走査符号");
    }
}

#[test]
fn 原器のベクトルが核と一致する() {
    use yatate_core::genki::{self, type_keys, Genki};

    let v = load("genki.json");
    assert_eq!(v["shift"].as_str(), Some(genki::SHIFT.to_string().as_str()));
    assert_eq!(
        v["dakuten"].as_str(),
        Some(genki::DAKUTEN.to_string().as_str())
    );

    for (name, plane) in [
        ("first_plane", &genki::FIRST_PLANE[..]),
        ("second_plane", &genki::SECOND_PLANE[..]),
    ] {
        let arr = v[name]
            .as_array()
            .unwrap_or_else(|| panic!("{name} が無い"));
        assert_eq!(arr.len(), plane.len(), "{name} の鍵数");
        for case in arr {
            let key = case["key"].as_str().unwrap().chars().next().unwrap();
            let want = case["kana"].as_str().unwrap();
            let mut g = Genki::new();
            if name == "second_plane" {
                g.press(genki::SHIFT, None);
            }
            match g.press(key, None) {
                yatate_core::genki::Edit::Insert(got) => {
                    assert_eq!(got, want, "{name} の {key}")
                }
                other => panic!("{name} の {key} が仮名を出さない: {other:?}"),
            }
        }
    }

    let seqs = v["sequences"].as_array().expect("sequences");
    assert!(
        !seqs.is_empty(),
        "sequences が空（ゲートが無効化されてゐる）"
    );
    for case in seqs {
        let keys = case["keys"].as_str().expect("keys");
        let want = case["kana"].as_str().expect("kana");
        assert_eq!(type_keys(keys), want, "打鍵列 {keys:?}");
    }
}
