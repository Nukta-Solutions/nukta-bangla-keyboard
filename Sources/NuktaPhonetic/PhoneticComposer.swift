// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of
// the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Adapted from Lekho (https://github.com/ARahim3/Lekho), Lekho/Sources/InputController.swift: the
// key handling and candidate ordering, without AppKit so it can be tested.

import Foundation

/// Phonetic typing with a candidate list. Keys go in; what to commit, what to show as marked
/// text, and the list to show come out. The input controller applies them to the app.
public final class PhoneticComposer {
    public enum Key: Equatable {
        /// A printable key, as on a US layout.
        case character(Character, shift: Bool)
        /// ⌫; with ⌥ it deletes the whole word.
        case backspace(wholeWord: Bool)
        case enter, escape, space
        case tab(backward: Bool)
        case up, down, left, right
    }

    public struct Output: Equatable {
        /// Final text to insert in place of the marked text.
        public var insert = ""
        /// Marked text to show after `insert`; empty when nothing is being composed.
        public var marked = ""
        /// False: the app should handle the key as well (Space, arrows…).
        public var handled = true
    }

    public private(set) var options: PhoneticOptions
    /// ←/→ move the selection (the list is drawn as a row).
    public var horizontalNavigation = false

    /// The list to show; empty when there's nothing to pick from.
    public private(set) var candidates: [String] = []
    public private(set) var selectedIndex = 0
    /// What was typed, shown above the list.
    public private(set) var auxiliary = ""

    public var isComposing: Bool { riti.hasSession }

    private let riti: Riti
    private let picks: PhoneticFirstPicks
    private var current: RitiSuggestion?
    /// `order[i]` is riti's index for `candidates[i]`: emoji may be filtered out and phonetic-first
    /// moves the transliteration to the top.
    private var order: [Int] = []
    /// Phonetic-first: the literal transliteration, when it is in the list (always at the top).
    private var literalCandidate: String?
    /// The selection was moved with arrows/Tab since the last letter.
    private var userNavigated = false

    /// Keys after which riti keeps the selection we pass in instead of resetting it.
    private static let selectionPreservingKeys: Set<Character> = [
        ".", "?", "!", ",", ":", ";", "-", "_", ")", "}", "]", "'", "\"",
    ]
    private static let banglaDigits = Array("০১২৩৪৫৬৭৮৯")

    /// `directory`: where riti and the pick memory keep what they learn.
    public init(options: PhoneticOptions, directory: URL) {
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        self.options = options
        riti = Riti(options: options, userDirectory: directory)
        picks = PhoneticFirstPicks(directory: directory)
    }

    /// Settings changed: the word in progress is dropped (clear the marked text).
    public func update(options: PhoneticOptions) {
        guard options != self.options else { return }
        let old = self.options
        self.options = options
        // Only these change riti itself; the rest apply from the next key.
        guard options.mode != old.mode || options.autocorrect != old.autocorrect
                || options.englishWord != old.englishWord else { return }
        riti.finish()
        reset()
        riti.rebuild(options: options)
    }

    // MARK: Keys

    public func handle(_ key: Key) -> Output {
        let composing = riti.hasSession
        let lonely = current?.lonely != nil

        switch key {
        case .enter:
            // Commits without a newline.
            return composing ? commit() : pass()

        case .escape:
            guard composing else { return pass() }
            riti.finish()
            reset()
            return Output()

        case .backspace(let wholeWord):
            guard composing else { return pass() }
            current = riti.backspace(wholeWord: wholeWord)
            guard riti.hasSession else {
                reset()
                return Output()
            }
            userNavigated = false
            refreshCandidates(preserveSelection: false)
            return Output(marked: markedText)

        case .space:
            return composing ? commit(handled: false) : pass()

        case .tab(let backward):
            return navigate(forward: !backward, composing: composing, lonely: lonely)
        case .up:
            return navigate(forward: false, composing: composing, lonely: lonely)
        case .down:
            return navigate(forward: true, composing: composing, lonely: lonely)
        case .left, .right:
            guard horizontalNavigation else { return composing ? commit(handled: false) : pass() }
            return navigate(forward: key == .right, composing: composing, lonely: lonely)

        case .character(let character, let shift):
            if character == ":", !options.colonIsBisarga {
                // A plain colon: finish the word, then type it.
                var out = composing ? commit() : Output()
                out.insert += ":"
                return out
            }
            if let digit = character.wholeNumberValue, character.isASCII {
                if !composing {
                    return Output(insert: String(Self.banglaDigits[digit]))
                }
                if digit >= 1 {
                    if lonely {
                        // No numbered list: finish the word, then type the Bangla digit.
                        var out = commit()
                        out.insert += String(Self.banglaDigits[digit])
                        return out
                    }
                    if digit - 1 < candidates.count {
                        return commit(at: digit - 1)
                    }
                }
            }
            guard let code = Riti.keycode(for: character) else {
                return composing ? commit(handled: false) : pass()
            }
            return feed(code, character: character, shift: shift)
        }
    }

    /// Commits the selected candidate (Enter, click elsewhere, another app taking over…).
    public func commit() -> Output {
        commit(handled: true)
    }

    /// Commits the candidate at `index` of `candidates` (a click in the list).
    public func commit(at index: Int) -> Output {
        guard let current, !current.isEmpty else {
            riti.finish()
            reset()
            return Output()
        }

        let text: String
        if let lonely = current.lonely {
            text = lonely
            riti.finish()
        } else if candidates.isEmpty {
            text = ""
            riti.finish()
        } else {
            let safe = min(max(index, 0), candidates.count - 1)
            text = candidates[safe]
            if options.mode == .phoneticFirst, let literal = literalCandidate {
                picks.record(typed: auxiliary, chosen: text, literal: literal)
            }
            riti.committed(index: order[safe])
        }
        reset()
        return Output(insert: text)
    }

    /// Drops the word in progress without committing it.
    public func discard() {
        riti.finish()
        reset()
    }

    // MARK: Internals

    private func pass() -> Output {
        Output(handled: false)
    }

    private func commit(handled: Bool) -> Output {
        guard riti.hasSession else { return Output(handled: handled) }
        var out = commit(at: selectedIndex)
        out.handled = handled
        return out
    }

    private func navigate(forward: Bool, composing: Bool, lonely: Bool) -> Output {
        guard composing else { return pass() }
        // Phonetic-only has no list: finish the word and let the key do its usual job.
        guard !lonely, !candidates.isEmpty else { return commit(handled: false) }
        let count = candidates.count
        selectedIndex = forward ? (selectedIndex + 1) % count : (selectedIndex + count - 1) % count
        userNavigated = true
        return Output(marked: markedText)
    }

    private func feed(_ code: UInt16, character: Character, shift: Bool) -> Output {
        // A selection the user moved to survives a punctuation key (riti keeps the index we pass);
        // any other key resets it.
        let preserve = riti.hasSession && userNavigated && Self.selectionPreservingKeys.contains(character)
        let selection = order.indices.contains(selectedIndex) ? order[selectedIndex] : 0

        current = riti.feed(code, shift: shift, selection: selection)

        if riti.hasSession {
            if !preserve { userNavigated = false }
            refreshCandidates(preserveSelection: preserve)
            return Output(marked: markedText)
        }

        // riti finished the word on its own (a lone punctuation mark…).
        var text = ""
        if let suggestion = current, !suggestion.isEmpty {
            if let lonely = suggestion.lonely {
                text = lonely
            } else {
                refreshCandidates(preserveSelection: false)
                text = candidates.indices.contains(selectedIndex) ? candidates[selectedIndex] : ""
            }
        }
        riti.finish()
        reset()
        return Output(insert: text)
    }

    private var markedText: String {
        if let lonely = current?.lonely { return lonely }
        return candidates.indices.contains(selectedIndex) ? candidates[selectedIndex] : ""
    }

    private func reset() {
        current = nil
        candidates = []
        order = []
        literalCandidate = nil
        auxiliary = ""
        selectedIndex = 0
        userNavigated = false
    }

    /// Rebuilds the list from `current` and picks the default selection. `preserveSelection`:
    /// punctuation typed after the user navigated, so riti's preserved index wins.
    private func refreshCandidates(preserveSelection: Bool) {
        candidates = []
        order = []
        literalCandidate = nil
        auxiliary = ""
        selectedIndex = 0

        guard let suggestion = current, suggestion.lonely == nil, !suggestion.candidates.isEmpty else { return }

        var items = suggestion.candidates.enumerated().map { (index: $0.offset, text: $0.element) }
        if !options.emoji {
            let withoutEmoji = items.filter { !Self.containsEmoji($0.text) }
            if !withoutEmoji.isEmpty { items = withoutEmoji }
        }

        // Phonetic-first: the literal transliteration goes on top.
        if options.mode == .phoneticFirst, let literal = riti.literal {
            let wanted = Self.straightenQuotes(literal)
            if let position = items.firstIndex(where: { Self.straightenQuotes($0.text) == wanted }) {
                let item = items.remove(at: position)
                items.insert(item, at: 0)
                literalCandidate = item.text
            }
        }

        candidates = items.map(\.text)
        order = items.map(\.index)
        auxiliary = suggestion.auxiliary

        // riti's remembered pick, or the index it preserved after punctuation (0 if none).
        let ritiSelection = order.firstIndex(of: suggestion.previouslySelected) ?? 0
        if options.mode == .phoneticFirst {
            if preserveSelection {
                selectedIndex = ritiSelection
            } else if let pick = picks.pick(forTyped: auxiliary),
                      let index = candidates.firstIndex(where: { PhoneticFirstPicks.core($0) == pick }) {
                selectedIndex = index
            }
        } else {
            selectedIndex = ritiSelection
        }

        // riti can rank an emoji matched by name above dictionary words (`boish` → 🗺️), which would
        // make Space commit an emoji. Only an explicit pick commits one, unless the word is an
        // emoticon like `:)`, where the emoji is the point.
        let typedWord = PhoneticFirstPicks.core(auxiliary)
        if !preserveSelection,
           typedWord.first?.isLetter == true,
           candidates.indices.contains(selectedIndex),
           Self.containsEmoji(candidates[selectedIndex]),
           let firstWord = candidates.firstIndex(where: { !Self.containsEmoji($0) }) {
            selectedIndex = firstWord
        }
    }

    static func containsEmoji(_ text: String) -> Bool {
        text.unicodeScalars.contains {
            // isEmoji alone is also true for ASCII digits, '#' and '*'. U+FE0F asks for emoji
            // presentation of a text symbol (©️, ❤️).
            $0.properties.isEmojiPresentation || ($0.properties.isEmoji && $0.value >= 0x2000) || $0.value == 0xFE0F
        }
    }

    /// riti's list uses smart quotes (“ ” ‘ ’) but its transliteration doesn't.
    private static func straightenQuotes(_ text: String) -> String {
        guard text.contains(where: { "“”‘’".contains($0) }) else { return text }
        return String(text.map { ch -> Character in
            switch ch {
            case "“", "”": return "\""
            case "‘", "’": return "'"
            default: return ch
            }
        })
    }
}
