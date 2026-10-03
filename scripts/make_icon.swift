// Draws the icons.
//   swift scripts/make_icon.swift Resources/icon.tiff           menu-bar icon: the icon.svg beside it as a template TIFF, @1x and @2x
//   swift scripts/make_icon.swift --app Resources/AppIcon.icns  app icon for Finder: "নু." on a white badge on a fire-gradient
//                                                               tile with Apple-style continuous corners
import AppKit

let glyph = "নু."
let font = "KohinoorBangla-Semibold"   // ships with macOS

/// Menu-bar icon size in points: icon.svg's 452 × 327 badge at the system input-source badges' height.
let menuSize = NSSize(width: 22, height: 16)

/// A rounded rect with continuous ("squircle") corners like Apple's icons: each corner is a quarter
/// superellipse spanning 1.6 × `radius`, so curvature eases in from the straight edges instead of jumping.
func continuousRect(_ r: NSRect, radius: CGFloat) -> NSBezierPath {
    let e = min(radius * 1.6, r.width / 2, r.height / 2)
    let n: CGFloat = 4.5
    let steps = 64
    let path = NSBezierPath()
    // Each corner's quadrant centre and starting angle, counter-clockwise from top-right.
    let corners: [(CGPoint, CGFloat)] = [
        (CGPoint(x: r.maxX - e, y: r.maxY - e), 0),
        (CGPoint(x: r.minX + e, y: r.maxY - e), .pi / 2),
        (CGPoint(x: r.minX + e, y: r.minY + e), .pi),
        (CGPoint(x: r.maxX - e, y: r.minY + e), 3 * .pi / 2),
    ]
    for (centre, start) in corners {
        for i in 0...steps {
            let t = start + CGFloat(i) / CGFloat(steps) * .pi / 2
            let c = cos(t), s = sin(t)
            let p = CGPoint(x: centre.x + e * copysign(pow(abs(c), 2 / n), c),
                            y: centre.y + e * copysign(pow(abs(s), 2 / n), s))
            path.isEmpty ? path.move(to: p) : path.line(to: p)
        }
    }
    path.close()
    return path
}

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

/// The tile drawn into an 824-point square at the origin; callers scale it to their canvas.
func drawTile() {
    let tile = continuousRect(NSRect(x: 0, y: 0, width: 824, height: 824), radius: 185)
    let ember = NSColor(red: 0.62, green: 0.07, blue: 0.04, alpha: 1)
    let flame = NSColor(red: 0.86, green: 0.25, blue: 0.06, alpha: 1)
    let gold = NSColor(red: 0.93, green: 0.62, blue: 0.10, alpha: 1)
    NSGradient(colors: [ember, flame, gold], atLocations: [0, 0.5, 1], colorSpace: .deviceRGB)!
        .draw(in: tile, angle: 90)

    let badge = NSRect(x: 112, y: 202, width: 600, height: 420)
    NSColor.white.setFill()
    continuousRect(badge, radius: 96).fill()
    drawGlyph(inkHeight: 320, centre: CGPoint(x: badge.midX, y: badge.midY), color: ember)
}

/// Draws the tile scaled into `rect`.
func drawTile(in rect: NSRect) {
    let t = NSAffineTransform()
    t.translateX(by: rect.minX, yBy: rect.minY)
    t.scale(by: rect.width / 824)
    NSGraphicsContext.saveGraphicsState()
    t.concat()
    drawTile()
    NSGraphicsContext.restoreGraphicsState()
}

/// `svg` scaled to fit the menu-bar icon, centred. The artwork is a white badge with "নু." cut out,
/// so macOS can tint it for light and dark menu bars.
func drawMenuIcon(_ svg: NSImage) {
    let scale = min(menuSize.width / svg.size.width, menuSize.height / svg.size.height)
    let size = NSSize(width: svg.size.width * scale, height: svg.size.height * scale)
    svg.draw(in: NSRect(x: (menuSize.width - size.width) / 2, y: (menuSize.height - size.height) / 2,
                        width: size.width, height: size.height))
}

/// 1024-point canvas, following the macOS icon grid (824pt tile, 100pt margin).
func drawAppIcon() {
    drawTile(in: NSRect(x: 100, y: 100, width: 824, height: 824))
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
    let svgURL = URL(fileURLWithPath: out).deletingLastPathComponent().appendingPathComponent("icon.svg")
    guard let svg = NSImage(contentsOf: svgURL), svg.size.height > 0 else { fatalError("can't read \(svgURL.path)") }
    let reps = [1, 2].map { scale in
        rep(pixels: NSSize(width: menuSize.width * CGFloat(scale), height: menuSize.height * CGFloat(scale)),
            points: menuSize, draw: { drawMenuIcon(svg) })
    }
    let data = NSBitmapImageRep.tiffRepresentationOfImageReps(in: reps, using: .lzw, factor: 0)!
    try! data.write(to: URL(fileURLWithPath: out))
    print("wrote \(out)")
}
