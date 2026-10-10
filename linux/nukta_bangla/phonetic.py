"""Phonetic typing: keys → Bangla with riti, and the suggestion list for the word in progress.

A port of ``Sources/NuktaPhonetic/PhoneticComposer.swift`` and ``PickMemory.swift``, behaviour for
behaviour — ``docs/phonetic-spec.md`` describes what both must do, and the test corpus in
``linux/tests/test_phonetic.py`` is the same one the Swift tests use.

Pure logic, no IBus: the front end in ``ibus_phonetic.py`` shows `Output`, `candidates` and
`auxiliary`. riti contexts are expensive, and each one rewrites riti's file of learned picks from
its own copy, so there is one composer per process.
"""

import json
import os
import queue
import threading

from .riti import RitiContext, keycode as riti_keycode

#: How phonetic typing picks the word that Space commits. The strings are what the settings file
#: holds, so they match the macOS `TypingMode` raw values.
PHONETIC_FIRST = "phoneticFirst"   # what was typed, transliterated; a pick is preferred next time
SMART = "smart"                    # riti's own choice: dictionary, autocorrect and past picks
PHONETIC_ONLY = "phoneticOnly"     # transliteration only, no list
TYPING_MODES = (PHONETIC_FIRST, SMART, PHONETIC_ONLY)


class PhoneticOptions:
    """The settings phonetic typing depends on."""

    __slots__ = ("mode", "autocorrect", "emoji", "english_word", "colon_is_bisarga")

    def __init__(self, mode=PHONETIC_FIRST, autocorrect=True, emoji=True, english_word=True,
                 colon_is_bisarga=False):
        self.mode = mode
        self.autocorrect = autocorrect
        #: List the typed roman word itself.
        self.english_word = english_word
        self.emoji = emoji
        #: False: ":" types a colon. True: it types ঃ, as in Avro.
        self.colon_is_bisarga = colon_is_bisarga

    def _fields(self):
        return (self.mode, self.autocorrect, self.emoji, self.english_word, self.colon_is_bisarga)

    def __eq__(self, other):
        return isinstance(other, PhoneticOptions) and self._fields() == other._fields()

    def __repr__(self):
        return ("PhoneticOptions(mode={!r}, autocorrect={}, emoji={}, english_word={}, "
                "colon_is_bisarga={})").format(*self._fields())


class Key:
    """One keystroke, as the front end reports it.

    Made with the factories and constants below: ``Key.printable("a")``, ``Key.backspace()``,
    ``Key.ENTER``…
    """

    __slots__ = ("kind", "character", "shift", "whole_word", "backward")

    def __init__(self, kind, character="", shift=False, whole_word=False, backward=False):
        self.kind = kind
        self.character = character
        self.shift = shift
        self.whole_word = whole_word
        self.backward = backward

    @classmethod
    def printable(cls, character, shift=False):
        """A printable key, as on a US layout."""
        return cls("printable", character=character, shift=shift)

    @classmethod
    def backspace(cls, whole_word=False):
        """⌫, or Ctrl/Alt+⌫ for the whole word."""
        return cls("backspace", whole_word=whole_word)

    @classmethod
    def tab(cls, backward=False):
        return cls("tab", backward=backward)

    def __repr__(self):
        return f"Key({self.kind}{'=' + self.character if self.character else ''})"


Key.ENTER = Key("enter")
Key.ESCAPE = Key("escape")
Key.SPACE = Key("space")
Key.UP = Key("up")
Key.DOWN = Key("down")
Key.LEFT = Key("left")
Key.RIGHT = Key("right")


class Output:
    """What one key does to the application's text."""

    __slots__ = ("insert", "marked", "handled")

    def __init__(self, insert="", marked="", handled=True):
        #: Final text to insert, replacing the marked text.
        self.insert = insert
        #: Marked (underlined, in the preedit) text to show afterwards; "" = none.
        self.marked = marked
        #: False: the application must also handle the key (space, arrows…).
        self.handled = handled

    def __eq__(self, other):
        return (isinstance(other, Output)
                and (self.insert, self.marked, self.handled)
                == (other.insert, other.marked, other.handled))

    def __repr__(self):
        return f"Output(insert={self.insert!r}, marked={self.marked!r}, handled={self.handled})"


# Scalars that count as emoji: `Emoji=Yes` from Unicode's emoji-data, keeping only U+2000 and
# above. Swift asks the character properties for this; Python's unicodedata has no emoji property,
# so the ranges are spelled out. Below U+2000 nothing is Emoji_Presentation, which is why digits,
# `#`, `*` and a plain © are not emoji here — only © followed by U+FE0F is.
_EMOJI_RANGES = (
    (0x203C, 0x203C), (0x2049, 0x2049), (0x2122, 0x2122), (0x2139, 0x2139),
    (0x2194, 0x2199), (0x21A9, 0x21AA), (0x231A, 0x231B), (0x2328, 0x2328),
    (0x23CF, 0x23CF), (0x23E9, 0x23F3), (0x23F8, 0x23FA), (0x24C2, 0x24C2),
    (0x25AA, 0x25AB), (0x25B6, 0x25B6), (0x25C0, 0x25C0), (0x25FB, 0x25FE),
    (0x2600, 0x2604), (0x260E, 0x260E), (0x2611, 0x2611), (0x2614, 0x2615),
    (0x2618, 0x2618), (0x261D, 0x261D), (0x2620, 0x2620), (0x2622, 0x2623),
    (0x2626, 0x2626), (0x262A, 0x262A), (0x262E, 0x262F), (0x2638, 0x263A),
    (0x2640, 0x2640), (0x2642, 0x2642), (0x2648, 0x2653), (0x265F, 0x2660),
    (0x2663, 0x2663), (0x2665, 0x2666), (0x2668, 0x2668), (0x267B, 0x267B),
    (0x267E, 0x267F), (0x2692, 0x2697), (0x2699, 0x2699), (0x269B, 0x269C),
    (0x26A0, 0x26A1), (0x26A7, 0x26A7), (0x26AA, 0x26AB), (0x26B0, 0x26B1),
    (0x26BD, 0x26BE), (0x26C4, 0x26C5), (0x26C8, 0x26C8), (0x26CE, 0x26CF),
    (0x26D1, 0x26D1), (0x26D3, 0x26D4), (0x26E9, 0x26EA), (0x26F0, 0x26F5),
    (0x26F7, 0x26FA), (0x26FD, 0x26FD), (0x2702, 0x2702), (0x2705, 0x2705),
    (0x2708, 0x270D), (0x270F, 0x270F), (0x2712, 0x2712), (0x2714, 0x2714),
    (0x2716, 0x2716), (0x271D, 0x271D), (0x2721, 0x2721), (0x2728, 0x2728),
    (0x2733, 0x2734), (0x2744, 0x2744), (0x2747, 0x2747), (0x274C, 0x274C),
    (0x274E, 0x274E), (0x2753, 0x2755), (0x2757, 0x2757), (0x2763, 0x2764),
    (0x2795, 0x2797), (0x27A1, 0x27A1), (0x27B0, 0x27B0), (0x27BF, 0x27BF),
    (0x2934, 0x2935), (0x2B05, 0x2B07), (0x2B1B, 0x2B1C), (0x2B50, 0x2B50),
    (0x2B55, 0x2B55), (0x3030, 0x3030), (0x303D, 0x303D), (0x3297, 0x3297),
    (0x3299, 0x3299),
    (0x1F004, 0x1F004), (0x1F0CF, 0x1F0CF), (0x1F170, 0x1F171), (0x1F17E, 0x1F17F),
    (0x1F18E, 0x1F18E), (0x1F191, 0x1F19A), (0x1F1E6, 0x1F1FF), (0x1F201, 0x1F202),
    (0x1F21A, 0x1F21A), (0x1F22F, 0x1F22F), (0x1F232, 0x1F23A), (0x1F250, 0x1F251),
    (0x1F300, 0x1F321), (0x1F324, 0x1F393), (0x1F396, 0x1F397), (0x1F399, 0x1F39B),
    (0x1F39E, 0x1F3F0), (0x1F3F3, 0x1F3F5), (0x1F3F7, 0x1F4FD), (0x1F4FF, 0x1F53D),
    (0x1F549, 0x1F54E), (0x1F550, 0x1F567), (0x1F56F, 0x1F570), (0x1F573, 0x1F57A),
    (0x1F587, 0x1F587), (0x1F58A, 0x1F58D), (0x1F590, 0x1F590), (0x1F595, 0x1F596),
    (0x1F5A4, 0x1F5A5), (0x1F5A8, 0x1F5A8), (0x1F5B1, 0x1F5B2), (0x1F5BC, 0x1F5BC),
    (0x1F5C2, 0x1F5C4), (0x1F5D1, 0x1F5D3), (0x1F5DC, 0x1F5DE), (0x1F5E1, 0x1F5E1),
    (0x1F5E3, 0x1F5E3), (0x1F5E8, 0x1F5E8), (0x1F5EF, 0x1F5EF), (0x1F5F3, 0x1F5F3),
    (0x1F5FA, 0x1F64F), (0x1F680, 0x1F6C5), (0x1F6CB, 0x1F6D2), (0x1F6D5, 0x1F6D7),
    (0x1F6DC, 0x1F6E5), (0x1F6E9, 0x1F6E9), (0x1F6EB, 0x1F6EC), (0x1F6F0, 0x1F6F0),
    (0x1F6F3, 0x1F6FC), (0x1F7E0, 0x1F7EB), (0x1F7F0, 0x1F7F0), (0x1F90C, 0x1F93A),
    (0x1F93C, 0x1F945), (0x1F947, 0x1F9FF), (0x1FA70, 0x1FA7C), (0x1FA80, 0x1FA88),
    (0x1FA90, 0x1FABD), (0x1FABF, 0x1FAC5), (0x1FACE, 0x1FADB), (0x1FAE0, 0x1FAE8),
    (0x1FAF0, 0x1FAF8),
)

VARIATION_SELECTOR_16 = 0xFE0F      # turns a text symbol like © into ©️


def contains_emoji(text):
    """Any emoji, including a text symbol made to show as one (© + U+FE0F = ©️)."""
    for character in text:
        code = ord(character)
        if code == VARIATION_SELECTOR_16:
            return True
        if code < 0x2000:
            continue
        if any(start <= code <= end for start, end in _EMOJI_RANGES):
            return True
    return False


class PickMemory:
    """Phonetic-first's memory: for a typed word, the list entry the user picked instead of the
    plain transliteration, so it is selected next time.

    Kept apart from riti's own learning (smart mode), which works on riti's ranking rather than on
    the transliteration. Stored as a JSON object, typed word → picked word, in
    ``phonetic-first-picks.json`` — the same file, with the same name and format, as the macOS
    build, so a synced or copied directory works on both.
    """

    FILE_NAME = "phonetic-first-picks.json"

    # What riti puts around a word (`sonar,`, `(ami)`…), so `sonar` and `sonar,` share a pick:
    # riti's `META` (riti-bridge/riti/src/utility.rs), plus the quotes its smart quoting writes.
    PUNCTUATION = "-]~!@#%&*()_=+[{}'\";<>/?|.,।“”‘’"

    def __init__(self, directory):
        """A missing or unreadable file is an empty memory."""
        self._file = os.path.join(directory, self.FILE_NAME)
        self._picks = self._load(self._file)
        #: Writes run in order, off the typing path.
        self._writes = None

    @staticmethod
    def _load(path):
        try:
            with open(path, "rb") as handle:
                loaded = json.load(handle)
        except (OSError, ValueError):
            return {}
        if not isinstance(loaded, dict):
            return {}
        return {key: value for key, value in loaded.items()
                if isinstance(key, str) and isinstance(value, str)}

    def pick(self, typed):
        """The remembered pick for `typed` (roman, as typed), trimmed of punctuation."""
        key = self.trim(typed)
        return self._picks.get(key) if key else None

    def remember(self, picked, typed):
        key, value = self.trim(typed), self.trim(picked)
        if not key or not value or self._picks.get(key) == value:
            return
        self._picks[key] = value
        self._save()

    def forget(self, typed):
        if self._picks.pop(self.trim(typed), None) is None:
            return
        self._save()

    @classmethod
    def trim(cls, text):
        return text.strip(cls.PUNCTUATION)

    def _save(self):
        if self._writes is None:
            self._writes = queue.SimpleQueue()
            # One worker, so saves land in the order they were made; a daemon thread never holds
            # up the engine's exit (the last snapshot is written or it is not — either is valid).
            threading.Thread(target=self._write_queued, name="nukta-picks", daemon=True).start()
        self._writes.put(dict(self._picks))

    def _write_queued(self):
        while True:
            snapshot = self._writes.get()
            temporary = f"{self._file}.new"
            try:
                with open(temporary, "w", encoding="utf-8") as handle:
                    json.dump(snapshot, handle, ensure_ascii=False, sort_keys=True)
                os.replace(temporary, self._file)
            except OSError:
                pass


class PhoneticComposer:
    """Turns keys into Bangla with riti (Avro Phonetic), keeping the suggestion list for the word
    in progress."""

    #: Keys riti handles as punctuation: typed after the user moved the selection, riti keeps the
    #: selection instead of picking its own default (riti-bridge/riti/src/phonetic/method.rs).
    SELECTION_KEEPERS = frozenset(".?!,:;-_)}]'\"")

    def __init__(self, options, directory):
        """`directory`: where riti and the pick memory keep what they learn; created if missing."""
        os.makedirs(directory, exist_ok=True)
        self.options = options
        #: ← → also move the selection (the list is a row).
        self.horizontal_navigation = False
        #: The list to show; empty = no list.
        self.candidates = []
        self.selected_index = 0
        #: What was typed (roman), shown above the list.
        self.auxiliary = ""

        self._directory = directory
        self._picks = PickMemory(directory)
        self._riti, self._literal_riti = self._make_contexts(options, directory)
        #: The word when riti gives no list (phonetic-only mode, a lone punctuation mark).
        self._lonely = ""
        #: For each candidate, its index in riti's own list (the order differs after filtering and
        #: moving the literal up).
        self._riti_indices = []
        #: Index in `candidates` of the literal transliteration, if it is there.
        self._literal_index = None
        #: The literal transliteration of what was typed, from the literal-only context.
        self._literal = ""
        #: The user moved the selection since the last key that went to riti.
        self._selection_moved = False

    @property
    def is_composing(self):
        """A word is in progress."""
        return bool(self.candidates) or bool(self._lonely)

    def update(self, options):
        """A change to the mode, autocorrect or the English word rebuilds riti and drops the word
        in progress. Emoji and the colon apply from the next key."""
        rebuild = (options.mode != self.options.mode
                   or options.autocorrect != self.options.autocorrect
                   or options.english_word != self.options.english_word)
        if rebuild:
            self.discard()
            self._close_contexts()
            self._riti, self._literal_riti = self._make_contexts(options, self._directory)
        self.options = options

    def handle(self, key):
        if key.kind == "printable":
            return self._type(key.character, key.shift)
        if key.kind == "backspace":
            return self._backspace(key.whole_word)
        if key.kind == "enter":
            return self.commit() if self.is_composing else Output(handled=False)
        if key.kind == "escape":
            if not self.is_composing:
                return Output(handled=False)
            self.discard()
            return Output()
        if key.kind == "space":
            if not self.is_composing:
                return Output(handled=False)
            out = self.commit()
            out.handled = False
            return out
        if key.kind == "tab":
            return self._move_selection(-1 if key.backward else 1)
        if key.kind == "down":
            return self._move_selection(1)
        if key.kind == "up":
            return self._move_selection(-1)
        if key.kind in ("left", "right"):
            if self.horizontal_navigation:
                return self._move_selection(-1 if key.kind == "left" else 1)
            return self._commit_and_pass()
        return Output(handled=False)

    def commit(self):
        """Commits the selected word (a click elsewhere, a switch to another application)."""
        if not self.is_composing:
            return Output()
        if not self.candidates:
            text = self._lonely
            self._end_word(learning=None)
            return Output(insert=text)
        return self.commit_at(self.selected_index)

    def commit_at(self, index):
        """Commits ``candidates[index]`` (a click in the list)."""
        if not 0 <= index < len(self.candidates):
            return Output(marked=self._marked) if self.is_composing else Output()
        text = self.candidates[index]
        if self.options.mode == PHONETIC_FIRST and self._literal_index is not None:
            if index == self._literal_index or PickMemory.trim(text) == PickMemory.trim(self._literal):
                self._picks.forget(self.auxiliary)
            elif not contains_emoji(text):
                self._picks.remember(text, self.auxiliary)
        learning = self._riti_indices[index] if self.options.mode == SMART else None
        self._end_word(learning=learning)
        return Output(insert=text)

    def discard(self):
        """Drops the word in progress, with no output."""
        self._end_word(learning=None)

    def close(self):
        """Lets riti go. The engine keeps one composer for its lifetime, so this is for tests."""
        self.discard()
        self._close_contexts()

    # MARK: Keys

    def _type(self, character, shift):
        if character == ":" and not self.options.colon_is_bisarga:
            return Output(insert=self.commit().insert + ":")

        digit = self._ascii_digit(character)
        if digit is not None:
            if not self.is_composing:
                return Output(insert=self._bangla_digit(digit))
            if 1 <= digit <= 9:
                if self.candidates and digit <= len(self.candidates):
                    return self.commit_at(digit - 1)
                if not self.candidates:
                    return Output(insert=self.commit().insert + self._bangla_digit(digit))

        code = riti_keycode(character)
        if code is None:
            return self._commit_and_pass()

        # After the user moved the selection, punctuation keeps it: riti is told which entry it was.
        keep_selection = (self._selection_moved and bool(self.candidates)
                          and character in self.SELECTION_KEEPERS)
        if not keep_selection:
            self._selection_moved = False
        selection = self._riti_indices[self.selected_index] if keep_selection else 0

        # riti only reports what was typed with a list; this keeps it for a lonely answer too.
        self.auxiliary += character
        suggestion = self._riti.key(code, shift=shift, selection=selection)
        if self._literal_riti is not None:
            answer = self._literal_riti.key(code, shift=shift)
            if answer.is_lonely:
                self._literal = answer.text
        self._show(suggestion, kept_selection=keep_selection)

        if not self._riti.is_composing:
            # riti ended the word itself: what it produced is final.
            text = self._marked
            self._end_word(learning=None)
            return Output(insert=text)
        return Output(marked=self._marked)

    def _backspace(self, whole_word):
        if not self.is_composing:
            return Output(handled=False)
        self._selection_moved = False
        if whole_word:
            self.auxiliary = ""
        elif self.auxiliary:
            self.auxiliary = self.auxiliary[:-1]
        suggestion = self._riti.backspace(whole_word)
        if self._literal_riti is not None:
            answer = self._literal_riti.backspace(whole_word)
            self._literal = answer.text if answer.is_lonely else ""
        if suggestion.is_empty or not self._riti.is_composing:
            self._end_word(learning=None)
            return Output()
        self._show(suggestion, kept_selection=False)
        return Output(marked=self._marked)

    def _move_selection(self, step):
        """Up/down, Tab: move the selection, wrapping. With no list, the word is committed and the
        key goes to the application."""
        if not self.is_composing:
            return Output(handled=False)
        if not self.candidates:
            return self._commit_and_pass()
        self.selected_index = (self.selected_index + step) % len(self.candidates)
        self._selection_moved = True
        return Output(marked=self._marked)

    def _commit_and_pass(self):
        out = self.commit()
        out.handled = False
        return out

    # MARK: State

    @property
    def _marked(self):
        """The marked text: the selected candidate, or the lonely text when there is no list."""
        if 0 <= self.selected_index < len(self.candidates):
            return self.candidates[self.selected_index]
        return self._lonely

    def _show(self, suggestion, kept_selection):
        """Builds the list from riti's answer."""
        if suggestion.is_empty:
            self._clear_list()
        elif suggestion.is_lonely:
            self._clear_list()
            self._lonely = suggestion.text
        else:
            self._lonely = ""
            self.auxiliary = suggestion.auxiliary
            self._build_list(suggestion.entries, suggestion.selection, kept_selection)

    def _build_list(self, entries, riti_selection, kept_selection):
        items = list(enumerate(entries))        # (riti index, text)
        if not self.options.emoji:
            plain = [item for item in items if not contains_emoji(item[1])]
            if plain:
                items = plain

        self._literal_index = None
        if self.options.mode == PHONETIC_FIRST and self._literal:
            target = _straightened(self._literal)
            found = next((position for position, item in enumerate(items)
                          if _straightened(item[1]) == target), None)
            if found is not None:
                items.insert(0, items.pop(found))
                self._literal_index = 0

        self.candidates = [text for _, text in items]
        self._riti_indices = [index for index, _ in items]
        if not self.candidates:
            self.selected_index = 0
            return
        riti_default = (self._riti_indices.index(riti_selection)
                        if riti_selection in self._riti_indices else 0)

        if kept_selection or self.options.mode != PHONETIC_FIRST:
            selection = riti_default
        elif self._literal_index is not None:
            selection = self._literal_index
            pick = self._picks.pick(self.auxiliary)
            if pick is not None:
                picked = next((position for position, text in enumerate(self.candidates)
                               if PickMemory.trim(text) == pick), None)
                if picked is not None:
                    selection = picked
        else:
            selection = riti_default

        # Space must never commit an emoji nobody chose. A typed emoticon (`:)`) is exempt.
        if (not kept_selection and self.auxiliary[:1].isalpha()
                and contains_emoji(self.candidates[selection])):
            plain = next((position for position, text in enumerate(self.candidates)
                          if not contains_emoji(text)), None)
            if plain is not None:
                selection = plain
        self.selected_index = selection

    def _clear_list(self):
        self.candidates = []
        self._riti_indices = []
        self._literal_index = None
        self.selected_index = 0

    def _end_word(self, learning):
        """Ends the word in both contexts and resets. `learning`: riti index of the pick for riti
        to learn (smart mode)."""
        if learning is not None:
            self._riti.committed(learning)
        else:
            self._riti.finish()
        if self._literal_riti is not None:
            self._literal_riti.finish()
        self._clear_list()
        self._lonely = ""
        self.auxiliary = ""
        self._literal = ""
        self._selection_moved = False

    def _close_contexts(self):
        self._riti.close()
        if self._literal_riti is not None:
            self._literal_riti.close()
            self._literal_riti = None

    # MARK: Helpers

    @staticmethod
    def _make_contexts(options, directory):
        """The main context, and — in phonetic-first — a second one with suggestions off, fed the
        same keys, whose (lonely) answer is the literal transliteration. riti's list contains the
        literal but doesn't say which entry it is."""
        main = RitiContext(directory, suggestions=options.mode != PHONETIC_ONLY,
                           english_word=options.english_word, autocorrect=options.autocorrect)
        literal = (RitiContext(directory, suggestions=False, english_word=False, autocorrect=False)
                   if options.mode == PHONETIC_FIRST else None)
        return main, literal

    @staticmethod
    def _ascii_digit(character):
        return int(character) if character.isascii() and character.isdigit() else None

    @staticmethod
    def _bangla_digit(digit):
        return chr(0x09E6 + digit)


def _straightened(text):
    """riti's list has curly quotes where the literal transliteration has straight ones."""
    return text.replace("“", '"').replace("”", '"').replace("‘", "'").replace("’", "'")
