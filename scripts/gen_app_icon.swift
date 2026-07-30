#!/usr/bin/env swift
// gen_app_icon.swift — アプリアイコン（1024×1024 PNG）を生成する。
// 意匠: 墨色の紙面に、縦組の白抜き「文」。矢立＝懐中の筆記具の主題に合はせ、
// 右側に朱の一画（筆脈）を添へる。小さい寸法でも潰れぬやう字画は太めに。
//
//   swift scripts/gen_app_icon.swift ios/Sources/Assets.xcassets/AppIcon.appiconset/icon-1024.png

import AppKit
import CoreGraphics
import Foundation

let outPath = CommandLine.arguments.count > 1
    ? CommandLine.arguments[1] : "icon-1024.png"
let S: CGFloat = 1024

guard let ctx = CGContext(
    data: nil, width: Int(S), height: Int(S), bitsPerComponent: 8, bytesPerRow: 0,
    space: CGColorSpace(name: CGColorSpace.sRGB)!,
    bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue
) else { fatalError("context") }

// 地: 墨から藍鼠へのごく浅い縦のぼかし（App Store はアルファ不可ゆゑ不透明で塗る）
let cs = CGColorSpace(name: CGColorSpace.sRGB)!
let grad = CGGradient(colorsSpace: cs, colors: [
    CGColor(srgbRed: 0.13, green: 0.15, blue: 0.19, alpha: 1),
    CGColor(srgbRed: 0.21, green: 0.24, blue: 0.30, alpha: 1),
] as CFArray, locations: [0, 1])!
ctx.drawLinearGradient(grad, start: CGPoint(x: 0, y: S), end: CGPoint(x: 0, y: 0),
                       options: [])

// 紙面の目安となる縦罫（極薄）。五十音の紙面といふ主題の名残り。
ctx.setStrokeColor(CGColor(srgbRed: 1, green: 1, blue: 1, alpha: 0.055))
ctx.setLineWidth(2)
for i in 1..<5 {
    let x = S * CGFloat(i) / 5
    ctx.move(to: CGPoint(x: x, y: S * 0.10))
    ctx.addLine(to: CGPoint(x: x, y: S * 0.90))
}
ctx.strokePath()

// 朱の一画（筆脈）— 右肩から左下へ、わづかに撓ませる
ctx.setStrokeColor(CGColor(srgbRed: 0.78, green: 0.25, blue: 0.20, alpha: 0.95))
ctx.setLineWidth(26)
ctx.setLineCap(.round)
ctx.move(to: CGPoint(x: S * 0.80, y: S * 0.82))
ctx.addQuadCurve(to: CGPoint(x: S * 0.72, y: S * 0.20),
                 control: CGPoint(x: S * 0.94, y: S * 0.50))
ctx.strokePath()

// 「文」— 明朝体で大きく白抜き
let nsctx = NSGraphicsContext(cgContext: ctx, flipped: false)
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = nsctx
let font = NSFont(name: "HiraMinProN-W6", size: S * 0.62)
    ?? NSFont(name: "HiraginoSans-W6", size: S * 0.62)
    ?? NSFont.systemFont(ofSize: S * 0.62, weight: .semibold)
let attrs: [NSAttributedString.Key: Any] = [
    .font: font,
    .foregroundColor: NSColor(srgbRed: 0.97, green: 0.97, blue: 0.95, alpha: 1),
]
let str = NSAttributedString(string: "文", attributes: attrs)
let size = str.size()
// 朱の一画を避けてやや左へ寄せる
str.draw(at: NSPoint(x: (S - size.width) / 2 - S * 0.045,
                     y: (S - size.height) / 2 - S * 0.02))
NSGraphicsContext.restoreGraphicsState()

guard let img = ctx.makeImage() else { fatalError("image") }
let rep = NSBitmapImageRep(cgImage: img)
guard let png = rep.representation(using: .png, properties: [:]) else { fatalError("png") }
try! png.write(to: URL(fileURLWithPath: outPath))
print("wrote \(outPath) (1024×1024, opaque)")
