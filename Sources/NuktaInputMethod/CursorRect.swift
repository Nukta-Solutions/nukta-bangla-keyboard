// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of
// the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Adapted from Lekho (https://github.com/ARahim3/Lekho), Lekho/Sources/InputController.swift.

import Cocoa
import InputMethodKit

/// Where the word being composed is on screen, for placing the suggestion list. Apps report it
/// with varying reliability, so this tries several ways and remembers the last good answer.
enum CursorRect {
    private static var lastKnown = NSRect.zero

    /// Call after setting the marked text, so the marked range is valid.
    static func of(_ client: Client) -> NSRect {
        let marked = client.markedRange()
        var tries: [NSRange] = []
        if marked.location != NSNotFound {
            // The whole word first: the list lines up with its start.
            tries.append(marked)
            tries.append(NSRange(location: marked.location + marked.length, length: 0))
        }
        let selection = client.selectedRange()
        if selection.location != NSNotFound {
            tries.append(selection)
        }
        for range in tries {
            let rect = client.firstRect(forCharacterRange: range, actualRange: nil)
            if isValid(rect) { return remember(rect) }
        }

        for index in [marked.location, selection.location, 0] where index != NSNotFound {
            var lineRect = NSRect.zero
            client.attributes(forCharacterIndex: index, lineHeightRectangle: &lineRect)
            if isValid(lineRect) { return remember(lineRect) }
        }

        if lastKnown.height >= 1 { return lastKnown }

        // Last resort: the mouse pointer.
        let mouse = NSEvent.mouseLocation
        return remember(NSRect(x: mouse.x, y: mouse.y - 20, width: 0, height: 20))
    }

    private static func remember(_ rect: NSRect) -> NSRect {
        lastKnown = rect
        return rect
    }

    /// Rejects what Chrome and Electron sometimes return: uninitialised (subnormal) values, the
    /// screen corner, zero height, or a point on no screen.
    private static func isValid(_ rect: NSRect) -> Bool {
        if rect.origin.x.isSubnormal || rect.origin.y.isSubnormal
            || rect.size.width.isSubnormal || rect.size.height.isSubnormal { return false }
        if rect.origin.x < 1 && rect.origin.y < 1 { return false }
        if rect.size.height < 1 { return false }
        return NSScreen.screens.contains { $0.frame.contains(rect.origin) }
    }
}
