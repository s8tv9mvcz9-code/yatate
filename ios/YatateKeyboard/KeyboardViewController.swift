// 矢立 — キーボード拡張の入口（M0: 楷の骨格のみ。docs/ime/ 参照）。
// フルアクセス不要・ネットワーク不使用。textDocumentProxy への insertText だけを行ふ。

import SwiftUI
import UIKit
import YatateCore

final class KeyboardViewController: UIInputViewController {
    private let state = KeyboardState()

    override func viewDidLoad() {
        super.viewDidLoad()

        state.onInsert = { [weak self] text in
            self?.textDocumentProxy.insertText(text)
        }
        state.onDeleteBackward = { [weak self] in
            self?.textDocumentProxy.deleteBackward()
        }
        state.onGlobe = { [weak self] in
            self?.advanceToNextInputMode()
        }

        let host = UIHostingController(rootView: FutakudariView(state: state))
        addChild(host)
        host.view.translatesAutoresizingMaskIntoConstraints = false
        host.view.backgroundColor = .clear
        view.addSubview(host.view)
        NSLayoutConstraint.activate([
            host.view.topAnchor.constraint(equalTo: view.topAnchor),
            host.view.bottomAnchor.constraint(equalTo: view.bottomAnchor),
            host.view.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            host.view.trailingAnchor.constraint(equalTo: view.trailingAnchor),
        ])
        host.didMove(toParent: self)

        // キーボードの高さ（作業帯 + 5 段の行キー）
        let height = view.heightAnchor.constraint(equalToConstant: 300)
        height.priority = UILayoutPriority(999)
        height.isActive = true
    }

    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        // 地球儀キーは必要なときだけ出す（審査 4.4.1「次のキーボードへ進む手段」）
        state.needsGlobe = needsInputModeSwitchKey
    }
}
