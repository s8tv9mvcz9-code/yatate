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
