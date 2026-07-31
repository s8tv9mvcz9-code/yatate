//! 黄金ベクトルの書き出し（核 → `core/vectors/*.json`）。
//!
//!     cargo run --bin gen-vectors
//!
//! **役割の分かれ目**（`docs/ime/cross-platform.md` §6）:
//!
//! - **表**（旧字 248 字・仮名 bigram）の SSOT は Python 側にある。
//!   よつて `kyuji.json` は `scripts/gen_parity_vectors.py` が Python を実行して書く。
//! - **ロジック**（五十音の幾何・氣配の射影）の SSOT は**この核**である。
//!   よつて `gojuon.json` / `kehai.json` はここが書き、Swift・Kotlin・C++ の殻が
//!   それに従ふ（従はせるのは各言語のテスト。M5-b で Swift が最初の客になる）。
//!
//! JSON は手で組む（核の依存ゼロを守るため。出力は小さく形も決まつてゐる）。
//! 浮動小数は 6 桁で丸めて書く——言語ごとの既定書式の違ひでベクトルが揺れないやうに。

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use yatate_core::genki::{self, type_keys};
use yatate_core::gojuon::{self, Deflect};
use yatate_core::kehai::{self, KeyId, MIN_EVIDENCE};

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn num(v: f64) -> String {
    format!("{v:.6}")
}

fn key_json(k: &KeyId) -> String {
    match k {
        KeyId::Gyo(name) => format!("{{\"kind\":\"gyo\",\"id\":\"{}\"}}", esc(name)),
        KeyId::Moji(c) => format!("{{\"kind\":\"moji\",\"id\":\"{}\"}}", esc(&c.to_string())),
    }
}

/// 五十音の幾何 — 行×段×逸らしの全格子（10 行 × 5 段 × 3 面 = 150 通り）。
fn gojuon_vectors() -> String {
    let mut out = String::from(
        "{\n \"_comment\": \"自動生成（cargo run --bin gen-vectors）— 手で編集しないこと。\
核（core/src/gojuon.rs）がロジックの SSOT である。\",\n \"source\": \"core/src/gojuon.rs\",\n",
    );

    let names: Vec<String> = gojuon::all()
        .map(|g| format!("\"{}\"", esc(g.name)))
        .collect();
    let _ = write!(
        out,
        " \"reading_order\": [{}],\n \"kana\": [\n",
        names.join(", ")
    );

    let mut rows: Vec<String> = Vec::new();
    for g in gojuon::all() {
        for dan in 0..5 {
            for (label, d) in [
                ("none", Deflect::None),
                ("daku", Deflect::Daku),
                ("ko", Deflect::Ko),
            ] {
                let v = match g.kana(dan, d) {
                    Some(k) => format!("\"{}\"", esc(k)),
                    None => "null".to_string(),
                };
                rows.push(format!(
                    "  {{\"gyo\": \"{}\", \"dan\": {dan}, \"deflect\": \"{label}\", \"kana\": {v}}}",
                    esc(g.name)
                ));
            }
        }
    }
    out.push_str(&rows.join(",\n"));
    out.push_str("\n ],\n \"reverse\": [\n");

    // 逆引き（濁点・半濁点・小書きが基底の鍵へ畳まれること）
    let mut rev: Vec<String> = Vec::new();
    let mut seen: Vec<char> = Vec::new();
    for g in gojuon::all() {
        for dan in 0..5 {
            for d in [Deflect::None, Deflect::Daku, Deflect::Ko] {
                if let Some(kana) = g.kana(dan, d) {
                    let c = kana.chars().next().unwrap();
                    if seen.contains(&c) {
                        continue;
                    }
                    seen.push(c);
                    let (gyo, dd) = gojuon::reverse_lookup(c).expect("逆引きできない仮名");
                    rev.push(format!(
                        "  {{\"kana\": \"{}\", \"gyo\": \"{}\", \"dan\": {dd}}}",
                        esc(&c.to_string()),
                        esc(gyo)
                    ));
                }
            }
        }
    }
    // 体系外（逆引きできないもの）も明示的に載せる
    for c in ['ん', '、', '。'] {
        assert!(gojuon::reverse_lookup(c).is_none());
        rev.push(format!(
            "  {{\"kana\": \"{}\", \"gyo\": null, \"dan\": null}}",
            esc(&c.to_string())
        ));
    }
    out.push_str(&rev.join(",\n"));
    out.push_str("\n ]\n}\n");
    out
}

/// 墨の氣配 — 代表的な前字に対する場（鍵の墨・段の墨・峰）。
fn kehai_vectors() -> String {
    let mut out = String::from(
        "{\n \"_comment\": \"自動生成（cargo run --bin gen-vectors）— 手で編集しないこと。\
核（core/src/kehai.rs）がロジックの SSOT である。\",\n \"source\": \"core/src/kehai.rs\",\n",
    );
    let _ = write!(out, " \"min_evidence\": {MIN_EVIDENCE},\n \"cases\": [\n");

    // 開始（^）・高頻度の仮名・体系外の字・表に無い字、を代表として並べる
    let probes: Vec<Option<char>> = vec![
        None,
        Some('か'),
        Some('い'),
        Some('な'),
        Some('り'),
        Some('つ'),
        Some('ゐ'),
        Some('が'),
        Some('ん'),
        Some('。'),
        Some('々'),
        Some('A'),
    ];

    let mut cases: Vec<String> = Vec::new();
    for p in probes {
        let f = kehai::field(p);
        let prev = match p {
            Some(c) => format!("\"{}\"", esc(&c.to_string())),
            None => "null".to_string(),
        };
        let ink: Vec<String> = f
            .ink
            .iter()
            .map(|(k, v)| format!("{{\"key\": {}, \"ink\": {}}}", key_json(k), num(*v)))
            .collect();
        let dan: Vec<String> = f
            .dan
            .iter()
            .map(|(g, row)| {
                let vals: Vec<String> = row.iter().map(|v| num(*v)).collect();
                format!(
                    "{{\"gyo\": \"{}\", \"dan\": [{}]}}",
                    esc(g),
                    vals.join(", ")
                )
            })
            .collect();
        let peak = match &f.peak {
            Some(k) => key_json(k),
            None => "null".to_string(),
        };
        cases.push(format!(
            "  {{\"prev\": {prev}, \"empty\": {}, \"peak\": {peak},\n   \"ink\": [{}],\n   \"dan\": [{}]}}",
            f.is_empty(),
            ink.join(", "),
            dan.join(", ")
        ));
    }
    out.push_str(&cases.join(",\n"));
    out.push_str("\n ]\n}\n");
    out
}

/// 原器（縦組五十音配列）— 面ごとの鍵→仮名と、打鍵列の例。
fn genki_vectors() -> String {
    let mut out = String::from(
        "{\n \"_comment\": \"自動生成（cargo run --bin gen-vectors）— 手で編集しないこと。\
核（core/src/genki.rs）がロジックの SSOT である。\",\n \"source\": \"core/src/genki.rs\",\n",
    );
    let _ = writeln!(
        out,
        " \"shift\": \"{}\", \"dakuten\": \"{}\", \"handakuten\": \"{}\",",
        genki::SHIFT,
        genki::DAKUTEN,
        genki::HANDAKUTEN
    );

    for (name, plane) in [
        ("first_plane", &genki::FIRST_PLANE[..]),
        ("second_plane", &genki::SECOND_PLANE[..]),
    ] {
        let rows: Vec<String> = plane
            .iter()
            .map(|(k, kana)| {
                format!(
                    "  {{\"key\": \"{}\", \"kana\": \"{}\"}}",
                    esc(&k.to_string()),
                    esc(kana)
                )
            })
            .collect();
        let _ = write!(out, " \"{name}\": [\n{}\n ],\n", rows.join(",\n"));
    }

    // 打鍵列 → 仮名列（前置シフトの逐次性・後置濁点・^^＝ん を含む）
    let seqs = [
        "0987",
        "0pj",
        "5ta",
        "^p^o^i^u^y",
        "^t^r^e^w^q",
        "^^",
        "t^^",
        "pb",
        "gb",
        "gv",
        "0b",
        "pv",
        "udg^yo2^^ot^4",
        "z/",
    ];
    let rows: Vec<String> = seqs
        .iter()
        .map(|k| {
            format!(
                "  {{\"keys\": \"{}\", \"kana\": \"{}\"}}",
                esc(k),
                esc(&type_keys(k))
            )
        })
        .collect();
    let _ = write!(out, " \"sequences\": [\n{}\n ]\n}}\n", rows.join(",\n"));
    out
}

fn main() {
    let dir: PathBuf = [env!("CARGO_MANIFEST_DIR"), "vectors"].iter().collect();
    fs::create_dir_all(&dir).expect("vectors/ を作れない");
    for (name, body) in [
        ("gojuon.json", gojuon_vectors()),
        ("kehai.json", kehai_vectors()),
        ("genki.json", genki_vectors()),
    ] {
        let path = dir.join(name);
        fs::write(&path, body).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        println!("wrote {}", path.display());
    }
}
