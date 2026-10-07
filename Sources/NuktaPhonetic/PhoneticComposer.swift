import Foundation

/// Turns keys into Bangla with riti (Avro Phonetic), and keeps the suggestion list for the word in
/// progress. Pure logic, no AppKit: the input controller shows `Output` and `candidates`.
///
/// riti contexts are expensive, and each one rewrites riti's file of learned picks from its own
/// copy, so the app keeps one composer per process.
public final class PhoneticComposer {
    public enum Key: Equatable {
        /// A printable key, as on a US layout.
        case character(Character, shift: Bool)
        /// ⌫, or ⌥⌫ for the whole word.
        case backspace(wholeWord: Bool)
        case enter, escape, space
        case tab(backward: Bool)
        case up, down, left, right
    }

    /// What one key does to the app's text.
    public struct Output: Equatable {
        /// Final text to insert, replacing the marked text.
        public var insert = ""
        /// Marked (underlined) text to show afterwards; "" = none.
        public var marked = ""
        /// False: the app must also handle the key (space, arrows…).
        public var handled = true

        public init(insert: String = "", marked: String = "", handled: Bool = true) {
            self.insert = insert
            self.marked = marked
            self.handled = handled
        }
    }

    public private(set) var options: PhoneticOptions
    /// ← → also move the selection (the list is a row).
    public var horizontalNavigation = false
    /// The list to show; empty = no list.
    public private(set) var candidates: [String] = []
    public private(set) var selectedIndex = 0
    /// What was typed (roman), shown above the list.
    public private(set) var auxiliary = ""

    /// A word is in progress.
    public var isComposing: Bool {
        !candidates.isEmpty || !lonely.isEmpty
    }

    private let directory: URL
    private var riti: RitiContext
    /// Phonetic-first only: a context with suggestions off, fed the same keys, whose (lonely) answer
    /// is the literal transliteration. riti's list contains it but doesn't say which entry it is.
    private var literalRiti: RitiContext?
    private let picks: PickMemory

    /// The word when riti gives no list (phonetic-only mode, a lone punctuation mark).
    private var lonely = ""
    /// For each candidate, its index in riti's own list (the order differs after filtering and
    /// moving the literal up).
    private var ritiIndices: [Int] = []
    /// Index in `candidates` of the literal transliteration, if it is there.
    private var literalIndex: Int?
    /// The literal transliteration of what was typed, from `literalRiti`.
    private var literal = ""
    /// The user moved the selection since the last key that went to riti.
    private var selectionMoved = false

    /// Keys riti handles as punctuation: typed after the user moved the selection, riti keeps the
    /// selection instead of picking its own default (riti-bridge/riti/src/phonetic/method.rs).
    private static let selectionKeepers: Set<Character> = [".", "?", "!", ",", ":", ";", "-", "_", ")", "}", "]", "'", "\""]

    /// `directory`: where riti and the pick memory keep what they learn; created if missing.
    public init(options: PhoneticOptions, directory: URL) {
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        self.options = options
        self.directory = directory
        picks = PickMemory(directory: directory)
        (riti, literalRiti) = Self.makeContexts(options: options, directory: directory)
    }

    /// A change to the mode, autocorrect or the English word rebuilds riti and drops the word in
    /// progress. Emoji and the colon apply from the next key.
    public func update(options newOptions: PhoneticOptions) {
        let rebuild = newOptions.mode != options.mode || newOptions.autocorrect != options.autocorrect
            || newOptions.englishWord != options.englishWord
        if rebuild {
            discard()
            literalRiti = nil
            (riti, literalRiti) = Self.makeContexts(options: newOptions, directory: directory)
        }
        options = newOptions
    }

    public func handle(_ key: Key) -> Output {
        switch key {
        case .character(let character, let shift):
            return type(character, shift: shift)
        case .backspace(let wholeWord):
            return backspace(wholeWord: wholeWord)
        case .enter:
            guard isComposing else { return Output(handled: false) }
            return commit()
        case .escape:
            guard isComposing else { return Output(handled: false) }
            discard()
            return Output()
        case .space:
            guard isComposing else { return Output(handled: false) }
            var out = commit()
            out.handled = false
            return out
        case .tab(let backward):
            return moveSelection(by: backward ? -1 : 1)
        case .down:
            return moveSelection(by: 1)
        case .up:
            return moveSelection(by: -1)
        case .left, .right:
            if horizontalNavigation { return moveSelection(by: key == .left ? -1 : 1) }
            return commitAndPass()
        }
    }

    /// Commits the selected word (a click elsewhere, a switch to another app).
    public func commit() -> Output {
        guard isComposing else { return Output() }
        if candidates.isEmpty {
            let text = lonely
            endWord(learning: nil)
            return Output(insert: text)
        }
        return commit(at: selectedIndex)
    }

    /// Commits `candidates[index]` (a click in the list).
    public func commit(at index: Int) -> Output {
        guard candidates.indices.contains(index) else {
            return isComposing ? Output(marked: marked) : Output()
        }
        let text = candidates[index]
        if options.mode == .phoneticFirst, let literalIndex {
            if index == literalIndex || PickMemory.trim(text) == PickMemory.trim(literal) {
                picks.forget(auxiliary)
            } else if !Self.containsEmoji(text) {
                picks.remember(text, for: auxiliary)
            }
        }
        endWord(learning: options.mode == .smart ? ritiIndices[index] : nil)
        return Output(insert: text)
    }

    /// Drops the word in progress, with no output.
    public func discard() {
        endWord(learning: nil)
    }

    /// Any emoji, including a text symbol made to show as one (© + U+FE0F = ©️). Plain ©, digits,
    /// `#` and `*` (emoji only with U+FE0F or a keycap after them) are not.
    static func containsEmoji(_ text: String) -> Bool {
        text.unicodeScalars.contains { scalar in
            scalar.properties.isEmojiPresentation || scalar.value == 0xFE0F
                || (scalar.properties.isEmoji && scalar.value >= 0x2000)
        }
    }

    // MARK: Keys

    private func type(_ character: Character, shift: Bool) -> Output {
        if character == ":" && !options.colonIsBisarga {
            return Output(insert: commit().insert + ":")
        }

        if let digit = Self.asciiDigit(character) {
            if !isComposing {
                return Output(insert: Self.banglaDigit(digit))
            }
            if (1...9).contains(digit) {
                if !candidates.isEmpty, digit <= candidates.count {
                    return commit(at: digit - 1)
                }
                if candidates.isEmpty {
                    return Output(insert: commit().insert + Self.banglaDigit(digit))
                }
            }
        }

        guard let code = RitiContext.keycode(for: character) else {
            return commitAndPass()
        }

        // After the user moved the selection, punctuation keeps it: riti is told which entry it was.
        let keepSelection = selectionMoved && !candidates.isEmpty && Self.selectionKeepers.contains(character)
        if !keepSelection { selectionMoved = false }
        let selection = keepSelection ? ritiIndices[selectedIndex] : 0

        // riti only reports what was typed with a list; this keeps it for a lonely answer too.
        auxiliary.append(character)
        let suggestion = riti.key(code, shift: shift, selection: selection)
        if let literalRiti, case .lonely(let text) = literalRiti.key(code, shift: shift) {
            literal = text
        }
        show(suggestion, keptSelection: keepSelection)

        if !riti.isComposing {
            // riti ended the word itself: what it produced is final.
            let text = marked
            endWord(learning: nil)
            return Output(insert: text)
        }
        return Output(marked: marked)
    }

    private func backspace(wholeWord: Bool) -> Output {
        guard isComposing else { return Output(handled: false) }
        selectionMoved = false
        if wholeWord { auxiliary = "" } else if !auxiliary.isEmpty { auxiliary.removeLast() }
        let suggestion = riti.backspace(wholeWord: wholeWord)
        if let literalRiti {
            if case .lonely(let text) = literalRiti.backspace(wholeWord: wholeWord) {
                literal = text
            } else {
                literal = ""
            }
        }
        if suggestion == .empty || !riti.isComposing {
            endWord(learning: nil)
            return Output()
        }
        show(suggestion, keptSelection: false)
        return Output(marked: marked)
    }

    /// Up/down, Tab: move the selection, wrapping. With no list, the word is committed and the key
    /// goes to the app.
    private func moveSelection(by step: Int) -> Output {
        guard isComposing else { return Output(handled: false) }
        guard !candidates.isEmpty else { return commitAndPass() }
        selectedIndex = (selectedIndex + step + candidates.count) % candidates.count
        selectionMoved = true
        return Output(marked: marked)
    }

    private func commitAndPass() -> Output {
        var out = commit()
        out.handled = false
        return out
    }

    // MARK: State

    /// The marked text: the selected candidate, or the lonely text when there is no list.
    private var marked: String {
        candidates.indices.contains(selectedIndex) ? candidates[selectedIndex] : lonely
    }

    /// Builds the list from riti's answer.
    private func show(_ suggestion: RitiSuggestion, keptSelection: Bool) {
        switch suggestion {
        case .empty:
            clearList()
        case .lonely(let text):
            clearList()
            lonely = text
        case .list(let entries, let typed, let ritiSelection):
            lonely = ""
            auxiliary = typed
            buildList(entries, ritiSelection: ritiSelection, keptSelection: keptSelection)
        }
    }

    private func buildList(_ entries: [String], ritiSelection: Int, keptSelection: Bool) {
        var items = entries.enumerated().map { (text: $0.element, riti: $0.offset) }
        if !options.emoji {
            let plain = items.filter { !Self.containsEmoji($0.text) }
            if !plain.isEmpty { items = plain }
        }

        literalIndex = nil
        if options.mode == .phoneticFirst, !literal.isEmpty {
            let target = Self.straightened(literal)
            if let found = items.firstIndex(where: { Self.straightened($0.text) == target }) {
                items.insert(items.remove(at: found), at: 0)
                literalIndex = 0
            }
        }

        candidates = items.map(\.text)
        ritiIndices = items.map(\.riti)
        guard !candidates.isEmpty else {
            selectedIndex = 0
            return
        }
        let ritiDefault = ritiIndices.firstIndex(of: ritiSelection) ?? 0

        var selection: Int
        if keptSelection || options.mode != .phoneticFirst {
            selection = ritiDefault
        } else if let literalIndex {
            selection = literalIndex
            if let pick = picks.pick(for: auxiliary),
               let picked = candidates.firstIndex(where: { PickMemory.trim($0) == pick }) {
                selection = picked
            }
        } else {
            selection = ritiDefault
        }

        // Space must never commit an emoji nobody chose. A typed emoticon (`:)`) is exempt.
        if !keptSelection, auxiliary.first?.isLetter == true, Self.containsEmoji(candidates[selection]),
           let plain = candidates.firstIndex(where: { !Self.containsEmoji($0) }) {
            selection = plain
        }
        selectedIndex = selection
    }

    private func clearList() {
        candidates = []
        ritiIndices = []
        literalIndex = nil
        selectedIndex = 0
    }

    /// Ends the word in both contexts and resets. `learning`: riti index of the pick for riti to
    /// learn (smart mode).
    private func endWord(learning ritiIndex: Int?) {
        if let ritiIndex {
            riti.committed(index: ritiIndex)
        } else {
            riti.finish()
        }
        literalRiti?.finish()
        clearList()
        lonely = ""
        auxiliary = ""
        literal = ""
        selectionMoved = false
    }

    // MARK: Helpers

    private static func makeContexts(options: PhoneticOptions, directory: URL) -> (RitiContext, RitiContext?) {
        let main = RitiContext(directory: directory, suggestions: options.mode != .phoneticOnly,
                               englishWord: options.englishWord, autocorrect: options.autocorrect)
        let literal = options.mode == .phoneticFirst
            ? RitiContext(directory: directory, suggestions: false, englishWord: false, autocorrect: false)
            : nil
        return (main, literal)
    }

    /// riti's list has curly quotes where the literal transliteration has straight ones.
    private static func straightened(_ text: String) -> String {
        var result = ""
        for character in text {
            switch character {
            case "“", "”": result.append("\"")
            case "‘", "’": result.append("'")
            default: result.append(character)
            }
        }
        return result
    }

    private static func asciiDigit(_ character: Character) -> Int? {
        guard character.isASCII else { return nil }
        return character.wholeNumberValue
    }

    private static func banglaDigit(_ digit: Int) -> String {
        String(UnicodeScalar(0x09E6 + UInt32(digit))!)
    }
}
