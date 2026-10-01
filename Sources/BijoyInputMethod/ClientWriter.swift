import Cocoa
import InputMethodKit
import os
import BijoyEngine

typealias Client = IMKTextInput & NSObjectProtocol

let log = Logger(subsystem: "com.asifmahmud.inputmethod.BijoyBangla", category: "writer")

/// Applies engine output to the client app.
///
/// `.direct`: text goes straight into the document; the few spellings that reorder text already
/// on screen (reph: র্ম; a waiting kar + juktakkhor: ক্তি) rewrite the last few characters with
/// `insertText(_:replacementRange:)` after checking they are still what we put there.
/// `.holdBack`: for apps that can't rewrite (terminals, and editors like Facebook or Google Docs
/// that ignore rewrites). The syllable in progress stays off screen and is inserted, finished,
/// on the key that completes it. Nothing is ever highlighted or underlined.
struct ClientWriter {
    enum Mode { case direct, holdBack }
    enum Failure { case textChanged, unsupported }

    /// Apps known not to support replacementRange/attributedSubstring.
    static let holdBackBundleIDs: Set<String> = [
        "com.apple.Terminal",
        "com.googlecode.iterm2",
        "net.kovidgoyal.kitty",
        "com.mitchellh.ghostty",
        "dev.warp.Warp-Stable",
        "org.alacritty",
        "io.alacritty",
        "com.github.wez.wezterm",
    ]
    /// Apps that can't report a cursor position, remembered for this login session.
    static var detectedHoldBackBundleIDs: Set<String> = []

    private var bundleID: String?
    /// The app never supports rewriting (terminal, or no cursor position).
    private var alwaysHoldBack = false
    /// A rewrite failed during this activation: stop rewriting in this field.
    private var untrusted = false
    private var mode: Mode { alwaysHoldBack || untrusted ? .holdBack : .direct }
    /// Exactly what is currently in the document for the syllable in progress (direct mode).
    private var shown = ""
    /// After a rewrite: where the cursor must be if the app really did it. Checked on the next key,
    /// because some editors (Facebook) accept the rewrite but just insert at the cursor.
    private var pendingRewriteCheck: Int?

    mutating func configure(for client: Client) {
        bundleID = client.bundleIdentifier()
        alwaysHoldBack = bundleID.map {
            Self.holdBackBundleIDs.contains($0) || Self.detectedHoldBackBundleIDs.contains($0)
        } ?? false
        untrusted = false
        shown = ""
        pendingRewriteCheck = nil
        let app = bundleID ?? "?", holdBack = alwaysHoldBack
        log.notice("activate \(app, privacy: .public) holdBack=\(holdBack)")
    }

    /// The syllable ended.
    mutating func reset() {
        shown = ""
    }

    /// The cursor may have moved (arrows, Enter, click, shortcut…).
    mutating func forgetContext() {
        pendingRewriteCheck = nil
    }

    /// Returns nil on success.
    @discardableResult
    mutating func apply(_ out: Output, to client: Client) -> Failure? {
        if let expected = pendingRewriteCheck {
            pendingRewriteCheck = nil
            let location = client.selectedRange().location
            if location != expected {
                return fail(.textChanged, "rewrite ignored (cursor \(location), expected \(expected))")
            }
        }
        switch mode {
        case .holdBack:
            applyHoldBack(out, to: client)
            return nil
        case .direct:
            return applyDirect(out, to: client)
        }
    }

    private mutating func applyDirect(_ out: Output, to client: Client) -> Failure? {
        let edit = TextDiff.edit(from: shown, to: out.committed + out.visible)
        let notFound = NSRange(location: NSNotFound, length: 0)

        if edit.deleted.isEmpty {
            if !edit.inserted.isEmpty {
                client.insertText(edit.inserted, replacementRange: notFound)
            }
            shown = TextDiff.remainder(of: shown + edit.inserted, after: out.committed)
            return nil
        }

        let deleteLength = edit.deleted.utf16.count
        let selection = client.selectedRange()
        guard selection.location != NSNotFound else {
            alwaysHoldBack = true
            if let id = bundleID { Self.detectedHoldBackBundleIDs.insert(id) }
            return fail(.unsupported, "no cursor position")
        }
        guard selection.length == 0, selection.location >= deleteLength else {
            return fail(.textChanged, "selection \(selection.location),\(selection.length) too short")
        }
        let range = NSRange(location: selection.location - deleteLength, length: deleteLength)
        guard let actual = client.attributedSubstring(from: range)?.string else {
            return fail(.unsupported, "read-back nil")
        }
        // String == uses canonical equivalence, so a client that normalises is fine.
        guard actual == edit.deleted else {
            return fail(.textChanged, "read-back mismatch")
        }
        client.insertText(edit.inserted, replacementRange: range)
        pendingRewriteCheck = range.location + edit.inserted.utf16.count
        let doc = TextDiff.dropLast(edit.deleted.unicodeScalars.count, of: shown) + edit.inserted
        shown = TextDiff.remainder(of: doc, after: out.committed)
        return nil
    }

    /// Only finished text goes in; the syllable in progress waits in the engine.
    private mutating func applyHoldBack(_ out: Output, to client: Client) {
        if !out.committed.isEmpty {
            client.insertText(out.committed, replacementRange: NSRange(location: NSNotFound, length: 0))
        }
        shown = ""
    }

    /// Rewriting failed: from now on this field holds syllables back instead.
    private mutating func fail(_ failure: Failure, _ reason: String) -> Failure {
        let app = bundleID ?? "?"
        log.notice("rewrite failed in \(app, privacy: .public): \(reason, privacy: .public) → hold back in this field")
        untrusted = true
        shown = ""
        pendingRewriteCheck = nil
        return failure
    }
}
