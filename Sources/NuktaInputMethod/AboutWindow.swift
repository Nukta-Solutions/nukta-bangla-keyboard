import Cocoa

/// নুকতা বাংলা সম্পর্কে, opened from the নু menu: the same design as on Windows
/// (windows/design/about-preview.html). Esc or ⌘W closes it. The full licence notices ship in
/// Resources/Licenses.
final class AboutWindow: NSObject {
    static let shared = AboutWindow()

    private var window: NSWindow?
    private static let width: CGFloat = 400
    /// "এখনই খুঁজুন" and what it last found.
    private var checkButton: DialogButton?
    private var status: NSTextField?

    func show() {
        // Opened again, it shows the version, not an old answer.
        if window?.isVisible != true { window = makeWindow() }
        // An agent app has to bring itself forward, or the window opens behind the current app.
        NSApp.activate(ignoringOtherApps: true)
        window?.center()
        window?.makeKeyAndOrderFront(nil)
        window?.orderFrontRegardless()
    }

    private func makeWindow() -> NSWindow {
        let window = DarkWindow(contentRect: NSRect(x: 0, y: 0, width: Self.width, height: 10),
                                styleMask: [.titled, .closable], backing: .buffered, defer: false)
        window.title = "নুকতা বাংলা সম্পর্কে"
        Theme.style(window)

        let logo = Logo(width: 92)
        let title = Theme.label("নুকতা বাংলা", Fonts.ui(24, weight: .bold), Theme.ink)
        let version = Theme.label("সংস্করণ \(Updates.banglaDigits(Updates.current))", Fonts.ui(13), Theme.dim)
        let tagline = Theme.wrapping("বিজয় লেআউট আর ফোনেটিক টাইপিং, ইউনিকোডে। সব অ্যাপে, একই নিয়মে।",
                                     Fonts.ui(14), Theme.ink, width: 300, alignment: .center)
        let chips = NSStackView(views: ["Windows", "macOS", "Linux"].map(Theme.pill))
        chips.spacing = 6
        let update = updateCard(width: Self.width - 56)
        let credits = creditsCard(width: Self.width - 56)
        let copyright = Theme.label("© ২০২৬ Nukta Solutions", Fonts.ui(12), Theme.faint)

        let stack = Theme.vstack([logo, title, version, tagline, chips, update, credits, copyright], spacing: 0,
                                 alignment: .centerX)
        stack.setCustomSpacing(16, after: logo)
        stack.setCustomSpacing(14, after: version)
        stack.setCustomSpacing(12, after: tagline)
        stack.setCustomSpacing(22, after: chips)
        stack.setCustomSpacing(12, after: update)
        stack.setCustomSpacing(18, after: credits)
        stack.edgeInsets = NSEdgeInsets(top: 34, left: 28, bottom: 24, right: 28)
        let content = Theme.content(stack, width: Self.width)

        // The glow behind the logo, which takes no room of its own.
        let glow = Glow()
        glow.translatesAutoresizingMaskIntoConstraints = false
        content.addSubview(glow, positioned: .below, relativeTo: stack)
        NSLayoutConstraint.activate([
            glow.centerXAnchor.constraint(equalTo: logo.centerXAnchor),
            glow.centerYAnchor.constraint(equalTo: logo.centerYAnchor),
            glow.widthAnchor.constraint(equalTo: logo.widthAnchor, constant: 80),
            glow.heightAnchor.constraint(equalTo: logo.heightAnchor, constant: 52),
        ])

        window.contentView = content
        window.setContentSize(content.fittingSize)
        return window
    }

    /// আপডেট: "এখনই খুঁজুন" right here (the daily check's switch stays in সেটিংস).
    private func updateCard(width: CGFloat) -> NSView {
        let inner = width - 28
        let button = DialogButton(title: "এখনই খুঁজুন", brand: false, target: self, action: #selector(checkNow(_:)))
        let buttonWidth = button.constraints.first { $0.firstAttribute == .width }?.constant ?? 100
        let status = Theme.wrapping("এখন আছে সংস্করণ \(Updates.banglaDigits(Updates.current))", Fonts.ui(12.5),
                                    Theme.dim, width: inner - buttonWidth - 16)
        checkButton = button
        self.status = status
        let row = NSStackView(views: [status, button])
        row.spacing = 16
        row.alignment = .centerY
        let stack = Theme.vstack([Theme.caption("আপডেট"), row], spacing: 6)
        return Card(stack, width: width, padding: NSEdgeInsets(top: 12, left: 14, bottom: 12, right: 14))
    }

    /// "এখনই খুঁজুন": asks GitHub now. A new version opens the update dialog, even one skipped
    /// before: the user asked.
    @objc private func checkNow(_ sender: Any?) {
        checkButton?.isEnabled = false
        show("খোঁজা হচ্ছে…", Theme.dim)
        Updates.shared.checkNow { [weak self] result in
            guard let self else { return }
            self.checkButton?.isEnabled = true
            switch result {
            case .found(let release):
                self.show("নতুন সংস্করণ পাওয়া গেছে।", Theme.orange)
                UpdateWindow.shared.show(release, activate: true)
            case .latest:
                self.show("আপনি সর্বশেষ সংস্করণ ব্যবহার করছেন।", Theme.ink)
            case .failed:
                self.show("খোঁজা গেল না। ইন্টারনেট সংযোগ দেখে আবার চেষ্টা করুন।", Theme.dim)
            }
        }
    }

    /// The status beside "এখনই খুঁজুন"; the window follows its height, its top staying put.
    private func show(_ text: String, _ color: NSColor) {
        guard let status, let window, let content = window.contentView else { return }
        status.stringValue = text
        status.textColor = color
        content.layoutSubtreeIfNeeded()
        let old = window.frame
        let size = window.frameRect(forContentRect: NSRect(origin: .zero, size: content.fittingSize)).size
        window.setFrame(NSRect(x: old.minX, y: old.maxY - size.height, width: size.width, height: size.height),
                        display: true)
    }

    /// কৃতজ্ঞতা: what we use from others, and under which licence.
    private func creditsCard(width: CGFloat) -> NSView {
        let inner = width - 28
        let credits = [
            ("ফোনেটিক ইঞ্জিন", "riti · OpenBangla", "Mozilla Public License 2.0"),
            ("বাংলা ফন্ট", "Noto Serif Bengali · Google", "SIL Open Font License 1.1"),
        ]
        var views: [NSView] = [Theme.caption("কৃতজ্ঞতা")]
        for (index, (what, who, licence)) in credits.enumerated() {
            if index > 0 { views.append(Theme.rule(width: inner)) }
            let names = Theme.vstack([
                Theme.label(who, Fonts.ui(13), Theme.ink),
                Theme.label(licence, Fonts.ui(11.5), Theme.faint),
            ], spacing: 1, alignment: .trailing)
            let row = NSStackView(views: [Theme.label(what, Fonts.ui(13), Theme.dim), names])
            row.alignment = .top
            row.distribution = .equalSpacing
            row.widthAnchor.constraint(equalToConstant: inner).isActive = true
            views.append(row)
        }
        let stack = Theme.vstack(views, spacing: 8)
        stack.setCustomSpacing(6, after: views[0])
        return Card(stack, width: width, padding: NSEdgeInsets(top: 12, left: 14, bottom: 12, right: 14))
    }
}
