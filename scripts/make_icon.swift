// Draws the icons: "নু." on a rounded badge, like the system's "A" for ABC.
//   swift scripts/make_icon.swift Resources/icon.tiff           menu-bar icon: template TIFF, @1x and @2x
//   swift scripts/make_icon.swift --app Resources/AppIcon.icns  app icon for Finder: white badge on a fire gradient
import AppKit

let glyph = "নু."
let font = "KohinoorBangla-Semibold"   // ships with macOS

/// Menu-bar icon size in points: a little wider than tall, like the system's input source badges.
let menuSize = NSSize(width: 20, height: 16)

/// Draws `glyph` centred on its ink (not its very tall line box) at `centre`, `inkHeight` tall.
func drawGlyph(inkHeight: CGFloat, centre: CGPoint, color: NSColor) {
    func line(_ size: CGFloat) -> CTLine {
        CTLineCreateWithAttributedString(NSAttributedString(string: glyph, attributes: [
            .font: NSFont(name: font, size: size) ?? NSFont.systemFont(ofSize: size, weight: .semibold),
            .foregroundColor: color,
        ]))
    }
    let probe = CTLineGetBoundsWithOptions(line(100), .useGlyphPathBounds)
    let l = line(100 * inkHeight / probe.height)
    let ink = CTLineGetBoundsWithOptions(l, .useGlyphPathBounds)
    let ctx = NSGraphicsContext.current!.cgContext
    ctx.textPosition = CGPoint(x: centre.x - ink.width / 2 - ink.minX, y: centre.y - ink.height / 2 - ink.minY)
    CTLineDraw(l, ctx)
}

/// A bitmap of `pixels` that draws in a `points`-sized coordinate space.
func rep(pixels: NSSize, points: NSSize, draw: () -> Void) -> NSBitmapImageRep {
    let rep = NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: Int(pixels.width), pixelsHigh: Int(pixels.height),
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    rep.size = points
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    draw()
    NSGraphicsContext.restoreGraphicsState()
    return rep
}

/// A filled badge with the glyph cut out; macOS tints it for light and dark menu bars.
func drawMenuIcon() {
    let badge = NSRect(origin: .zero, size: menuSize).insetBy(dx: 0.5, dy: 1)
    NSColor.black.setFill()
    NSBezierPath(roundedRect: badge, xRadius: 3.5, yRadius: 3.5).fill()
    let ctx = NSGraphicsContext.current!.cgContext
    ctx.setBlendMode(.clear)
    drawGlyph(inkHeight: 9.5, centre: CGPoint(x: badge.midX, y: badge.midY), color: .black)
    ctx.setBlendMode(.normal)
}

/// 1024-point canvas, following the macOS icon grid (824pt tile, 100pt margin).
func drawAppIcon() {
    let tile = NSBezierPath(roundedRect: NSRect(x: 100, y: 100, width: 824, height: 824), xRadius: 185, yRadius: 185)
    let ember = NSColor(red: 0.62, green: 0.07, blue: 0.04, alpha: 1)
    let flame = NSColor(red: 0.86, green: 0.25, blue: 0.06, alpha: 1)
    let gold = NSColor(red: 0.93, green: 0.62, blue: 0.10, alpha: 1)
    NSGradient(colors: [ember, flame, gold], atLocations: [0, 0.5, 1], colorSpace: .deviceRGB)!
        .draw(in: tile, angle: 90)

    let badge = NSRect(x: 212, y: 302, width: 600, height: 420)
    NSColor.white.setFill()
    NSBezierPath(roundedRect: badge, xRadius: 96, yRadius: 96).fill()
    drawGlyph(inkHeight: 320, centre: CGPoint(x: badge.midX, y: badge.midY), color: ember)
}

var args = CommandLine.arguments.dropFirst()
if args.first == "--app" {
    args.removeFirst()
    let out = args.first ?? "AppIcon.icns"
    let iconset = FileManager.default.temporaryDirectory.appendingPathComponent("AppIcon.iconset")
    try? FileManager.default.removeItem(at: iconset)
    try! FileManager.default.createDirectory(at: iconset, withIntermediateDirectories: true)
    for points in [16, 32, 128, 256, 512] {
        for scale in [1, 2] {
            let name = "icon_\(points)x\(points)\(scale == 2 ? "@2x" : "").png"
            let px = CGFloat(points * scale)
            let png = rep(pixels: NSSize(width: px, height: px), points: NSSize(width: 1024, height: 1024), draw: drawAppIcon)
                .representation(using: .png, properties: [:])!
            try! png.write(to: iconset.appendingPathComponent(name))
        }
    }
    let iconutil = Process()
    iconutil.executableURL = URL(fileURLWithPath: "/usr/bin/iconutil")
    iconutil.arguments = ["-c", "icns", iconset.path, "-o", out]
    try! iconutil.run()
    iconutil.waitUntilExit()
    precondition(iconutil.terminationStatus == 0, "iconutil failed")
    print("wrote \(out)")
} else {
    let out = args.first ?? "icon.tiff"
    let reps = [1, 2].map { scale in
        rep(pixels: NSSize(width: menuSize.width * CGFloat(scale), height: menuSize.height * CGFloat(scale)),
            points: menuSize, draw: drawMenuIcon)
    }
    let data = NSBitmapImageRep.tiffRepresentationOfImageReps(in: reps, using: .lzw, factor: 0)!
    try! data.write(to: URL(fileURLWithPath: out))
    print("wrote \(out)")
}
