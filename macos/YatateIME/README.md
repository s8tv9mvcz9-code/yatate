# `macos/YatateIME/` — macOS の殻（IMKit テキストサービス）

**ハード鍵盤で原器を打つための実装である。** Windows の TSF 殻
（[`windows/`](../../windows/README.md)）と対になる。設計は
[`docs/ime/cross-platform.md`](../../docs/ime/cross-platform.md) §4、
配列は [`docs/ime/layout.md`](../../docs/ime/layout.md) §1。

iOS のキーボード拡張は**ハードキーを受け取れない**ので、原器（縦組五十音配列）を
物理鍵盤でそのまま打てる場は macOS と Windows しかない。

## これは殻である

配列（原器）も旧字確定も**一つも持たない**。すべて核（`YatateCore`）から来る。
ここが受け持つのは macOS の作法だけ —— IMKit の契約と、鍵盤の翻訳。

| ファイル | 中身 |
|---|---|
| `main.swift` | `IMKServer` を立てて待つ |
| `YatateInputController.swift` | `IMKInputController` — 打鍵を受け、marked text を置く |
| `Info.plist` | 入力ソースとしての名乗り（接続名・制御子・文字体系） |

## 鍵は kVK で引く — **英字（US）配列を前提とする**

`NSEvent.characters`（出る字）ではなく `NSEvent.keyCode`（Carbon の `kVK_ANSI_*`
＝ **物理位置**）で引く。Windows 殻が `ToUnicodeEx` を使はないのと同じ一手である。

```
        位置(kVK)  JIS の刻印   US の刻印
  さ    0x27       :            '
  し    0x29       ;            ;          ← 刻印が同じ
  ^     0x18       ^            =
```

「し」が最も危い。**刻印も物理位置も同じなのに意味だけが違ふ**ので、出る字で引くと
US 配列の機で黙つて「さ」になる。例外も警告も出ない。位置で引けばこの誤爆はそもそも
生まれず、「JIS か US か」を実行時に当てる必要も消える。

英字配列でも原器の 33 鍵が要る物理位置はすべて在るので、**配列ごとの分岐は要らない**。
地図の出所は核の [`Kagi.swift`](../../ios/YatateCore/Sources/YatateCore/Kagi.swift) 一枚で、
Rust の [`core/src/kagi.rs`](../../core/src/kagi.rs) がその SSOT である
（`KagiGenkiParityTests` が `core/vectors/kagi.json` で両者を縛る）。

## 打ち方（原器・英字配列）

```
  左半面（第二の行）              右半面（第一の行）
［1］［2］［3］［4］［5］    ［6］［7］［8］［9］［0］   ［=］前置シフト
 と   て   つ   ち   た      お   え   う   い   あ     （== で「ん」）
［q］［w］［e］［r］［t］    ［y］［u］［i］［o］［p］
 の   ね   ぬ   に   な      こ   け   く   き   か
［a］［s］［d］［f］［g］    ［j］［k］［l］［;］［'］    b＝濁点・v＝半濁点
 ほ   へ   ふ   ひ   は      そ   せ   す   し   さ     （いづれも後置打鍵）
```

JIS 機なら `=` は `^`、`'` は `:` と読み替へる（**同じ指の置き場**である）。

```
  u d g =y o 2 == o t =4   →   けふはよきてんきなり
  p b                      →   が （か＋濁点）
```

Enter か Space で確定。**確定のとき新字体は旧字体へ機械で直る**（核の `toKyuji`）。
Esc で取り消し、Delete で一字消し。

## 組んで入れる

`Package.swift` は `YatateCore` を macOS 13 以降で組めるやうにしてある。
入力方式は app bundle の形をしてゐる必要があるので、Xcode か `swiftc` で
`YatateIME.app` を作り、`~/Library/Input Methods/` へ置く。

```bash
# 例（Xcode を使はない最小の組み方）
mkdir -p YatateIME.app/Contents/MacOS
cp Info.plist YatateIME.app/Contents/
swiftc -O \
  -I ../../ios/YatateCore/.build/release/Modules \
  main.swift YatateInputController.swift \
  -framework Cocoa -framework InputMethodKit \
  -o YatateIME.app/Contents/MacOS/YatateIME
cp -R YatateIME.app ~/Library/Input\ Methods/
```

そのあと **ログアウトして入り直し**、システム設定 → キーボード → 入力ソース →
「＋」→ 日本語 →「矢立（文語 IME）」を足す。`Ctrl+Space` で切り替へる。

> 未署名である。手元の検証はこれで通るが、配るには署名と公証が要る。

## うまくいかないときの見どころ

| 症状 | 見るところ |
|---|---|
| 入力ソース一覧に出ない | `~/Library/Input Methods/` に在るか／ログインし直したか／`Info.plist` の `ComponentInputModeDict` |
| 切り替へても何も起きない | `InputMethodConnectionName` と `IMKServer` に渡す名が一致してゐるか |
| 「し」が「さ」になる | `NSEvent.characters` を見てゐる経路が混ざつてゐないか（この殻は `keyCode` だけを見る） |
| `=` を打つと `=` が入る | 前置シフトの打鍵を食ひ損ねてゐる（`.swallow` を `false` で返してゐないか） |

## まだ無いもの（正直に）

| 項目 | 現況 |
|---|---|
| **漢字変換・候補窓** | **無い。** 確定は仮名のまま（旧字は機械で直る） |
| 表示属性（未確定の下線・色） | `kTSMHiliteSelectedRawText` のみ。共感覚（情調 → 伝統色）は未着手 |
| 署名・公証 | 未署名 |

### 変換をどう入れるか — 四枚目の手写しは作らない

変換の状態機械は**すでに Rust の核に在る**（[`core/src/henkan.rs`](../../core/src/henkan.rs)：
段の管理・候補送り・区切り修正）。辞書引きと文節分割も同様
（`core/src/jisho.rs`・`core/src/bunsetsu.rs`、合はせて約 900 行）。

これを Swift へ手で写せば**四枚目の地図**になり、このリポジトリが繰り返し
学んできた「二枚持つと必ずずれる」を自ら踏むことになる。`Kagi` と `Genki` は
表が小さく黄金ベクトルで完全に縛れるので写したが、格子探索と費用計算は性質が違ふ。

筋は **Rust の核を静的ライブラリとして繋ぐ**ことである（web が wasm で
さうしてゐるのと同じ形で、入出力は「文字列を入れて文字列を貰ふ」だけで足りる）。
`aarch64-apple-darwin` / `aarch64-apple-ios` は cargo の標準ターゲットなので、
道具立ても増えない。
