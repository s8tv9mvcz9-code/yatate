# プロトコル — /convert の現行契約と制約格子への拡張

> 矢立はサーバに新しい変換器を要求しない。現行 `/convert` は既に IME の契約であり、
> 必要なのは**後方互換な一般化**——「かな文字列」を「制約の列」の特殊例として扱ふこと——だけである。

## 1. 現行契約の確認（変更なしで IME が使へる部分）

`POST /convert`（`backend/main.py` → `app/rag.py: convert_kana` → `app/convert.py: convert`）:

```jsonc
// 要求
{
  "text": "けふはよきひなり",          // 必須。1〜MAX_KANA_CHARS(200) 字
  "top_k": 4,                          // 手本検索の件数 0〜10（0=検索なし）
  "synesthesia": true,                 // 共感覚（情調→伝統色）
  "segmentation": ["けふは", "よき", "ひなり"]   // 任意。区切りを固定して再変換
}
```

```jsonc
// 応答（app/convert.py: to_payload）
{
  "text": "今日は良き日なり",          // 既定選択での合成文
  "accepted": true,                    // 読みの完全被覆を通つたか
  "attempts": 1,
  "warnings": [],                      // 送り仮名のゆるい整合警告など
  "error": null,                       // 未受理時の最後の指摘
  "palette": { /* 入力全体の情調パレット。pattern（和柄）を含む */ },
  "segments": [
    { "yomi": "けふは", "chosen": 0,
      "candidates": [
        { "surface": "今日は", "note": "副詞。本日の意",
          "score": 0.72, "mood": 0.2, "in_corpus": true,
          "color": "#3a4a6b", "color_name": "藍鼠" },
        { "surface": "けふは", "note": "", "score": 0.075, "mood": 0.0,
          "in_corpus": false, "color": "#F8FBF8", "color_name": "" }
      ] },
      // score は決定的な計算値: W_CORPUS*in_corpus + W_MOOD*mood + W_ORDER*(1-i/n)
      //（重み 0.5/0.35/0.15 — app/kana.py）。無信号候補の色は中立の白磁 #F8FBF8。
    ...
  ]
}
```

IME にとつての意味:

- **候補選択は端末内で完結**する（`segments[].candidates[]` を持ち帰るため、選び直しに
  通信は不要）。区切り修正だけ `segmentation` を付けて再要求する——`app/kana.py` の
  `merge_segments` / `split_segment` / `yomi_list` がこの往復の端末側半分である。
- ガード（レート制限・入力クランプ・エラー非開示）は `/chat` と同一系。IME クライアントは
  429/422/413 を「静かな失敗」として扱ひ、端末内変換へ退避する（[architecture.md](./architecture.md)）。
- 関係者ビルドは `X-API-Key` を送つて信頼枠に入る（既存機構のまま）。

## 2. 拡張 A — 制約格子（constraint lattice）

行打ち（[layout.md](./layout.md) §3）が生む入力は「かな文字列」ではなく**一モーラ一制約の列**である。
これを運ぶ追加フィールドを定義する。

### 制約オブジェクト

```jsonc
{ "gyo": "か", "dan": null, "daku": null, "ko": false }
// gyo : 行頭の仮名 "あかさたなはまやらわ" のいづれか。lit があるときは省略
// dan : 0..4（あ〜お段）。null = 未指定（行打ち）
// daku: true=濁点確定 / false=清音確定 / null=未指定
// ko  : true=小書き（ゃゅょっぁ〜ぉ）
// lit : "ん" "ー" "、" など、行・段の体系外の字をそのまま指定する。 {"lit": "ん"}
```

楷入力の一字は `{gyo:"か", dan:1, daku:false}`（＝き）のやうに**全指定の制約**になる。
つまり現行の `text` は「全指定制約列」の略記であり、拡張は真の一般化である。

### 要求の追加フィールド

```jsonc
{
  "text": "かははやかたんかなら",     // 従来通り必須（後述の互換規約）
  "constraints": [                      // 任意。存在すれば text の解釈より優先
    {"gyo":"か"}, {"gyo":"は"}, {"gyo":"は"}, {"gyo":"や"}, {"gyo":"か"},
    {"gyo":"た"}, {"lit":"ん"}, {"gyo":"か"}, {"gyo":"な"}, {"gyo":"ら"}
  ],
  "top_k": 4, "synesthesia": true
}
```

- `constraints` の要素数上限は `MAX_KANA_CHARS`（同じクランプを流用）。
- `text` には**行頭かなの列**（各制約の代表字）を入れる。pydantic は未知フィールドを
  無視するため、旧サーバに送ると `text` が字義通り変換されて無意味な結果になる——
  ゆゑにクライアントは**機能検出**（§4）を通過した場合のみ `constraints` を送る。

### 応答は不変

`segments[].yomi` は**解決済みの具体かな**で返る（「けふは」）。新キーは不要。
候補・色・出典の構造もそのまま使ふ。

### 検証の一般化（`app/kana.py`）

中核の不変条件「読みの完全被覆」は、制約整合へ自然に一般化される:

```
現行:   "".join(seg.yomi) == 入力かな                    （coverage_error）
拡張:   "".join(seg.yomi) が制約列と位置ごとに整合する      （constraint_error・新設）
```

- 新設テーブル `KANA_INFO: かな → (行, 段, 濁, 小)`（純データ・依存ゼロ。
  Swift 移植のパリティ対象——§5）。
- `constraint_error(constraints, segments)` は ①連結長が制約数と一致 ②各位置の仮名が
  `{gyo, dan, daku, ko, lit}` を満たす、を検査し、違反は現行同様
  **具体的指摘**（「3 文字目: 制約は な行 だが読みは『か』」）としてリフレクションに差し戻す。
  `app/convert.py` の折り返しループはそのまま流用し、被覆検査だけ差し替はる。
- プロンプト（`CONVERT_SYSTEM`）には制約記法の節を足す。モデルへは
  「制約列を満たす最尤の仮名文を復元し、それを従来通り文節分割して JSON で返せ」と指示する。
  **検証が機械である限り、モデルの自由度を上げても安全**——これがこのリポジトリの一貫した設計である。

## 3. 拡張 B — 候補の出典（校合の可視化）

現行の `in_corpus` は bool のみで、**どの作品に**実例があるかを落としてゐる。
出典チップ（[interaction.md](./interaction.md)）のため、候補に additive キーを足す:

```jsonc
{ "surface": "今日は", ..., "in_corpus": true,
  "provenance": [ {"title": "草枕", "author": "夏目漱石"} ] }   // 最大2件・無ければ省略
```

`app/rag.py: convert_kana` は手本チャンク（`search_chunks` の戻り）を連結して
`corpus_text` を作つてゐるので、連結前のチャンク列を `rank_candidates` へ渡せば
どのチャンクに当たつたかは即座に分かる（実装は数行）。旧クライアントは未知キーを無視する
（Android は `ignoreUnknownKeys`、iOS は Decodable の部分定義——3PF で確認済みの性質）。

## 4. 機能検出 — /health の additive キー

新クライアント→旧サーバ事故（§2）を防ぐため、`GET /health` に additive キーを足す:

```jsonc
{ "status": "ok", "version": "<BUILD_SHA>",
  "features": ["convert.constraints", "convert.provenance"] }
```

IME は起動時（と日次）に取得してキャッシュし、無ければ楷入力のみで運用する
（行打ちキーは「サーバ未対応」を示す淡色になる——端末内辞書の骨格引きは変はらず動く）。

## 5. パリティ — 検証器は一つ、実装は二つ

`KANA_INFO`・`constraint_error`・`canon`・旧字表（`app/kyuji.py` の 248 字）は
**Python が SSOT** であり、Swift 移植は写しである。写しが SSOT から逸れる事故は
このリポジトリで既に一度防ぎ方が確立してゐる（3PF の「振舞ひパリティをゲートで守る」）ので、
同じ機構を敷く:

1. `scripts/gen_rust_tables.py` — `kyuji.py` の対応表から核の Rust ソース
   （`core/src/generated/*.rs`）を機械生成する。手書き禁止。
   （M5-b2 より前は `gen_swift_tables.py` が Swift へも同じ表を写してゐたが、
   殻が核を直に呼ぶやうになつて写しが要らなくなり、生成器ごと消えた。）
2. `scripts/gen_parity_vectors.py` — Python 実装に代表入力（境界例・曖昧仮名・空・最長）を
   食はせた入出力対を `eval/parity/kana_vectors.json` に吐く。
3. Swift 側のテスト（`YatateCoreTests`）が同じベクトルを読み、出力一致を検査する。
   `ios-ci.yml` に **生成物が最新であること**（`git diff --exit-code` 方式）と
   テスト通過をゲートとして足す。

これで「Python を直したのに Swift が古い」は**機械的に落ちる**やうになり、
検証器の二重化ではなく**検証器の写像**になる。

## 6. 変へないもの

- NDJSON の `/chat` 契約・イベント順序（IME は `/chat` を使はない）。
- `/convert` の既存フィールドの意味・必須性（すべて additive）。
- レート制限・日次上限・クランプの閾値（IME は既存の公開枠/信頼枠に乗る。
  IME 起因で枠が窮屈になつたら env で調整——コードは触らない）。
- `segmentation` の形（解決後の再変換にそのまま使ふ。制約付き再変換で区切りを固定する場合も、
  解決済み yomi 列を送ればよい——制約はもう不要になつてゐる）。
