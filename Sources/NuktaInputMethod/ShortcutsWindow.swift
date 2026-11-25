import Cocoa
import NuktaPhonetic

/// নিজের শর্টকাট: where the user's shortcuts are added, changed and removed, opened by সাজান… in
/// the settings. The design is the dialog in windows/design/settings-preview.html, as on Windows.
///
/// The rules and the file live in riti-bridge (`Shortcuts`); this is only the window. The code box
/// takes English letters only (the system switches to an English keyboard while it has the focus).
/// The text box takes Bangla, or English letters, which are turned into Bangla as phonetic typing
/// would (`shuvo kamona` → শুভ কামনা), with a preview under it.
final class ShortcutsWindow: NSObject, NSWindowDelegate, NSTextFieldDelegate {
    static let shared = ShortcutsWindow()

    /// Called after every change: the settings show how many there are.
    var onChange: (() -> Void)?

    private var window: NSWindow?
    private lazy var store = Shortcuts(directory: Settings.dataDirectory)
    private let codeField = RomanTextField()
    private let textField = NSTextField()
    /// The code whose text is being changed (picked in the list, or typed).
    private var editing: String?
    /// A code the user was warned about: pressing the button again saves it.
    private var pending: String?
    private var warning: String?

    private static let width: CGFloat = 460
    private static let pad: CGFloat = 24
    private static let codeWidth: CGFloat = 96
    private static let rowHeight: CGFloat = 38

    func show() {
        let first = window == nil
        if first { build() }
        update()
        NSApp.activate(ignoringOtherApps: true)
        if first { window?.center() }
        window?.makeKeyAndOrderFront(nil)
        window?.orderFrontRegardless()
        if first { window?.makeFirstResponder(codeField) }
    }

    // MARK: Building

    /// What changes as the user types: the button, the preview, the warning, the list. The text
    /// boxes themselves stay where they are, so typing in them (Bangla being composed included) is
    /// never interrupted.
    private let buttonHolder = NSView()
    private var buttonWidth: NSLayoutConstraint?
    private var textBoxWidth: NSLayoutConstraint?
    private let preview = NSTextField(labelWithString: "")
    private lazy var warningLabel = Theme.wrapping("", Fonts.ui(12.5), .hex(0xF0A24E), width: Self.inner)
    private let listHolder = NSView()
    private var listConstraints: [NSLayoutConstraint] = []

    private static let inner = width - 2 * pad

    /// The window and everything in it that stays: done once.
    private func build() {
        let window = DarkWindow(contentRect: NSRect(x: 0, y: 0, width: Self.width, height: 10),
                                styleMask: [.titled, .closable], backing: .buffered, defer: false)
        window.title = "নুকতা বাংলা — নিজের শর্টকাট"
        Theme.style(window)
        window.delegate = self
        self.window = window
        setUpFields()

        let title = Theme.label("নিজের শর্টকাট", Fonts.ui(19, weight: .bold), Theme.ink)
        let lead = Theme.wrapping("কোডটা পুরো লিখলে লেখাটা তালিকার প্রথমে আসে, সাধারণ শব্দটা থাকে দ্বিতীয়তে। কোডের শুধু শুরুটা মিললে কিছু আসে না।",
                                  Fonts.ui(12.5), Theme.dim, width: Self.inner)
        let textBox = Self.box(textField)
        textBoxWidth = textBox.widthAnchor.constraint(equalToConstant: 200)
        textBoxWidth?.isActive = true
        let codeBox = Self.box(codeField)
        codeBox.widthAnchor.constraint(equalToConstant: Self.codeWidth).isActive = true
        buttonHolder.translatesAutoresizingMaskIntoConstraints = false
        buttonHolder.heightAnchor.constraint(equalToConstant: 34).isActive = true
        buttonWidth = buttonHolder.widthAnchor.constraint(equalToConstant: 100)
        buttonWidth?.isActive = true
        let row = NSStackView(views: [codeBox, textBox, buttonHolder])
        row.spacing = 8
        row.alignment = .centerY
        listHolder.translatesAutoresizingMaskIntoConstraints = false
        let hint = Theme.wrapping("কোড হবে ২ থেকে ১২টা ইংরেজি অক্ষর, সংখ্যা ছাড়া (লেখার সময় সংখ্যা চাপলে তালিকা থেকে শব্দ বাছাই হয়)। কোডের ঘরে নিজে থেকেই ইংরেজি কিবোর্ড আসে। লেখার ঘরে বাংলা লিখুন, বা ইংরেজি অক্ষরে: তা বাংলায় বদলে যায়। কোনো সারিতে ক্লিক করলে সেটা বদলানো যায়।",
                                  Fonts.ui(12.5), Theme.dim, width: Self.inner)
        let foot = Theme.wrapping("পরিবর্তন সাথে সাথে সব অ্যাপে চালু হয়", Fonts.ui(12), Theme.faint, width: Self.inner,
                                  alignment: .center)

        let stack = Theme.vstack([title, lead, row, preview, warningLabel, listHolder, hint, foot], spacing: 14)
        stack.setCustomSpacing(4, after: title)
        stack.setCustomSpacing(6, after: row)
        stack.edgeInsets = NSEdgeInsets(top: 16, left: Self.pad, bottom: 18, right: Self.pad)
        window.contentView = Theme.content(stack, width: Self.width)
    }

    /// Shows the state: the button's words, the preview, the warning and the list; the window
    /// follows the content's height, its top staying where it is.
    private func update() {
        guard let window else { return }
        let title = pending != nil ? "তবুও যোগ করুন" : editing != nil ? "বদলান" : "যোগ করুন"
        buttonHolder.subviews.forEach { $0.removeFromSuperview() }
        let button = DialogButton(title: title, brand: true, target: self, action: #selector(save(_:)))
        button.translatesAutoresizingMaskIntoConstraints = false
        buttonHolder.addSubview(button)
        let width = button.constraints.first { $0.firstAttribute == .width }?.constant ?? 100
        NSLayoutConstraint.activate([
            button.topAnchor.constraint(equalTo: buttonHolder.topAnchor),
            button.leadingAnchor.constraint(equalTo: buttonHolder.leadingAnchor),
        ])
        buttonWidth?.constant = width
        textBoxWidth?.constant = Self.inner - Self.codeWidth - width - 16

        updatePreview()
        warningLabel.stringValue = warning ?? ""
        warningLabel.isHidden = warning == nil

        NSLayoutConstraint.deactivate(listConstraints)
        listHolder.subviews.forEach { $0.removeFromSuperview() }
        let list = list(width: Self.inner)
        list.translatesAutoresizingMaskIntoConstraints = false
        listHolder.addSubview(list)
        listConstraints = [
            list.topAnchor.constraint(equalTo: listHolder.topAnchor),
            list.leadingAnchor.constraint(equalTo: listHolder.leadingAnchor),
            list.trailingAnchor.constraint(equalTo: listHolder.trailingAnchor),
            list.bottomAnchor.constraint(equalTo: listHolder.bottomAnchor),
        ]
        NSLayoutConstraint.activate(listConstraints)
        fit(window)
    }

    /// What the text will be, when it was typed in English letters.
    private func updatePreview() {
        let typed = textField.stringValue
        guard typed.contains(where: { $0.isASCII && $0.isLetter }) else {
            preview.isHidden = true
            return
        }
        let text = NSMutableAttributedString(string: "বসবে: ", attributes: [.font: Fonts.ui(12.5), .foregroundColor: Theme.dim])
        text.append(NSAttributedString(string: Shortcuts.transliterated(typed),
                                       attributes: [.font: Fonts.ui(13), .foregroundColor: Theme.ink]))
        preview.attributedStringValue = text
        preview.isHidden = false
    }

    private func fit(_ window: NSWindow) {
        guard let content = window.contentView else { return }
        content.layoutSubtreeIfNeeded()
        let old = window.frame
        let size = window.frameRect(forContentRect: NSRect(origin: .zero, size: content.fittingSize)).size
        window.setFrame(NSRect(x: old.minX, y: old.maxY - size.height, width: size.width, height: size.height),
                        display: true)
    }

    private func setUpFields() {
        for (field, placeholder, font) in [
            (codeField as NSTextField, "কোড: asw", NSFont.monospacedSystemFont(ofSize: 13.5, weight: .regular)),
            (textField, "পুরো লেখা", Fonts.ui(14)),
        ] {
            field.isBordered = false
            field.drawsBackground = false
            field.focusRingType = .none
            field.font = font
            field.textColor = Theme.ink
            field.usesSingleLineMode = true
            field.lineBreakMode = .byTruncatingTail
            field.placeholderAttributedString = NSAttributedString(
                string: placeholder, attributes: [.foregroundColor: Theme.faint, .font: Fonts.ui(13)])
            field.delegate = self
            field.target = self
            field.action = #selector(save(_:))
        }
    }

    /// A text field in a dark box with a border, 36 points high.
    private static func box(_ field: NSTextField) -> NSView {
        let box = Fill(Theme.box, radius: 6, border: Theme.border)
        field.translatesAutoresizingMaskIntoConstraints = false
        box.addSubview(field)
        NSLayoutConstraint.activate([
            box.heightAnchor.constraint(equalToConstant: 36),
            field.leadingAnchor.constraint(equalTo: box.leadingAnchor, constant: 9),
            field.trailingAnchor.constraint(equalTo: box.trailingAnchor, constant: -9),
            field.centerYAnchor.constraint(equalTo: box.centerYAnchor),
        ])
        return box
    }

    /// Every shortcut in a card, or a line saying there are none. Over six, the list scrolls.
    private func list(width: CGFloat) -> NSView {
        let entries = store.all
        guard !entries.isEmpty else {
            let empty = Theme.wrapping("এখনো কোনো শর্টকাট নেই।\nযেমন: asw → আসসালামু আলাইকুম", Fonts.ui(13), Theme.dim,
                                       width: width - 28, alignment: .center)
            return Card(empty, width: width, padding: NSEdgeInsets(top: 18, left: 14, bottom: 18, right: 14))
        }
        let rows = entries.map { entry in
            ShortcutRow(code: entry.code, text: entry.text, editing: entry.code == editing, width: width - 2,
                        height: Self.rowHeight,
                        onEdit: { [weak self] in self?.edit(entry.code, entry.text) },
                        onRemove: { [weak self] in self?.remove(entry.code) })
        }
        let stack = Theme.vstack(rows, spacing: 0)
        let height = CGFloat(min(entries.count, 6)) * Self.rowHeight
        let scroll = NSScrollView()
        scroll.drawsBackground = false
        scroll.hasVerticalScroller = entries.count > 6
        scroll.scrollerStyle = .overlay
        scroll.documentView = FlippedView(stack)
        scroll.translatesAutoresizingMaskIntoConstraints = false
        scroll.widthAnchor.constraint(equalToConstant: width - 2).isActive = true
        scroll.heightAnchor.constraint(equalToConstant: height).isActive = true
        return Card(scroll, width: width, padding: NSEdgeInsets(top: 1, left: 1, bottom: 1, right: 1))
    }

    // MARK: Changes

    func controlTextDidChange(_ notification: Notification) {
        guard let field = notification.object as? NSTextField else { return }
        let hadWarning = warning != nil || pending != nil
        pending = nil
        warning = nil
        if field === codeField {
            // Only English letters stay (pasted text too). Nothing is composed in this box, so the
            // whole state can be shown again.
            let typed = codeField.stringValue
            let kept = String(typed.filter { $0.isASCII && $0.isLetter }.lowercased().prefix(Shortcuts.maxCode))
            if kept != typed { codeField.stringValue = kept }
            editing = store.all.contains { $0.code == kept } ? kept : nil
            update()
        } else if hadWarning {
            update()
        } else {
            // The text box: only the preview, so a word being composed in it is left alone.
            updatePreview()
            if let window { fit(window) }
        }
    }

    /// যোগ করুন / বদলান / তবুও যোগ করুন, or Return in a box.
    @objc private func save(_ sender: Any?) {
        let code = codeField.stringValue.trimmingCharacters(in: .whitespaces).lowercased()
        let typed = textField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        guard (2...Shortcuts.maxCode).contains(code.count), code.allSatisfy({ $0.isASCII && $0.isLetter }) else {
            return warn("কোড হবে ২ থেকে ১২টা ইংরেজি অক্ষর, সংখ্যা ছাড়া।")
        }
        guard !typed.isEmpty else { return warn("পুরো লেখাটা দিন।") }
        let text = typed.contains(where: { $0.isASCII && $0.isLetter }) ? Shortcuts.transliterated(typed) : typed
        let exists = store.all.contains { $0.code == code }
        if !exists, pending != code, let word = Shortcuts.dictionaryWord(for: code) {
            return warn("\(code) দিয়ে অভিধানের শব্দ «\(word)»-ও লেখা হয়। শর্টকাট করলে \(word) লিখতে গেলে আগে শর্টকাট আসবে। তবুও রাখতে আবার চাপুন।",
                        pending: code)
        }
        switch store.set(code: code, text: text) {
        case .success:
            clear()
        case .failure(.textTooLong):
            warn("লেখাটা ৩০০ অক্ষরের বেশি হতে পারবে না।")
        case .failure:
            warn("সেভ করা গেল না। আবার চেষ্টা করুন।")
        }
    }

    private func warn(_ text: String, pending: String? = nil) {
        warning = text
        self.pending = pending
        update()
    }

    private func edit(_ code: String, _ text: String) {
        editing = code
        pending = nil
        warning = nil
        codeField.stringValue = code
        textField.stringValue = text
        update()
        window?.makeFirstResponder(textField)
    }

    private func remove(_ code: String) {
        store.remove(code: code)
        if editing == code {
            clear()
        } else {
            changed()
        }
    }

    /// After a save: empty boxes, the caret in the code box.
    private func clear() {
        editing = nil
        pending = nil
        warning = nil
        codeField.stringValue = ""
        textField.stringValue = ""
        changed()
        window?.makeFirstResponder(codeField)
    }

    private func changed() {
        update()
        onChange?()
    }
}

/// A text field that takes English letters only: while it has the focus, the system switches to
/// an English keyboard, so `asw` stays `asw` even with নুকতা বাংলা selected.
final class RomanTextField: NSTextField {
    override func becomeFirstResponder() -> Bool {
        let became = super.becomeFirstResponder()
        if became, let editor = currentEditor() as? NSTextView {
            editor.inputContext?.allowedInputSourceLocales = [NSAllRomanInputSourcesLocaleIdentifier]
        }
        return became
    }
}

/// One shortcut in the list: its code, its text, and ✕. A click on the row picks it for changing.
private final class ShortcutRow: HoverView {
    private let code: String
    private let text: String
    private let editing: Bool
    private let onEdit: () -> Void
    private let onRemove: () -> Void

    init(code: String, text: String, editing: Bool, width: CGFloat, height: CGFloat,
         onEdit: @escaping () -> Void, onRemove: @escaping () -> Void) {
        self.code = code
        self.text = text
        self.editing = editing
        self.onEdit = onEdit
        self.onRemove = onRemove
        super.init(frame: .zero)
        translatesAutoresizingMaskIntoConstraints = false
        widthAnchor.constraint(equalToConstant: width).isActive = true
        heightAnchor.constraint(equalToConstant: height).isActive = true
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    private var removeRect: NSRect {
        NSRect(x: bounds.maxX - 6 - 26, y: bounds.midY - 13, width: 26, height: 26)
    }

    override func mouseDown(with event: NSEvent) {
        let point = convert(event.locationInWindow, from: nil)
        if removeRect.contains(point) { onRemove() } else { onEdit() }
    }

    override func draw(_ dirtyRect: NSRect) {
        let hovered = pointer != nil
        if editing || hovered {
            (editing ? Theme.modeOn : Theme.box).setFill()
            NSBezierPath(roundedRect: bounds.insetBy(dx: 1, dy: 1), xRadius: 8, yRadius: 8).fill()
        }
        // The rule above every row but the first is the card's own top edge for the first.
        Theme.border.setFill()
        NSRect(x: 0, y: bounds.maxY - 1, width: bounds.width, height: 1).fill()

        let codeAttributes: [NSAttributedString.Key: Any] = [
            .font: NSFont.monospacedSystemFont(ofSize: 13, weight: .regular), .foregroundColor: Theme.orange,
        ]
        let codeSize = (code as NSString).size(withAttributes: codeAttributes)
        (code as NSString).draw(at: NSPoint(x: 12, y: bounds.midY - codeSize.height / 2), withAttributes: codeAttributes)

        let style = NSMutableParagraphStyle()
        style.lineBreakMode = .byTruncatingTail
        let textAttributes: [NSAttributedString.Key: Any] = [
            .font: Fonts.ui(14), .foregroundColor: Theme.ink, .paragraphStyle: style,
        ]
        let left: CGFloat = 12 + 96 + 8
        let textHeight = (text as NSString).size(withAttributes: textAttributes).height
        (text as NSString).draw(in: NSRect(x: left, y: bounds.midY - textHeight / 2,
                                           width: removeRect.minX - 8 - left, height: textHeight),
                                withAttributes: textAttributes)

        let overRemove = pointer.map { removeRect.contains($0) } ?? false
        if overRemove {
            Theme.boxHover.setFill()
            NSBezierPath(roundedRect: removeRect, xRadius: 6, yRadius: 6).fill()
        }
        HoverView.draw("✕", font: Fonts.ui(12), color: overRemove ? Theme.ink : Theme.dim,
                       centredAt: NSPoint(x: removeRect.midX, y: removeRect.midY))
    }
}

/// A scroll view's document: its content from the top down.
private final class FlippedView: NSView {
    init(_ content: NSView) {
        super.init(frame: .zero)
        content.translatesAutoresizingMaskIntoConstraints = false
        addSubview(content)
        NSLayoutConstraint.activate([
            content.topAnchor.constraint(equalTo: topAnchor),
            content.leadingAnchor.constraint(equalTo: leadingAnchor),
            content.trailingAnchor.constraint(equalTo: trailingAnchor),
            content.bottomAnchor.constraint(equalTo: bottomAnchor),
        ])
        frame.size = content.fittingSize
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    override var isFlipped: Bool { true }
}
