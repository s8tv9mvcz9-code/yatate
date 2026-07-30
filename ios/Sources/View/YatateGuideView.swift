// 矢立の入口 — 有効化の案内と、鍵盤の作法の説明。
// このアプリは IME の器であり、ここには通信するものが一つも無い。

import SwiftUI
import YatateCore

struct YatateGuideView: View {
    @Environment(\.colorScheme) private var scheme

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 22) {
                    header
                    section("使ひ始める", [
                        ("1", "設定 → 一般 → キーボード → キーボード"),
                        ("2", "「新しいキーボードを追加」→ **矢立**"),
                        ("3", "文字を書く場所で、地球儀キーから矢立へ切り替へる"),
                    ])
                    fullAccessNote
                    section("鍵盤の作法", [
                        ("行", "行キーは縦書き二行に並ぶ。右列があ・か・さ・た・な、左列がは・ま・や・ら・わ。"),
                        ("段", "鍵を押へたまま**下へ滑らせる**と、あいうえおの梯子が降りる。離した段が入る。"),
                        ("濁", "右へ逸らすと濁点。濁点は字の右肩に打つもの、といふ作法に合はせてある。"),
                        ("小", "左へ逸らすと半濁点と小書き（ぱ・ゃ・っ）。"),
                        ("旧", "確定すると新字体は自動で旧字体になる（國・學・氣）。"),
                    ])
                    kehaiNote
                    Spacer(minLength: 8)
                }
                .padding(20)
            }
            .navigationTitle("矢立")
        }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("文語のための鍵盤")
                .font(.title2).bold()
            Text("歴史的仮名遣ひをそのまま打ち、旧字体で書くための入力法です。"
                 + "ゐ・ゑ が一動作で出ます。")
                .font(.subheadline)
                .foregroundStyle(.secondary)
        }
    }

    private func section(_ title: String, _ rows: [(String, String)]) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(title).font(.headline)
            ForEach(rows, id: \.0) { key, body in
                HStack(alignment: .top, spacing: 12) {
                    Text(key)
                        .font(.system(size: 15, weight: .semibold))
                        .frame(width: 26, height: 26)
                        .background(Sumi.fringe(scheme), in: RoundedRectangle(cornerRadius: 6))
                    Text(.init(body))
                        .font(.callout)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
        }
    }

    private var fullAccessNote: some View {
        VStack(alignment: .leading, spacing: 6) {
            Label("フルアクセスは不要です", systemImage: "lock.shield")
                .font(.headline)
            Text("矢立は許可しなくても全ての機能が働きます。打鍵を記録せず、通信もせず、"
                 + "他のアプリの文章も読みません。")
                .font(.footnote)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Sumi.slip(scheme), in: RoundedRectangle(cornerRadius: 10))
    }

    private var kehaiNote: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("墨の氣配").font(.headline)
            Text("鍵の濃淡は「次に来やすい文字」を示します。青空文庫の旧字旧仮名から数へた"
                 + "文語の連なりで、端末の中だけで求めてゐます。最有力の一手へは"
                 + "筆脈（淡い一画）が走ります。")
                .font(.footnote)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            Text("「文机」では、画面全体を五十音の紙面として一筆で書けます。")
                .font(.footnote)
                .foregroundStyle(.secondary)
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Sumi.slip(scheme), in: RoundedRectangle(cornerRadius: 10))
    }
}

#Preview {
    YatateGuideView()
}
