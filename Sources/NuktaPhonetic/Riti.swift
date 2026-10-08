import Foundation
import CRiti

/// How phonetic typing picks the word that Space commits.
public enum TypingMode: String, CaseIterable {
    /// What was typed, transliterated; a word picked from the list instead is preferred next time.
    case phoneticFirst
    /// riti's own choice: dictionary, autocorrect and past picks.
    case smart
    /// Transliteration only, no list.
    case phoneticOnly
}

/// The settings phonetic typing depends on.
public struct PhoneticOptions: Equatable {
    public var mode: TypingMode = .phoneticFirst
    public var autocorrect = true
    public var emoji = true
    /// List the typed roman word itself.
    public var englishWord = true
    /// False: ":" types a colon. True: it types ঃ, as in Avro.
    public var colonIsBisarga = false

    public init() {}
}

/// One riti answer, copied out of riti so nothing points into its memory afterwards.
enum RitiSuggestion: Equatable {
    case empty
    /// A single string and no list: phonetic-only mode, or a lone punctuation mark.
    case lonely(String)
    /// `selection`: riti's default pick (a past choice, or the selection passed in for punctuation).
    case list([String], auxiliary: String, selection: Int)
}

/// A riti context for Avro Phonetic, behind a Swift interface. Internal to this module.
///
/// riti panics (and so aborts the whole input method) on misuse, so every C call is made the way
/// riti expects: lonely suggestions are never asked for list details, and everything riti hands
/// out is freed here.
final class RitiContext {
    /// riti's learned picks and the user's autocorrect entries, inside the user directory.
    static let userFiles = ["phonetic-candidate-selection.json", "autocorrect.json"]

    private let context: OpaquePointer

    /// `directory` must exist; riti reads and writes its user files there.
    /// `suggestions` false gives lonely transliterations only.
    init(directory: URL, suggestions: Bool, englishWord: Bool, autocorrect: Bool) {
        Self.setAsideBadFiles(in: directory)
        let config = riti_config_new()!
        defer { riti_config_free(config) }
        _ = "avro_phonetic".withCString { riti_config_set_layout_file(config, $0) }
        _ = directory.path.withCString { riti_config_set_user_dir(config, $0) }
        riti_config_set_phonetic_suggestion(config, suggestions)
        riti_config_set_suggestion_include_english(config, englishWord)
        riti_config_set_autocorrect(config, autocorrect)
        // The context keeps its own copy of the config.
        context = riti_context_new_with_config(config)
    }

    deinit {
        riti_context_free(context)
    }

    /// Feeds one key. `selection` is the riti index of the entry the user has selected; riti keeps
    /// it as the default when the key is a punctuation mark.
    func key(_ code: UInt16, shift: Bool, selection: Int = 0) -> RitiSuggestion {
        let modifier = shift ? UInt8(MODIFIER_SHIFT) : 0
        let index = UInt8(clamping: max(selection, 0))
        return Self.take(riti_get_suggestion_for_key(context, code, modifier, index))
    }

    /// Deletes the last letter, or the whole word. An empty result means the word is gone.
    func backspace(wholeWord: Bool) -> RitiSuggestion {
        Self.take(riti_context_backspace_event(context, wholeWord))
    }

    var isComposing: Bool {
        riti_context_ongoing_input_session(context)
    }

    /// Ends the word, letting riti learn that `index` was picked.
    func committed(index: Int) {
        riti_context_candidate_committed(context, UInt(max(index, 0)))
    }

    /// Ends the word without telling riti what was picked.
    func finish() {
        riti_context_finish_input_session(context)
    }

    /// riti's keycode for a character on a US layout, or nil if riti has no key for it.
    static func keycode(for character: Character) -> UInt16? {
        guard character.unicodeScalars.count == 1, let scalar = character.unicodeScalars.first else { return nil }
        let code = nukta_riti_keycode(scalar.value)
        return code == 0 ? nil : code
    }

    // MARK: Helpers

    /// Copies a suggestion into Swift values and frees it.
    private static func take(_ pointer: OpaquePointer?) -> RitiSuggestion {
        guard let pointer else { return .empty }
        defer { riti_suggestion_free(pointer) }
        if riti_suggestion_is_empty(pointer) { return .empty }
        if riti_suggestion_is_lonely(pointer) {
            return .lonely(string(riti_suggestion_get_lonely_suggestion(pointer)))
        }
        let count = riti_suggestion_get_length(pointer)
        let entries = (0..<count).map { string(riti_suggestion_get_suggestion(pointer, $0)) }
        let auxiliary = string(riti_suggestion_get_auxiliary_text(pointer))
        let selection = Int(riti_suggestion_previously_selected_index(pointer))
        return .list(entries, auxiliary: auxiliary, selection: selection)
    }

    /// Copies a string riti returned and frees it.
    private static func string(_ pointer: UnsafeMutablePointer<CChar>?) -> String {
        guard let pointer else { return "" }
        defer { riti_string_free(pointer) }
        return String(cString: pointer)
    }

    /// riti aborts the process if a user file isn't a flat JSON object of strings. A file like that
    /// (half-written, edited by hand) is renamed, not deleted, so nothing the user made is lost.
    private static func setAsideBadFiles(in directory: URL) {
        let fileManager = FileManager.default
        for name in userFiles {
            let file = directory.appendingPathComponent(name)
            guard fileManager.fileExists(atPath: file.path), !isFlatStringObject(file) else { continue }
            let stamp = Int(Date().timeIntervalSince1970)
            let aside = directory.appendingPathComponent("\(name).bad-\(stamp)")
            try? fileManager.moveItem(at: file, to: aside)
        }
    }

    private static func isFlatStringObject(_ file: URL) -> Bool {
        guard let data = try? Data(contentsOf: file),
              // riti's JSON reader rejects a byte order mark.
              !data.starts(with: [0xEF, 0xBB, 0xBF]),
              let object = try? JSONSerialization.jsonObject(with: data) else { return false }
        return object is [String: String]
    }
}
