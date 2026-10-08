import Cocoa
import InputMethodKit
import NuktaEngine
import NuktaPhonetic

@objc(NuktaInputController)
final class NuktaInputController: IMKInputController {
    /// Bijoy: this controller's syllable engine and its writer (direct or marked text).
    private let engine = Engine()
    private var writer = ClientWriter()

    /// The layout this controller is typing with; follows `Settings.layout`.
    private var layout = Settings.layout
    /// Phonetic: marked text is on screen in this controller's client.
    private var hasMarked = false
    /// Last client seen, for committing outside a key event (a click in the list, a takeover).
    private weak var lastClient: Client?

    override func activateServer(_ sender: Any!) {
        super.activateServer(sender)
        layout = Settings.layout
        engine.reset()
        hasMarked = false
        if let client = sender as? Client {
            writer.configure(for: client)
            lastClient = client
        }
        if layout == .phonetic {
            // Nothing can be in flight for a client that is only now becoming active.
            Phonetic.shared.claim(self)
            Phonetic.shared.composer.discard()
            Phonetic.shared.panel.hide()
        }
    }

    override func deactivateServer(_ sender: Any!) {
        commitAll(sender as? Client)
        super.deactivateServer(sender)
    }

    override func commitComposition(_ sender: Any!) {
        commitAll(sender as? Client)
    }

    // MARK: Input menu (the নু icon)

    override func menu() -> NSMenu! {
        let menu = NSMenu()
        for (title, layout, action) in [
            ("বিজয় লেআউট", Layout.bijoy, #selector(selectBijoy(_:))),
            ("ফোনেটিক", Layout.phonetic, #selector(selectPhonetic(_:))),
        ] {
            let item = NSMenuItem(title: title, action: action, keyEquivalent: "")
            item.state = Settings.layout == layout ? .on : .off
            menu.addItem(item)
        }
        menu.addItem(.separator())
        let settings = NSMenuItem(title: "সেটিংস…", action: #selector(showSettings(_:)), keyEquivalent: "")
        settings.image = NSImage(systemSymbolName: "gearshape", accessibilityDescription: nil)
        menu.addItem(settings)
        let about = NSMenuItem(title: "নুকতা বাংলা সম্পর্কে…", action: #selector(showAbout(_:)), keyEquivalent: "")
        about.image = NSImage(systemSymbolName: "info.circle", accessibilityDescription: nil)
        menu.addItem(about)
        return menu
    }

    @objc func selectBijoy(_ sender: Any?) { switchLayout(to: .bijoy) }
    @objc func selectPhonetic(_ sender: Any?) { switchLayout(to: .phonetic) }

    private func switchLayout(to newLayout: Layout) {
        // Finish the word in progress with the layout it was typed in.
        commitAll(client())
        writer.forgetContext()
        Settings.layout = newLayout
        layout = newLayout
    }

    @objc func showSettings(_ sender: Any?) {
        SettingsWindow.shared.show()
    }

    @objc func showAbout(_ sender: Any?) {
        // Name, version, icon and copyright come from Info.plist.
        // An agent app has to bring itself forward, or the panel opens behind the current app.
        NSApp.activate(ignoringOtherApps: true)
        NSApp.orderFrontStandardAboutPanel(options: [.credits: Self.credits])
        NSApp.windows.filter(\.isVisible).forEach { $0.orderFrontRegardless() }
    }

    /// A one-word credit for the phonetic engine. The full licence notices ship in Resources/Licenses.
    private static var credits: NSAttributedString {
        let centered = NSMutableParagraphStyle()
        centered.alignment = .center
        return NSAttributedString(string: "কৃতজ্ঞতা: riti", attributes: [
            .font: NSFont.systemFont(ofSize: NSFont.smallSystemFontSize),
            .foregroundColor: NSColor.secondaryLabelColor,
            .paragraphStyle: centered,
        ])
    }

    // MARK: Keys

    override func recognizedEvents(_ sender: Any!) -> Int {
        Int(NSEvent.EventTypeMask([.keyDown, .leftMouseDown, .rightMouseDown]).rawValue)
    }

    override func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event, let client = sender as? Client else { return false }
        lastClient = client
        if layout != Settings.layout {
            // Picked in another app's menu.
            commitAll(client)
            layout = Settings.layout
        }

        guard event.type == .keyDown else {
            // A click may move the cursor: finish the word.
            commitAll(client)
            writer.forgetContext()
            return false
        }

        switch layout {
        case .bijoy: return handleBijoy(event, client: client)
        case .phonetic: return handlePhonetic(event, client: client)
        }
    }

    private func handleBijoy(_ event: NSEvent, client: Client) -> Bool {
        // Shortcuts (⌘C, ⌃A, ⌥…) go to the app untouched.
        if !event.modifierFlags.intersection([.command, .control, .option]).isEmpty {
            commitAll(client)
            writer.forgetContext()
            return false
        }

        if event.keyCode == KeyCodes.delete {
            guard let out = engine.backspace() else {
                writer.reset()
                writer.forgetContext()
                return false
            }
            if writer.apply(out, to: client) != nil {
                // Text under the cursor isn't ours any more: let the app delete normally.
                engine.reset()
                writer.reset()
                writer.forgetContext()
                return false
            }
            return true
        }

        let shift = event.modifierFlags.contains(.shift)
        guard let character = KeyCodes.character(for: event.keyCode, shift: shift),
              let out = engine.process(character: character) else {
            // Space, Enter, Tab, arrows, punctuation…: finish the syllable, let the app handle the key.
            commitAll(client)
            if let text = event.characters, !text.isEmpty, text.unicodeScalars.allSatisfy(Self.isPrintable) {
                writer.noteAppTyped(text)
            } else {
                writer.forgetContext()
            }
            return false
        }

        if writer.apply(out, to: client) != nil {
            // The app can't rewrite text here: start a fresh (marked) syllable.
            engine.reset()
            writer.reset()
            if let retry = engine.process(character: character) {
                writer.apply(retry, to: client)
            }
        }
        return true
    }

    private func handlePhonetic(_ event: NSEvent, client: Client) -> Bool {
        let phonetic = Phonetic.shared
        phonetic.claim(self)
        let flags = event.modifierFlags
        let option = flags.contains(.option)

        // Shortcuts go to the app; ⌥⌫ (delete the whole word) is ours.
        let isWordDelete = option && event.keyCode == KeyCodes.delete
        guard flags.intersection([.command, .control]).isEmpty, !option || isWordDelete,
              let key = Self.phoneticKey(for: event) else {
            apply(phonetic.composer.commit(), to: client)
            return false
        }
        let out = phonetic.composer.handle(key)
        apply(out, to: client)
        return out.handled
    }

    private static func phoneticKey(for event: NSEvent) -> PhoneticComposer.Key? {
        let shift = event.modifierFlags.contains(.shift)
        switch event.keyCode {
        case 36, 76: return .enter
        case 53: return .escape
        case KeyCodes.delete: return .backspace(wholeWord: event.modifierFlags.contains(.option))
        case 49: return .space
        case 48: return .tab(backward: shift)
        case 123: return .left
        case 124: return .right
        case 125: return .down
        case 126: return .up
        default:
            return KeyCodes.character(for: event.keyCode, shift: shift).map { .character($0, shift: shift) }
        }
    }

    /// Puts a phonetic result into the app and updates the suggestion list.
    private func apply(_ out: PhoneticComposer.Output, to client: Client) {
        let notFound = NSRange(location: NSNotFound, length: 0)
        if !out.insert.isEmpty {
            // Replaces the marked text.
            client.insertText(out.insert, replacementRange: notFound)
            hasMarked = false
        }
        if !out.marked.isEmpty || hasMarked {
            let attributed = NSAttributedString(string: out.marked, attributes: [
                .underlineStyle: NSUnderlineStyle.single.rawValue,
            ])
            client.setMarkedText(attributed,
                                 selectionRange: NSRange(location: out.marked.utf16.count, length: 0),
                                 replacementRange: notFound)
            hasMarked = !out.marked.isEmpty
        }
        Phonetic.shared.updatePanel(for: client)
    }

    // MARK: Phonetic callbacks

    /// Another app took over phonetic typing while this one still had a word in progress.
    func phoneticTakenOver() {
        guard Phonetic.shared.composer.isComposing, let client = client() ?? lastClient else { return }
        apply(Phonetic.shared.composer.commit(), to: client)
    }

    /// A suggestion was clicked.
    func pickCandidate(at index: Int) {
        guard let client = client() ?? lastClient else { return }
        apply(Phonetic.shared.composer.commit(at: index), to: client)
    }

    /// Settings changed: the word in progress is about to be dropped.
    func clearComposition() {
        guard hasMarked, let client = client() ?? lastClient else { return }
        client.setMarkedText("", selectionRange: NSRange(location: 0, length: 0),
                             replacementRange: NSRange(location: NSNotFound, length: 0))
        hasMarked = false
    }

    // MARK: Helpers

    /// Excludes control characters and the function-key range (arrows, F-keys…).
    private static func isPrintable(_ scalar: Unicode.Scalar) -> Bool {
        scalar.value >= 0x20 && scalar.value != 0x7F && !(0xF700...0xF8FF).contains(scalar.value)
    }

    /// Finishes whatever is being composed, in either layout.
    private func commitAll(_ client: Client?) {
        let out = engine.commit()
        if let client {
            writer.apply(out, to: client)
        }
        writer.reset()

        if let phonetic = Phonetic.loaded, phonetic.isOwner(self) {
            if phonetic.composer.isComposing, let client = client ?? lastClient {
                apply(phonetic.composer.commit(), to: client)
            }
            phonetic.panel.hide()
        }
    }
}

/// Physical key → character on a US layout, so both layouts work whatever ABC layout is underneath.
/// Keys a layout doesn't use (most punctuation in Bijoy) are passed through to the app.
enum KeyCodes {
    static let delete: UInt16 = 51

    private static let map: [UInt16: (Character, Character)] = [
        0: ("a", "A"), 1: ("s", "S"), 2: ("d", "D"), 3: ("f", "F"), 4: ("h", "H"),
        5: ("g", "G"), 6: ("z", "Z"), 7: ("x", "X"), 8: ("c", "C"), 9: ("v", "V"),
        11: ("b", "B"), 12: ("q", "Q"), 13: ("w", "W"), 14: ("e", "E"), 15: ("r", "R"),
        16: ("y", "Y"), 17: ("t", "T"), 31: ("o", "O"), 32: ("u", "U"), 34: ("i", "I"),
        35: ("p", "P"), 37: ("l", "L"), 38: ("j", "J"), 40: ("k", "K"), 45: ("n", "N"),
        46: ("m", "M"),
        18: ("1", "!"), 19: ("2", "@"), 20: ("3", "#"), 21: ("4", "$"), 23: ("5", "%"),
        22: ("6", "^"), 26: ("7", "&"), 28: ("8", "*"), 25: ("9", "("), 29: ("0", ")"),
        42: ("\\", "|"),
        27: ("-", "_"), 24: ("=", "+"), 33: ("[", "{"), 30: ("]", "}"), 41: (";", ":"),
        39: ("'", "\""), 43: (",", "<"), 47: (".", ">"), 44: ("/", "?"), 50: ("`", "~"),
    ]

    static func character(for keyCode: UInt16, shift: Bool) -> Character? {
        guard let pair = map[keyCode] else { return nil }
        return shift ? pair.1 : pair.0
    }
}
