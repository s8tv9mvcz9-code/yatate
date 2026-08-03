# `windows/` — Windows の殻（TSF テキストサービス）

**JIS 物理鍵盤で原器を打つための実装である。** 設計は
[`docs/ime/cross-platform.md`](../docs/ime/cross-platform.md) §4・M7、
配列は [`docs/ime/layout.md`](../docs/ime/layout.md) §1。

iOS のキーボード拡張は**ハードキーを受け取れない**ので、原器（縦組五十音配列）を
物理鍵盤でそのまま打てる場は macOS と Windows しかない。
つまりこれは「iOS の劣化移植」ではなく、**設計の原点を実装する**仕事である。

## これは殻である

配列（原器）・墨の氣配・旧字確定は**一つも持たない**。すべて核（[`core/`](../core/README.md)）から来る。
ここが受け持つのは Windows の作法だけ —— COM、TSF の契約、鍵盤の翻訳。

| module | OS 依存 | 中身 |
|---|---|---|
| `Session`（核から再エクスポート） | **なし** | 打鍵 → 未確定文字列の状態機械 |
| `keymap` | **なし**（整数のみ） | **JIS 物理鍵盤 → 原器**。配列で動く 3 鍵を物理位置で縛る |
| `registration` | 値は無依存／書込は Windows | CLSID・プロファイル・カテゴリ |
| `dll` | Windows | COM の四つの出口（`DllGetClassObject` 等） |
| `factory` | Windows | `IClassFactory` |
| `tip` | Windows | `ITfTextInputProcessor` ＋ `ITfKeyEventSink` |
| `edit` | Windows | 未確定文字列を文書へ置く編集セッション |

## 鍵盤の翻訳が肝である（`keymap`）

原器は「`0` が あ、`:` が さ」のやうに**文字**で定義されてゐるが、
TSF が寄越すのは**仮想キーコード**（VK）と**走査符号**（物理位置）である。
そして**写像は配列によつて動く**。

```
        物理位置      JIS の VK              US の VK
  さ    sc 0x28   VK_OEM_1   (0xBA)     VK_OEM_7   (0xDE)   ← 刻印は「:」対「'」
  し    sc 0x27   VK_OEM_PLUS(0xBB)     VK_OEM_1   (0xBA)   ← **刻印は両方「;」**
  ^     sc 0x0D   VK_OEM_7   (0xDE)     VK_OEM_PLUS(0xBB)   ← 刻印は「^」対「=」
```

「し」が最も危い。**刻印も物理位置も同じなのに VK だけが違ふ**ので、
世間の（ほぼ US 前提の）VK 表を写すと、実機で「し」が黙つて「さ」になる。
例外も警告も出ない。

そこで矢立は、**配列で意味の変はる 3 鍵を走査符号（物理位置）で引く**。
原器はもともと「指の位置の地図」なので、これは思想とも噛み合ふ。
おかげで「JIS か US か」を実行時に当てる必要が消え、当て損なひが無音の誤爆になる箇所が
そもそも無くなる。残る 30 鍵（数字・英字）は全配列共通なので VK で引いてよい。

`ToUnicodeEx`（「今の配列で何の字が出るか」）は**使はない**。
かなロック（VK_KANA）が立つと `:` ではなく半角カタカナ `ｹ` が返り、
文字を鍵にした写像表は全面的に破綻するからである。加へて `OnTestKeyDown` は
**副作用禁止**の契約であり、`ToUnicodeEx` はデッドキー緩衝を書き換へ得る。

## 打ち方（原器・JIS 鍵盤）

```
  左半面（第二の行）              右半面（第一の行）
［1］［2］［3］［4］［5］    ［6］［7］［8］［9］［0］   ［^］前置シフト
 と   て   つ   ち   た      お   え   う   い   あ     （^^ で「ん」）
［q］［w］［e］［r］［t］    ［y］［u］［i］［o］［p］
 の   ね   ぬ   に   な      こ   け   く   き   か
［a］［s］［d］［f］［g］    ［j］［k］［l］［;］［:］    b＝濁点・v＝半濁点
 ほ   へ   ふ   ひ   は      そ   せ   す   し   さ     （いづれも後置打鍵）
```

第二面（`^` の後）は ま・や・ら・わ行。**ゐ・ゑ が第一級の鍵を持つ**（`^r`・`^w`）。

```
  u d g ^y o 2 ^^ o t ^4   →   けふはよきてんきなり
  p b                      →   が （か＋濁点）
```

Enter か Space で確定。**確定のとき新字体は旧字体へ機械で直る**（核の `to_kyuji`）。
Esc で取り消し、BackSpace で一字消し。

## 検証 — どこまで機械で守れてゐるか

**Windows 機が無くても、実機に触れる直前まで機械で守れる。**

| 関門 | 何を捕まへるか | どこで回るか |
|---|---|---|
| `cargo test` | 原器の全 33 鍵の往復・JIS/US の取り違へ・打鍵列 → 仮名 → 確定 | **ubuntu** |
| `cargo clippy`（両ターゲット） | 危い書き方 | ubuntu |
| `cargo fmt --check` | 体裁 | ubuntu |
| `cargo build --target x86_64-pc-windows-gnu` | **実 DLL としてリンクが通るか**（COM の vtable まで） | ubuntu（MinGW） |
| **エクスポート表の検査** | `DllGetClassObject` 等が実在するか＝**「読み込まれない DLL」の防止** | ubuntu（objdump） |
| MSVC ビルド（x64・**ARM64**） | 配る形で組めるか | windows ランナー |

```bash
cd windows
cargo test                                    # 頭脳と鍵盤の翻訳
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo build --release --target x86_64-pc-windows-gnu
x86_64-w64-mingw32-objdump -p target/x86_64-pc-windows-gnu/release/yatate_windows.dll | grep Dll
```

`rustup target add x86_64-pc-windows-gnu` と `apt install mingw-w64` が要る。
CI（[`windows-ci.yml`](../.github/workflows/windows-ci.yml)）が同じ関門を回す。

### 機械で守れないもの

**「TSF に読まれて実際に打てるか」だけ**が残る。これは Windows 実機でしか確かめられない
——COM 登録が効くか、鍵盤の目が刺さるか、composition が描かれるか、
アプリごとの癖（Word・Chrome・メモ帳）に嵌まらないか。

## 実機での確認

配布物は GitHub Releases の **`windows-latest`** に置いてある（`x64` と `arm64` の zip）。

> **⚠ 機械の種別に合はせること。** ARM64 機のネイティブ処理（メモ帳・Edge 等）に
> **x64 の DLL は読み込まれない**。逆も同じ。`install.ps1` が入れる前に確かめて止める。

```powershell
# 管理者の PowerShell で
Expand-Archive yatate-windows-arm64.zip -DestinationPath .\yatate
cd .\yatate
.\install.ps1              # 種別の確認 → Program Files へ写す → regsvr32 で登録
# 取り外しは .\install.ps1 -Uninstall
```

そのあと:

1. 設定 → 時刻と言語 → 言語と地域 → 日本語 →「…」→ 言語のオプション → キーボード
   に「矢立（文語 IME）」が居るか（居なければ「キーボードの追加」から選ぶ）
2. `Win+Space` で矢立へ切り替へ
3. メモ帳で `u d g ^y o 2 ^^ o t ^4` と打つ → **けふはよきてんきなり**

### うまくいかないときの見どころ

| 症状 | 見るところ |
|---|---|
| 一覧に出てこない | 管理者で `regsvr32` したか／機械と DLL の種別（ARM64 対 x64） |
| 切り替へても何も起きない | `HKCR\CLSID\{3F3A263D-…}\InprocServer32` が DLL の実在する道を指してゐるか |
| 「し」が「さ」になる | 走査符号が来てゐない経路（`keymap` の予備が JIS の VK として読む段）。実機の `lParam` を見る |
| 一部のアプリだけ動かない | そのアプリが TSF ではなく IMM32 を使つてゐる可能性（互換層の調査が要る） |
| 打つと二重に入る | `OnTestKeyDown` と `OnKeyDown` の食ひ違ひ（食ふ判断が両者で違ふ） |

## まだ無いもの（正直に）

| 項目 | 現況 |
|---|---|
| 候補窓（かな → 漢字の変換） | **無い。** 確定は仮名のまま（旧字は機械で直る）。辞書層（T1）が入つてから |
| 表示属性（未確定の下線・色） | GUID だけ用意。**共感覚（情調 → 伝統色）**をここに載せる余地を残してある |
| 署名 | 未署名。手元検証は通るが、配布には要る（OSS は SignPath Foundation の道） |
| 記号・小書き（ゃゅょっ） | 原器で**未定**ゆゑ実装しない（発明しない） |

## 依存の置き方（注意）

COM 側の依存は **`[target.'cfg(windows)'.dependencies]`** に置いてある。
ここへ入れておかないと `windows` crate の連れてくるものが Linux で壊れ、
`Session` と `keymap` の試験まで道連れになる——それでは層を切り分けた意味が無い。
