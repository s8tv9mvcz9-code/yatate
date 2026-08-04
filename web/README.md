# `web/` — 矢立の web 殻（wasm）

**鍵盤だけを web に出し、漢字は手で直させ、その直しを覚える。**
設計は [`docs/ime/web.md`](../docs/ime/web.md)。

```bash
./build.sh --serve      # 組んで http://localhost:8000/ で配る
```

道具は **cargo と python3 だけ**。`wasm-pack` も `npm` も要らない。

## 何が入つてゐるか

| ファイル | 中身 |
|---|---|
| `src/lib.rs` | **素の `extern "C"`** で核を JS へ出す。ここに頭脳は無い |
| `index.html` / `style.css` | 紙・候補窓・原器の図・覚えたことの一覧 |
| `yatate.js` | 鍵を取る・描く・覚えたことを仕舞ふ。**配列表も辞書も持たない** |
| `check_wasm.py` | 組み上がつた wasm を検める（下記） |
| `build.sh` | 組む → 検める → `dist/` へ揃へる |

## なぜ wasm-bindgen を使はないのか

核の入出力は「文字を入れる／文字列を貰ふ」だけである。それだけなら
`extern "C"` で足り、道具立てが cargo だけで済む。CI は ubuntu（課金 1 倍）のまま増えない。

文字列は wasm の線形メモリを介して渡す:

```text
JS → wasm : yatate_alloc(len) で場所を貰ひ、UTF-8 を書き込んで渡す
wasm → JS : 関数は長さを返す。中身は yatate_out_ptr() の先に置いてある
```

## 鍵は `code` で取る

`KeyboardEvent.key`（出る文字）ではなく `code`（物理位置）で引く。
原器は指の位置の地図だからで、`key` を見ると JIS と US で意味の入れ替はる三鍵が壊れる
——とりわけ「し」は刻印も物理位置も同じなので、黙つて「さ」になる。

地図は**核が一枚だけ持つ**（`core/src/kagi.rs`）。この殻は写しを作らず、
核の表をそのまま使ふ。Windows 殻は自分の表を核へ照合する試験を持つ。
`core/vectors/kagi.json` が両方を縛る。

原器の図も `yatate_layout()` が返す表を描くだけなので、
**図と実際の打鍵はずれやうが無い**。

## 組み上がつた wasm を検める

`check_wasm.py` が二つを見る。Windows 殻の CI が実 DLL のエクスポート表を
検べてゐるのと同じ役回りである。

1. **出口が揃つてゐるか** — `#[no_mangle]` を書き忘れると、頁は「関数が undefined」で黙つて死ぬ。
2. **入口が一つも無いか** — こちらが本命。wasm に `import` が無いといふことは、
   **外の世界の関数を一つも呼べない**といふことで、通信も保存も出来ない。
   「通信するコードを持たない」といふ公開の約束（[`docs/privacy.md`](../docs/privacy.md)）が、
   作文ではなくバイナリの性質として立つ。

```
[OK] 出口 18 件・import 無し（119,797 バイト）
```

## 試験

```bash
cargo test                                  # C ABI・打鍵・学習の助け（host で回る）
cargo build --release --target wasm32-unknown-unknown
python3 check_wasm.py
```

試験は**すべて Rust 側**にある。JS には頭脳が無いので試験する対象が薄い
——これは意図した配分である。

## 配る

`dist/` は静的な四つのファイルだけ（頁・JS・CSS・wasm）なので、
GitHub Pages にそのまま置ける。サーバへ何も送らない。

> `file://` では開けない。`fetch` が wasm を撥ねるので、`--serve` か
> 任意の静的サーバを使ふこと。
