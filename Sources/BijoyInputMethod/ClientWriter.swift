import Cocoa
import InputMethodKit
import os
import BijoyEngine

typealias Client = IMKTextInput & NSObjectProtocol

let log = Logger(subsystem: "com.asifmahmud.inputmethod.BijoyBangla", category: "writer")

/// Applies engine output to the client app.
///
/// Text normally goes straight into the document as pure appends. The few spellings that
/// reorder text already on screen (reph: র্ম; a single-press kar + juktakkhor: চ্ছে) need a
/// rewrite, done one of two ways:
/// - native apps: `insertText(_:replacementRange:)`, after checking the text is still ours;
/// - web editors, Electron apps and terminals, which ignore that (they add instead of replace):
///   Avro-style Backspaces via `Keystrokes`.
/// Without the Accessibility permission those apps fall back to `.holdBack`: the syllable in
/// progress stays off screen and is inserted finished. Nothing is ever highlighted.
struct ClientWriter {
    enum Mode { case direct, holdBack }
    enum Failure { case textChanged, unsupported }

    /// Apps whose text boxes don't honour replacementRange.
    static let keystrokeBundleIDs: Set<String> = [
        // terminals
        "com.apple.Terminal", "com.googlecode.iterm2", "net.kovidgoyal.kitty", "com.mitchellh.ghostty",
        "dev.warp.Warp-Stable", "org.alacritty", "io.alacritty", "com.github.wez.wezterm",
        // browsers (Facebook, Google Docs, Messenger… run in these)
        "com.google.Chrome", "com.google.Chrome.canary", "com.google.Chrome.beta", "org.chromium.Chromium",
        "com.microsoft.edgemac", "com.brave.Browser", "company.thebrowser.Browser", "com.operasoftware.Opera",
        "com.vivaldi.Vivaldi", "com.apple.Safari", "com.apple.SafariTechnologyPreview", "org.mozilla.firefox",
    ]
    /// Apps found ignoring a rewrite, remembered for this login session.
    static var detectedKeystrokeBundleIDs: Set<String> = []

    private var bundleID: String?
    /// Rewrite with Backspaces instead of replacementRange in this app.
    private var rewriteByKeystrokes = false
    private var mode: Mode {
        rewriteByKeystrokes && !Keystrokes.shared.isTrusted ? .holdBack : .direct
    }
    /// Exactly what is currently in the document for the syllable in progress.
    private var shown = ""
    /// After a replacementRange rewrite: where the cursor must be if the app really did it.
    /// Checked on the next key; a mismatch means this app ignores rewrites.
    private var pendingRewriteCheck: Int?

    mutating func configure(for client: Client) {
        bundleID = client.bundleIdentifier()
        rewriteByKeystrokes = bundleID.map(Self.needsKeystrokes) ?? false
        if rewriteByKeystrokes { Keystrokes.shared.requestTrustIfNeeded() }
        shown = ""
        pendingRewriteCheck = nil
        let app = bundleID ?? "?", keys = rewriteByKeystrokes, trusted = Keystrokes.shared.isTrusted
        log.notice("activate \(app, privacy: .public) keystrokes=\(keys) trusted=\(trusted)")
    }

    /// Listed, already caught ignoring a rewrite, or built on Electron (VS Code, Slack, Discord…).
    private static func needsKeystrokes(_ id: String) -> Bool {
        if keystrokeBundleIDs.contains(id) || detectedKeystrokeBundleIDs.contains(id) { return true }
        guard let app = NSWorkspace.shared.urlForApplication(withBundleIdentifier: id) else { return false }
        let electron = app.appendingPathComponent("Contents/Frameworks/Electron Framework.framework")
        return FileManager.default.fileExists(atPath: electron.path)
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

        let doc = TextDiff.dropLast(edit.deleted.unicodeScalars.count, of: shown) + edit.inserted

        if rewriteByKeystrokes {
            guard Keystrokes.shared.replace(deleting: edit.deleted.unicodeScalars.count, with: edit.inserted) else {
                return fail(.unsupported, "can't post Backspaces")
            }
            shown = TextDiff.remainder(of: doc, after: out.committed)
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

    /// Rewriting with replacementRange failed: from now on this app rewrites with Backspaces
    /// (or holds syllables back until the Accessibility permission is granted).
    private mutating func fail(_ failure: Failure, _ reason: String) -> Failure {
        let app = bundleID ?? "?"
        log.notice("rewrite failed in \(app, privacy: .public): \(reason, privacy: .public) → keystrokes for this app")
        rewriteByKeystrokes = true
        if let id = bundleID { Self.detectedKeystrokeBundleIDs.insert(id) }
        Keystrokes.shared.requestTrustIfNeeded()
        shown = ""
        pendingRewriteCheck = nil
        return failure
    }
}
