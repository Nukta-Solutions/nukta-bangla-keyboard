import Cocoa
import ApplicationServices

/// Avro-style rewriting for apps that ignore `insertText(_:replacementRange:)` (web editors,
/// Electron, terminals): press Backspace for the user, then insert the corrected text.
///
/// The Backspaces are real key events posted into the event stream, so the app processes them in
/// order. They are followed by a marker key (F19); when the marker comes back through
/// `BijoyInputController.handle`, the Backspaces have been delivered and the new text is inserted.
/// Posting key events needs the Accessibility permission.
final class Keystrokes {
    static let shared = Keystrokes()

    static let deleteKeyCode: CGKeyCode = 51
    static let markerKeyCode: CGKeyCode = 80 // F19: no app uses it

    private var pendingBackspaces = 0
    private var pendingTexts: [String] = []
    private var prompted = false

    var isTrusted: Bool { AXIsProcessTrusted() }

    /// Shows the system Accessibility prompt, once per run.
    func requestTrustIfNeeded() {
        guard !isTrusted, !prompted else { return }
        prompted = true
        let options = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true] as CFDictionary
        _ = AXIsProcessTrustedWithOptions(options)
        log.notice("requested Accessibility permission")
    }

    /// Deletes `count` characters before the cursor, then inserts `text`.
    /// Returns false (and does nothing) if events can't be posted.
    func replace(deleting count: Int, with text: String) -> Bool {
        guard isTrusted else { return false }
        let source = CGEventSource(stateID: .privateState)
        var events: [CGEvent] = []
        for _ in 0..<count {
            events += keyPress(Self.deleteKeyCode, source: source)
        }
        events += keyPress(Self.markerKeyCode, source: source)
        guard events.count == (count + 1) * 2 else { return false }

        pendingBackspaces += count
        pendingTexts.append(text)
        for event in events {
            event.post(tap: .cghidEventTap)
        }
        return true
    }

    /// A Backspace we posted: let the app handle it.
    func consumeBackspace() -> Bool {
        guard pendingBackspaces > 0 else { return false }
        pendingBackspaces -= 1
        return true
    }

    /// Our marker arrived: the Backspaces before it are done. Returns the text to insert now.
    func consumeMarker() -> String? {
        pendingTexts.isEmpty ? nil : pendingTexts.removeFirst()
    }

    private func keyPress(_ key: CGKeyCode, source: CGEventSource?) -> [CGEvent] {
        [true, false].compactMap { down in
            let event = CGEvent(keyboardEventSource: source, virtualKey: key, keyDown: down)
            event?.flags = [] // the user may still be holding Shift (Shift+A for reph)
            return event
        }
    }
}
