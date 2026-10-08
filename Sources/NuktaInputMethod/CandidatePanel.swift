import Cocoa

/// The suggestion list shown under (or over) the word being typed.
///
/// A borderless, non-activating panel: the app being typed in keeps focus, even while the list is
/// clicked. It floats above normal windows, on every Space and over full-screen apps.
final class CandidatePanel {
    /// A candidate was clicked (index into the whole list).
    var onSelect: ((Int) -> Void)?

    private let panel: NSPanel
    private let list = CandidateListView()

    init() {
        panel = ListPanel(contentRect: NSRect(x: 0, y: 0, width: 100, height: 40),
                          styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: true)
        panel.level = .popUpMenu
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .stationary, .ignoresCycle]
        panel.isFloatingPanel = true
        panel.hidesOnDeactivate = false
        panel.becomesKeyOnlyIfNeeded = true
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = true
        panel.isReleasedWhenClosed = false
        panel.contentView = list
        list.onSelect = { [weak self] index in self?.onSelect?(index) }
    }

    func show(candidates: [String], auxiliary: String, selected: Int, cursor: NSRect,
              position: PopupPosition, direction: PopupDirection) {
        guard !candidates.isEmpty else {
            hide()
            return
        }
        list.update(candidates: candidates, auxiliary: auxiliary, selected: selected,
                    horizontal: direction == .horizontal)
        let size = list.fittingSize
        panel.setFrame(Self.frame(for: size, cursor: cursor, position: position), display: false)
        list.needsDisplay = true
        panel.display()
        // The shadow follows the drawn (rounded) shape, so it is redone after drawing.
        panel.invalidateShadow()
        panel.orderFrontRegardless()
    }

    func hide() {
        list.cancelPress()
        panel.orderOut(nil)
    }

    /// Left edge at the cursor; under it (or over it) with a 4 pt gap, on the other side when the
    /// preferred one has no room, and always inside the visible part of the cursor's screen.
    private static func frame(for size: NSSize, cursor: NSRect, position: PopupPosition) -> NSRect {
        let gap: CGFloat = 4
        let screen = NSScreen.screens.first { $0.frame.contains(cursor.origin) }
            ?? NSScreen.screens.first { $0.frame.intersects(cursor) }
            ?? NSScreen.main
        let visible = screen?.visibleFrame ?? NSRect(origin: cursor.origin, size: size)

        let belowY = cursor.minY - gap - size.height
        let aboveY = cursor.maxY + gap
        let fitsBelow = belowY >= visible.minY
        let fitsAbove = aboveY + size.height <= visible.maxY
        var y: CGFloat
        switch position {
        case .below: y = fitsBelow || !fitsAbove ? belowY : aboveY
        case .above: y = fitsAbove || !fitsBelow ? aboveY : belowY
        }
        let x = min(max(cursor.minX, visible.minX), visible.maxX - size.width)
        y = min(max(y, visible.minY), visible.maxY - size.height)
        return NSRect(origin: NSPoint(x: x, y: y), size: size)
    }
}

/// Never becomes key or main: typing stays in the app.
private final class ListPanel: NSPanel {
    override var canBecomeKey: Bool { false }
    override var canBecomeMain: Bool { false }
}

/// Draws the typed text and up to nine candidates, as a column or a row, and handles clicks.
private final class CandidateListView: NSView {
    var onSelect: ((Int) -> Void)?

    /// At most this many candidates show at once; the list scrolls to keep the selection in view.
    private static let pageSize = 9
    private static let padding: CGFloat = 6
    private static let cellPadding = NSSize(width: 8, height: 3)
    private static let cornerRadius: CGFloat = 8
    private static let wordFont = NSFont.systemFont(ofSize: 16)
    private static let numberFont = NSFont.monospacedDigitSystemFont(ofSize: 11, weight: .regular)
    private static let auxiliaryFont = NSFont.systemFont(ofSize: NSFont.smallSystemFontSize)

    private var candidates: [String] = []
    private var auxiliary = ""
    private var selected = 0
    private var horizontal = false
    /// Index of the first candidate shown.
    private var first = 0
    /// Candidate under a mouse press that hasn't been released yet.
    private var pressed: Int?

    /// Laid out by `update`: each shown candidate's index and frame (flipped coordinates).
    private var cells: [(index: Int, frame: NSRect)] = []
    private var auxiliaryFrame = NSRect.zero
    private var moreFrame = NSRect.zero
    private var contentSize = NSSize.zero

    override var isFlipped: Bool { true }
    override var fittingSize: NSSize { contentSize }
    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }

    func update(candidates: [String], auxiliary: String, selected: Int, horizontal: Bool) {
        if candidates != self.candidates { first = 0 }
        self.candidates = candidates
        self.auxiliary = auxiliary
        self.selected = min(max(selected, 0), candidates.count - 1)
        self.horizontal = horizontal
        pressed = nil

        // Scroll just enough to show the selection.
        if self.selected < first { first = self.selected }
        if self.selected >= first + Self.pageSize { first = self.selected - Self.pageSize + 1 }
        first = max(0, min(first, candidates.count - Self.pageSize))
        arrange()
    }

    func cancelPress() {
        guard pressed != nil else { return }
        pressed = nil
        needsDisplay = true
    }

    // MARK: Layout

    private var shown: Range<Int> {
        first..<min(first + Self.pageSize, candidates.count)
    }

    /// Where more candidates exist beyond those shown, as a small arrow.
    private var moreSign: String? {
        let before = first > 0, after = shown.upperBound < candidates.count
        switch (before, after) {
        case (false, false): return nil
        case (true, true): return horizontal ? "◂▸" : "▴▾"
        case (true, false): return horizontal ? "◂" : "▴"
        case (false, true): return horizontal ? "▸" : "▾"
        }
    }

    private func arrange() {
        let padding = Self.padding
        let auxiliarySize = text(auxiliary.isEmpty ? " " : auxiliary, font: Self.auxiliaryFont, color: .secondaryLabelColor).size()
        let moreSize = moreSign.map { text($0, font: Self.auxiliaryFont, color: .tertiaryLabelColor).size() } ?? .zero
        auxiliaryFrame = NSRect(x: padding + Self.cellPadding.width, y: padding,
                                width: ceil(auxiliarySize.width), height: ceil(auxiliarySize.height))

        let sizes = shown.map { cellSize(for: $0) }
        let rowHeight = sizes.map(\.height).max() ?? 0
        var cells: [(index: Int, frame: NSRect)] = []
        let top = auxiliaryFrame.maxY + 2
        var width: CGFloat = 0, height: CGFloat = 0
        if horizontal {
            var x = padding
            for (index, size) in zip(shown, sizes) {
                cells.append((index, NSRect(x: x, y: top, width: size.width, height: rowHeight)))
                x += size.width + 2
            }
            width = x - 2 + padding
            height = top + rowHeight + padding
        } else {
            let columnWidth = sizes.map(\.width).max() ?? 0
            var y = top
            for index in shown {
                cells.append((index, NSRect(x: padding, y: y, width: columnWidth, height: rowHeight)))
                y += rowHeight
            }
            width = columnWidth + 2 * padding
            height = y + padding
        }

        // The typed text and the "more" sign share the top line.
        let topLine = auxiliaryFrame.maxX + (moreSize.width > 0 ? 12 + ceil(moreSize.width) : 0) + Self.cellPadding.width + padding
        width = max(width, topLine, 60)
        moreFrame = NSRect(x: width - padding - Self.cellPadding.width - ceil(moreSize.width), y: padding,
                           width: ceil(moreSize.width), height: ceil(moreSize.height))
        if !horizontal {
            cells = cells.map { ($0.index, NSRect(x: $0.frame.minX, y: $0.frame.minY,
                                                  width: width - 2 * padding, height: $0.frame.height)) }
        }
        self.cells = cells
        contentSize = NSSize(width: ceil(width), height: ceil(height))
        setFrameSize(contentSize)
    }

    private func cellSize(for index: Int) -> NSSize {
        let number = numberText(index, highlighted: false).size()
        let word = text(candidates[index], font: Self.wordFont, color: .labelColor).size()
        return NSSize(width: ceil(number.width + 4 + word.width + 2 * Self.cellPadding.width),
                      height: ceil(max(number.height, word.height) + 2 * Self.cellPadding.height))
    }

    // MARK: Drawing

    override func draw(_ dirtyRect: NSRect) {
        let background = NSBezierPath(roundedRect: bounds, xRadius: Self.cornerRadius, yRadius: Self.cornerRadius)
        NSColor.windowBackgroundColor.setFill()
        background.fill()
        NSColor.separatorColor.setStroke()
        let border = NSBezierPath(roundedRect: bounds.insetBy(dx: 0.5, dy: 0.5),
                                  xRadius: Self.cornerRadius - 0.5, yRadius: Self.cornerRadius - 0.5)
        border.lineWidth = 1
        border.stroke()

        text(auxiliary, font: Self.auxiliaryFont, color: .secondaryLabelColor).draw(at: auxiliaryFrame.origin)
        if let moreSign {
            text(moreSign, font: Self.auxiliaryFont, color: .tertiaryLabelColor).draw(at: moreFrame.origin)
        }

        let highlightedIndex = pressed ?? selected
        for (index, frame) in cells {
            let highlighted = index == highlightedIndex
            if highlighted {
                NSColor.selectedContentBackgroundColor.setFill()
                NSBezierPath(roundedRect: frame, xRadius: 5, yRadius: 5).fill()
            }
            let number = numberText(index, highlighted: highlighted)
            let word = text(candidates[index], font: Self.wordFont,
                            color: highlighted ? .alternateSelectedControlTextColor : .labelColor)
            let wordSize = word.size(), numberSize = number.size()
            let x = frame.minX + Self.cellPadding.width
            number.draw(at: NSPoint(x: x, y: frame.midY - numberSize.height / 2 + 1))
            word.draw(at: NSPoint(x: x + ceil(numberSize.width) + 4, y: frame.midY - wordSize.height / 2))
        }
    }

    private func numberText(_ index: Int, highlighted: Bool) -> NSAttributedString {
        text("\(index + 1)", font: Self.numberFont,
             color: highlighted ? .alternateSelectedControlTextColor : .secondaryLabelColor)
    }

    private func text(_ string: String, font: NSFont, color: NSColor) -> NSAttributedString {
        NSAttributedString(string: string, attributes: [.font: font, .foregroundColor: color])
    }

    // MARK: Mouse

    private func candidate(at event: NSEvent) -> Int? {
        let point = convert(event.locationInWindow, from: nil)
        return cells.first { $0.frame.contains(point) }?.index
    }

    override func mouseDown(with event: NSEvent) {
        pressed = candidate(at: event)
        needsDisplay = true
    }

    override func mouseUp(with event: NSEvent) {
        let released = candidate(at: event)
        let wasPressed = pressed
        pressed = nil
        needsDisplay = true
        if let released, released == wasPressed {
            onSelect?(released)
        }
    }
}
