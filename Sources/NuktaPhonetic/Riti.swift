// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of
// the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Adapted from Lekho (https://github.com/ARahim3/Lekho), Lekho/Sources/Engine.swift.

import CRiti
import Foundation

/// How phonetic typing behaves.
public enum TypingMode: String, CaseIterable {
    /// The literal transliteration of your spelling is listed first and committed by default;
    /// dictionary suggestions are one key away, and a word you pick over it is remembered.
    case phoneticFirst
    /// Dictionary, autocorrect and remembered picks choose the committed word.
    case smart
    /// Transliteration only: no suggestions, no popup.
    case phoneticOnly
}

public struct PhoneticOptions: Equatable {
    public var mode: TypingMode = .phoneticFirst
    public var autocorrect = true
    public var emoji = true
    /// List the typed roman word itself as a candidate.
    public var englishWord = true
    /// `:` types ঃ as in Avro. Off: it types a colon, and words with ঃ come from the list
    /// (`dukho` → দুঃখ).
    public var colonIsBisarga = false

    public init() {}
}

/// One riti suggestion, copied out of riti's memory.
struct RitiSuggestion {
    /// riti's Single variant: one transliteration and no list (phonetic-only, punctuation…).
    var lonely: String?
    var candidates: [String] = []
    /// What was typed (shown above the list).
    var auxiliary = ""
    /// riti's remembered pick for this word, or the selection it preserved after punctuation.
    var previouslySelected = 0

    var isEmpty: Bool { lonely == nil && candidates.isEmpty }
}

/// The riti contexts. One per process: riti contexts are large, and each rewrites riti's
/// learned-selections file from its own copy, so several would overwrite each other.
final class Riti {
    private(set) var options: PhoneticOptions
    let userDirectory: URL

    private var main: OpaquePointer?
    private var mainConfig: OpaquePointer?
    /// Phonetic-first only: a suggestion-free context fed the same keys, whose output is the
    /// literal transliteration of the word.
    private var shadow: OpaquePointer?
    private var shadowConfig: OpaquePointer?

    /// The literal transliteration of the word in progress (phonetic-first only).
    private(set) var literal: String?

    init(options: PhoneticOptions, userDirectory: URL) {
        self.options = options
        self.userDirectory = userDirectory
        build()
    }

    deinit { teardown() }

    func rebuild(options: PhoneticOptions) {
        teardown()
        self.options = options
        build()
    }

    var hasSession: Bool {
        main.map { riti_context_ongoing_input_session($0) } ?? false
    }

    static func keycode(for character: Character) -> UInt16? {
        guard let scalar = character.unicodeScalars.first, character.unicodeScalars.count == 1 else { return nil }
        let code = nukta_riti_keycode(scalar.value)
        return code == 0 ? nil : code
    }

    /// `selection`: the riti index currently selected; riti keeps it if the key is punctuation.
    func feed(_ key: UInt16, shift: Bool, selection: Int) -> RitiSuggestion {
        let modifier = shift ? UInt8(MODIFIER_SHIFT) : 0
        let result = Self.take(riti_get_suggestion_for_key(main, key, modifier, UInt8(clamping: selection)))
        if let shadow {
            literal = Self.take(riti_get_suggestion_for_key(shadow, key, modifier, 0)).lonely
        }
        syncShadow()
        return result
    }

    func backspace(wholeWord: Bool) -> RitiSuggestion {
        let result = Self.take(riti_context_backspace_event(main, wholeWord))
        if let shadow {
            literal = Self.take(riti_context_backspace_event(shadow, wholeWord)).lonely
        }
        syncShadow()
        return result
    }

    /// Ends the session after the candidate at riti index `index` was committed. Only smart mode
    /// lets riti learn the pick; phonetic-first keeps its own memory (`PhoneticFirstPicks`),
    /// because riti can't learn a pick of its index 0.
    func committed(index: Int) {
        if options.mode == .smart, let main {
            riti_context_candidate_committed(main, UInt(index))
        }
        finish()
    }

    /// Ends any session in both contexts.
    func finish() {
        for ctx in [main, shadow].compactMap({ $0 }) where riti_context_ongoing_input_session(ctx) {
            riti_context_finish_input_session(ctx)
        }
        literal = nil
    }

    /// The shadow must never outlive the main session.
    private func syncShadow() {
        guard !hasSession else { return }
        if let shadow, riti_context_ongoing_input_session(shadow) {
            riti_context_finish_input_session(shadow)
        }
        literal = nil
    }

    /// Copies a suggestion into Swift and frees it.
    private static func take(_ pointer: OpaquePointer?) -> RitiSuggestion {
        guard let pointer else { return RitiSuggestion() }
        defer { riti_suggestion_free(pointer) }
        guard !riti_suggestion_is_empty(pointer) else { return RitiSuggestion() }

        // riti's length, auxiliary text and previous-selection calls panic on a lonely
        // suggestion: never make them on one.
        if riti_suggestion_is_lonely(pointer) {
            return RitiSuggestion(lonely: string(riti_suggestion_get_lonely_suggestion(pointer)))
        }
        var suggestion = RitiSuggestion()
        for i in 0..<riti_suggestion_get_length(pointer) {
            suggestion.candidates.append(string(riti_suggestion_get_suggestion(pointer, i)))
        }
        suggestion.auxiliary = string(riti_suggestion_get_auxiliary_text(pointer))
        suggestion.previouslySelected = Int(riti_suggestion_previously_selected_index(pointer))
        return suggestion
    }

    private static func string(_ pointer: UnsafeMutablePointer<CChar>?) -> String {
        guard let pointer else { return "" }
        defer { riti_string_free(pointer) }
        return String(cString: pointer)
    }

    // MARK: Build / teardown

    private func build() {
        // riti aborts on a JSON file it can't parse; move bad ones aside first.
        Self.quarantineIfCorrupt(userDirectory.appendingPathComponent("phonetic-candidate-selection.json"))
        Self.quarantineIfCorrupt(userDirectory.appendingPathComponent("autocorrect.json"))

        mainConfig = makeConfig(suggestions: options.mode != .phoneticOnly)
        main = riti_context_new_with_config(mainConfig)
        if options.mode == .phoneticFirst {
            shadowConfig = makeConfig(suggestions: false)
            shadow = riti_context_new_with_config(shadowConfig)
        }
    }

    private func teardown() {
        finish()
        if let main { riti_context_free(main) }
        if let mainConfig { riti_config_free(mainConfig) }
        if let shadow { riti_context_free(shadow) }
        if let shadowConfig { riti_config_free(shadowConfig) }
        main = nil; mainConfig = nil; shadow = nil; shadowConfig = nil
    }

    /// riti carries its dictionary, autocorrect, suffix and emoji data inside the library, so it
    /// only needs a writable user directory.
    private func makeConfig(suggestions: Bool) -> OpaquePointer? {
        let config = riti_config_new()
        _ = riti_config_set_layout_file(config, "avro_phonetic")
        _ = riti_config_set_user_dir(config, userDirectory.path)
        riti_config_set_phonetic_suggestion(config, suggestions)
        riti_config_set_suggestion_include_english(config, options.englishWord)
        riti_config_set_autocorrect(config, options.autocorrect)
        return config
    }

    /// riti expects a flat `{string: string}` object; anything else is renamed, not deleted.
    private static func quarantineIfCorrupt(_ url: URL) {
        guard let data = FileManager.default.contents(atPath: url.path) else { return }
        if (try? JSONSerialization.jsonObject(with: data)) is [String: String] { return }
        let aside = url.path + ".corrupt-\(Int(Date().timeIntervalSince1970))"
        try? FileManager.default.moveItem(atPath: url.path, toPath: aside)
    }
}
