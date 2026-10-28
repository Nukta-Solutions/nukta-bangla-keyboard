import Cocoa

/// The update dialog (windows/design/update-preview.html): what's new in a release, and ডাউনলোড,
/// পরে or এই সংস্করণ বাদ দিন.
///
/// The daily check opens it as a panel that doesn't take the keyboard, so typing in the app
/// underneath goes on; "এখনই খুঁজুন" opens it as an ordinary window. Its buttons wait a second
/// before they answer, and Return and Space press none of them; Esc is "পরে".
final class UpdateWindow: NSObject, NSWindowDelegate {
    static let shared = UpdateWindow()

    private var panel: NSPanel?
    private var release: Updates.Release?
    /// Off for the first second.
    private var buttons: [NSButton] = []

    private static let width: CGFloat = 420
    private static let pad: CGFloat = 26

    private static let ink = NSColor.hex(0xFFFFFF, 0.92)
    private static let dim = NSColor.hex(0xFFFFFF, 0.56)
    private static let orange = NSColor.hex(0xE87A1E)

    func show(_ release: Updates.Release, activate: Bool) {
        panel?.close()
        self.release = release
        // Not again for a day, whichever button is clicked.
        Updates.shared.markShown()
        let panel = makePanel(for: release)
        self.panel = panel
        panel.center()
        if activate {
            // An agent app has to bring itself forward, or the window opens behind the current app.
            NSApp.activate(ignoringOtherApps: true)
            panel.makeKeyAndOrderFront(nil)
        } else {
            panel.orderFrontRegardless()
        }
        let guarded = buttons
        guarded.forEach { $0.isEnabled = false }
        DispatchQueue.main.asyncAfter(deadline: .now() + 1) {
            guarded.forEach { $0.isEnabled = true }
        }
    }

    // MARK: Building

    private func makePanel(for release: Updates.Release) -> NSPanel {
        let panel = NSPanel(contentRect: NSRect(x: 0, y: 0, width: Self.width, height: 10),
                            styleMask: [.titled, .closable, .nonactivatingPanel], backing: .buffered, defer: false)
        panel.title = "নুকতা বাংলা আপডেট"
        panel.appearance = NSAppearance(named: .darkAqua)
        panel.backgroundColor = .hex(0x0D0807)
        panel.titlebarAppearsTransparent = true
        panel.isReleasedWhenClosed = false
        panel.hidesOnDeactivate = false
        panel.level = .floating
        panel.delegate = self

        // The head: logo, and from which version to which.
        let icon = NSImageView(image: NSApp.applicationIconImage)
        icon.imageScaling = .scaleProportionallyUpOrDown
        icon.widthAnchor.constraint(equalToConstant: 56).isActive = true
        icon.heightAnchor.constraint(equalToConstant: 56).isActive = true
        let title = Self.label("নতুন সংস্করণ এসেছে", Fonts.ui(19, weight: .bold), Self.ink)
        let versions = NSMutableAttributedString(
            string: "আপনার: \(Updates.banglaDigits(Updates.current))  →  ",
            attributes: [.font: Fonts.ui(13), .foregroundColor: Self.dim])
        versions.append(NSAttributedString(
            string: "নতুন: \(Updates.banglaDigits(release.version))",
            attributes: [.font: Fonts.ui(13, weight: .semibold), .foregroundColor: Self.orange]))
        let titles = NSStackView(views: [title, NSTextField(labelWithAttributedString: versions)])
        titles.orientation = .vertical
        titles.alignment = .leading
        titles.spacing = 2
        let head = NSStackView(views: [icon, titles])
        head.spacing = 16
        head.alignment = .centerY

        var rows: [NSView] = [head]
        if !release.notes.isEmpty {
            rows.append(notesCard(release.notes))
        }
        rows.append(buttonRow())

        let stack = NSStackView(views: rows)
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 20
        stack.edgeInsets = NSEdgeInsets(top: 14, left: Self.pad, bottom: 22, right: Self.pad)
        stack.translatesAutoresizingMaskIntoConstraints = false
        for row in rows.dropFirst() {
            row.widthAnchor.constraint(equalToConstant: Self.width - 2 * Self.pad).isActive = true
        }

        let content = NSView()
        content.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: content.topAnchor),
            stack.leadingAnchor.constraint(equalTo: content.leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: content.trailingAnchor),
            stack.bottomAnchor.constraint(equalTo: content.bottomAnchor),
            content.widthAnchor.constraint(equalToConstant: Self.width),
        ])
        panel.contentView = content
        return panel
    }

    /// "নতুন যা আছে": the release's bullet lines.
    private func notesCard(_ notes: [String]) -> NSView {
        let heading = Self.label("নতুন যা আছে", Fonts.ui(12, weight: .semibold), Self.orange)
        var items: [NSView] = [heading]
        for note in notes {
            let line = NSTextField(wrappingLabelWithString: "•  " + note)
            line.font = Fonts.ui(13)
            line.textColor = Self.ink
            line.preferredMaxLayoutWidth = Self.width - 2 * Self.pad - 28
            items.append(line)
        }
        let more = NSButton(title: "", target: self, action: #selector(openReleasePage(_:)))
        more.isBordered = false
        more.attributedTitle = NSAttributedString(string: "সব পরিবর্তন দেখুন", attributes: [
            .font: Fonts.ui(12), .foregroundColor: Self.dim, .underlineStyle: NSUnderlineStyle.single.rawValue,
        ])
        items.append(more)

        let list = NSStackView(views: items)
        list.orientation = .vertical
        list.alignment = .leading
        list.spacing = 4
        list.setCustomSpacing(6, after: heading)
        if let last = items.dropLast().last {
            list.setCustomSpacing(10, after: last)
        }
        list.translatesAutoresizingMaskIntoConstraints = false

        let card = NSView()
        card.wantsLayer = true
        card.layer?.backgroundColor = NSColor.hex(0x161110).cgColor
        card.layer?.borderColor = NSColor.hex(0x2D2827).cgColor
        card.layer?.borderWidth = 1
        card.layer?.cornerRadius = 10
        card.addSubview(list)
        NSLayoutConstraint.activate([
            list.topAnchor.constraint(equalTo: card.topAnchor, constant: 12),
            list.leadingAnchor.constraint(equalTo: card.leadingAnchor, constant: 14),
            list.trailingAnchor.constraint(lessThanOrEqualTo: card.trailingAnchor, constant: -14),
            list.bottomAnchor.constraint(equalTo: card.bottomAnchor, constant: -12),
        ])
        return card
    }

    /// এই সংস্করণ বাদ দিন on the left; পরে and ডাউনলোড on the right. None is the default button.
    private func buttonRow() -> NSView {
        let skip = NSButton(title: "", target: self, action: #selector(skipVersion(_:)))
        skip.isBordered = false
        skip.attributedTitle = NSAttributedString(string: "এই সংস্করণ বাদ দিন", attributes: [
            .font: Fonts.ui(13), .foregroundColor: Self.dim,
        ])
        let later = DialogButton(title: "পরে", brand: false, target: self, action: #selector(later(_:)))
        later.keyEquivalent = "\u{1b}"
        let download = DialogButton(title: "ডাউনলোড", brand: true, target: self, action: #selector(download(_:)))
        buttons = [skip, later, download]

        let row = NSStackView()
        row.spacing = 8
        row.setViews([skip], in: .leading)
        row.setViews([later, download], in: .trailing)
        return row
    }

    private static func label(_ text: String, _ font: NSFont, _ color: NSColor) -> NSTextField {
        let label = NSTextField(labelWithString: text)
        label.font = font
        label.textColor = color
        return label
    }

    // MARK: The buttons

    @objc private func download(_ sender: Any?) {
        NSWorkspace.shared.open(Updates.website)
        // Asked again in a day, in case it wasn't installed.
        close()
    }

    @objc private func later(_ sender: Any?) {
        close()
    }

    @objc private func skipVersion(_ sender: Any?) {
        if let version = release?.version {
            Updates.shared.skip(version)
        }
        close()
    }

    @objc private func openReleasePage(_ sender: Any?) {
        if let url = release?.url {
            NSWorkspace.shared.open(url)
        }
    }

    private func close() {
        Updates.shared.markShown()
        panel?.close()
    }

    func windowWillClose(_ notification: Notification) {
        panel = nil
        buttons = []
    }
}

/// পরে and ডাউনলোড, drawn as in the design: a dark box, or the brand gradient.
private final class DialogButton: NSButton {
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

    required init?(coder: NSCoder) {
        fatalError("not used")
    }

    override var isEnabled: Bool {
        didSet { needsDisplay = true }
    }

    private static func font(_ brand: Bool) -> NSFont {
        brand ? Fonts.ui(13, weight: .semibold) : Fonts.ui(13)
    }

    override func draw(_ dirtyRect: NSRect) {
        let path = NSBezierPath(roundedRect: bounds.insetBy(dx: 0.5, dy: 0.5), xRadius: 6, yRadius: 6)
        if brand {
            NSGradient(starting: .hex(0xD13A0E), ending: .hex(0xE87A1E))?.draw(in: path, angle: 27)
            if !isEnabled {
                NSColor.hex(0x0D0807, 0.55).setFill()
                path.fill()
            }
        } else {
            NSColor.hex(isHighlighted ? 0x2B2220 : 0x241C1A).setFill()
            path.fill()
            NSColor.hex(0x2D2827).setStroke()
            path.stroke()
        }
        let color: NSColor = brand ? .white : .hex(0xFFFFFF, isEnabled ? 0.92 : 0.32)
        let attributes: [NSAttributedString.Key: Any] = [.font: Self.font(brand), .foregroundColor: color]
        let size = (title as NSString).size(withAttributes: attributes)
        (title as NSString).draw(at: NSPoint(x: (bounds.width - size.width) / 2, y: (bounds.height - size.height) / 2),
                                 withAttributes: attributes)
    }
}

private extension NSColor {
    static func hex(_ rgb: UInt32, _ alpha: CGFloat = 1) -> NSColor {
        NSColor(srgbRed: CGFloat((rgb >> 16) & 0xFF) / 255, green: CGFloat((rgb >> 8) & 0xFF) / 255,
                blue: CGFloat(rgb & 0xFF) / 255, alpha: alpha)
    }
}
