import Foundation

/// Phonetic-first's memory: for a typed word, the list entry the user picked instead of the plain
/// transliteration, so it is selected next time.
///
/// Kept apart from riti's own learning (smart mode), which works on riti's ranking rather than on
/// the transliteration. Stored as a JSON object, typed word → picked word, in
/// `phonetic-first-picks.json`; installed copies already have that file, so its name and format stay.
final class PickMemory {
    static let fileName = "phonetic-first-picks.json"

    /// What riti puts around a word (`"sonar,"`, `(ami)`…), so `sonar` and `sonar,` share a pick:
    /// riti's `META` (riti-bridge/riti/src/utility.rs), plus the quotes its smart quoting writes.
    private static let punctuation = CharacterSet(charactersIn: "-]~!@#%&*()_=+[{}'\";<>/?|.,।“”‘’")

    private let file: URL
    private var picks: [String: String]
    /// Writes run in order, off the typing path.
    private let queue = DispatchQueue(label: "com.nuktasolutions.NuktaBangla.picks", qos: .utility)

    /// A missing or unreadable file is an empty memory.
    init(directory: URL) {
        file = directory.appendingPathComponent(Self.fileName)
        picks = (try? Data(contentsOf: file))
            .flatMap { try? JSONSerialization.jsonObject(with: $0) as? [String: String] } ?? [:]
    }

    /// The remembered pick for `typed` (roman, as typed), trimmed of punctuation.
    func pick(for typed: String) -> String? {
        let key = Self.trim(typed)
        return key.isEmpty ? nil : picks[key]
    }

    func remember(_ picked: String, for typed: String) {
        let key = Self.trim(typed), value = Self.trim(picked)
        guard !key.isEmpty, !value.isEmpty, picks[key] != value else { return }
        picks[key] = value
        save()
    }

    func forget(_ typed: String) {
        let key = Self.trim(typed)
        guard picks.removeValue(forKey: key) != nil else { return }
        save()
    }

    static func trim(_ text: String) -> String {
        text.trimmingCharacters(in: punctuation)
    }

    private func save() {
        let snapshot = picks, file = file
        queue.async {
            guard let data = try? JSONSerialization.data(withJSONObject: snapshot, options: [.sortedKeys]) else { return }
            try? data.write(to: file, options: .atomic)
        }
    }
}
