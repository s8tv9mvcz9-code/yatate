# `android/` — Android の殻（`InputMethodService`）

設計は [`docs/ime/cross-platform.md`](../docs/ime/cross-platform.md) §4・M8。

## Android だけ核を手で写さない

| 殻 | 核との繋がり | 変換（かな → 漢字） |
|---|---|---|
| Windows | Rust をそのままリンク | **在る** |
| web | Rust を wasm で | **在る** |
| **Android** | **Rust を JNI で** | **在る** |
| iOS / macOS | Swift の写し（黄金ベクトルで拘束） | 無い（要 FFI） |

iOS / macOS の殻は Swift なので、核の一部（`Kagi`・`Genki`・`Session`）を写して
黄金ベクトルで縛つてゐる。表が小さく、機械で完全に縛れるからである。

Android は `InputMethodService` が Kotlin だが、**共有ライブラリを読める**。
ゆゑに核をそのまま呼べる。すると `henkan`・`bunsetsu`・`jisho`（約 900 行、
格子探索と費用計算を含む）が丸ごと載る——**初日から漢字変換が使へる**のはこの一点による。

```
  android/rust/   JNI の橋（ここに頭脳は無い）→ libyatate_android.so
  android/app/    IMS の殻・二行配列（ここにも頭脳は無い）
```

## 打ち方（二行配列）

硝子の鍵盤である。**行を押して、書き下ろすやうに滑らせて段を選ぶ。**

```
   第二の行（左）        第一の行（右）
     は ま や ら わ        あ か さ た な
```

- **縦に滑らせる** … 段（あ→い→う→え→お）。押した位置が「あ段」
- **右へ逸らす** … 濁点（か→が）。字の右肩に打つものだから右である
- **左へ逸らす** … 半濁点（は→ぱ）。は行だけが持つ
- **指を離す** … その仮名が入る

下段の四つ:

| 鍵 | 働き |
|---|---|
| 変換 | 未変換なら変換、変換中は次の候補へ |
| 次候補 | 次の候補へ |
| 削除 | 未確定があれば一字消し、無ければアプリの一字消し |
| 確定 | 確定（**ここで新字体は旧字体へ機械で直る**） |

作業帯の候補は触れると選べる。

**この鍵盤は配列表を持たない。** 描くのは核が返した五十音表（`Core.layout()`）だけで、
「図と実際の打鍵がずれる」事故は起きやうが無い（web の殻と同じ作法）。

### 外付け鍵盤（原器）

Android は iOS のキーボード拡張と違ひ**ハードキーを受け取れる**ので、
`Space` で変換・`Enter` で確定・`←→` で文節・`Shift+←→` で区切り修正、が
Windows 殻と同じ手つきで効く。

## 権限を一つも要求しない

`AndroidManifest.xml` に `uses-permission` が一行も無い。通信もしないし記憶も
持ち出さない（[`docs/privacy.md`](../docs/privacy.md)）。IME は打つた字がすべて
通る場所なので、「要らないものを求めない」ことが最も強い約束になる。

## 組む

**すべて ubuntu で $0 で回る**（[`android-ci.yml`](../.github/workflows/android-ci.yml)）。

```bash
cd android/rust
cargo test                       # 橋の論理（host で回る）
cargo clippy --all-targets -- -D warnings
cargo fmt --check

cargo install cargo-ndk
rustup target add aarch64-linux-android x86_64-linux-android
cargo ndk -t arm64-v8a -t x86_64 -o ../app/src/main/jniLibs build --release

cd ..
gradle :app:testDebugUnitTest
gradle :app:assembleDebug
```

CI が同じ関門を回し、加へて

- **出口表の検査** — `Java_jp_yatate_ime_Core_*` が `.so` に実在するか
  （＝端末で初めて `UnsatisfiedLinkError` に気づくのを防ぐ）
- **APK の中身の検査** — `lib/arm64-v8a/` と `lib/x86_64/` に `.so` が入つてゐるか

を見る。Windows 殻が実 DLL のエクスポート表を検べ、web 殻が wasm の import を
検べてゐるのと同じ役回りである。

> Gradle の wrapper（`gradlew` と jar）は**置いてゐない**。バイナリを配らずに済むので、
> CI は `gradle/actions/setup-gradle` が入れた `gradle` を直に呼ぶ。手元でも
> Gradle 8.9 以上を入れて `gradle` で叩けばよい。

## 実機で確かめる

配布物は GitHub Releases の **`android-latest`** に置いてある（`yatate-android.apk`）。
`main` へ入るたびに同じ URL で差し替はるので、拾ふ側は URL を覚えるだけでよい
（Windows 殻の `windows-latest` と同じ流儀）。**未署名の debug ビルド**である。

```bash
gh release download android-latest --repo s8tv9mvcz9-code/yatate --pattern 'yatate-android.apk'
adb install -r yatate-android.apk
```

そのあと 設定 → システム → 言語と入力 → 画面キーボード →「矢立（文語 IME）」を有効にし、
入力欄で鍵盤の切替から選ぶ。

```
  は→ひ→ふ… と滑らせて「けふはよきてんきなり」→ 変換 → 確定
                                              → 今日は良き天氣なり
```

## まだ無いもの（正直に）

| 項目 | 現況 |
|---|---|
| 署名・Play 配布 | 未署名の debug のみ。upload key は未設定 |
| **物理配列の取り違への検出** | 外付け鍵盤は `KeyEvent.unicodeChar`（＝出る字）で引いてゐる。**US 配列の機で「し」が「さ」になり得る**。核は `kagi` に位置の地図を持つてゐるので、`getKeyCharacterMap` から走査符号を取つて位置で引く形へ直すべき（M8-b） |
| 墨の氣配（次の一打の濃淡） | 核は `field()` を持つてゐるが、まだ橋に出してゐない |
| 設定画面 | 無い。IME の有効化は OS の設定から |
| 候補窓の作り込み | 作業帯に先頭四つを出すだけ。文節ごとの下線の引き分けも未実装 |
