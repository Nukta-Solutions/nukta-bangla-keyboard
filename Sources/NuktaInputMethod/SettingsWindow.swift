import Cocoa
import NuktaPhonetic

/// সেটিংস, opened from the নু menu. Every change is saved and applied straight away.
final class SettingsWindow: NSObject, NSWindowDelegate {
    static let shared = SettingsWindow()

    private var window: NSWindow?
    private let layoutPopup = NSPopUpButton()
    private var modeButtons: [(TypingMode, NSButton)] = []
    private let autocorrect = NSButton(checkboxWithTitle: "অটোকারেক্ট: চেনা ভুল বানান ঠিক করে দেখাও", target: nil, action: nil)
    private let emoji = NSButton(checkboxWithTitle: "ইমোজি সাজেশন দেখাও (হাসি → 😄)", target: nil, action: nil)
    private let englishWord = NSButton(checkboxWithTitle: "টাইপ করা ইংরেজি শব্দটিও তালিকায় রাখো", target: nil, action: nil)
    private let colonPopup = NSPopUpButton()
    private let positionPopup = NSPopUpButton()
    private let directionPopup = NSPopUpButton()
    /// Everything that only matters for phonetic typing.
    private var phoneticControls: [NSControl] = []
    /// Everything that only matters while the suggestion list is in use.
    private var suggestionControls: [NSControl] = []

    private static let layouts: [(Layout, String)] = [(.bijoy, "বিজয় লেআউট"), (.phonetic, "ফোনেটিক")]
    private static let modes: [(TypingMode, String, String)] = [
        (.phoneticFirst, "ফোনেটিক আগে (প্রস্তাবিত)",
         "যেমন বানান লিখবেন ঠিক তেমন বাংলা বসবে; তালিকা থেকে অন্য শব্দ বেছে নিলে পরের বার সেটাই আগে আসবে।"),
        (.smart, "স্মার্ট",
         "অভিধান আর অটোকারেক্ট সবচেয়ে মানানসই শব্দ বেছে দেবে (sonar → সোনার)।"),
        (.phoneticOnly, "শুধু ফোনেটিক",
         "সাজেশন তালিকা ছাড়া, অক্ষর ধরে ধরে লেখা।"),
    ]
    private static let colonChoices: [(Bool, String)] = [(false, "কোলন  :"), (true, "বিসর্গ  ঃ")]
    private static let positions: [(PopupPosition, String)] = [(.below, "লেখার নিচে"), (.above, "লেখার উপরে")]
    private static let directions: [(PopupDirection, String)] = [(.vertical, "উপর-নিচে তালিকা"), (.horizontal, "পাশাপাশি এক সারি")]

    func show() {
        if window == nil { window = makeWindow() }
        refresh()
        // An agent app has to bring itself forward, or the window opens behind the current app.
        NSApp.activate(ignoringOtherApps: true)
        window?.center()
        window?.makeKeyAndOrderFront(nil)
        window?.orderFrontRegardless()
    }

    // MARK: Building

    private func makeWindow() -> NSWindow {
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 480, height: 10),
                              styleMask: [.titled, .closable], backing: .buffered, defer: false)
        window.title = "নুকতা বাংলা — সেটিংস"
        window.isReleasedWhenClosed = false
        window.delegate = self

        for (_, title) in Self.layouts { layoutPopup.addItem(withTitle: title) }
        for (_, title) in Self.colonChoices { colonPopup.addItem(withTitle: title) }
        for (_, title) in Self.positions { positionPopup.addItem(withTitle: title) }
        for (_, title) in Self.directions { directionPopup.addItem(withTitle: title) }
        for control in [layoutPopup, colonPopup, positionPopup, directionPopup, autocorrect, emoji, englishWord] {
            control.target = self
            control.action = #selector(changed(_:))
        }

        let modeStack = NSStackView()
        modeStack.orientation = .vertical
        modeStack.alignment = .leading
        modeStack.spacing = 8
        for (mode, title, detail) in Self.modes {
            let button = NSButton(radioButtonWithTitle: title, target: self, action: #selector(changed(_:)))
            modeButtons.append((mode, button))
            modeStack.addArrangedSubview(button)
            modeStack.addArrangedSubview(Self.note(detail, indent: 20))
        }

        suggestionControls = [autocorrect, emoji, englishWord, positionPopup, directionPopup]
        phoneticControls = modeButtons.map(\.1) + [colonPopup] + suggestionControls

        let stack = NSStackView(views: [
            Self.header("কিবোর্ড"),
            Self.row("লেআউট:", layoutPopup),
            Self.note("মেনু বারের নু আইকন থেকেও বদলানো যায়।"),
            Self.separator(),
            Self.header("ফোনেটিক টাইপিং"),
            modeStack,
            Self.row("কোলন (:) চাপলে:", colonPopup),
            Self.note("কোলন বেছে নিলেও ঃ-ওয়ালা শব্দ তালিকায় পাবেন (dukho → দুঃখ)।"),
            Self.separator(),
            Self.header("সাজেশন"),
            autocorrect,
            emoji,
            englishWord,
            Self.row("তালিকা দেখাও:", positionPopup),
            Self.row("সাজানো:", directionPopup),
            Self.note("Space বা Enter: বাছাই করা শব্দ বসবে · ↑↓ বা Tab: বাছাই বদলাও · ১–৯: সরাসরি বাছাই · Esc: বাদ দাও"),
        ])
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 10
        stack.edgeInsets = NSEdgeInsets(top: 20, left: 24, bottom: 24, right: 24)
        stack.translatesAutoresizingMaskIntoConstraints = false

        let content = NSView()
        content.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: content.topAnchor),
            stack.leadingAnchor.constraint(equalTo: content.leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: content.trailingAnchor),
            stack.bottomAnchor.constraint(equalTo: content.bottomAnchor),
            content.widthAnchor.constraint(equalToConstant: 480),
        ])
        window.contentView = content
        return window
    }

    private static func header(_ text: String) -> NSTextField {
        let label = NSTextField(labelWithString: text)
        label.font = .boldSystemFont(ofSize: NSFont.systemFontSize)
        return label
    }

    private static func note(_ text: String, indent: CGFloat = 0) -> NSView {
        let label = NSTextField(wrappingLabelWithString: text)
        label.font = .systemFont(ofSize: NSFont.smallSystemFontSize)
        label.textColor = .secondaryLabelColor
        label.preferredMaxLayoutWidth = 432 - indent
        guard indent > 0 else { return label }
        let row = NSStackView(views: [label])
        row.edgeInsets = NSEdgeInsets(top: -4, left: indent, bottom: 0, right: 0)
        return row
    }

    private static func row(_ title: String, _ control: NSView) -> NSView {
        let row = NSStackView(views: [NSTextField(labelWithString: title), control])
        row.spacing = 8
        return row
    }

    private static func separator() -> NSView {
        let box = NSBox()
        box.boxType = .separator
        box.widthAnchor.constraint(equalToConstant: 432).isActive = true
        return box
    }

    // MARK: State

    /// Shows the saved settings.
    private func refresh() {
        layoutPopup.selectItem(at: Self.layouts.firstIndex { $0.0 == Settings.layout } ?? 0)
        let mode = Settings.typingMode
        for (buttonMode, button) in modeButtons {
            button.state = buttonMode == mode ? .on : .off
        }
        autocorrect.state = Settings.autocorrect ? .on : .off
        emoji.state = Settings.emoji ? .on : .off
        englishWord.state = Settings.englishWord ? .on : .off
        colonPopup.selectItem(at: Self.colonChoices.firstIndex { $0.0 == Settings.colonIsBisarga } ?? 0)
        positionPopup.selectItem(at: Self.positions.firstIndex { $0.0 == Settings.popupPosition } ?? 0)
        directionPopup.selectItem(at: Self.directions.firstIndex { $0.0 == Settings.popupDirection } ?? 0)
        // Bijoy uses none of these, and phonetic-only has no list.
        let phonetic = Settings.layout == .phonetic
        for control in phoneticControls {
            control.isEnabled = phonetic
        }
        for control in suggestionControls {
            control.isEnabled = phonetic && mode != .phoneticOnly
        }
    }

    @objc private func changed(_ sender: NSControl) {
        switch sender {
        case layoutPopup:
            Settings.layout = Self.layouts[layoutPopup.indexOfSelectedItem].0
        case colonPopup:
            Settings.colonIsBisarga = Self.colonChoices[colonPopup.indexOfSelectedItem].0
        case autocorrect:
            Settings.autocorrect = autocorrect.state == .on
        case emoji:
            Settings.emoji = emoji.state == .on
        case englishWord:
            Settings.englishWord = englishWord.state == .on
        case positionPopup:
            Settings.popupPosition = Self.positions[positionPopup.indexOfSelectedItem].0
        case directionPopup:
            Settings.popupDirection = Self.directions[directionPopup.indexOfSelectedItem].0
        default:
            if let mode = modeButtons.first(where: { $0.1 === sender })?.0 {
                Settings.typingMode = mode
            }
        }
        refresh()
    }
}
