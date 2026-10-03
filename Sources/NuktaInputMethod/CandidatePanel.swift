// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of
// the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Adapted from Lekho (https://github.com/ARahim3/Lekho), Lekho/Sources/CandidatePanel.swift: adds the
// horizontal row and the above/below choice, and drops the font settings.

import Cocoa

/// The suggestion list: a borderless panel that never takes focus from the app being typed in.
final class CandidatePanel {
    private var panel: NSPanel?
    private var view: CandidateView?

    /// A candidate was clicked. Parameter: its index.
    var onSelect: ((Int) -> Void)?

    func show(candidates: [String], auxiliary: String, selected: Int, cursor: NSRect,
              position: PopupPosition, direction: PopupDirection) {
        if panel == nil { createPanel() }
        guard let panel, let view else { return }

        view.update(candidates: candidates, auxiliary: auxiliary, selected: selected, direction: direction)
        let size = view.idealSize()
        panel.setContentSize(size)

        let gap: CGFloat = 4
        let below = cursor.minY - size.height - gap
        let above = cursor.maxY + gap
        var origin = NSPoint(x: cursor.minX, y: position == .below ? below : above)

        let center = NSPoint(x: cursor.midX, y: cursor.midY)
        if let screen = NSScreen.screens.first(where: { $0.frame.contains(center) }) ?? NSScreen.main {
            let frame = screen.visibleFrame
            origin.x = max(frame.minX, min(origin.x, frame.maxX - size.width))
            // No room on the preferred side: use the other one.
            if position == .below, origin.y < frame.minY {
                origin.y = above
            } else if position == .above, origin.y + size.height > frame.maxY {
                origin.y = below
            }
            origin.y = max(frame.minY, min(origin.y, frame.maxY - size.height))
        }

        panel.setFrameOrigin(origin)
        panel.orderFront(nil)
    }

    func hide() {
        panel?.orderOut(nil)
    }

    private func createPanel() {
        let panel = NSPanel(
            contentRect: NSRect(x: 0, y: 0, width: 240, height: 100),
            styleMask: [.borderless, .nonactivatingPanel],
            backing: .buffered,
            defer: true
        )
        panel.level = .popUpMenu
        panel.isFloatingPanel = true
        panel.hidesOnDeactivate = false
        panel.hasShadow = true
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]

        let view = CandidateView()
        view.onClick = { [weak self] index in self?.onSelect?(index) }
        panel.contentView = view
        self.panel = panel
        self.view = view
    }
}

// MARK: - CandidateView

private final class CandidateView: NSView {
    private var candidates: [String] = []
    private var auxiliary = ""
    private var selected = 0
    private var direction = PopupDirection.vertical
    /// First candidate shown when there are more than fit.
    private var offset = 0

    var onClick: ((Int) -> Void)?

    private let maxVisible = 9
    private let padding: CGFloat = 6
    private let auxHeight: CGFloat = 18
    private let numberWidth: CGFloat = 18
    private let font = NSFont.systemFont(ofSize: 16)
    private let numberFont = NSFont.monospacedDigitSystemFont(ofSize: 11, weight: .medium)
    private var rowHeight: CGFloat { ceil(font.pointSize * 1.6) }

    private var visible: Range<Int> { offset..<min(offset + maxVisible, candidates.count) }

    func update(candidates: [String], auxiliary: String, selected: Int, direction: PopupDirection) {
        self.candidates = candidates
        self.auxiliary = auxiliary
        self.direction = direction
        self.selected = candidates.isEmpty ? 0 : min(selected, candidates.count - 1)
        // Keep the selection on screen.
        if self.selected < offset { offset = self.selected }
        if self.selected >= offset + maxVisible { offset = self.selected - maxVisible + 1 }
        offset = max(0, min(offset, max(0, candidates.count - maxVisible)))
        needsDisplay = true
    }

    private func textWidth(_ text: String) -> CGFloat {
        ceil((text as NSString).size(withAttributes: [.font: font]).width)
    }

    /// Cell rects in view coordinates, one per visible candidate.
    private func cells() -> [(index: Int, rect: NSRect)] {
        let top = bounds.height - padding - auxHeight
        switch direction {
        case .vertical:
            return visible.enumerated().map { i, index in
                (index, NSRect(x: padding, y: top - CGFloat(i + 1) * rowHeight,
                               width: bounds.width - padding * 2, height: rowHeight))
            }
        case .horizontal:
            var x = padding
            return visible.map { index in
                let width = numberWidth + textWidth(candidates[index]) + 12
                defer { x += width + 2 }
                return (index, NSRect(x: x, y: top - rowHeight, width: width, height: rowHeight))
            }
        }
    }

    func idealSize() -> NSSize {
        let height = padding * 2 + auxHeight + (direction == .vertical ? CGFloat(visible.count) : 1) * rowHeight
        let auxWidth = ceil((auxiliary as NSString).size(withAttributes: [.font: NSFont.systemFont(ofSize: 11)]).width)
        let contentWidth: CGFloat
        switch direction {
        case .vertical:
            contentWidth = (visible.map { textWidth(candidates[$0]) }.max() ?? 0) + numberWidth + 32
        case .horizontal:
            contentWidth = visible.reduce(0) { $0 + numberWidth + textWidth(candidates[$1]) + 14 } + 12
        }
        return NSSize(width: max(160, contentWidth, auxWidth + 24) + padding * 2, height: height)
    }

    override func draw(_ dirtyRect: NSRect) {
        let background = NSBezierPath(roundedRect: bounds, xRadius: 8, yRadius: 8)
        NSColor.windowBackgroundColor.setFill()
        background.fill()
        NSColor.separatorColor.setStroke()
        background.lineWidth = 0.5
        background.stroke()

        // What was typed, above the list.
        (auxiliary as NSString).draw(
            in: NSRect(x: padding + 4, y: bounds.height - padding - auxHeight,
                       width: bounds.width - padding * 2 - 24, height: auxHeight),
            withAttributes: [.font: NSFont.systemFont(ofSize: 11), .foregroundColor: NSColor.secondaryLabelColor]
        )
        // More candidates than fit.
        if candidates.count > maxVisible {
            let more = offset > 0 && visible.upperBound < candidates.count ? "↕" : offset > 0 ? "↑" : "↓"
            (more as NSString).draw(
                at: NSPoint(x: bounds.width - padding - 14, y: bounds.height - padding - auxHeight + 2),
                withAttributes: [.font: NSFont.systemFont(ofSize: 11), .foregroundColor: NSColor.tertiaryLabelColor]
            )
        }

        for (index, rect) in cells() {
            let isSelected = index == selected
            if isSelected {
                NSColor.selectedContentBackgroundColor.setFill()
                NSBezierPath(roundedRect: rect, xRadius: 5, yRadius: 5).fill()
            }
            let textColor = isSelected ? NSColor.alternateSelectedControlTextColor : NSColor.labelColor
            let numberColor = isSelected ? NSColor.alternateSelectedControlTextColor : NSColor.tertiaryLabelColor

            let numberY = rect.minY + (rect.height - numberFont.pointSize) / 2 - 2
            ("\(index + 1)" as NSString).draw(
                at: NSPoint(x: rect.minX + 5, y: numberY),
                withAttributes: [.font: numberFont, .foregroundColor: numberColor]
            )
            let textY = rect.minY + (rect.height - font.pointSize) / 2 - 3
            (candidates[index] as NSString).draw(
                at: NSPoint(x: rect.minX + numberWidth + 4, y: textY),
                withAttributes: [.font: font, .foregroundColor: textColor]
            )
        }
    }

    // MARK: Mouse

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }

    override func mouseDown(with event: NSEvent) {
        guard let index = candidateIndex(at: convert(event.locationInWindow, from: nil)) else { return }
        selected = index
        needsDisplay = true
    }

    override func mouseUp(with event: NSEvent) {
        if let index = candidateIndex(at: convert(event.locationInWindow, from: nil)), index == selected {
            onClick?(index)
        }
    }

    private func candidateIndex(at point: NSPoint) -> Int? {
        cells().first { $0.rect.contains(point) }?.index
    }
}
