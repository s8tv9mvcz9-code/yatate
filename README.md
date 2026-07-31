# 矢立（Yatate）— 文語 IME

**歴史的仮名遣ひをそのまま打ち、旧字体で書くための iOS キーボード。**
青空文庫の著作権満了作品から作つた統計と辞書を同梱し、**すべて端末の中だけで動く**。
通信するコードを持たない（[プライバシーポリシー](docs/privacy.md)）。

矢立とは、筆と墨壺を一つに納めて腰に差した携帯筆記具のこと。

## 何が違ふのか

市販の IME は歴史的仮名遣ひに無力で、「けふはよきてんきなり」をまともな漢字混じり文に
できない。ゐ・ゑ を打つのにも迂回が要る。矢立はその不便を正面から解く。

- **二行（ふたくだり）配列** — 行キーを縦書き二行に並べる。右列があ・か・さ・た・な、
  左列がは・ま・や・ら・わ。日本語の文がもともと流れる向きを、入力の座標系に採る。
- **書き下ろし** — 鍵を押へたまま下へ滑らせると、あいうえおの梯子が降りる。
  **右への逸らしが濁点**——濁点は字の右肩に打たれるもの、といふ作法と同型にした。
- **ゐ・ゑ が第一級** — わ行の 1 段目・3 段目として一動作で書ける。
- **旧字体は機械が確定する** — 新字体→旧字体は 1:1 の写像なので、
  モデルの気分に任せず 248 字の表で決める（國・學・氣・體）。
- **墨の氣配** — 鍵の濃淡が「次に来やすい文字」を示す。青空文庫の旧字旧仮名から
  数へた文語の連なりで、端末内で求める。最有力の一手へは**筆脈**（淡い一画）が走る。
- **文机（ふづくえ）** — 画面全体を五十音の紙面とし、一筆で連ねて書く稽古場。

## 設計

思想から実装まで [`docs/ime/`](docs/ime/README.md) に置いてある。

| 文書 | 内容 |
|---|---|
| [README](docs/ime/README.md) | 全体像と設計思想 |
| [layout](docs/ime/layout.md) | 配列 — 原器（PC 縦組五十音配列）と二行配列、楷・行・草 |
| [architecture](docs/ime/architecture.md) | 三層（決定的核／端末内辞書／任意のサーバ校合）、メモリ予算 |
| [vla](docs/ime/vla.md) | 鍵盤を行為空間として見る — 墨の氣配・筆脈・滲み |
| [fuzukue](docs/ime/fuzukue.md) | 紙面ステートマシン（macOS 版の先行像） |
| [interaction](docs/ime/interaction.md) | 候補 UI・触覚・先行研究との位置づけ |
| [protocol](docs/ime/protocol.md) | 変換 API の契約（任意のサーバ校合を使ふ場合） |
| [roadmap](docs/ime/roadmap.md) | 工程・リスク・ライセンス整理 |

## つくる

```bash
brew install xcodegen
cd ios && xcodegen generate      # Yatate.xcodeproj が出来る
```

```bash
cd ios/YatateCore && swift test  # 決定的核の検証（シミュレータ不要）
cd ios && ./build-device.sh      # 実機へ署名ビルド＋導入
```

**共有核（`core/`・Rust）** — macOS・Windows・Android へ広げるための一つの核。
Mac は要らず ubuntu で回る（[設計](docs/ime/cross-platform.md)）:

```bash
cd core && cargo test            # 単体 ＋ 黄金ベクトル（Python SSOT との一致）
```

機械生成物は Python が SSOT で、Swift と Rust は写し。SSOT を変へたら再生成する
（CI が鮮度を検査して、写しが古いままなら落とす）:

```bash
python3 scripts/gen_swift_tables.py     # ssot/kyuji.py → Generated/KyujiTable.swift
python3 scripts/gen_rust_tables.py      # ssot/kyuji.py → core/src/generated/kyuji_table.rs
python3 scripts/gen_parity_vectors.py   # ssot/kyuji.py → core/vectors/kyuji.json（黄金ベクトル）
python3 scripts/gen_yatate_ngram.py     # 青空文庫      → Generated/KanaBigram.swift
```

## 構成

```
core/               共有核（Rust・依存ゼロ）。全 OS の殻が使ふ決定的な部分
                    ＋ vectors/（黄金ベクトル＝Python SSOT との一致を縛る表）
ios/
  Sources/          ホストアプリ（有効化の案内・文机）。通信コードは無い
  YatateCore/       iOS の決定的核（行×段の地図・旧字変換・墨の氣配）。アプリと拡張が共有
  YatateKeyboard/   キーボード拡張（com.apple.keyboard-service）
ssot/               新字体→旧字体 248 字の対応表（Swift・Rust 双方の生成元）
scripts/            テーブル生成・黄金ベクトル生成・アイコン生成・リリース仕込み
docs/ime/           設計一式
```

## ライセンスと帰属

[MIT](LICENSE)。コーパスは青空文庫の著作権保護期間満了作品——
帰属と第三者ソフトウェアの扱ひは [NOTICE](NOTICE.md)、免責は [DISCLAIMER](DISCLAIMER.md)。

## リリース

タグを打つだけで App Store Connect まで無人で上がる:

```bash
git tag v1.0.1 && git push origin v1.0.1
```

`.github/workflows/ios-release.yml` が署名・検証・アップロードまで行ふ。
審査への提出だけは App Store Connect の画面で押す（TestFlight は使はない）。
初回の仕込みは `scripts/setup-asc-secrets.sh`（App Store Connect API キーを
一度だけ用意すれば、以後の人手は不要）。
