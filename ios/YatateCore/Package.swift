// swift-tools-version:5.9
// YatateCore — 矢立（文語 IME）の決定的核。本体アプリと拡張が共有する（docs/ime/architecture.md §1）。
// macOS を含むのは swift test をホストで直接走らせるためと、遠景の IMKit 版のため。
import PackageDescription

let package = Package(
    name: "YatateCore",
    platforms: [.iOS(.v16), .macOS(.v13)],
    products: [
        .library(name: "YatateCore", targets: ["YatateCore"]),
    ],
    targets: [
        .target(name: "YatateCore", path: "Sources/YatateCore"),
        .testTarget(name: "YatateCoreTests", dependencies: ["YatateCore"],
                    path: "Tests/YatateCoreTests"),
    ]
)
