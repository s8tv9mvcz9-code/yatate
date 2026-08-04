import SwiftUI

/// 文語 IME「矢立」の器。
///   矢立 … 有効化の案内と鍵盤の作法（YatateGuideView）
///   文机 … 紙面ステートマシンの稽古場（FuzukueView — docs/ime/fuzukue.md）
///   原器 … ハード鍵盤の稽古場（GenkiHardKeyboardView）
///
/// 「原器」は外付け鍵盤でしか意味を持たない。キーボード拡張はハードキーを
/// 受け取れないので、これは本体アプリにしか置けない（docs/ime/cross-platform.md §4）。
///
/// **このアプリは通信しない。** 公開範囲を IME に絞つたため、サーバ校合を使ふ
/// 文語チャット・變換タブは同梱しない（コードはリポジトリに残してあり、Web 版と
/// Android 版が引き続き使ふ。ios/project.yml の excludes でビルドから外してゐる）。
struct RootView: View {
    var body: some View {
        TabView {
            YatateGuideView()
                .tabItem { Label("矢立", systemImage: "keyboard") }
            FuzukueView()
                .tabItem { Label("文机", systemImage: "square.grid.3x3") }
            GenkiHardKeyboardView()
                .tabItem { Label("原器", systemImage: "keyboard.badge.ellipsis") }
        }
    }
}

#Preview {
    RootView()
}
