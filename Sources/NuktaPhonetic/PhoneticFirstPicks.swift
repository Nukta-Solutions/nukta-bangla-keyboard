// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of
// the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Adapted from Lekho (https://github.com/ARahim3/Lekho), Lekho/Sources/Engine.swift.

import Foundation

/// Phonetic-first: the candidates a user deliberately picked over the literal transliteration,
/// keyed by the typed word. Picking the transliteration again forgets the entry. Kept apart from
/// riti's own selections so it never changes smart mode.
final class PhoneticFirstPicks {
    private var picks: [String: String] = [:]
    private let fileURL: URL
    private let ioQueue = DispatchQueue(label: "com.asifmahmud.inputmethod.NuktaBangla.picks", qos: .utility)

    /// Characters riti treats as punctuation around a word, plus their Bangla forms.
    private static let affixCharacters = CharacterSet(charactersIn: "-]~!@#%&*()_=+[{}'\";<>/?|.,।“”‘’")

    init(directory: URL) {
        fileURL = directory.appendingPathComponent("phonetic-first-picks.json")
        if let data = try? Data(contentsOf: fileURL),
           let stored = try? JSONSerialization.jsonObject(with: data) as? [String: String] {
            picks = stored
        }
    }

    /// The bare word: `"sonar."` → `sonar`, `“সোনার।”` → `সোনার`.
    static func core(_ text: String) -> String {
        text.trimmingCharacters(in: affixCharacters)
    }

    /// The remembered pick (bare word) for the typed text, if any.
    func pick(forTyped typed: String) -> String? {
        let key = Self.core(typed)
        return key.isEmpty ? nil : picks[key]
    }

    /// Records a commit. `literal` is the transliteration that was on offer.
    func record(typed: String, chosen: String, literal: String) {
        let key = Self.core(typed)
        let chosenCore = Self.core(chosen)
        guard !key.isEmpty, !chosenCore.isEmpty else { return }

        if chosenCore == Self.core(literal) {
            guard picks.removeValue(forKey: key) != nil else { return }
        } else {
            // An emoji is a one-off decoration, not a spelling preference.
            guard !PhoneticComposer.containsEmoji(chosen), picks[key] != chosenCore else { return }
            picks[key] = chosenCore
        }
        save()
    }

    private func save() {
        let snapshot = picks
        let url = fileURL
        ioQueue.async {
            guard let data = try? JSONSerialization.data(withJSONObject: snapshot, options: [.sortedKeys]) else { return }
            try? data.write(to: url, options: .atomic)
        }
    }

    /// Waits for pending writes (tests).
    func flush() {
        ioQueue.sync {}
    }
}
