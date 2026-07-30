# 帰属表示 / Attribution

本プロジェクト本体のライセンスは [MIT](./LICENSE)。以下は同梱・利用するデータと、
将来依存する第三者ソフトウェアの帰属である。免責は [DISCLAIMER.md](./DISCLAIMER.md)。

## コーパス

- **青空文庫（Aozora Bunko）** — https://www.aozora.gr.jp/
  文体手本・候補の実例・IME 辞書は、**著作権保護期間が満了した作品**に由来する。
  取得時にカタログの著作権フラグで機械的に絞り込んでゐる（`scripts/gen_yatate_ngram.py`）。
  本文そのものは同梱せず、**仮名の連なりの統計**（どの字の次にどの字が来やすいか）
  だけを数へて用ゐる。

## 第三者ソフトウェア

現時点でリポジトリに同梱してゐる第三者コードは無い。以下は
[docs/ime/architecture.md](./docs/ime/architecture.md) で採用を予定するもの:

| 対象 | ライセンス | 扱ひ |
|---|---|---|
| AzooKeyKanaKanjiConverter | MIT | 依存として利用予定（未導入） |
| azooKey 既定辞書データ | Apache-2.0 | 改変再配布可。利用時は表示義務のみ |
| zenz モデル重み | CC-BY-SA-4.0 | **同梱しない**（表示＋継承義務があるため実験枠に隔離） |

## 商標

Apple、Xcode、iOS、その他の名称・商標は各権利者に帰属する。
本プロジェクトはこれらの団体の公認・提携によるものではない。
