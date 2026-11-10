import Cocoa

/// The dark look the settings, About and update windows share with Windows
/// (windows/design/*-preview.html): its colours, cards, and the controls drawn as there.
enum Theme {
    static let background = NSColor.hex(0x0D0807)
    static let card = NSColor.hex(0x161110)
    static let box = NSColor.hex(0x241C1A)
    static let boxHover = NSColor.hex(0x2B2220)
    static let border = NSColor.hex(0x2D2827)
    static let ink = NSColor.hex(0xFFFFFF, 0.92)
    static let dim = NSColor.hex(0xFFFFFF, 0.56)
    static let faint = NSColor.hex(0xFFFFFF, 0.32)
    static let orange = NSColor.hex(0xE87A1E)
    static let modeOn = NSColor.hex(0x2A1A13)
    static let toggleOff = NSColor.hex(0x3A3230)
    /// The brand gradient, #D13A0E → #E87A1E, drawn at 27° (CSS 63.35deg).
    static let brand = NSGradient(starting: .hex(0xD13A0E), ending: .hex(0xE87A1E))!

    /// A window in the dark design: a dark title bar, our black behind the content.
    static func style(_ window: NSWindow) {
        window.appearance = NSAppearance(named: .darkAqua)
        window.backgroundColor = background
        window.titlebarAppearsTransparent = true
        window.isReleasedWhenClosed = false
    }

    /// `stack` as a window's content, `width` wide; the window takes its height.
    static func content(_ stack: NSView, width: CGFloat) -> NSView {
        stack.translatesAutoresizingMaskIntoConstraints = false
        let content = NSView()
        content.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: content.topAnchor),
            stack.leadingAnchor.constraint(equalTo: content.leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: content.trailingAnchor),
            stack.bottomAnchor.constraint(equalTo: content.bottomAnchor),
            content.widthAnchor.constraint(equalToConstant: width),
        ])
        return content
    }

    static func label(_ text: String, _ font: NSFont, _ color: NSColor) -> NSTextField {
        let label = NSTextField(labelWithString: text)
        label.font = font
        label.textColor = color
        return label
    }

    /// A label that wraps at `width`.
    static func wrapping(_ text: String, _ font: NSFont, _ color: NSColor, width: CGFloat,
                         alignment: NSTextAlignment = .left) -> NSTextField {
        let label = NSTextField(wrappingLabelWithString: text)
        label.font = font
        label.textColor = color
        label.alignment = alignment
        label.preferredMaxLayoutWidth = width
        label.widthAnchor.constraint(equalToConstant: width).isActive = true
        return label
    }

    static func vstack(_ views: [NSView], spacing: CGFloat, alignment: NSLayoutConstraint.Attribute = .leading) -> NSStackView {
        let stack = NSStackView(views: views)
        stack.orientation = .vertical
        stack.alignment = alignment
        stack.spacing = spacing
        return stack
    }

    /// The 1-point rule between a card's rows.
    static func rule(width: CGFloat) -> NSView {
        let rule = Fill(Theme.border)
        rule.widthAnchor.constraint(equalToConstant: width).isActive = true
        rule.heightAnchor.constraint(equalToConstant: 1).isActive = true
        return rule
    }

    /// A card's heading: an icon and a title.
    static func heading(_ symbol: String, _ text: String) -> NSView {
        let icon = NSImageView()
        icon.image = NSImage(systemSymbolName: symbol, accessibilityDescription: nil)?
            .withSymbolConfiguration(.init(pointSize: 13, weight: .regular))
        icon.contentTintColor = .hex(0xFFFFFF, 0.75)
        let row = NSStackView(views: [icon, label(text, Fonts.ui(14.5, weight: .semibold), ink)])
        row.spacing = 8
        return row
    }

    /// The small orange heading inside a card (কৃতজ্ঞতা, নতুন যা আছে, সবসময়…).
    static func caption(_ text: String) -> NSTextField {
        label(text, Fonts.ui(12, weight: .semibold), orange)
    }

    /// A title with a line under it on the left, `accessory` (a switch, a segmented control…)
    /// on the right, `width` wide.
    static func row(_ title: String, detail: String? = nil, accessory: NSView, accessoryWidth: CGFloat,
                    width: CGFloat) -> NSView {
        let labelWidth = width - accessoryWidth - 16
        var lines: [NSView] = [label(title, Fonts.ui(14), ink)]
        if let detail {
            lines.append(wrapping(detail, Fonts.ui(12.5), dim, width: labelWidth))
        }
        let labels = vstack(lines, spacing: 2)
        labels.widthAnchor.constraint(equalToConstant: labelWidth).isActive = true
        let row = NSStackView(views: [labels, accessory])
        row.alignment = .centerY
        row.spacing = 16
        row.widthAnchor.constraint(equalToConstant: width).isActive = true
        return row
    }

    /// A pill: the platform names in About.
    static func pill(_ text: String) -> NSView {
        let pill = Fill(box, radius: 12, border: border)
        let label = label(text, Fonts.ui(12), ink)
        label.translatesAutoresizingMaskIntoConstraints = false
        pill.addSubview(label)
        NSLayoutConstraint.activate([
            label.centerXAnchor.constraint(equalTo: pill.centerXAnchor),
            label.centerYAnchor.constraint(equalTo: pill.centerYAnchor),
            pill.widthAnchor.constraint(equalTo: label.widthAnchor, constant: 20),
            pill.heightAnchor.constraint(equalToConstant: 24),
        ])
        return pill
    }
}

/// A card: our dark box with a border, `content` inside it with 16 points around.
final class Card: NSView {
    init(_ content: NSView, width: CGFloat, padding: NSEdgeInsets = NSEdgeInsets(top: 16, left: 16, bottom: 16, right: 16)) {
        super.init(frame: .zero)
        wantsLayer = true
        layer?.backgroundColor = Theme.card.cgColor
        layer?.borderColor = Theme.border.cgColor
        layer?.borderWidth = 1
        layer?.cornerRadius = 10
        content.translatesAutoresizingMaskIntoConstraints = false
        addSubview(content)
        NSLayoutConstraint.activate([
            content.topAnchor.constraint(equalTo: topAnchor, constant: padding.top),
            content.leadingAnchor.constraint(equalTo: leadingAnchor, constant: padding.left),
            content.trailingAnchor.constraint(lessThanOrEqualTo: trailingAnchor, constant: -padding.right),
            content.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -padding.bottom),
            widthAnchor.constraint(equalToConstant: width),
        ])
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    /// Shown, but not for changing: faded under a veil that takes the clicks.
    func disable() {
        let veil = Veil()
        veil.translatesAutoresizingMaskIntoConstraints = false
        addSubview(veil)
        NSLayoutConstraint.activate([
            veil.topAnchor.constraint(equalTo: topAnchor),
            veil.leadingAnchor.constraint(equalTo: leadingAnchor),
            veil.trailingAnchor.constraint(equalTo: trailingAnchor),
            veil.bottomAnchor.constraint(equalTo: bottomAnchor),
        ])
    }

    private final class Veil: NSView {
        override init(frame: NSRect) {
            super.init(frame: frame)
            wantsLayer = true
            layer?.backgroundColor = NSColor.hex(0x161110, 0.62).cgColor
            layer?.cornerRadius = 10
        }

        required init?(coder: NSCoder) { fatalError("not used") }

        override func hitTest(_ point: NSPoint) -> NSView? { frame.contains(point) ? self : nil }
        override func mouseDown(with event: NSEvent) {}
    }
}

/// A plain coloured rectangle, optionally rounded and outlined.
final class Fill: NSView {
    init(_ color: NSColor, radius: CGFloat = 0, border: NSColor? = nil) {
        super.init(frame: .zero)
        wantsLayer = true
        layer?.backgroundColor = color.cgColor
        layer?.cornerRadius = radius
        if let border {
            layer?.borderColor = border.cgColor
            layer?.borderWidth = 1
        }
    }

    required init?(coder: NSCoder) { fatalError("not used") }
}

/// A view that knows when the pointer is over it, and takes every click inside it.
class HoverView: NSView {
    private(set) var pointer: NSPoint?

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        trackingAreas.forEach(removeTrackingArea)
        addTrackingArea(NSTrackingArea(rect: .zero, options: [.mouseEnteredAndExited, .mouseMoved, .activeAlways, .inVisibleRect],
                                       owner: self))
    }

    override func mouseEntered(with event: NSEvent) { track(event) }
    override func mouseMoved(with event: NSEvent) { track(event) }
    override func mouseExited(with event: NSEvent) {
        pointer = nil
        needsDisplay = true
    }

    private func track(_ event: NSEvent) {
        pointer = convert(event.locationInWindow, from: nil)
        needsDisplay = true
    }

    override func hitTest(_ point: NSPoint) -> NSView? { frame.contains(point) ? self : nil }

    /// `text` centred on `point`.
    static func draw(_ text: String, font: NSFont, color: NSColor, centredAt point: NSPoint) {
        let attributes: [NSAttributedString.Key: Any] = [.font: font, .foregroundColor: color]
        let size = (text as NSString).size(withAttributes: attributes)
        (text as NSString).draw(at: NSPoint(x: point.x - size.width / 2, y: point.y - size.height / 2),
                                withAttributes: attributes)
    }
}

/// A segmented control: one choice of several, the chosen one in the brand gradient. With a
/// second line under each choice (the layout's) it is the big one.
final class Segmented: HoverView {
    struct Choice {
        var title: String
        var sub: String? = nil
    }

    private let choices: [Choice]
    private let selected: Int
    private let onPick: (Int) -> Void

    init(_ choices: [Choice], selected: Int, height: CGFloat, onPick: @escaping (Int) -> Void) {
        self.choices = choices
        self.selected = selected
        self.onPick = onPick
        super.init(frame: .zero)
        heightAnchor.constraint(equalToConstant: height).isActive = true
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    private func part(_ index: Int) -> NSRect {
        let inner = bounds.insetBy(dx: 3, dy: 3)
        let count = CGFloat(choices.count)
        let each = (inner.width - 3 * (count - 1)) / count
        return NSRect(x: inner.minX + CGFloat(index) * (each + 3), y: inner.minY, width: each, height: inner.height)
    }

    override func mouseDown(with event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)
        if let index = choices.indices.first(where: { part($0).contains(point) }), index != selected {
            onPick(index)
        }
    }

    override func draw(_ dirtyRect: NSRect) {
        let box = NSBezierPath(roundedRect: bounds.insetBy(dx: 0.5, dy: 0.5), xRadius: 8, yRadius: 8)
        Theme.box.setFill()
        box.fill()
        Theme.border.setStroke()
        box.stroke()
        for (index, choice) in choices.enumerated() {
            let rect = part(index)
            let on = index == selected
            let shape = NSBezierPath(roundedRect: rect, xRadius: 6, yRadius: 6)
            if on {
                Theme.brand.draw(in: shape, angle: 27)
            } else if let pointer, rect.contains(pointer) {
                Theme.boxHover.setFill()
                shape.fill()
            }
            let color: NSColor = on ? .white : Theme.ink
            let weight: NSFont.Weight = on ? .semibold : .regular
            if let sub = choice.sub {
                Self.draw(choice.title, font: Fonts.ui(15, weight: weight), color: color,
                          centredAt: NSPoint(x: rect.midX, y: rect.midY + 9))
                Self.draw(sub, font: Fonts.ui(11.5), color: .hex(0xFFFFFF, 0.72),
                          centredAt: NSPoint(x: rect.midX, y: rect.midY - 11))
            } else {
                Self.draw(choice.title, font: Fonts.ui(rect.height > 32 ? 14 : 13, weight: weight), color: color,
                          centredAt: NSPoint(x: rect.midX, y: rect.midY))
            }
        }
    }
}

/// An on/off switch, 40 × 22.
final class Switch: HoverView {
    private let on: Bool
    private let onToggle: () -> Void

    init(on: Bool, onToggle: @escaping () -> Void) {
        self.on = on
        self.onToggle = onToggle
        super.init(frame: .zero)
        widthAnchor.constraint(equalToConstant: 40).isActive = true
        heightAnchor.constraint(equalToConstant: 22).isActive = true
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    override func mouseDown(with event: NSEvent) { onToggle() }

    override func draw(_ dirtyRect: NSRect) {
        let track = NSBezierPath(roundedRect: bounds, xRadius: 11, yRadius: 11)
        if on {
            Theme.brand.draw(in: track, angle: 27)
        } else {
            Theme.toggleOff.setFill()
            track.fill()
        }
        NSColor.white.setFill()
        let x: CGFloat = on ? 29 : 11
        NSBezierPath(ovalIn: NSRect(x: x - 8, y: bounds.midY - 8, width: 16, height: 16)).fill()
    }
}

/// A typing mode: a radio button, its name, and what it does underneath.
final class ModeCard: HoverView {
    private let on: Bool
    private let onPick: () -> Void

    init(title: String, tag: String?, detail: String, on: Bool, width: CGFloat, onPick: @escaping () -> Void) {
        self.on = on
        self.onPick = onPick
        super.init(frame: .zero)
        let textWidth = width - 14 - 16 - 12 - 14
        let titleLabel = Theme.label(title, Fonts.ui(14.5, weight: .semibold), Theme.ink)
        var titleViews: [NSView] = [titleLabel]
        if let tag {
            titleViews.append(Theme.label(tag, Fonts.ui(12), Theme.orange))
        }
        let titleRow = NSStackView(views: titleViews)
        titleRow.spacing = 8
        titleRow.alignment = .firstBaseline
        let text = Theme.vstack([titleRow, Theme.wrapping(detail, Fonts.ui(12.5), Theme.dim, width: textWidth)], spacing: 3)
        text.translatesAutoresizingMaskIntoConstraints = false
        addSubview(text)
        NSLayoutConstraint.activate([
            text.topAnchor.constraint(equalTo: topAnchor, constant: 10),
            text.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 14 + 16 + 12),
            text.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -12),
            widthAnchor.constraint(equalToConstant: width),
        ])
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    override func mouseDown(with event: NSEvent) {
        if !on { onPick() }
    }

    override func draw(_ dirtyRect: NSRect) {
        let shape = NSBezierPath(roundedRect: bounds.insetBy(dx: 0.5, dy: 0.5), xRadius: 8, yRadius: 8)
        (on ? Theme.modeOn : pointer != nil ? Theme.boxHover : Theme.box).setFill()
        shape.fill()
        (on ? Theme.orange : Theme.border).setStroke()
        shape.stroke()
        // The radio button, level with the title.
        let centre = NSPoint(x: 14 + 8, y: bounds.maxY - 12 - 11)
        let ring = NSBezierPath(ovalIn: NSRect(x: centre.x - 7.25, y: centre.y - 7.25, width: 14.5, height: 14.5))
        ring.lineWidth = 1.5
        (on ? Theme.orange : Theme.faint).setStroke()
        ring.stroke()
        if on {
            Theme.orange.setFill()
            NSBezierPath(ovalIn: NSRect(x: centre.x - 4, y: centre.y - 4, width: 8, height: 8)).fill()
        }
    }
}

/// A key cap, as in the shortcuts card.
final class KeyCap: NSView {
    private let text: String
    private static let font = Fonts.ui(12)

    init(_ text: String) {
        self.text = text
        super.init(frame: .zero)
        let width = (text as NSString).size(withAttributes: [.font: Self.font]).width + 14
        widthAnchor.constraint(equalToConstant: ceil(width)).isActive = true
        heightAnchor.constraint(equalToConstant: 22).isActive = true
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    override func draw(_ dirtyRect: NSRect) {
        let shape = NSBezierPath(roundedRect: bounds.insetBy(dx: 0.5, dy: 0.5), xRadius: 5, yRadius: 5)
        Theme.box.setFill()
        shape.fill()
        Theme.border.setStroke()
        shape.stroke()
        // The thicker bottom edge of a key.
        Theme.border.setFill()
        NSRect(x: 2, y: 0.5, width: bounds.width - 4, height: 1).fill()
        HoverView.draw(text, font: Self.font, color: Theme.ink, centredAt: NSPoint(x: bounds.midX, y: bounds.midY + 0.5))
    }
}

/// The নুকতা বাংলা logo (Resources/nu-icon.svg), drawn from the SVG's own path data so it stays
/// sharp at any size: নু and a dot, in white, on the brand gradient.
final class Logo: NSView {
    /// The glyph নু, in its own coordinates (drawn at 10, 4.989 in the 37 × 28 logo).
    private static let glyph = svgPath("M10.541 13.665L9.372 12.773C9.33 12.277 9.245 11.749 9.117 11.189C8.99 10.63 8.82 10.084 8.607 9.553C8.395 9.022 8.147 8.543 7.863 8.118C7.58 7.693 7.254 7.35 6.886 7.088C6.517 6.826 6.121 6.694 5.696 6.694C5.54 6.694 5.38 6.712 5.217 6.748C5.055 6.783 4.916 6.836 4.803 6.907L5.419 5.802C5.632 5.944 5.788 6.142 5.887 6.397C5.986 6.652 6.036 6.907 6.036 7.162C6.036 7.559 5.951 7.916 5.781 8.235C5.611 8.554 5.366 8.806 5.047 8.99C4.729 9.174 4.328 9.266 3.847 9.266C3.039 9.266 2.423 9.025 1.998 8.543C1.573 8.062 1.36 7.452 1.36 6.716C1.36 5.894 1.622 5.256 2.146 4.803C2.671 4.35 3.351 4.123 4.187 4.123C4.81 4.123 5.387 4.233 5.919 4.452C6.45 4.672 6.921 4.977 7.332 5.366C7.743 5.756 8.104 6.202 8.416 6.705C8.728 7.208 8.983 7.739 9.181 8.299C9.379 8.859 9.514 9.415 9.585 9.967L9.287 9.989C9.273 9.705 9.248 9.326 9.213 8.852C9.177 8.377 9.146 7.87 9.117 7.332C9.089 6.794 9.075 6.305 9.075 5.866L9.075 1.36L0.829 1.36L0 0L12.263 0L13.091 1.36L10.69 1.36L10.69 13.644L10.541 13.665ZM4.608 14.877C4.608 14.154 4.873 13.563 5.405 13.102C5.936 12.642 6.69 12.411 7.668 12.411C8.461 12.411 9.223 12.553 9.953 12.836C10.682 13.12 11.362 13.502 11.993 13.984C12.623 14.466 13.194 15.008 13.704 15.61C14.214 16.212 14.646 16.839 15 17.491L14.192 18.022C13.64 17.186 13.027 16.463 12.354 15.854C11.681 15.245 10.934 14.77 10.112 14.43C9.29 14.09 8.391 13.92 7.413 13.92C6.917 13.92 6.559 14.002 6.34 14.165C6.12 14.328 6.01 14.551 6.01 14.834C6.01 15.117 6.117 15.348 6.329 15.525C6.542 15.702 6.846 15.79 7.243 15.79C7.625 15.79 7.965 15.702 8.263 15.525C8.561 15.348 8.798 15.061 8.975 14.664C9.152 14.267 9.241 13.736 9.241 13.07L10.686 13.772C10.686 14.437 10.558 15.043 10.303 15.589C10.048 16.134 9.673 16.57 9.177 16.896C8.681 17.221 8.072 17.384 7.349 17.384C6.556 17.384 5.9 17.151 5.383 16.683C4.866 16.216 4.608 15.613 4.608 14.877Z")

    init(width: CGFloat) {
        super.init(frame: .zero)
        widthAnchor.constraint(equalToConstant: width).isActive = true
        heightAnchor.constraint(equalToConstant: width * 28 / 37).isActive = true
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    /// SVG coordinates: y down.
    override var isFlipped: Bool { true }

    override func draw(_ dirtyRect: NSRect) {
        guard let context = NSGraphicsContext.current?.cgContext else { return }
        context.saveGState()
        context.scaleBy(x: bounds.width / 37, y: bounds.height / 28)
        NSGraphicsContext.saveGraphicsState()
        NSBezierPath(roundedRect: NSRect(x: 0, y: 0, width: 37, height: 28), xRadius: 6, yRadius: 6).addClip()
        Theme.brand.draw(from: NSPoint(x: 1.761, y: 20.8), to: NSPoint(x: 35.665, y: 3.782),
                         options: [.drawsBeforeStartingLocation, .drawsAfterEndingLocation])
        NSGraphicsContext.restoreGraphicsState()
        NSColor.hex(0xFFFFFF, 0.9).setFill()
        context.translateBy(x: 10, y: 4.989)
        Self.glyph.fill()
        context.translateBy(x: -10, y: -4.989)
        NSBezierPath(ovalIn: NSRect(x: 24, y: 15.989, width: 3, height: 3)).fill()
        context.restoreGState()
    }

    /// An SVG path of absolute M, L, C and Z commands (all nu-icon.svg uses).
    private static func svgPath(_ data: String) -> NSBezierPath {
        let path = NSBezierPath()
        let scanner = Scanner(string: data)
        scanner.charactersToBeSkipped = CharacterSet(charactersIn: " ,\n")
        var command: Character = "M"
        func point() -> NSPoint? {
            guard let x = scanner.scanDouble(), let y = scanner.scanDouble() else { return nil }
            return NSPoint(x: x, y: y)
        }
        while !scanner.isAtEnd {
            if let letter = scanner.scanCharacter(), "MLCZ".contains(letter) {
                command = letter
            } else {
                scanner.currentIndex = data.index(before: scanner.currentIndex)
            }
            switch command {
            case "M": if let p = point() { path.move(to: p); command = "L" }
            case "L": if let p = point() { path.line(to: p) }
            case "C":
                if let a = point(), let b = point(), let p = point() { path.curve(to: p, controlPoint1: a, controlPoint2: b) }
            default: path.close()
            }
        }
        return path
    }
}

/// The soft orange glow behind the logo in About and the update dialog.
final class Glow: NSView {
    override func draw(_ dirtyRect: NSRect) {
        guard let context = NSGraphicsContext.current?.cgContext,
              let gradient = CGGradient(colorsSpace: CGColorSpace(name: CGColorSpace.sRGB),
                                        colors: [NSColor.hex(0xE87A1E, 0.28).cgColor, NSColor.hex(0xE87A1E, 0).cgColor] as CFArray,
                                        locations: [0, 1]) else { return }
        context.translateBy(x: bounds.midX, y: bounds.midY)
        context.scaleBy(x: bounds.width / 2, y: bounds.height / 2)
        context.drawRadialGradient(gradient, startCenter: .zero, startRadius: 0, endCenter: .zero, endRadius: 1, options: [])
    }
}

/// A button as in the designs: a dark box, or the brand gradient.
final class DialogButton: NSButton {
    private let brand: Bool

    init(title: String, brand: Bool, target: AnyObject, action: Selector) {
        self.brand = brand
        super.init(frame: .zero)
        self.title = title
        self.target = target
        self.action = action
        isBordered = false
        focusRingType = .none
        let width = (title as NSString).size(withAttributes: [.font: Self.font(brand)]).width + (brand ? 36 : 28)
        widthAnchor.constraint(equalToConstant: ceil(width)).isActive = true
        heightAnchor.constraint(equalToConstant: 34).isActive = true
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    override var isEnabled: Bool {
        didSet { needsDisplay = true }
    }

    private static func font(_ brand: Bool) -> NSFont {
        brand ? Fonts.ui(13, weight: .semibold) : Fonts.ui(13)
    }

    override func draw(_ dirtyRect: NSRect) {
        let path = NSBezierPath(roundedRect: bounds.insetBy(dx: 0.5, dy: 0.5), xRadius: 6, yRadius: 6)
        if brand {
            Theme.brand.draw(in: path, angle: 27)
            if !isEnabled {
                NSColor.hex(0x0D0807, 0.55).setFill()
                path.fill()
            }
        } else {
            (isHighlighted ? Theme.boxHover : Theme.box).setFill()
            path.fill()
            Theme.border.setStroke()
            path.stroke()
        }
        let color: NSColor = brand ? .white : isEnabled ? Theme.ink : Theme.faint
        HoverView.draw(title, font: Self.font(brand), color: color, centredAt: NSPoint(x: bounds.midX, y: bounds.midY))
    }
}

/// A window that closes on Esc and ⌘W (an input method has no menu bar of its own for ⌘W).
final class DarkWindow: NSWindow {
    override func keyDown(with event: NSEvent) {
        let command = event.modifierFlags.contains(.command)
        if event.keyCode == 53 || (command && event.charactersIgnoringModifiers == "w") {
            performClose(nil)
        } else {
            super.keyDown(with: event)
        }
    }

    override func performKeyEquivalent(with event: NSEvent) -> Bool {
        if event.modifierFlags.contains(.command), event.charactersIgnoringModifiers == "w" {
            performClose(nil)
            return true
        }
        return super.performKeyEquivalent(with: event)
    }
}

extension NSColor {
    static func hex(_ rgb: UInt32, _ alpha: CGFloat = 1) -> NSColor {
        NSColor(srgbRed: CGFloat((rgb >> 16) & 0xFF) / 255, green: CGFloat((rgb >> 8) & 0xFF) / 255,
                blue: CGFloat(rgb & 0xFF) / 255, alpha: alpha)
    }
}
