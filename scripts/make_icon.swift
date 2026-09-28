// Draws the menu-bar icon (a boxed "ব") as a 16pt template TIFF with @1x and @2x reps.
// Usage: swift scripts/make_icon.swift Resources/icon.tiff
import AppKit

func rep(pixels: Int) -> NSBitmapImageRep {
    let rep = NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: pixels, pixelsHigh: pixels,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    rep.size = NSSize(width: 16, height: 16)
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)

    let box = NSBezierPath(roundedRect: NSRect(x: 0.75, y: 0.75, width: 14.5, height: 14.5), xRadius: 3, yRadius: 3)
    box.lineWidth = 1.2
    NSColor.black.setStroke()
    box.stroke()

    let text = NSAttributedString(string: "ব", attributes: [
        .font: NSFont(name: "BanglaSangamMN-Bold", size: 11) ?? NSFont.systemFont(ofSize: 11, weight: .bold),
        .foregroundColor: NSColor.black,
    ])
    // Centre on the glyph's ink, not its (very tall) line box.
    let line = CTLineCreateWithAttributedString(text)
    let ink = CTLineGetBoundsWithOptions(line, .useGlyphPathBounds)
    let ctx = NSGraphicsContext.current!.cgContext
    ctx.textPosition = CGPoint(x: (16 - ink.width) / 2 - ink.minX, y: (16 - ink.height) / 2 - ink.minY)
    CTLineDraw(line, ctx)

    NSGraphicsContext.restoreGraphicsState()
    return rep
}

let out = CommandLine.arguments.dropFirst().first ?? "icon.tiff"
let data = NSBitmapImageRep.tiffRepresentationOfImageReps(in: [rep(pixels: 16), rep(pixels: 32)], using: .lzw, factor: 0)!
try! data.write(to: URL(fileURLWithPath: out))
print("wrote \(out)")
