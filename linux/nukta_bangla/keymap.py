"""What a Bijoy key means, independent of how it will be reordered.

A key is a ``(kind, value)`` tuple:

``("consonant", c)``     a consonant.
``("hasanta",)``         ``g`` — the link key (্) used to build juktakkhor, or ``g`` + kar for a
                         full vowel.
``("phala", p)``         ``z`` → ্র, ``Z`` → ্য.
``("kar", k)``           vowel sign. ি ে ৈ are pre-base (typed before the consonant); the rest are
                         post-base.
``("reph",)``            ``A`` — র্ typed after the consonant it sits on.
``("sign", s)``          ং ঃ ঁ — attach to the end of the current syllable.
``("vowel", v)``         অ — a full vowel that a following া turns into আ (``F f``).
``("literal", t)``       stand-alone text: digits, ।, ৳, ৎ.

This mirrors ``Sources/NuktaEngine/KeyMap.swift`` key for key.
"""

HASANTA = "্"

# Keyed by the character the key produces on a US QWERTY layout (shifted where applicable).
KEYS = {
    "q": ("consonant", "ঙ"), "Q": ("sign", "ং"),
    "w": ("consonant", "য"), "W": ("consonant", "য়"),  # য়
    "e": ("consonant", "ড"), "E": ("consonant", "ঢ"),
    "r": ("consonant", "প"), "R": ("consonant", "ফ"),
    "t": ("consonant", "ট"), "T": ("consonant", "ঠ"),
    "y": ("consonant", "চ"), "Y": ("consonant", "ছ"),
    "u": ("consonant", "জ"), "U": ("consonant", "ঝ"),
    "i": ("consonant", "হ"), "I": ("consonant", "ঞ"),
    "o": ("consonant", "গ"), "O": ("consonant", "ঘ"),
    "p": ("consonant", "ড়"), "P": ("consonant", "ঢ়"),  # ড় ঢ়

    "a": ("kar", "ৃ"), "A": ("reph",),              # ৃ, র্
    "s": ("kar", "ু"), "S": ("kar", "ূ"),      # ু ূ
    "d": ("kar", "ি"), "D": ("kar", "ী"),      # ি ী
    "f": ("kar", "া"), "F": ("vowel", "অ"),         # া
    "g": ("hasanta",), "G": ("literal", "।"),
    "h": ("consonant", "ব"), "H": ("consonant", "ভ"),
    "j": ("consonant", "ক"), "J": ("consonant", "খ"),
    "k": ("consonant", "ত"), "K": ("consonant", "থ"),
    "l": ("consonant", "দ"), "L": ("consonant", "ধ"),

    "z": ("phala", HASANTA + "র"), "Z": ("phala", HASANTA + "য"),
    "x": ("kar", "ো"), "X": ("kar", "ৗ"),      # ো, ৗ (au length mark)
    "c": ("kar", "ে"), "C": ("kar", "ৈ"),      # ে ৈ
    "v": ("consonant", "র"), "V": ("consonant", "ল"),
    "b": ("consonant", "ন"), "B": ("consonant", "ণ"),
    "n": ("consonant", "স"), "N": ("consonant", "ষ"),
    "m": ("consonant", "ম"), "M": ("consonant", "শ"),

    "0": ("literal", "০"), "1": ("literal", "১"), "2": ("literal", "২"), "3": ("literal", "৩"),
    "4": ("literal", "৪"), "5": ("literal", "৫"), "6": ("literal", "৬"), "7": ("literal", "৭"),
    "8": ("literal", "৮"), "9": ("literal", "৯"),
    "$": ("literal", "৳"), "&": ("sign", "ঁ"),      # ঁ
    "\\": ("literal", "ৎ"), "|": ("sign", "ঃ"),
}


def key(character):
    """The Bijoy key a character stands for, or None if it is not one."""
    return KEYS.get(character)
