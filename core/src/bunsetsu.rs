//! 文節 — 格子探索・費用計算・区切り修正。**変換のうち決定的な部分**。
//!
//! `docs/ime/protocol.md` §1 の応答は `segments[].candidates[]` といふ形をしてゐて、
//! 「候補選択は端末内で完結する／区切り修正だけ再要求する」と決めてある。
//! その**端末側の半分**（`merge_segments` / `split_segment` / `yomi_list`）が
//! ここである。サーバ側（`app/kana.py`）と同じ物を各殻に書けば三度書くことになるので、
//! 核に置く（`docs/ime/cross-platform.md` §3「殻に頭脳を置かない」）。
//!
//! ## 何がここに入り、何が入らないか
//!
//! 入る: 辞書引き・格子探索・費用・文節分割・区切り修正・読みの被覆検査。
//! いづれも決定的で、依存ゼロで、画面を知らない。
//!
//! 入らない: 埋め込み・近傍検索・情調の推定・文の生成。
//! それらはモデルと通信を要するので別リポジトリに残る（`docs/ime/artifacts.md` §2）。
//!
//! ## 文節の定めかた
//!
//! **文節 ＝ 自立語 ひとつ ＋ 付属語 いくつか。** これだけで決まる。
//! 格子の最短路を採つた後、自立語のたびに新しい文節を起こし、
//! 付属語は直前の文節へぶら下げる。辞書に無い仮名が続く区間は
//! **一続きで一つの文節**にする（一字づつ文節にすると、直しやうが無くなるため）。

use crate::jisho::{self, Pos, Word, UNKNOWN_CHAR_COST};
use crate::kyuji::to_kyuji;

// ── 接続の費用 ──────────────────────────────────────────────
//
// 語そのものの費用（度数から出る）に、前後の品詞の繋がりやすさを足す。
// 値は「文節をむやみに切らない／文頭にいきなり助詞を置かない」を表すだけの
// 素朴なもので、辞書が育つたら測り直す余地がある（そのときも整数のまま）。

/// 新しい文節を起こす費用（直前が自立語のとき）。
pub const NEW_BUNSETSU_AFTER_JIRITSU: i32 = 1500;
/// 新しい文節を起こす費用（直前が付属語のとき）。付属語で文節が閉ぢるのは自然なので安い。
pub const NEW_BUNSETSU_AFTER_FUZOKU: i32 = 1200;
/// 付属語が続く費用。
pub const FUZOKU_CHAIN: i32 = 200;
/// 文頭にいきなり付属語が来る費用。ほぼ起きないので高く取る。
pub const FUZOKU_AT_HEAD: i32 = 4000;

const INF: i32 = i32::MAX / 4;
const BOS: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Jiritsu = 0,
    Fuzoku = 1,
    /// 辞書に無い仮名。仮名のまま置く。
    Unknown = 2,
}

fn conn(prev: usize, cur: Kind) -> i32 {
    match (prev, cur) {
        (BOS, Kind::Fuzoku) => FUZOKU_AT_HEAD,
        (BOS, _) => 0,
        (_, Kind::Fuzoku) if prev == Kind::Fuzoku as usize => FUZOKU_CHAIN,
        (_, Kind::Fuzoku) => 0,
        // 自立語・未知語は文節を起こす。ただし未知語が続く間は一続きにする。
        (p, Kind::Unknown) if p == Kind::Unknown as usize => 0,
        (p, _) if p == Kind::Fuzoku as usize => NEW_BUNSETSU_AFTER_FUZOKU,
        _ => NEW_BUNSETSU_AFTER_JIRITSU,
    }
}

// ── 格子の最短路 ────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Back {
    prev_pos: usize,
    prev_kind: usize,
    word: Option<Word>,
    start: usize,
}

/// 最短路の一節点。
struct PathNode {
    yomi: String,
    word: Option<Word>,
    kind: Kind,
}

fn node_kind(w: &Option<Word>) -> Kind {
    match w {
        Some(w) if w.pos == Pos::Fuzoku => Kind::Fuzoku,
        Some(_) => Kind::Jiritsu,
        None => Kind::Unknown,
    }
}

/// 読みの列に格子を張り、費用が最も小さい語の並びを返す。
fn best_path(yomi: &str) -> Vec<PathNode> {
    if yomi.is_empty() {
        return Vec::new();
    }
    let n = yomi.len();
    let mut best = vec![[INF; 4]; n + 1];
    let mut back: Vec<[Option<Back>; 4]> = vec![[None; 4]; n + 1];
    best[0][BOS] = 0;

    for pos in 0..n {
        if !yomi.is_char_boundary(pos) {
            continue;
        }
        for prev_kind in 0..4 {
            let base = best[pos][prev_kind];
            if base >= INF {
                continue;
            }
            let rest = &yomi[pos..];

            // ① 辞書の前方一致
            for (len, w) in jisho::prefix_matches(rest) {
                let kind = node_kind(&Some(w));
                let cost = base + conn(prev_kind, kind) + w.cost();
                let end = pos + len;
                let k = kind as usize;
                if cost < best[end][k] {
                    best[end][k] = cost;
                    back[end][k] = Some(Back {
                        prev_pos: pos,
                        prev_kind,
                        word: Some(w),
                        start: pos,
                    });
                }
            }

            // ② 辞書に無い一字（仮名のまま置く）
            let ch_len = rest.chars().next().map(|c| c.len_utf8()).unwrap_or(0);
            if ch_len > 0 {
                let kind = Kind::Unknown;
                let cost = base + conn(prev_kind, kind) + UNKNOWN_CHAR_COST;
                let end = pos + ch_len;
                let k = kind as usize;
                if cost < best[end][k] {
                    best[end][k] = cost;
                    back[end][k] = Some(Back {
                        prev_pos: pos,
                        prev_kind,
                        word: None,
                        start: pos,
                    });
                }
            }
        }
    }

    // 終端で最も安い状態を採る。同点は種別の番号順（決定的にするため）。
    let mut end_kind = 0;
    let mut end_cost = INF;
    for (k, cost) in best[n].iter().enumerate().take(3) {
        if *cost < end_cost {
            end_cost = *cost;
            end_kind = k;
        }
    }
    if end_cost >= INF {
        return Vec::new();
    }

    let mut out: Vec<PathNode> = Vec::new();
    let (mut pos, mut kind) = (n, end_kind);
    while pos > 0 {
        let b = match back[pos][kind] {
            Some(b) => b,
            None => break,
        };
        out.push(PathNode {
            yomi: yomi[b.start..pos].to_string(),
            word: b.word,
            kind: node_kind(&b.word),
        });
        let (p, k) = (b.prev_pos, b.prev_kind);
        pos = p;
        kind = k;
    }
    out.reverse();
    out
}

// ── 文節と候補 ──────────────────────────────────────────────

/// 一つの候補（この文節をどう書くか）。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Cand {
    /// 表記。**新字体のまま**であり、旧字体は確定のとき核が定める。
    pub surface: String,
    /// 費用（小さいほど確からしい）。並びはこの昇順・同点は表記の辞書順。
    pub cost: i32,
    /// 辞書に有る語から出た候補か（仮名のままの控へは false）。
    pub in_jisho: bool,
}

/// 一つの文節。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Bunsetsu {
    /// この文節の読み（仮名）。連結すると入力全体に戻る（[`coverage_error`] が縛る）。
    pub yomi: String,
    /// 候補。必ず一つ以上ある（最低でも仮名のままの控へが入る）。
    pub candidates: Vec<Cand>,
    /// いま選ばれてゐる候補の番号。
    pub chosen: usize,
}

impl Bunsetsu {
    /// 選ばれてゐる表記。
    pub fn surface(&self) -> &str {
        &self.candidates[self.chosen].surface
    }

    /// 候補を選び直す。範囲外なら何もせず `false`。
    pub fn choose(&mut self, index: usize) -> bool {
        if index < self.candidates.len() {
            self.chosen = index;
            true
        } else {
            false
        }
    }
}

/// 節点の並びから一つの文節を組む。
///
/// 頭（最初の節点）の書き方を差し替へたものが候補になる。
/// 尻尾（付属語・未知語）は固定である——助詞を選び直す IME は無い。
fn build(nodes: &[PathNode]) -> Bunsetsu {
    let yomi: String = nodes.iter().map(|n| n.yomi.as_str()).collect();
    let head = &nodes[0];
    let tail: String = nodes[1..]
        .iter()
        .map(|n| match &n.word {
            Some(w) => w.surface,
            None => n.yomi.as_str(),
        })
        .collect();

    // 頭の候補 — 辞書の同じ読みの語すべて ＋ 仮名のままの控へ
    let mut heads: Vec<(String, i32, bool)> = jisho::lookup(&head.yomi)
        .into_iter()
        .map(|w| (w.surface.to_string(), w.cost(), true))
        .collect();
    let kana_cost = UNKNOWN_CHAR_COST * head.yomi.chars().count() as i32;
    if !heads.iter().any(|(s, _, _)| *s == head.yomi) {
        heads.push((head.yomi.clone(), kana_cost, false));
    }

    // 費用の昇順・同点は表記の辞書順（環境に依らない並びにするため）
    heads.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

    let chosen_surface = match &head.word {
        Some(w) => w.surface.to_string(),
        None => head.yomi.clone(),
    };

    let mut candidates: Vec<Cand> = Vec::new();
    let mut chosen = 0;
    for (surface, cost, in_jisho) in heads {
        let is_chosen = surface == chosen_surface;
        let full = format!("{surface}{tail}");
        // 同じ表記になる候補は畳む（安い方が残る＝先に来た方）
        if let Some(i) = candidates.iter().position(|c| c.surface == full) {
            if is_chosen {
                chosen = i;
            }
            continue;
        }
        if is_chosen {
            chosen = candidates.len();
        }
        candidates.push(Cand {
            surface: full,
            cost,
            in_jisho,
        });
    }

    // **仮名のままの控へを必ず一つ残す。**
    //
    // 頭を仮名に戻しただけでは尻尾が漢字のまま残る（「けふは良き」）。
    // 繋げた文節では尻尾に自立語が入り得るので、それでは「全部仮名で書く」に辿り着けない。
    // 一字あたりの費用は辞書のどの語よりも高く、文節全体はその文字数倍なので、
    // この候補は必ず最後尾に来る（並びは費用の昇順のまま崩れない）。
    if !candidates.iter().any(|c| c.surface == yomi) {
        candidates.push(Cand {
            surface: yomi.clone(),
            cost: UNKNOWN_CHAR_COST * yomi.chars().count() as i32,
            in_jisho: false,
        });
    }

    Bunsetsu {
        yomi,
        candidates,
        chosen,
    }
}

/// 読みの列を文節へ分ける。**これが変換の入口**である。
///
/// ```
/// use yatate_core::bunsetsu;
/// let segs = bunsetsu::segment("けふはよきてんきなり");
/// assert_eq!(bunsetsu::yomi_list(&segs), ["けふは", "よき", "てんきなり"]);
/// assert_eq!(bunsetsu::commit(&segs), "今日は良き天氣なり");
/// ```
pub fn segment(yomi: &str) -> Vec<Bunsetsu> {
    let path = best_path(yomi);
    if path.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<Bunsetsu> = Vec::new();
    let mut group: Vec<PathNode> = Vec::new();
    let mut prev_kind: Option<Kind> = None;
    for node in path {
        let starts_new = match node.kind {
            Kind::Fuzoku => false,
            Kind::Unknown => prev_kind != Some(Kind::Unknown),
            Kind::Jiritsu => true,
        };
        if starts_new && !group.is_empty() {
            out.push(build(&group));
            group = Vec::new();
        }
        prev_kind = Some(node.kind);
        group.push(node);
    }
    if !group.is_empty() {
        out.push(build(&group));
    }
    out
}

/// 読みを**一つの文節として**組む（区切りを固定して組み直すときに使ふ）。
///
/// 中では同じ格子を張るので、頭の語は最も安い読み方が選ばれる
/// （「けふは」なら 今日＋は で、候補は「今日は」と「けふは」になる）。
pub fn one(yomi: &str) -> Option<Bunsetsu> {
    let path = best_path(yomi);
    if path.is_empty() {
        return None;
    }
    Some(build(&path))
}

/// 区切りを外から与へて組み直す（`docs/ime/protocol.md` の `segmentation`）。
pub fn segment_fixed(parts: &[&str]) -> Vec<Bunsetsu> {
    parts.iter().filter_map(|p| one(p)).collect()
}

// ── 区切り修正 ──────────────────────────────────────────────

/// `i` 番と `i+1` 番の文節を繋げる。範囲外なら何もせず `false`。
pub fn merge(segs: &mut Vec<Bunsetsu>, i: usize) -> bool {
    if i + 1 >= segs.len() {
        return false;
    }
    let joined = format!("{}{}", segs[i].yomi, segs[i + 1].yomi);
    match one(&joined) {
        Some(b) => {
            segs.splice(i..i + 2, [b]);
            true
        }
        None => false,
    }
}

/// `i` 番の文節を、頭から `at` **文字目**で二つに割る。
///
/// `at` が 0 か文節の長さ以上なら割れないので `false`（空の文節を作らない）。
pub fn split(segs: &mut Vec<Bunsetsu>, i: usize, at: usize) -> bool {
    let Some(seg) = segs.get(i) else {
        return false;
    };
    let total = seg.yomi.chars().count();
    if at == 0 || at >= total {
        return false;
    }
    let byte = seg
        .yomi
        .char_indices()
        .nth(at)
        .map(|(b, _)| b)
        .unwrap_or(seg.yomi.len());
    let (head, tail) = seg.yomi.split_at(byte);
    match (one(head), one(tail)) {
        (Some(a), Some(b)) => {
            segs.splice(i..i + 1, [a, b]);
            true
        }
        _ => false,
    }
}

/// 文節ごとの読み（区切りを固定して再変換するときにそのまま送れる形）。
pub fn yomi_list(segs: &[Bunsetsu]) -> Vec<String> {
    segs.iter().map(|s| s.yomi.clone()).collect()
}

/// 選ばれてゐる表記を繋げた文（**新字体のまま**）。
pub fn compose(segs: &[Bunsetsu]) -> String {
    segs.iter().map(|s| s.surface()).collect()
}

/// 確定 — 繋げた上で**旧字体を機械で定める**（`core/src/kyuji.rs`）。
///
/// 辞書は新字体で持つてゐるので、旧字は必ずここで決まる。
/// 辞書に旧字を混ぜないのは、同じ知識を二か所に置かないためである。
pub fn commit(segs: &[Bunsetsu]) -> String {
    to_kyuji(&compose(segs))
}

// ── 被覆検査 ────────────────────────────────────────────────

/// 文節の読みを繋げたものが入力と一致するか。ずれてゐたら**具体的な指摘**を返す。
///
/// これは変換の中核の不変条件である（`docs/ime/protocol.md` §2
/// 「読みの完全被覆」）。落ちる・捏造する・並びが入れ替はる、のいづれも
/// ここで捕まる。
pub fn coverage_error(input: &str, segs: &[Bunsetsu]) -> Option<String> {
    let joined: String = segs.iter().map(|s| s.yomi.as_str()).collect();
    if joined == input {
        return None;
    }
    let pos = input
        .chars()
        .zip(joined.chars())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| input.chars().count().min(joined.chars().count()));
    Some(format!(
        "{} 文字目から読みがずれてゐる: 入力は「{}」だが文節を繋げると「{}」",
        pos + 1,
        input,
        joined
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn surfaces(segs: &[Bunsetsu]) -> Vec<String> {
        segs.iter().map(|s| s.surface().to_string()).collect()
    }

    #[test]
    fn 設計書の例が通る() {
        // docs/ime/roadmap.md M2 の出口条件そのもの
        let segs = segment("けふはよきてんきなり");
        assert_eq!(yomi_list(&segs), ["けふは", "よき", "てんきなり"]);
        assert_eq!(compose(&segs), "今日は良き天気なり");
        assert_eq!(commit(&segs), "今日は良き天氣なり", "確定で旧字が定まる");
    }

    #[test]
    fn 文節は自立語ひとつと付属語いくつかになる() {
        let segs = segment("やまのうへにくもあり");
        for s in &segs {
            assert!(!s.yomi.is_empty());
            assert!(!s.candidates.is_empty(), "候補が空の文節を作らない");
        }
        assert_eq!(coverage_error("やまのうへにくもあり", &segs), None);
    }

    #[test]
    fn 候補は費用の昇順で並ぶ() {
        let segs = segment("ものをおもふ");
        for s in &segs {
            for w in s.candidates.windows(2) {
                assert!(
                    w[0].cost < w[1].cost
                        || (w[0].cost == w[1].cost && w[0].surface < w[1].surface),
                    "{}: {:?} → {:?} の並びが決定的でない",
                    s.yomi,
                    w[0],
                    w[1]
                );
            }
        }
    }

    #[test]
    fn 同じ読みの別表記が候補に出る() {
        let segs = segment("もの");
        assert_eq!(segs.len(), 1);
        let surfaces: Vec<&str> = segs[0]
            .candidates
            .iter()
            .map(|c| c.surface.as_str())
            .collect();
        assert!(surfaces.contains(&"者"), "{surfaces:?}");
        assert!(surfaces.contains(&"物"), "{surfaces:?}");
        assert!(
            surfaces.contains(&"もの"),
            "仮名のままの控へが要る: {surfaces:?}"
        );
    }

    #[test]
    fn 仮名のままの控へは辞書由来でない() {
        let segs = segment("もの");
        let kana = segs[0]
            .candidates
            .iter()
            .find(|c| c.surface == "もの")
            .expect("控へ");
        assert!(!kana.in_jisho);
        assert!(segs[0].candidates.iter().any(|c| c.in_jisho));
    }

    #[test]
    fn 辞書に無い読みは仮名のまま一つの文節になる() {
        let segs = segment("ぷりん");
        assert_eq!(surfaces(&segs), ["ぷりん"], "一字づつ切らない");
        assert_eq!(segs.len(), 1);
    }

    #[test]
    fn 辞書に無い読みが混じつても被覆は崩れない() {
        let input = "ぷりんをたべ";
        let segs = segment(input);
        assert_eq!(coverage_error(input, &segs), None);
        assert!(compose(&segs).contains('を'));
    }

    #[test]
    fn 空の入力は空の文節列になる() {
        assert!(segment("").is_empty());
        assert_eq!(compose(&[]), "");
        assert_eq!(coverage_error("", &[]), None);
    }

    #[test]
    fn 文頭の助詞より自立語を選ぶ() {
        // 「なり」は 付属語（なり）と 自立語（成り）の両方にある。
        // 文頭なら自立語が勝たねばならない。
        let segs = segment("なり");
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].surface(), "成り", "文頭にいきなり助動詞は来ない");
    }

    #[test]
    fn 候補を選び直せる() {
        let mut segs = segment("もの");
        let before = segs[0].surface().to_string();
        assert!(segs[0].choose(1));
        assert_ne!(segs[0].surface(), before);
        assert!(!segs[0].choose(999), "範囲外は撥ねる");
    }

    #[test]
    fn 区切りを繋げられる() {
        let mut segs = segment("けふはよきてんきなり");
        let n = segs.len();
        assert!(merge(&mut segs, 0));
        assert_eq!(segs.len(), n - 1);
        assert_eq!(segs[0].yomi, "けふはよき");
        assert_eq!(
            coverage_error("けふはよきてんきなり", &segs),
            None,
            "繋げても読みは失はれない"
        );
        assert!(!merge(&mut segs, 99), "範囲外は撥ねる");
    }

    #[test]
    fn 区切りを割れる() {
        let mut segs = segment("けふはよきてんきなり");
        assert!(split(&mut segs, 0, 2));
        assert_eq!(segs[0].yomi, "けふ");
        assert_eq!(segs[1].yomi, "は");
        assert_eq!(coverage_error("けふはよきてんきなり", &segs), None);
    }

    #[test]
    fn 空の文節は作らない() {
        let mut segs = segment("けふは");
        assert!(!split(&mut segs, 0, 0), "頭で割ると空が出来る");
        let n = segs[0].yomi.chars().count();
        assert!(!split(&mut segs, 0, n), "末尾で割ると空が出来る");
    }

    #[test]
    fn 繋げた文節にも候補が立つ() {
        let mut segs = segment("けふはよきてんきなり");
        merge(&mut segs, 0);
        let surfaces: Vec<&str> = segs[0]
            .candidates
            .iter()
            .map(|c| c.surface.as_str())
            .collect();
        assert!(
            surfaces.iter().any(|s| s.starts_with("今日")),
            "{surfaces:?}"
        );
        assert!(surfaces.contains(&"けふはよき"), "仮名の控へ: {surfaces:?}");
    }

    #[test]
    fn 区切りを固定して組み直せる() {
        let segs = segment_fixed(&["けふ", "は", "よき"]);
        assert_eq!(yomi_list(&segs), ["けふ", "は", "よき"]);
        assert_eq!(compose(&segs), "今日は良き");
    }

    #[test]
    fn 被覆が崩れたら位置つきで指摘する() {
        let segs = segment("けふは");
        let err = coverage_error("けふはよき", &segs).expect("ずれてゐる");
        assert!(err.contains("4 文字目"), "{err}");

        let err = coverage_error("けふを", &segs).expect("ずれてゐる");
        assert!(err.contains("3 文字目"), "{err}");
    }

    #[test]
    fn 分割は決定的である() {
        for _ in 0..8 {
            assert_eq!(
                surfaces(&segment("やまのうへのしろきくも")),
                surfaces(&segment("やまのうへのしろきくも"))
            );
        }
    }

    #[test]
    fn 長い入力でも壊れない() {
        let input = "けふはよきてんきなり".repeat(20);
        let segs = segment(&input);
        assert_eq!(coverage_error(&input, &segs), None);
    }
}
