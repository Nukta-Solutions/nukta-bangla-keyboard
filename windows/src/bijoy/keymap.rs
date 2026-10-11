//! What each Bijoy key means: a port of `Sources/NuktaEngine/KeyMap.swift`.

/// What a Bijoy key means, independent of how it will be reordered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BanglaKey {
    Consonant(&'static str),
    /// `g`: the link key (্) used to build juktakkhor, or `g` + kar for a full vowel.
    Hasanta,
    /// `z` → ্র, `Z` → ্য
    Phala(&'static str),
    /// Vowel sign. ি ে ৈ are pre-base (typed before the consonant); the rest are post-base.
    Kar(&'static str),
    /// `A`: র্ typed after the consonant it sits on.
    Reph,
    /// ং ঃ ঁ: attach to the end of the current syllable.
    Sign(&'static str),
    /// অ: a full vowel that a following া turns into আ (F f).
    Vowel(&'static str),
    /// Stand-alone text: digits, ।, ৳, ৎ.
    Literal(&'static str),
}

use BanglaKey::*;

/// The key for a character typed on a US QWERTY layout (shifted where applicable), or None when
/// the character is not a Bijoy key.
pub fn key(character: char) -> Option<BanglaKey> {
    Some(match character {
        'q' => Consonant("ঙ"), 'Q' => Sign("ং"),
        'w' => Consonant("য"), 'W' => Consonant("\u{09DF}"), // য়
        'e' => Consonant("ড"), 'E' => Consonant("ঢ"),
        'r' => Consonant("প"), 'R' => Consonant("ফ"),
        't' => Consonant("ট"), 'T' => Consonant("ঠ"),
        'y' => Consonant("চ"), 'Y' => Consonant("ছ"),
        'u' => Consonant("জ"), 'U' => Consonant("ঝ"),
        'i' => Consonant("হ"), 'I' => Consonant("ঞ"),
        'o' => Consonant("গ"), 'O' => Consonant("ঘ"),
        'p' => Consonant("\u{09DC}"), 'P' => Consonant("\u{09DD}"), // ড় ঢ়

        'a' => Kar("\u{09C3}"), 'A' => Reph,              // ৃ, র্
        's' => Kar("\u{09C1}"), 'S' => Kar("\u{09C2}"),   // ু ূ
        'd' => Kar("\u{09BF}"), 'D' => Kar("\u{09C0}"),   // ি ী
        'f' => Kar("\u{09BE}"), 'F' => Vowel("অ"),        // া
        'g' => Hasanta, 'G' => Literal("।"),
        'h' => Consonant("ব"), 'H' => Consonant("ভ"),
        'j' => Consonant("ক"), 'J' => Consonant("খ"),
        'k' => Consonant("ত"), 'K' => Consonant("থ"),
        'l' => Consonant("দ"), 'L' => Consonant("ধ"),

        'z' => Phala("\u{09CD}র"), 'Z' => Phala("\u{09CD}য"),
        'x' => Kar("\u{09CB}"), 'X' => Kar("\u{09D7}"),   // ো, ৗ (au length mark)
        'c' => Kar("\u{09C7}"), 'C' => Kar("\u{09C8}"),   // ে ৈ
        'v' => Consonant("র"), 'V' => Consonant("ল"),
        'b' => Consonant("ন"), 'B' => Consonant("ণ"),
        'n' => Consonant("স"), 'N' => Consonant("ষ"),
        'm' => Consonant("ম"), 'M' => Consonant("শ"),

        '0' => Literal("০"), '1' => Literal("১"), '2' => Literal("২"), '3' => Literal("৩"),
        '4' => Literal("৪"), '5' => Literal("৫"), '6' => Literal("৬"), '7' => Literal("৭"),
        '8' => Literal("৮"), '9' => Literal("৯"),
        '$' => Literal("৳"), '&' => Sign("\u{0981}"),     // ঁ
        '\\' => Literal("ৎ"), '|' => Sign("ঃ"),
        _ => return None,
    })
}
