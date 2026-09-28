/// Result of one keystroke.
/// `committed` is text that is final; `display` is the syllable still being built
/// (it can still be reordered by later keys).
public struct Output: Equatable {
    public var committed: String
    public var display: String

    public init(committed: String, display: String) {
        self.committed = committed
        self.display = display
    }
}

/// Where ি ে ৈ are typed relative to their consonant.
public enum KarOrder: String {
    /// Avro style: after a bare consonant the kar attaches to it (`j c` → কে); anywhere else
    /// it waits for the next consonant (`c j` → কে).
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
                    syllable.append(.preKar(k)) // belongs to the consonant just typed; rendered after the cluster
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

        return Output(committed: committed, display: syllable.rendered)
    }

    /// Undoes the last keystroke of the current syllable. Returns nil when nothing
    /// is being composed (the caller should let the app handle backspace).
    public func backspace() -> Output? {
        guard !syllable.isEmpty else { return nil }
        syllable.removeLast()
        return Output(committed: "", display: syllable.rendered)
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
        committed += syllable.rendered
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
        var deleted = String.UnicodeScalarView()
        deleted.append(contentsOf: o[i...])
        var inserted = String.UnicodeScalarView()
        inserted.append(contentsOf: n[i...])
        return Edit(deleted: String(deleted), inserted: String(inserted))
    }
}
