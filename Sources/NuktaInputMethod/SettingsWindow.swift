import Cocoa
import NuktaPhonetic

/// সেটিংস, opened from the নু menu. Every change is saved and applied straight away.
///
/// The same design as on Windows (windows/design/settings-preview.html): dark, two columns, the
/// layout and typing mode on the left, suggestions and shortcuts on the right. With বিজয় only the
/// layout shows, across the whole width: Bijoy uses none of the rest. Under them, the update card.
/// What is Windows-only (F12, the top bar, its sound) is left out; the Mac's own keys are shown.
final class SettingsWindow: NSObject, NSWindowDelegate {
    static let shared = SettingsWindow()

    private var window: NSWindow?
    /// What "এখনই খুঁজুন" last found, shown beside it.
    private var status: (text: String, color: NSColor)?
    private var checking = false

    private static let width: CGFloat = 860
    private static let pad: CGFloat = 24
    private static let gap: CGFloat = 16

    private static let modes: [(TypingMode, String, String)] = [
        (.phoneticFirst, "ফোনেটিক আগে",
         "যেমন বানান লিখবেন ঠিক তেমন বাংলা বসবে। তালিকা থেকে অন্য শব্দ বেছে নিলে পরের বার সেটাই আগে আসবে।"),
        (.smart, "স্মার্ট", "অভিধান আর অটোকারেক্ট সবচেয়ে মানানসই শব্দ বেছে দেবে (sonar → সোনার)।"),
        (.phoneticOnly, "শুধু ফোনেটিক", "সাজেশন তালিকা ছাড়া, অক্ষর ধরে ধরে লেখা।"),
    ]

    func show() {
        let first = window == nil
        if first {
            let window = DarkWindow(contentRect: NSRect(x: 0, y: 0, width: Self.width, height: 10),
                                    styleMask: [.titled, .closable, .miniaturizable], backing: .buffered, defer: false)
            window.title = "নুকতা বাংলা — সেটিংস"
            Theme.style(window)
            window.delegate = self
            self.window = window
        }
        render()
        // An agent app has to bring itself forward, or the window opens behind the current app.
        NSApp.activate(ignoringOtherApps: true)
        if first { window?.center() }
        window?.makeKeyAndOrderFront(nil)
        window?.orderFrontRegardless()
    }

    // MARK: Building

    /// Builds the window's content from the saved settings; the window follows its height
    /// (বিজয় ⇄ ফোনেটিক), its top staying where it is.
    private func render() {
        guard let window else { return }
        let phonetic = Settings.layout == .phonetic
        let full = Self.width - 2 * Self.pad
        let column = (full - Self.gap) / 2
        let leftWidth = phonetic ? column : full

        var left: [NSView] = [layoutCard(width: leftWidth)]
        if phonetic { left.append(typingCard(width: column)) }
        left.append(updateCard(width: leftWidth))
        let leftColumn = Theme.vstack(left, spacing: Self.gap)
        var columns: NSView = leftColumn
        if phonetic {
            let right = Theme.vstack([suggestionsCard(width: column), shortcutsCard(width: column)], spacing: Self.gap)
            // Side by side, each column as tall as its own cards.
            let row = NSView()
            let fit = row.heightAnchor.constraint(equalToConstant: 0)
            fit.priority = .defaultLow
            for column in [leftColumn, right] {
                column.translatesAutoresizingMaskIntoConstraints = false
                row.addSubview(column)
                column.topAnchor.constraint(equalTo: row.topAnchor).isActive = true
                column.bottomAnchor.constraint(lessThanOrEqualTo: row.bottomAnchor).isActive = true
            }
            NSLayoutConstraint.activate([
                fit,
                leftColumn.leadingAnchor.constraint(equalTo: row.leadingAnchor),
                right.leadingAnchor.constraint(equalTo: leftColumn.trailingAnchor, constant: Self.gap),
                right.trailingAnchor.constraint(equalTo: row.trailingAnchor),
            ])
            columns = row
        }

        let titles = Theme.vstack([
            Theme.label("নুকতা বাংলা", Fonts.ui(21, weight: .bold), Theme.ink),
            Theme.label("সেটিংস · সংস্করণ \(Updates.banglaDigits(Updates.current))", Fonts.ui(13), Theme.dim),
        ], spacing: 0)
        let head = NSStackView(views: [Logo(width: 49), titles])
        head.spacing = 14
        head.alignment = .centerY

        let foot = Theme.wrapping("পরিবর্তন সাথে সাথে সব অ্যাপে চালু হয়", Fonts.ui(12), Theme.faint, width: full,
                                  alignment: .center)

        let stack = Theme.vstack([head, columns, foot], spacing: 14)
        stack.setCustomSpacing(20, after: head)
        stack.edgeInsets = NSEdgeInsets(top: 16, left: Self.pad, bottom: 18, right: Self.pad)

        let content = Theme.content(stack, width: Self.width)
        let old = window.frame
        window.contentView = content
        let size = window.frameRect(forContentRect: NSRect(origin: .zero, size: content.fittingSize)).size
        window.setFrame(NSRect(x: old.minX, y: old.maxY - size.height, width: size.width, height: size.height),
                        display: true)
    }

    /// A card: `views` one under another, 10 points apart (12 after the heading, or as `spacing`
    /// says after a view), `width` wide.
    private static func card(_ views: [NSView], width: CGFloat, spacing: [(NSView, CGFloat)] = []) -> Card {
        let stack = Theme.vstack(views, spacing: 10)
        if let heading = views.first {
            stack.setCustomSpacing(12, after: heading)
        }
        for (view, space) in spacing {
            stack.setCustomSpacing(space, after: view)
        }
        return Card(stack, width: width)
    }

    private func layoutCard(width: CGFloat) -> NSView {
        let inner = width - 32
        let layouts: [Layout] = [.bijoy, .phonetic]
        let segments = Segmented(
            [.init(title: "বিজয়", sub: "চেনা বিজয় লেআউট"), .init(title: "ফোনেটিক", sub: "ami → আমি")],
            selected: layouts.firstIndex(of: Settings.layout) ?? 0, height: 54
        ) { [weak self] index in
            self?.change { Settings.layout = layouts[index] }
        }
        segments.widthAnchor.constraint(equalToConstant: inner).isActive = true
        let hint = "⌃ Space চাপলে বাংলা চালু বা বন্ধ হয়। লেআউট মেনু বারের নু আইকন থেকেও বদলানো যায়।"
        return Self.card([
            Theme.heading("keyboard", "কিবোর্ড লেআউট"),
            segments,
            Theme.wrapping(hint, Fonts.ui(12.5), Theme.dim, width: inner),
        ], width: width)
    }

    private func typingCard(width: CGFloat) -> NSView {
        let inner = width - 32
        let mode = Settings.typingMode
        let modes = Self.modes.enumerated().map { index, entry in
            ModeCard(title: entry.1, tag: index == 0 ? "প্রস্তাবিত" : nil, detail: entry.2, on: mode == entry.0,
                     width: inner) { [weak self] in
                self?.change { Settings.typingMode = entry.0 }
            }
        }
        let bisarga = Settings.colonIsBisarga
        let colon = Segmented([.init(title: "কোলন  :"), .init(title: "বিসর্গ  দুঃ")], selected: bisarga ? 1 : 0,
                              height: 36) { [weak self] index in
            self?.change { Settings.colonIsBisarga = index == 1 }
        }
        colon.widthAnchor.constraint(equalToConstant: 190).isActive = true
        let colonRow = Theme.row("কোলন (:) চাপলে",
                                 detail: "কোলন বেছে নিলেও বিসর্গ-ওয়ালা শব্দ তালিকায় পাবেন (dukho → দুঃখ)।",
                                 accessory: colon, accessoryWidth: 190, width: inner)
        let list = Theme.vstack(modes, spacing: 8)
        let rule = Theme.rule(width: inner)
        return Self.card([Theme.heading("textformat", "টাইপিং মোড"), list, rule, colonRow], width: width,
                         spacing: [(list, 12), (rule, 13)])
    }

    private func suggestionsCard(width: CGFloat) -> NSView {
        let inner = width - 32
        var views: [NSView] = [Theme.heading("list.bullet", "সাজেশন")]
        let toggles: [(String, String, Bool, (Bool) -> Void)] = [
            ("অটোকারেক্ট", "চেনা ভুল বানান ঠিক করে দেখাও", Settings.autocorrect, { Settings.autocorrect = $0 }),
            ("ইমোজি", "হাসি → 😄", Settings.emoji, { Settings.emoji = $0 }),
            ("ইংরেজি শব্দ", "টাইপ করা ইংরেজি শব্দটিও তালিকায় রাখো", Settings.englishWord, { Settings.englishWord = $0 }),
        ]
        for (index, (title, detail, on, set)) in toggles.enumerated() {
            if index > 0 { views.append(Theme.rule(width: inner)) }
            let toggle = Switch(on: on) { [weak self] in self?.change { set(!on) } }
            views.append(Theme.row(title, detail: detail, accessory: toggle, accessoryWidth: 40, width: inner))
        }
        let positions: [PopupPosition] = [.below, .above]
        let directions: [PopupDirection] = [.vertical, .horizontal]
        let position = Segmented([.init(title: "লেখার নিচে"), .init(title: "লেখার উপরে")],
                                 selected: positions.firstIndex(of: Settings.popupPosition) ?? 0, height: 36) { [weak self] index in
            self?.change { Settings.popupPosition = positions[index] }
        }
        let direction = Segmented([.init(title: "উপর-নিচে"), .init(title: "পাশাপাশি")],
                                  selected: directions.firstIndex(of: Settings.popupDirection) ?? 0, height: 36) { [weak self] index in
            self?.change { Settings.popupDirection = directions[index] }
        }
        for (title, control) in [("তালিকার জায়গা", position), ("সাজানো", direction)] {
            control.widthAnchor.constraint(equalToConstant: 190).isActive = true
            views.append(Theme.rule(width: inner))
            views.append(Theme.row(title, accessory: control, accessoryWidth: 190, width: inner))
        }
        let card = Self.card(views, width: width)
        // Phonetic-only has no list: these don't apply to it.
        if Settings.typingMode == .phoneticOnly { card.disable() }
        return card
    }

    private func shortcutsCard(width: CGFloat) -> NSView {
        let inner = width - 32
        typealias Row = ([String], String)
        let always: [Row] = [(["⌃", "+", "Space"], "বাংলা চালু বা বন্ধ (আগের কিবোর্ডে ফেরা)")]
        let typing: [Row] = [
            (["Space"], "বাছাই করা শব্দটা বসবে, সাথে একটা স্পেস"),
            (["Return"], "বাছাই করা শব্দটা বসবে, নতুন লাইন হবে না"),
            (["↑", "↓", "বা", "Tab"], "তালিকায় অন্য শব্দ বাছাই"),
            (["1", "থেকে", "9"], "তালিকার ওই নম্বরের শব্দটা সরাসরি বসবে"),
            (["Esc"], "শব্দটা বাতিল, কিছুই বসবে না"),
            (["⌥", "+", "⌫"], "লিখতে থাকা পুরো শব্দটা একবারে মুছবে"),
        ]
        let keysWidth: CGFloat = 150
        var views: [NSView] = [Theme.heading("command", "শর্টকাট")]
        for (caption, rows) in [("সবসময়", always), ("ফোনেটিকে লেখার সময়, সাজেশন তালিকা খোলা থাকলে", typing)] {
            let caption = Theme.caption(caption)
            var lines: [NSView] = []
            for (keys, meaning) in rows {
                // Keys right-aligned in their column, with "বা" / "থেকে" / "+" between them.
                let caps = NSStackView(views: keys.map { key -> NSView in
                    ["বা", "থেকে", "+"].contains(key) ? Theme.label(key, Fonts.ui(11.5), Theme.dim) : KeyCap(key)
                })
                caps.spacing = 4
                caps.translatesAutoresizingMaskIntoConstraints = false
                let column = NSView()
                column.addSubview(caps)
                NSLayoutConstraint.activate([
                    caps.trailingAnchor.constraint(equalTo: column.trailingAnchor),
                    caps.topAnchor.constraint(equalTo: column.topAnchor),
                    caps.bottomAnchor.constraint(equalTo: column.bottomAnchor),
                    column.widthAnchor.constraint(equalToConstant: keysWidth),
                ])
                let line = NSStackView(views: [
                    column,
                    Theme.wrapping(meaning, Fonts.ui(13), .hex(0xFFFFFF, 0.78), width: inner - keysWidth - 14),
                ])
                line.spacing = 14
                line.alignment = .centerY
                lines.append(line)
            }
            let group = Theme.vstack([caption] + lines, spacing: 9)
            group.setCustomSpacing(8, after: caption)
            views.append(group)
        }
        return Self.card(views, width: width, spacing: [(views[1], 18)])
    }

    private func updateCard(width: CGFloat) -> NSView {
        let inner = width - 32
        let auto = Settings.autoUpdateCheck
        let toggle = Switch(on: auto) { [weak self] in self?.change { Settings.autoUpdateCheck = !auto } }
        let button = DialogButton(title: "এখনই খুঁজুন", brand: false, target: self, action: #selector(checkNow(_:)))
        button.isEnabled = !checking
        let buttonWidth = button.constraints.first { $0.firstAttribute == .width }?.constant ?? 100
        let status = status ?? ("এখন আছে সংস্করণ \(Updates.banglaDigits(Updates.current))", Theme.dim)
        let label = Theme.wrapping(status.text, Fonts.ui(12.5), status.color, width: inner - buttonWidth - 16)
        let checkRow = NSStackView(views: [label, button])
        checkRow.spacing = 16
        checkRow.alignment = .centerY
        return Self.card([
            Theme.heading("arrow.triangle.2.circlepath", "আপডেট"),
            Theme.row("নিজে থেকে আপডেট খুঁজুন", detail: "দিনে একবার দেখে নেয় নতুন সংস্করণ এসেছে কি না",
                      accessory: toggle, accessoryWidth: 40, width: inner),
            Theme.rule(width: inner),
            checkRow,
        ], width: width)
    }

    // MARK: Changes

    /// Saves a change and shows it. Rebuilt after the click that made it has been handled: the
    /// control clicked is replaced.
    private func change(_ save: @escaping () -> Void) {
        save()
        DispatchQueue.main.async { [weak self] in self?.render() }
    }

    /// "এখনই খুঁজুন": asks GitHub now. A new version opens the update dialog, even one skipped
    /// before: the user asked.
    @objc private func checkNow(_ sender: Any?) {
        checking = true
        status = ("খোঁজা হচ্ছে…", Theme.dim)
        DispatchQueue.main.async { [weak self] in self?.render() }
        Updates.shared.checkNow { [weak self] result in
            guard let self else { return }
            self.checking = false
            switch result {
            case .found(let release):
                self.status = ("নতুন সংস্করণ পাওয়া গেছে।", Theme.orange)
                UpdateWindow.shared.show(release, activate: true)
            case .latest:
                self.status = ("আপনি সর্বশেষ সংস্করণ ব্যবহার করছেন।", Theme.ink)
            case .failed:
                self.status = ("খোঁজা গেল না। ইন্টারনেট সংযোগ দেখে আবার চেষ্টা করুন।", Theme.dim)
            }
            self.render()
        }
    }

    func windowWillClose(_ notification: Notification) {
        // Opened again, it shows the version, not an old answer.
        status = nil
    }
}
