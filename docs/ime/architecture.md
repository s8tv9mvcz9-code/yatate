# 技術アーキテクチャ — 拡張の構成・三層変換・辞書・予算

> キーボード拡張といふ最も窮屈な実行環境（メモリ非公開上限・フルアクセス既定 OFF・
> サイレント kill）の中で、bungo-rag の資産を最大限生かす構成。事実関係は 2026-07 の調査に
> 基づく（出典は各所に付す）。

## 1. ターゲット構成 — モノレポの第4のクライアント

```
ios/
  project.yml            # XcodeGen（既存）に 2 ターゲットを追加
  Sources/               # 既存 BungoRag アプリ（本体＝containing app）
  YatateKeyboard/        # 新規: キーボード拡張ターゲット
  YatateCore/            # 新規: ローカル Swift Package（本体・拡張で共有）
    Sources/YatateCore/
      Generated/         # Python SSOT からの機械生成（手書き禁止—protocol.md §5）
    Tests/YatateCoreTests/
```

- **YatateCore** に移すもの: 既存 `Net/BungoAPI.swift`・`Model/Models.swift`
  （**iOS には `/convert` クライアントが既に完全実装されてゐる** — `BungoAPI.convert`、
  `ConvertRequest/Response`、reqId＋cancel の競合制御、選択引き継ぎの字面照合まで。
  ※CLAUDE.md の「iOS 未実装」記述は古い——本設計と同時に訂正する）、
  ＋新規の決定的核（§2 T0）。本体アプリと拡張の双方が依存する。
- **YatateKeyboard**: `UIInputViewController` サブクラス＋二行配列ビュー。
  `Info.plist` の要点:

```yaml
# project.yml 追記の骨子
targets:
  YatateCore:            # local package として packages: で参照でも可
    type: framework      # または SPM local package（後者を推奨）
    platform: iOS
  YatateKeyboard:
    type: app-extension
    platform: iOS
    sources: [YatateKeyboard]
    dependencies: [{ target: BungoRag は不可 — package: YatateCore }]
    info:
      path: YatateKeyboard/Info.plist
      properties:
        NSExtension:
          NSExtensionPointIdentifier: com.apple.keyboard-service
          NSExtensionPrincipalClass: $(PRODUCT_MODULE_NAME).KeyboardViewController
          NSExtensionAttributes:
            IsASCIICapable: false
            PrefersRightToLeft: false
            PrimaryLanguage: ja-JP
            RequestsOpenAccess: true      # 宣言のみ。許可なしでも全機能が劣化動作（§6）
  BungoRag:
    dependencies: [{ target: YatateKeyboard, embed: true }]   # 拡張を同梱
```

- 配布は既存の流儀のまま: `ios-ci.yml` のシミュレータビルド検証＋未署名 IPA の
  ローリングリリース（拡張は .app に同梱されるので成果物は変はらない）。
  実機は `docs/device-testing.md` の自己署名手順で、**キーボードの有効化**
  （設定→一般→キーボード）だけが追加手順になる。

## 2. 三層変換 — T0 / T1 / T2

| 層 | 実体 | 依存 | 応答 | 賄ふ範囲 |
|---|---|---|---|---|
| **T0 決定的核** | YatateCore（Swift・純関数） | なし | <1ms | 打鍵エコー・制約整合・旧字確定・区切り操作 |
| **T1 端末内辞書** | AzooKeyKanaKanjiConverter＋文語辞書（§3） | 拡張バンドル同梱 | <50ms 目標 | かな→漢字候補・行打ちの骨格引き |
| **T2 サーバ校合** | `POST /convert`（既存） | フルアクセス＋網 | 数百 ms〜（cold start は分単位もあり得る） | LLM 文節分割・コーパス実例・情調・出典 |

### T0 — 決定的核（`app/kana.py`・`app/kyuji.py` の写し）

Python が SSOT、Swift は機械生成＋パリティテストで従属（[protocol.md](./protocol.md) §5）。
移植対象は精読で確定済み: `canon`／`coverage_error`／`rank_candidates` の並べ替へ骨格／
`merge_segments`・`split_segment`／`compose`、旧字表 248 字（`AMBIGUOUS_SHINJI`
「弁予余欠芸」は除外のまま）、新設の `KANA_INFO`（行・段・濁・小）と `constraint_error`。
`eval/test_kana.py` の assert 群が**そのままパリティベクトルの写し元**になる。
依存ゼロ・状態最小なので移植は素直である。

### T1 — 端末内辞書層

**エンジンは AzooKeyKanaKanjiConverter（MIT・iOS 16+・Swift Package）を採用**する。
理由: LOUDS 簡潔データ構造で辞書を頭文字ごとに遅延ロードし、キーボード拡張の
メモリ予算内で長年動いてきた実績（azooKey 本体が同構成）。公開 API は
`KanaKanjiConverter` + `ComposingText` + `requestCandidates`。
独自エントリの差し込み経路も公開されてゐる:

- 試作期: `importDynamicUserDictionary([DicdataElement])` — コード一行、数千語まで。
- 本採用: `DictionaryBuilder.exportDictionary` で TSV → `user.louds` を**事前ビルドして
  拡張バンドルに同梱**（フルアクセス OFF でも読める必須データはバンドル同梱が定石——
  App Group はフルアクセス OFF だと書き込みが遮断される実証があるため当てにしない）。
- 徹底期: `anco dict build` でシステム辞書ごと再構成（新仮名エントリの排除・
  旧字旧仮名コストの優遇までやる場合）。辞書データは Apache-2.0 で改変再配布可。

**行打ちの骨格引き**は AzooKey エンジンの外に持つ: 収穫辞書（§3）に
行シグネチャ索引（読みの各字を行頭字に落とした列。「けふは」→「かはは」）を付け、
制約列→候補列を**直接引く**。エンジンに格子を食はせる迂回より小さく確実で、
T2 の `constraints`（[protocol.md](./protocol.md) §2）と意味論が揃ふ。

Zenzai（ニューラル変換。GPT-2 系 90M・GGUF・llama.cpp 同梱・拡張内動作の先行実績あり）は
**既定 OFF のまま将来枠**とする。重みが CC-BY-SA 4.0 である点（コードの MIT と別）と、
旧仮名読みが分布外である点から、まづ決定的辞書で立ち上げ、zenz の文語 fine-tune は
稽古場（本体アプリ）での実験として [roadmap.md](./roadmap.md) M4 に置く。

### T2 — サーバ校合（既存 `/convert`）

- 拡張から呼べるのは**フルアクセス許可時のみ**（ネットワーク解禁のため）。
  既存 `BungoAPI` をそのまま使ふ（`timeoutIntervalForRequest=200` は scale-to-zero の
  cold start 対応として既に正しい値になつてゐる）。
- **T2 は常に非同期の上乗せ**であり、入力を一切ブロックしない。T1 の候補が先に出て、
  T2 到着分は未確定文節に**追記**される（Sumibi が実証した「速い変換器の仮確定→
  LLM で後から上書き」のプログレッシブ方式と同型。ただし矢立は自動上書きせず、
  候補への追加に留める——指の下の並べ替へ禁止の不変条件を守る）。
- レート制限（公開枠 15req/60s・日次 500）を**クライアントが尊重する**:
  打鍵ごとの自動要求は禁止、明示引金（変換キー・読点・区切り修正）のみ、
  ジェスチャ由来は 350ms デバウンス（Android 變換タブで確立済みの規約）、
  429 は静かに T1 へ退避。関係者ビルドは既存の `X-API-Key` 注入（`project.yml` →
  Info.plist）をそのまま拡張ターゲットにも適用する。

## 3. 文語辞書 — 青空文庫ルビの収穫

現行インデックスは約 7,000 チャンク／300 作品だが、**ルビ（読み）を捨ててゐる**。
`scripts/build_index.py` の `AOZORA_NOISE` 正規表現が `《…》` を削除する**前**に
`｜親文字《よみ》` と `漢字連《よみ》`（｜なし・直前の漢字連続を親とみなす青空記法）を
抽出すれば、**旧字の字面と旧仮名の読みの対**が、出典（作品・著者）と頻度つきで採れる。
取得パイプライン（カタログ CSV → ZIP → shift_jis）は全て流用できる。

新設 `scripts/build_ime_dict.py` の出力（一回のビルドで三つ）:

1. **変換辞書 TSV** — `DictionaryBuilder`／`anco dict build` の入力
   （ruby はカタカナ・品詞 cid/mid は既存 ID 流用・value は頻度から算出）。
2. **行シグネチャ索引** — 行打ち用の小型バイナリ（読み→行列シグネチャ→エントリ群）。
3. **出典表** — surface→(title, author) の最頻 2 件。オフラインでも出典チップ
   （[interaction.md](./interaction.md) §1）を灯すため。

規模感: 旧字旧仮名 300 作品のルビ対はユニーク数万のオーダーと見積もる（要実測——
LOUDS 化で数 MB、遅延ロードなので常駐は一部）。**辞書の質の回帰は `anco evaluate`**
（かな→正解表記の TSV で機械採点）を CI 外の手元検証として使ひ、代表ケースは
パリティベクトルに昇格させる。

## 4. メモリ・遅延の予算

メモリ上限は Apple 非公表。実務値は「**快適 ~30MB／kill 閾値 ~60-70MB**（機種・OS 依存、
超過はログなしのサイレント kill →既定キーボードへ戻される）」。予算はかう切る:

| 項目 | 予算 | 根拠・備考 |
|---|---|---|
| UI（二行配列＋作業帯＋候補列） | ~8MB | SwiftUI 最小構成。候補列の縦書きは Core Text 自前描画（§5） |
| AzooKey 変換器＋辞書常駐分 | ~15MB | LOUDS 遅延ロード。`preloadDictionary` は**使はない** |
| 行シグネチャ索引＋出典表 | ~3MB | mmap で読み、常駐最小 |
| 余白（jetsam 余裕） | ~10MB | 合計 ~36MB を平時目標、瞬間 50MB を超えない |

- 拡張には `increased-memory-limit` entitlement が効かないといふ報告があるため、
  **予算で守る**しかない。稽古場に jetsam event report の読み方を記した開発者向け頁を置く。
- 遅延: 打鍵エコー・梯子追従は 16ms（1 フレーム）以内で T0 のみが担当。T1 候補 50ms 目標。
  T2 は無期限（非同期・表示は到着時に追記）。

## 5. ホスト統合 — 作業帯を正とする

- **合成テキストの正はキーボード内の作業帯**（[interaction.md](./interaction.md) §1）。
  `setMarkedText` はホストアプリの `UITextInput` 実装に依存し、`UIKeyInput` のみの
  ビューでは機能しない（変換中下線を全ホストで保証できない）ことが確認されてゐるため、
  **既定では使はず**、設定で「ホスト下線合成」を任意 ON にする位置づけ。
- 確定は `insertText` 一発（文節ごとの細切れ挿入をしない——proxy 連続操作の
  カーソル不整合報告への防御）。反故（確定の呼び戻し）は**自前の直前確定記録**だけを
  信頼して `deleteBackward` を確定字数分発行する。`documentContextBeforeInput` は
  近傍 ~300 字・ホスト依存なので、**読まない・送らない・頼らない**（§6）。
- 地球儀キー: `needsInputModeSwitchKey` が真なら表示し `handleInputModeList` を接続
  （審査 4.4.1 の「次のキーボードへ進む手段」義務）。
- 候補列の縦書きは Core Text（`kCTVerticalFormsAttributeName` ＋
  `kCTFrameProgressionRightToLeft`）で自前描画する。TextKit 1/2 とも iOS に真の縦組は
  無いが、**候補列といふ小さな矩形**に限れば自前描画のコストは小さく、メモリも軽い。
- **ハードウェアキーボードのキーイベントは拡張には届かない**（`pressesBegan` 等は
  呼ばれず、ハードキー入力はホストへ直接配送される。サードパーティがハードキー IME を
  作る公式手段は存在しない——2026-07 調査で確定）。ゆゑに原器の物理キーボード対応は
  本体アプリ内エディタと将来の macOS IMKit 版が受け持つ（[roadmap.md](./roadmap.md) 遠景）。

## 6. プライバシーと審査 — 中墨でも成立する道具

- **審査 4.4.1（2026-07 時点の現行文言）**: フルネットワークアクセス無し・フルアクセス
  要求無しでも機能し続けること／次のキーボードへ進む手段／設定以外のアプリを起動しない
  ／キーの転用禁止。矢立の三層は最初からこの形（T0+T1 で完結、T2 は上乗せ）に
  設計されてをり、**フルアクセス OFF が既定の姿**である。
- サーバへ送るのは**変換対象の仮名または制約列だけ**。ホスト文書の前後文脈・アプリ名・
  識別子は送らない。端末外テレメトリ皆無（計測は稽古場のみ・端末内のみ——
  [interaction.md](./interaction.md) §8）。
- フルアクセス許可画面には OS の警告（「開発者が入力内容を送信できる」）が出る。
  本体アプリの説明頁に**何を送り何を送らないか**を一枚で明記する（上記そのまま）。
- App Group（設定同期・稽古場計測の共有）は**フルアクセス許可時のみ有効になる
  快適装備**とし、必須データ（辞書・設定既定値）は拡張バンドルに同梱する。

## 7. 劣化の設計（墨色三態の実体）

| 状態 | 検知 | T0 | T1 | T2 | 表示 |
|---|---|---|---|---|---|
| 濃墨 | フルアクセス＋`/health` 疎通＋機能検出 | ✓ | ✓ | ✓ | 全機能 |
| 中墨 | フルアクセス無し／網なし／429・5xx | ✓ | ✓ | ✗ | 出典・情調はオフライン辞書の範囲のみ |
| 淡墨 | 辞書破損等の異常 | ✓ | ✗ | ✗ | かな直書き＋旧字確定のみ |

- 状態遷移は打鍵を止めない。T2 失敗は表示の淡化のみで、再試行は次の明示引金まで待つ。
- `/health` の `features` キー（[protocol.md](./protocol.md) §4）で行打ちのサーバ解消
  可否を判定し、無ければ骨格引き（T1）の範囲で動く。

## 8. CI とパリティ

- `eval-ci.yml`（$0・依存ゼロの純関数テスト）に: `KANA_INFO`／`constraint_error` の
  Python テストと、`scripts/gen_swift_tables.py`・`gen_parity_vectors.py` の
  **生成物鮮度チェック**（再生成して `git diff --exit-code`）を追加。
- `ios-ci.yml` に: `swift test`（YatateCoreTests がパリティベクトルを読む）を追加。
  既存のシミュレータビルドは拡張ターゲットを自動的に含む（同一プロジェクト）。
- 既存の教訓をそのまま適用: workflow YAML はブロックスタイル、path フィルタに
  `ios/YatateCore/**`・`ios/YatateKeyboard/**`・生成スクリプトを**最初から**入れる
  （`deploy-backend.yml` の path 漏れ事故の再発防止）。
