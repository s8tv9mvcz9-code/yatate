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
use yatate_core::jisho;
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

/// 文節分割 — **golden query セット**による検索品質の回帰検査（issue #4）。
///
/// 索引（辞書）が決定的なので、品質の物差しを ubuntu の `cargo test` に載せられる。
/// ここが捕まへるのは「辞書を足したら別の語の候補順が変はつた」「費用の重みを弄つたら
/// 分割が壊れた」といふ、単体試験では見えない**回帰**である。
///
/// query は代表例を選ぶ: 設計書の例文・同音異表記・文頭の助詞/自立語の競合・
/// 辞書に無い語・長い語と短い語の競合・区切り修正の後。
fn bunsetsu_vectors() -> String {
    use yatate_core::bunsetsu::{self, Bunsetsu};

    let mut out = String::from(
        "{\n \"_comment\": \"自動生成（cargo run --bin gen-vectors）— 手で編集しないこと。\
核（core/src/bunsetsu.rs・core/src/jisho.rs）がロジックの SSOT である。\",\n \
\"source\": \"core/src/bunsetsu.rs\",\n",
    );
    let _ = writeln!(
        out,
        " \"jisho_entries\": {}, \"max_yomi_chars\": {},",
        jisho::ENTRIES,
        jisho::MAX_YOMI_CHARS
    );

    let seg_json = |segs: &[Bunsetsu]| -> String {
        let rows: Vec<String> = segs
            .iter()
            .map(|s| {
                let cands: Vec<String> = s
                    .candidates
                    .iter()
                    .map(|c| {
                        format!(
                            "{{\"surface\": \"{}\", \"cost\": {}, \"in_jisho\": {}}}",
                            esc(&c.surface),
                            c.cost,
                            c.in_jisho
                        )
                    })
                    .collect();
                format!(
                    "    {{\"yomi\": \"{}\", \"chosen\": {},\n     \"candidates\": [{}]}}",
                    esc(&s.yomi),
                    s.chosen,
                    cands.join(", ")
                )
            })
            .collect();
        rows.join(",\n")
    };

    let queries = [
        "けふはよきてんきなり", // 設計書（roadmap M2）の出口条件そのもの
        "やまのうへにくもあり",
        "はなのいろはうつくしき",
        "ものをおもふ",   // 同音異表記（者・物）
        "なり",           // 文頭の助動詞 対 自立語
        "もの",           // 候補が複数立つ最小の例
        "ぷりん",         // 辞書に無い読み（仮名のまま一続き）
        "ぷりんをたべ",   // 未知語と助詞の混在
        "てんきなり",     // 長い語（てんき）と短い語（てん＋き）の競合
        "われはうみのこ", // 一字の自立語が続く
        "",               // 空
    ];

    let mut cases: Vec<String> = Vec::new();
    for q in queries {
        let segs = bunsetsu::segment(q);
        // 被覆は変換の中核の不変条件。ベクトルにも載せて、崩れたら気づけるやうにする。
        assert_eq!(
            bunsetsu::coverage_error(q, &segs),
            None,
            "{q:?} で読みの被覆が崩れてゐる"
        );
        cases.push(format!(
            "  {{\"query\": \"{}\",\n   \"yomi_list\": [{}],\n   \"compose\": \"{}\", \
\"commit\": \"{}\",\n   \"segments\": [\n{}\n   ]}}",
            esc(q),
            bunsetsu::yomi_list(&segs)
                .iter()
                .map(|y| format!("\"{}\"", esc(y)))
                .collect::<Vec<_>>()
                .join(", "),
            esc(&bunsetsu::compose(&segs)),
            esc(&bunsetsu::commit(&segs)),
            seg_json(&segs)
        ));
    }
    let _ = write!(out, " \"queries\": [\n{}\n ],\n", cases.join(",\n"));

    // 区切り修正 — 繋げる／割るの後も読みが失はれないこと（往復の不変条件）
    let mut edits: Vec<String> = Vec::new();
    for (q, op, at) in [
        ("けふはよきてんきなり", "merge", 0),
        ("けふはよきてんきなり", "merge", 1),
        ("けふはよきてんきなり", "split", 2),
        ("やまのうへにくもあり", "split", 1),
    ] {
        let mut segs = bunsetsu::segment(q);
        let ok = match op {
            "merge" => bunsetsu::merge(&mut segs, at),
            _ => bunsetsu::split(&mut segs, 0, at),
        };
        assert!(ok, "{q:?} の {op}({at}) が失敗した");
        assert_eq!(
            bunsetsu::coverage_error(q, &segs),
            None,
            "{op} で読みが失はれた"
        );
        edits.push(format!(
            "  {{\"query\": \"{}\", \"op\": \"{op}\", \"at\": {at},\n   \
\"yomi_list\": [{}], \"compose\": \"{}\"}}",
            esc(q),
            bunsetsu::yomi_list(&segs)
                .iter()
                .map(|y| format!("\"{}\"", esc(y)))
                .collect::<Vec<_>>()
                .join(", "),
            esc(&bunsetsu::compose(&segs))
        ));
    }
    let _ = write!(out, " \"edits\": [\n{}\n ]\n}}\n", edits.join(",\n"));
    out
}

/// 鍵の物理位置 — **殻をまたぐ地図**。
///
/// web の殻は核の表をそのまま使ひ、Windows の殻は自分の表をこれに照合する。
/// 同じ地図を二枚持つと放つておいて必ずずれる、といふのがこのファイルの前提である。
fn kagi_vectors() -> String {
    use yatate_core::kagi::KAGI;

    let mut out = String::from(
        "{\n \"_comment\": \"自動生成（cargo run --bin gen-vectors）— 手で編集しないこと。\
核（core/src/kagi.rs）が鍵の物理位置の SSOT である。\",\n \"source\": \"core/src/kagi.rs\",\n",
    );
    let rows: Vec<String> = KAGI
        .iter()
        .map(|k| {
            format!(
                "  {{\"genki\": \"{}\", \"code\": \"{}\", \"scan\": {}, \"mac\": {}, \"hid\": {}}}",
                esc(&k.genki.to_string()),
                esc(k.code),
                k.scan,
                k.mac,
                k.hid
            )
        })
        .collect();
    let _ = write!(out, " \"keys\": [\n{}\n ]\n}}\n", rows.join(",\n"));
    out
}

fn main() {
    let dir: PathBuf = [env!("CARGO_MANIFEST_DIR"), "vectors"].iter().collect();
    fs::create_dir_all(&dir).expect("vectors/ を作れない");
    for (name, body) in [
        ("gojuon.json", gojuon_vectors()),
        ("kehai.json", kehai_vectors()),
        ("genki.json", genki_vectors()),
        ("bunsetsu.json", bunsetsu_vectors()),
        ("kagi.json", kagi_vectors()),
    ] {
        let path = dir.join(name);
        fs::write(&path, body).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        println!("wrote {}", path.display());
    }
}
