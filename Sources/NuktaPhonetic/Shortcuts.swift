import Foundation
import CRiti

/// The user's own shortcuts: a short roman code that types a longer text (`asw` → আসসালামু আলাইকুম).
///
/// The rules and the file live once, in riti-bridge (`riti-bridge/src/shortcuts.rs`), shared with
/// Windows and Linux; this is their C functions behind a Swift interface. The file is
/// `shortcuts.json` in the data directory, the same on every platform. Several processes can share
/// it: each change re-reads it first, and a lookup re-reads it when it has been written since.
public final class Shortcuts {
    /// Why a shortcut wasn't saved: the numbers riti-bridge's C function returns.
    public enum Failure: Int32, Error {
        case badCode = 1, emptyText, textTooLong, notSaved
    }

    public static let maxCode = 12
    public static let maxText = 300

    private let handle: OpaquePointer?

    public init(directory: URL) {
        handle = directory.path.withCString { nukta_shortcuts_new($0) }
    }

    deinit {
        nukta_shortcuts_free(handle)
    }

    /// The text for `typed` (roman, as typed) if it is a whole code; any case.
    public func text(for typed: String) -> String? {
        guard let handle else { return nil }
        return Self.take(typed.withCString { nukta_shortcuts_text(handle, $0) })
    }

    /// Every shortcut, ordered by code.
    public var all: [(code: String, text: String)] {
        guard let handle, let json = Self.take(nukta_shortcuts_json(handle)),
              let map = (try? JSONSerialization.jsonObject(with: Data(json.utf8))) as? [String: String]
        else { return [] }
        return map.sorted { $0.key < $1.key }.map { (code: $0.key, text: $0.value) }
    }

    /// Adds a shortcut, or changes an existing code's text.
    public func set(code: String, text: String) -> Result<Void, Failure> {
        guard let handle else { return .failure(.notSaved) }
        let result = code.withCString { code in text.withCString { nukta_shortcuts_set(handle, code, $0) } }
        if result == 0 { return .success(()) }
        return .failure(Failure(rawValue: result) ?? .notSaved)
    }

    /// True if the code was there.
    @discardableResult
    public func remove(code: String) -> Bool {
        guard let handle else { return false }
        return code.withCString { nukta_shortcuts_remove(handle, $0) }
    }

    /// A dictionary word the code also spells (`ami` → আমি): as a shortcut it would come first
    /// every time that word is typed, so the settings warn before saving it.
    public static func dictionaryWord(for code: String) -> String? {
        take(code.withCString { nukta_dictionary_word($0) })
    }

    /// Words typed in English letters turned into Bangla, as phonetic typing without a list would
    /// (`shuvo kamona` → শুভ কামনা); Bangla and everything else stay as they are. The settings use
    /// it for a shortcut's text typed with an English keyboard.
    public static func transliterated(_ text: String) -> String {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent("nukta-bangla-transliterate")
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let riti = RitiContext(directory: directory, suggestions: false, englishWord: false, autocorrect: false)
        var out = "", word = ""
        func finishWord() {
            out += word
            word = ""
            riti.finish()
        }
        for character in text {
            guard character.isASCII, !character.isWhitespace, let code = RitiContext.keycode(for: character) else {
                finishWord()
                out.append(character)
                continue
            }
            if case .lonely(let typed) = riti.key(code, shift: character.isUppercase) {
                word = typed
            }
            if !riti.isComposing {
                // riti ended the word itself (a punctuation mark).
                out += word
                word = ""
            }
        }
        finishWord()
        return out
    }

    /// A string one of the C functions returned, freed here.
    private static func take(_ pointer: UnsafeMutablePointer<CChar>?) -> String? {
        guard let pointer else { return nil }
        defer { nukta_string_free(pointer) }
        return String(cString: pointer)
    }
}
