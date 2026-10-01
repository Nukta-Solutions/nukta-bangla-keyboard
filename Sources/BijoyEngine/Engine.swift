import Foundation

/// Result of one keystroke.
/// `committed` is text that is final; `display` is the syllable still being built
/// (it can still be reordered by later keys).
/// `visible` is the part of `display` safe to put straight into a document: a syllable with no
/// consonant yet (a waiting ি ে ৈ, a lone অ or g) stays off screen until the next key, so it
/// never has to be rewritten.
public struct Output: Equatable {
    public var committed: String
    public var display: String
    public var visible: String

    public init(committed: String, display: String, visible: String? = nil) {
        self.committed = committed
        self.display = display
        self.visible = visible ?? display
    }
}

/// Where ি ে ৈ are typed relative to their consonant.
public enum KarOrder: String {
    /// Avro style: classic, plus pressing the kar twice right after a consonant attaches it to
    /// that consonant (`c j` → কে and `j c c` → কে; `j c V` → কলে).
    case mixed
    /// Strict classic Bijoy: always before the consonant (`c j` → কে, `j c V` → কলে).
    case classic
}

/// Bijoy state machine. Pure logic, no AppKit.
public final class Engine {
    private var syllable = Syllable()
    private var committed = ""

    public var karOrder: KarOrder

    public init(karOrder: KarOrder = .mixed) {
        self.karOrder = karOrder
    }

    public var isComposing: Bool { !syllable.isEmpty }
    public var display: String { syllable.rendered }

    /// Returns nil when the character is not a Bijoy key (the caller should commit and pass it through).
    public func process(character: Character) -> Output? {
        guard let key = KeyMap.key(for: character) else { return nil }
        return process(key)
    }

    public func process(_ key: BanglaKey) -> Output {
        committed = ""

        // Kar pressed once after a consonant: pressed again, it belongs to that consonant
        // (j c c → কে); anything else, and it waits for the next consonant as usual (j c V → কলে).
        if case .waitingKar(let k) = syllable.tokens.last {
            syllable.removeLast()
            if key == .kar(k) {
                syllable.append(.preKar(k)) // rendered after the cluster
                return Output(committed: committed, display: syllable.rendered, visible: syllable.visibleRendered)
            }
            commitSyllable()
            syllable.append(.preKar(k))
        }

        // অ waits one key: া makes it আ (F f), anything else leaves it as অ.
        if case .vowel = syllable.tokens.first {
            if key == .kar("\u{09BE}") {
                syllable = Syllable()
                committed = "আ"
                return Output(committed: committed, display: "")
            }
            commitSyllable()
        }

        switch key {
        case .literal(let text):
            commitSyllable()
            committed += text

        case .vowel(let v):
            commitSyllable()
            syllable.append(.vowel(v))

        case .consonant(let c):
            if syllable.hasCluster && syllable.lastIsHasanta {
                syllable.append(.consonant(c)) // juktakkhor: ক + ্ + ত
            } else if syllable.hasCluster || syllable.isClusterClosed || syllable.lastIsHasanta {
                commitSyllable()
                syllable.append(.consonant(c))
            } else {
                syllable.append(.consonant(c)) // empty, or only a pending pre-base kar
            }

        case .hasanta:
            if syllable.lastIsHasanta {
                // g g → visible hasanta (্ + ZWNJ)
                syllable.removeLast()
                if syllable.hasCluster {
                    syllable.append(.explicitHasanta)
                    commitSyllable()
                } else {
                    commitSyllable()
                    committed += Syllable.hasantaChar + "\u{200C}"
                }
            } else if syllable.hasCluster && !syllable.isClusterClosed {
                syllable.append(.hasanta)
            } else {
                commitSyllable()
                syllable.append(.hasanta) // may become a full vowel with the next kar
            }

        case .kar(let k):
            if syllable.lastIsHasanta {
                // g + kar → full vowel (কই = j g d)
                syllable.removeLast()
                commitSyllable()
                committed += Syllable.independentVowels[k] ?? k
            } else if Syllable.preBaseKars.contains(k) {
                if karOrder == .mixed && syllable.hasCluster && !syllable.isClusterClosed && !syllable.hasPreKar {
                    syllable.append(.waitingKar(k)) // a second press attaches it here; see top of process()
                } else {
                    commitSyllable()
                    syllable.append(.preKar(k))
                }
            } else if !syllable.isEmpty {
                syllable.append(.postKar(k))
            } else {
                committed += k
            }

        case .reph:
            if syllable.hasCluster && !syllable.hasReph {
                syllable.append(.reph)
            } else {
                commitSyllable()
                committed += "র" + Syllable.hasantaChar
            }

        case .phala(let p):
            if syllable.hasCluster && !syllable.isClusterClosed {
                if syllable.lastIsHasanta { syllable.removeLast() }
                syllable.append(.phala(p))
            } else {
                commitSyllable()
                committed += p
            }

        case .sign(let s):
            if syllable.isEmpty || (!syllable.hasCluster && syllable.lastIsHasanta) {
                commitSyllable()
                committed += s
            } else {
                syllable.append(.sign(s))
            }
        }

        return Output(committed: committed, display: syllable.rendered, visible: syllable.visibleRendered)
    }

    /// Undoes the last keystroke of the current syllable. Returns nil when nothing
    /// is being composed (the caller should let the app handle backspace).
    public func backspace() -> Output? {
        guard !syllable.isEmpty else { return nil }
        syllable.removeLast()
        return Output(committed: "", display: syllable.rendered, visible: syllable.visibleRendered)
    }

    /// Finalises the current syllable.
    public func commit() -> Output {
        committed = ""
        commitSyllable()
        return Output(committed: committed, display: "")
    }

    public func reset() {
        syllable = Syllable()
        committed = ""
    }

    private func commitSyllable() {
        if case .waitingKar(let k) = syllable.tokens.last {
            // Finished with a single press pending: same text classic would give (ক + ে = কে).
            syllable.removeLast()
            committed += syllable.rendered + k
        } else {
            committed += syllable.rendered
        }
        syllable = Syllable()
    }
}

/// Minimal edit that turns the text currently on screen into the new text.
public enum TextDiff {
    public struct Edit: Equatable {
        /// Tail of the old text that must be removed.
        public var deleted: String
        public var inserted: String
    }

    public static func edit(from old: String, to new: String) -> Edit {
        let o = Array(old.unicodeScalars)
        let n = Array(new.unicodeScalars)
        var i = 0
        while i < o.count, i < n.count, o[i] == n[i] { i += 1 }
        let plain = Edit(deleted: string(o[i...]), inserted: string(n[i...]))
        guard !plain.deleted.isEmpty else { return plain }

        // ে on screen that became ো or ৌ: append া / ৗ instead of rewriting.
        // (ে + া is canonically the same text as ো.)
        let oldD = Array(old.decomposedStringWithCanonicalMapping.unicodeScalars)
        for j in 0...n.count {
            let prefixD = Array(string(n[..<j]).decomposedStringWithCanonicalMapping.unicodeScalars)
            guard prefixD.count >= oldD.count else { continue }
            guard Array(prefixD[..<oldD.count]) == oldD else { break }
            return Edit(deleted: "", inserted: string(prefixD[oldD.count...]) + string(n[j...]))
        }
        return plain
    }

    /// `doc` with a leading run canonically equal to `prefix` removed.
    public static func remainder(of doc: String, after prefix: String) -> String {
        guard !prefix.isEmpty else { return doc }
        let s = Array(doc.unicodeScalars)
        for j in 0...s.count where string(s[..<j]) == prefix {
            return string(s[j...])
        }
        return doc
    }

    /// `text` with its last `count` Unicode scalars removed.
    public static func dropLast(_ count: Int, of text: String) -> String {
        string(Array(text.unicodeScalars).dropLast(count))
    }

    private static func string<S: Sequence>(_ scalars: S) -> String where S.Element == Unicode.Scalar {
        var view = String.UnicodeScalarView()
        view.append(contentsOf: scalars)
        return String(view)
    }
}
