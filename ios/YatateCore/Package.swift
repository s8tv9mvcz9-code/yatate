// swift-tools-version:5.9
// YatateCore — 矢立（文語 IME）の決定的核への入口。本体アプリ・鍵盤拡張・macOS の殻が共有する。
//
// **中身は Rust の核（core/）である**（M5-b2・docs/ime/cross-platform.md §10）。
// ここに在る Swift は「核を呼ぶ薄い層」で、配列も旧字の表も氣配の重みも持たない。
// 束縛は素の C ABI（apple/src/lib.rs）で、uniffi は使はない——道具立ては cargo だけ。
//
// ## 先に静的ライブラリを組むこと
//
//     ./scripts/build-apple-ffi.sh        # YatateFFI.xcframework が出来る
//     cd ios/YatateCore && swift test
//
// xcframework は生成物なので git に入れてゐない（.gitignore）。
// 無いまま `swift test` を叩くと SPM が「artifact が無い」と言つて止まる——
// それは壊れてゐるのではなく、上の一行を先に走らせよ、といふ意味である。
import PackageDescription

let package = Package(
    name: "YatateCore",
    platforms: [.iOS(.v16), .macOS(.v13)],
    products: [
        .library(name: "YatateCore", targets: ["YatateCore"]),
    ],
    targets: [
        // 核（Rust）の静的ライブラリ。macos / ios / ios-simulator の三枝を持つ。
        .binaryTarget(name: "YatateFFI", path: "YatateFFI.xcframework"),
        .target(name: "YatateCore", dependencies: ["YatateFFI"], path: "Sources/YatateCore"),
        .testTarget(name: "YatateCoreTests", dependencies: ["YatateCore"],
                    path: "Tests/YatateCoreTests"),
    ]
)
