// Draws the icons: a boxed "নু".
//   swift scripts/make_icon.swift Resources/icon.tiff           menu-bar icon: 16pt template TIFF, @1x and @2x
//   swift scripts/make_icon.swift --app Resources/AppIcon.icns  app icon for Finder: white box on green
import AppKit

let glyph = "নু"
let font = "BanglaSangamMN-Bold"

/// Draws `glyph` centred on its ink (not its very tall line box) at `centre`, `inkHeight` tall.
func drawGlyph(inkHeight: CGFloat, centre: CGPoint, color: NSColor) {
    func line(_ size: CGFloat) -> CTLine {
        CTLineCreateWithAttributedString(NSAttributedString(string: glyph, attributes: [
            .font: NSFont(name: font, size: size) ?? NSFont.systemFont(ofSize: size, weight: .bold),
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

/// A square bitmap of `pixels` px that draws in a `points`-wide coordinate space.
func rep(pixels: Int, points: CGFloat, draw: () -> Void) -> NSBitmapImageRep {
    let rep = NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: pixels, pixelsHigh: pixels,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    rep.size = NSSize(width: points, height: points)
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    draw()
    NSGraphicsContext.restoreGraphicsState()
    return rep
}

func drawMenuIcon() {
    let box = NSBezierPath(roundedRect: NSRect(x: 0.75, y: 0.75, width: 14.5, height: 14.5), xRadius: 3, yRadius: 3)
    box.lineWidth = 1.2
    NSColor.black.setStroke()
    box.stroke()
    drawGlyph(inkHeight: 10, centre: CGPoint(x: 8, y: 8), color: .black)
}

/// 1024-point canvas, following the macOS icon grid (824pt tile, 100pt margin).
func drawAppIcon() {
    let tile = NSBezierPath(roundedRect: NSRect(x: 100, y: 100, width: 824, height: 824), xRadius: 185, yRadius: 185)
    NSGradient(starting: NSColor(red: 0.05, green: 0.56, blue: 0.40, alpha: 1),
               ending: NSColor(red: 0, green: 0.38, blue: 0.27, alpha: 1))!.draw(in: tile, angle: -90)

    let box = NSBezierPath(roundedRect: NSRect(x: 262, y: 262, width: 500, height: 500), xRadius: 100, yRadius: 100)
    box.lineWidth = 40
    NSColor.white.setStroke()
    box.stroke()
    drawGlyph(inkHeight: 330, centre: CGPoint(x: 512, y: 512), color: .white)
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
            let png = rep(pixels: points * scale, points: 1024, draw: drawAppIcon).representation(using: .png, properties: [:])!
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
    let reps = [16, 32].map { rep(pixels: $0, points: 16, draw: drawMenuIcon) }
    let data = NSBitmapImageRep.tiffRepresentationOfImageReps(in: reps, using: .lzw, factor: 0)!
    try! data.write(to: URL(fileURLWithPath: out))
    print("wrote \(out)")
}
