// 矢立 macOS — 文机（紙面ステートマシン）の macOS 版。
//
// docs/ime/fuzukue.md §4 の主張「文机ビューの SwiftUI コードは両 OS でほぼ共有できる」
// を実地で確かめるための器。**FuzukueView も YatateCore も iOS 版と同一のソース**を
// そのまま使ふ（このファイルだけが macOS 固有）。
//
// 入力方式そのもの（IMKit の Input Method）は別物で、これはその前段——
// 紙面と運筆が macOS のポインタで成立するかを見るための先行像である。

import SwiftUI
import YatateCore

@main
struct YatateMacApp: App {
    var body: some Scene {
        WindowGroup("矢立 — 文机") {
            FuzukueView()
                .frame(minWidth: 720, minHeight: 460)
        }
        .defaultSize(width: 900, height: 560)
        .commands {
            CommandGroup(replacing: .newItem) {}   // 新規ウィンドウは要らない
        }
    }
}
