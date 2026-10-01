import Cocoa
import InputMethodKit
import BijoyEngine

@objc(BijoyInputController)
final class BijoyInputController: IMKInputController {
    private let engine = Engine()
    private var writer = ClientWriter()

    override func activateServer(_ sender: Any!) {
        super.activateServer(sender)
        engine.reset()
        if let client = sender as? Client {
            writer.configure(for: client)
        }
    }

    override func deactivateServer(_ sender: Any!) {
        commitAll(sender as? Client)
        super.deactivateServer(sender)
    }

    override func commitComposition(_ sender: Any!) {
        commitAll(sender as? Client)
    }

    override func recognizedEvents(_ sender: Any!) -> Int {
        Int(NSEvent.EventTypeMask([.keyDown, .leftMouseDown, .rightMouseDown]).rawValue)
    }

    override func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event, let client = sender as? Client else { return false }

        guard event.type == .keyDown else {
            // A click may move the cursor: finish the syllable.
            commitAll(client)
            writer.forgetContext()
            return false
        }

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
              let key = KeyMap.key(for: character) else {
            // Space, Enter, Tab, arrows, punctuation…: finish the syllable, let the app handle the key.
            commitAll(client)
            if let text = event.characters, !text.isEmpty, text.unicodeScalars.allSatisfy(Self.isPrintable) {
                writer.noteAppTyped(text)
            } else {
                writer.forgetContext()
            }
            return false
        }

        if writer.apply(engine.process(key), to: client) != nil {
            // The app can't rewrite text here: start a fresh (marked) syllable.
            engine.reset()
            writer.reset()
            writer.apply(engine.process(key), to: client)
        }
        return true
    }

    /// Excludes control characters and the function-key range (arrows, F-keys…).
    private static func isPrintable(_ scalar: Unicode.Scalar) -> Bool {
        scalar.value >= 0x20 && scalar.value != 0x7F && !(0xF700...0xF8FF).contains(scalar.value)
    }

    private func commitAll(_ client: Client?) {
        let out = engine.commit()
        if let client {
            writer.apply(out, to: client)
        }
        writer.reset()
    }
}

/// Physical key → character on a US layout, so Bijoy works whatever ABC layout is underneath.
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
    ]

    static func character(for keyCode: UInt16, shift: Bool) -> Character? {
        guard let pair = map[keyCode] else { return nil }
        return shift ? pair.1 : pair.0
    }
}
