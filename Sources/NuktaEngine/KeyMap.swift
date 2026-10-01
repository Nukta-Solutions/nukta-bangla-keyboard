/// What a Bijoy key means, independent of how it will be reordered.
public enum BanglaKey: Equatable {
    case consonant(String)
    /// `g` — the link key (্) used to build juktakkhor, or `g` + kar for a full vowel.
    case hasanta
    /// `z` → ্র, `Z` → ্য
    case phala(String)
    /// Vowel sign. ি ে ৈ are pre-base (typed before the consonant); the rest are post-base.
    case kar(String)
    /// `A` — র্ typed after the consonant it sits on.
    case reph
    /// ং ঃ ঁ — attach to the end of the current syllable.
    case sign(String)
    /// অ — a full vowel that a following া turns into আ (F f).
    case vowel(String)
    /// Stand-alone text: digits, ।, ৳, ৎ.
    case literal(String)
}

public enum KeyMap {
    /// Keyed by the character the key produces on a US QWERTY layout (shifted where applicable).
    public static let keys: [Character: BanglaKey] = [
        "q": .consonant("ঙ"), "Q": .sign("ং"),
        "w": .consonant("য"), "W": .consonant("\u{09DF}"), // য়
        "e": .consonant("ড"), "E": .consonant("ঢ"),
        "r": .consonant("প"), "R": .consonant("ফ"),
        "t": .consonant("ট"), "T": .consonant("ঠ"),
        "y": .consonant("চ"), "Y": .consonant("ছ"),
        "u": .consonant("জ"), "U": .consonant("ঝ"),
        "i": .consonant("হ"), "I": .consonant("ঞ"),
        "o": .consonant("গ"), "O": .consonant("ঘ"),
        "p": .consonant("\u{09DC}"), "P": .consonant("\u{09DD}"), // ড় ঢ়

        "a": .kar("\u{09C3}"), "A": .reph,              // ৃ, র্
        "s": .kar("\u{09C1}"), "S": .kar("\u{09C2}"),   // ু ূ
        "d": .kar("\u{09BF}"), "D": .kar("\u{09C0}"),   // ি ী
        "f": .kar("\u{09BE}"), "F": .vowel("অ"),        // া
        "g": .hasanta, "G": .literal("।"),
        "h": .consonant("ব"), "H": .consonant("ভ"),
        "j": .consonant("ক"), "J": .consonant("খ"),
        "k": .consonant("ত"), "K": .consonant("থ"),
        "l": .consonant("দ"), "L": .consonant("ধ"),

        "z": .phala("\u{09CD}র"), "Z": .phala("\u{09CD}য"),
        "x": .kar("\u{09CB}"), "X": .kar("\u{09D7}"),   // ো, ৗ (au length mark)
        "c": .kar("\u{09C7}"), "C": .kar("\u{09C8}"),   // ে ৈ
        "v": .consonant("র"), "V": .consonant("ল"),
        "b": .consonant("ন"), "B": .consonant("ণ"),
        "n": .consonant("স"), "N": .consonant("ষ"),
        "m": .consonant("ম"), "M": .consonant("শ"),

        "0": .literal("০"), "1": .literal("১"), "2": .literal("২"), "3": .literal("৩"),
        "4": .literal("৪"), "5": .literal("৫"), "6": .literal("৬"), "7": .literal("৭"),
        "8": .literal("৮"), "9": .literal("৯"),
        "$": .literal("৳"), "&": .sign("\u{0981}"),     // ঁ
        "\\": .literal("ৎ"), "|": .sign("ঃ"),
    ]

    public static func key(for character: Character) -> BanglaKey? {
        keys[character]
    }
}
