import Cocoa
import InputMethodKit
import os
import NuktaEngine

typealias Client = IMKTextInput & NSObjectProtocol

let log = Logger(subsystem: "com.asifmahmud.inputmethod.NuktaBangla", category: "writer")

/// Applies engine output to the client app. The mode is chosen per syllable:
///
/// `.direct`: text goes straight into the document; reordering (কি, র্ম, আ…) rewrites the
/// last few characters with `insertText(_:replacementRange:)` after checking they are still
/// what we put there.
/// `.marked`: the syllable in progress is shown as (underlined) marked text and inserted once
/// finished. Used where earlier text can't be read back reliably: terminals, and editors like
/// Google Docs that move typed text out of the input field straight away.
struct ClientWriter {
    enum Mode { case direct, marked }
    enum Failure { case textChanged, unsupported }

    /// Apps known not to support replacementRange/attributedSubstring.
    static let markedModeBundleIDs: Set<String> = [
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
    static var detectedMarkedModeBundleIDs: Set<String> = []

    private var bundleID: String?
    /// The app never supports rewriting (terminal, or no cursor position).
    private var alwaysMarked = false
    /// A read-back failed while rewriting during this activation: stop trusting this field.
    private var untrusted = false
    private var syllableMode: Mode = .direct
    /// Exactly what is currently in the document (or marked text) for the syllable in progress.
    private var shown = ""
    /// After a rewrite: where the cursor must be if the app really did it. Checked on the next key,
    /// because some editors (Facebook) accept the rewrite but just insert at the cursor.
    private var pendingRewriteCheck: Int?
    /// Last few characters we believe sit right before the cursor; empty when unknown.
    private var context = ""

    mutating func configure(for client: Client) {
        bundleID = client.bundleIdentifier()
        alwaysMarked = bundleID.map {
            Self.markedModeBundleIDs.contains($0) || Self.detectedMarkedModeBundleIDs.contains($0)
        } ?? false
        untrusted = false
        shown = ""
        context = ""
        pendingRewriteCheck = nil
        let app = bundleID ?? "?", marked = alwaysMarked
        log.notice("activate \(app, privacy: .public) alwaysMarked=\(marked)")
    }

    /// The syllable ended.
    mutating func reset() {
        shown = ""
    }

    /// The cursor may have moved (arrows, Enter, click, shortcut…).
    mutating func forgetContext() {
        context = ""
        pendingRewriteCheck = nil
    }

    /// The app itself typed this (a key we passed through, like space or a comma).
    mutating func noteAppTyped(_ text: String) {
        remember(deleting: 0, inserting: text)
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
        if shown.isEmpty {
            syllableMode = chooseMode(for: client)
        }
        switch syllableMode {
        case .marked:
            applyMarked(out, to: client)
            return nil
        case .direct:
            return applyDirect(out, to: client)
        }
    }

    /// Before a syllable starts: can we see what we typed last? If not, the app isn't
    /// keeping our text where we can rewrite it, so compose this syllable as marked text.
    private mutating func chooseMode(for client: Client) -> Mode {
        if alwaysMarked || untrusted { return .marked }

        let selection = client.selectedRange()
        guard selection.location != NSNotFound else {
            alwaysMarked = true
            if let id = bundleID { Self.detectedMarkedModeBundleIDs.insert(id) }
            let app = bundleID ?? "?"
            log.notice("probe: no cursor position in \(app, privacy: .public) → marked for session")
            return .marked
        }
        guard !context.isEmpty else { return .direct }

        let tail = String(String.UnicodeScalarView(context.unicodeScalars.suffix(2)))
        let length = tail.utf16.count
        guard selection.length == 0, selection.location >= length else {
            log.notice("probe: selection \(selection.location),\(selection.length) can't hold context → marked")
            context = ""
            return .marked
        }
        let actual = client.attributedSubstring(from: NSRange(location: selection.location - length, length: length))?.string
        guard actual == tail else {
            log.notice("probe: read-back mismatch at \(selection.location) (got \(actual == nil ? "nil" : "\(actual!.utf16.count) units", privacy: .public)) → marked")
            context = ""
            return .marked
        }
        return .direct
    }

    private mutating func applyDirect(_ out: Output, to client: Client) -> Failure? {
        let edit = TextDiff.edit(from: shown, to: out.committed + out.visible)
        let notFound = NSRange(location: NSNotFound, length: 0)

        if edit.deleted.isEmpty {
            if !edit.inserted.isEmpty {
                client.insertText(edit.inserted, replacementRange: notFound)
            }
            remember(deleting: 0, inserting: edit.inserted)
            shown = TextDiff.remainder(of: shown + edit.inserted, after: out.committed)
            return nil
        }

        let deleteLength = edit.deleted.utf16.count
        let selection = client.selectedRange()
        guard selection.location != NSNotFound else {
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
        remember(deleting: edit.deleted.unicodeScalars.count, inserting: edit.inserted)
        let doc = TextDiff.dropLast(edit.deleted.unicodeScalars.count, of: shown) + edit.inserted
        shown = TextDiff.remainder(of: doc, after: out.committed)
        return nil
    }

    private mutating func applyMarked(_ out: Output, to client: Client) {
        let notFound = NSRange(location: NSNotFound, length: 0)
        if !out.committed.isEmpty {
            client.insertText(out.committed, replacementRange: notFound)
            remember(deleting: 0, inserting: out.committed)
        }
        // `visible`, not `display`: a waiting kar would sit beside the previous letter.
        if !out.visible.isEmpty || (out.committed.isEmpty && !shown.isEmpty) {
            client.setMarkedText(
                out.visible,
                selectionRange: NSRange(location: out.visible.utf16.count, length: 0),
                replacementRange: notFound
            )
        }
        shown = out.visible
    }

    /// Rewriting failed mid-syllable: from now on this field gets marked text.
    private mutating func fail(_ failure: Failure, _ reason: String) -> Failure {
        let app = bundleID ?? "?"
        log.notice("rewrite failed in \(app, privacy: .public): \(reason, privacy: .public) → marked for this field")
        untrusted = true
        shown = ""
        context = ""
        pendingRewriteCheck = nil
        return failure
    }

    private mutating func remember(deleting count: Int, inserting text: String) {
        var scalars = Array(context.unicodeScalars)
        scalars.removeLast(min(count, scalars.count))
        scalars.append(contentsOf: text.unicodeScalars)
        context = String(String.UnicodeScalarView(scalars.suffix(8)))
    }
}
