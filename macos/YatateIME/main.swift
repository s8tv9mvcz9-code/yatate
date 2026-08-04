// 矢立 macOS IME の入口 — IMKServer を立てて待つだけ。
//
// IMKit の入力方式は「常駐する小さな NSApplication」である。
// 接続名（Info.plist の InputMethodConnectionName）で OS と繋がり、
// 入力欄ごとに YatateInputController が作られる。

import Cocoa
import InputMethodKit

let bundle = Bundle.main

guard
    let connectionName = bundle.infoDictionary?["InputMethodConnectionName"] as? String,
    let identifier = bundle.bundleIdentifier
else {
    NSLog("矢立: Info.plist に InputMethodConnectionName / CFBundleIdentifier が無い")
    exit(1)
}

// 保持しておかないと解放されて接続が切れる（IMKServer は自分で自分を保たない）。
let server = IMKServer(name: connectionName, bundleIdentifier: identifier)

NSLog("矢立: IMKServer 起動 — \(connectionName)")
_ = server

NSApplication.shared.run()
