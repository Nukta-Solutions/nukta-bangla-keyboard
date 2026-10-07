import Cocoa
import InputMethodKit

/// Where the word being typed is on screen, for placing the suggestion list.
///
/// Apps answer the position questions with varying reliability. Chrome and Electron apps return
/// garbage for `firstRect` (uninitialised tiny values), and answer the line question for index 0
/// with the start of the page rather than the word. So the sources are tried in order of trust,
/// garbage is rejected, and the last good answer stands in when nothing better is available.
enum CursorRect {
    /// The last rect that passed the checks, in any app.
    private static var lastGood: NSRect?
    /// Per app, the last position of a word the app reported itself.
    private static var lastExact: [String: NSRect] = [:]

    /// Called after the marked text is set, so the marked range is the word in progress.
    /// `exact`: the app reported where the word is. Otherwise `rect` is a guess, and `exact(_:)`
    /// may know better a moment later: Chrome works the position out in the page's process, and
    /// for the first letters of a word it only knows the corner of the page.
    static func of(_ client: Client) -> (rect: NSRect, exact: Bool) {
        if let rect = exact(client) { return (rect, true) }
        let app = client.bundleIdentifier() ?? ""
        // The previous word in this app is usually on the same line; the page corner never is.
        if let rect = lastExact[app] {
            return (rect, false)
        }
        if let rect = lineRect(client, at: 0), isPlausible(rect) {
            // VS Code answers only this way, and correctly: the line of the word in progress.
            lastGood = rect
            return (rect, false)
        }
        if let lastGood { return (lastGood, false) }
        // Nothing to go on: put the list by the mouse pointer.
        return (NSRect(origin: NSEvent.mouseLocation, size: .zero), false)
    }

    /// The word's position, if the app reports it (yet).
    static func exact(_ client: Client) -> NSRect? {
        let marked = client.markedRange()
        let selection = client.selectedRange()
        let app = client.bundleIdentifier() ?? ""
        func found(_ rect: NSRect) -> NSRect {
            lastGood = rect
            lastExact[app] = rect
            return rect
        }

        // Mac apps answer this exactly; Chrome and Electron only ever with garbage, which fails the checks.
        for range in [marked, NSRange(location: selection.location, length: 0)] {
            if let rect = firstRect(client, at: range), isPlausible(rect) { return found(rect) }
        }

        // Chrome answers the line question late, and meanwhile with the corner of the page: the
        // same rect it gives for the start of the document. A word can't really be there.
        let documentStart = lineRect(client, at: 0)
        for index in [marked.location, selection.location] {
            guard let rect = lineRect(client, at: index), isPlausible(rect) else { continue }
            if let documentStart, rect.origin == documentStart.origin, rect.height == documentStart.height { continue }
            return found(rect)
        }
        return nil
    }

    // MARK: Sources

    /// The rect of a range of the document; the whole marked range lines the list up with the word.
    private static func firstRect(_ client: Client, at range: NSRange) -> NSRect? {
        guard range.location != NSNotFound else { return nil }
        // Some apps answer badly when there's nowhere to put the range they actually measured.
        var actual = NSRange(location: NSNotFound, length: 0)
        return client.firstRect(forCharacterRange: range, actualRange: &actual)
    }

    /// The line at a character index of the document.
    private static func lineRect(_ client: Client, at index: Int) -> NSRect? {
        guard index != NSNotFound else { return nil }
        var rect = NSRect.zero
        _ = client.attributes(forCharacterIndex: index, lineHeightRectangle: &rect)
        return rect
    }

    // MARK: Checks

    private static func isPlausible(_ rect: NSRect) -> Bool {
        let values = [rect.origin.x, rect.origin.y, rect.width, rect.height]
        guard values.allSatisfy(\.isFinite) else { return false }
        // A line of text has some height, and not more than a screen's worth.
        guard rect.height >= 2, rect.height < 1000, rect.width >= 0 else { return false }
        // Uninitialised memory tends to come back as values near zero.
        guard abs(rect.origin.x) >= 1 || abs(rect.origin.y) >= 1 else { return false }

        let screens = NSScreen.screens
        guard screens.contains(where: { $0.frame.insetBy(dx: -1, dy: -1).contains(rect.origin) }) else { return false }
        // A screen corner means the app didn't really know.
        let corners = screens.flatMap { screen -> [NSPoint] in
            let frame = screen.frame
            return [NSPoint(x: frame.minX, y: frame.minY), NSPoint(x: frame.minX, y: frame.maxY),
                    NSPoint(x: frame.maxX, y: frame.minY), NSPoint(x: frame.maxX, y: frame.maxY)]
        }
        let isCorner = corners.contains { abs($0.x - rect.minX) < 1 && (abs($0.y - rect.minY) < 1 || abs($0.y - rect.maxY) < 1) }
        return !isCorner
    }
}
