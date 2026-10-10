"""Phonetic typing: keystrokes → the text an application ends up with, against real riti.

The same corpus as ``Tests/NuktaPhoneticTests/PhoneticComposerTests.swift`` — the two
implementations of ``docs/phonetic-spec.md`` must behave alike, so **a case added on one platform
belongs in both files**.

Skipped when libnukta_riti.so is not built: phonetic typing is optional on Linux, and the Bijoy
tests must still run on a machine without Rust. Build it with ``linux/build_riti.sh``.
"""

import os
import shutil
import sys
import tempfile
import unicodedata
import unittest
import uuid

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from nukta_bangla import riti  # noqa: E402
from nukta_bangla.phonetic import (PHONETIC_FIRST, PHONETIC_ONLY, SMART,  # noqa: E402
                                   Key, PhoneticComposer, PhoneticOptions, contains_emoji)

requires_riti = unittest.skipUnless(
    riti.available(), "libnukta_riti.so is not built (linux/build_riti.sh)")


class Rig:
    """Plays keys into a composer the way the IBus layer does, keeping the application's text:
    committed text, plus the marked (preedit) text still being composed."""

    def __init__(self, directory=None, **options):
        self.directory = directory or os.path.join(
            tempfile.gettempdir(), f"nukta-tests-{uuid.uuid4()}")
        self.composer = PhoneticComposer(PhoneticOptions(**options), self.directory)
        self.doc = ""
        self.marked = ""

    def apply(self, out, app_text=""):
        self.doc += out.insert
        self.marked = out.marked
        if not out.handled:
            self.doc += app_text

    def type(self, keys):
        """Letters, digits and punctuation; ``␣`` space, ``⏎`` Enter, ``⎋`` Escape,
        ``⌫`` backspace, ``↓``/``↑`` move the selection."""
        for character in keys:
            if character == "␣":
                self.apply(self.composer.handle(Key.SPACE), app_text=" ")
            elif character == "⏎":
                self.apply(self.composer.handle(Key.ENTER), app_text="\n")
            elif character == "⎋":
                self.apply(self.composer.handle(Key.ESCAPE))
            elif character == "⌫":
                out = self.composer.handle(Key.backspace())
                if out.handled:
                    self.apply(out)
                elif self.doc:
                    self.doc = self.doc[:-1]
            elif character == "↓":
                self.apply(self.composer.handle(Key.DOWN))
            elif character == "↑":
                self.apply(self.composer.handle(Key.UP))
            else:
                self.apply(self.composer.handle(
                    Key.printable(character, shift=character.isupper())), app_text=character)
        return self

    @property
    def finished(self):
        """Everything on screen once the word in progress is committed."""
        self.apply(self.composer.commit())
        return self.doc

    @property
    def candidates(self):
        return self.composer.candidates

    @property
    def selected(self):
        return self.composer.candidates[self.composer.selected_index]


class PhoneticComposerTests(unittest.TestCase):
    def setUp(self):
        # Swift's `==` on strings compares canonical equivalents, so the Swift corpus can spell
        # য় either as U+09DF or as য + ় (riti writes the second). Python compares scalars, so
        # text is compared normalised and a case can be copied between the two files as it is.
        self.addTypeEqualityFunc(str, self._assert_same_text)

    def _assert_same_text(self, first, second, msg=None):
        if unicodedata.normalize("NFC", first) != unicodedata.normalize("NFC", second):
            self.fail(self._formatMessage(msg, f"{first!r} != {second!r}"))

    def tearDown(self):
        # Each rig has its own learned-picks directory; riti wrote into it.
        for directory in getattr(self, "_directories", []):
            shutil.rmtree(directory, ignore_errors=True)

    def rig(self, **options):
        rig = Rig(**options)
        self._directories = getattr(self, "_directories", []) + [rig.directory]
        return rig

    def test_phonetic_first_commits_the_transliteration(self):
        self.assertEqual(self.rig().type("ami␣banglay␣likhchi").finished, "আমি বাংলায় লিখছি")
        # Without dictionary help, `sonar` is সনার, exactly as typed.
        self.assertEqual(self.rig().type("sonar␣").doc, "সনার ")

    def test_marked_text_follows_the_word(self):
        rig = self.rig().type("kh")
        self.assertEqual(rig.marked, "খ")
        self.assertEqual(rig.doc, "")
        rig.type("i")
        self.assertEqual(rig.marked, "খি")
        self.assertEqual(rig.candidates[0], "খি")
        self.assertEqual(rig.composer.auxiliary, "khi")

    def test_list_offers_dictionary_words(self):
        rig = self.rig().type("sonar")
        self.assertEqual(rig.candidates[0], "সনার")
        self.assertIn("সোনার", rig.candidates)
        self.assertEqual(rig.composer.selected_index, 0)

    def test_number_key_picks_a_candidate(self):
        rig = self.rig().type("sonar")
        rig.type(str(rig.candidates.index("সোনার") + 1))
        self.assertEqual(rig.doc, "সোনার")
        self.assertFalse(rig.composer.is_composing)

    def test_arrows_move_the_selection_and_enter_commits_without_newline(self):
        rig = self.rig().type("sonar↓")
        self.assertEqual(rig.composer.selected_index, 1)
        second = rig.candidates[1]
        self.assertEqual(rig.marked, second)
        rig.type("↑↓⏎")
        self.assertEqual(rig.doc, second)

    def test_phonetic_first_remembers_a_pick(self):
        rig = self.rig()
        rig.type("sonar")
        rig.type(str(rig.candidates.index("সোনার") + 1))
        rig.type("␣sonar␣")
        self.assertEqual(rig.doc, "সোনার সোনার ")

        # Picking the plain transliteration again forgets it.
        rig.type("sonar1␣sonar␣")
        self.assertEqual(rig.doc, "সোনার সোনার সনার সনার ")

    def test_smart_mode_commits_the_dictionary_word(self):
        self.assertEqual(self.rig(mode=SMART).type("sonar␣").doc, "সোনার ")

    def test_autocorrect_switch(self):
        # riti's autocorrect list spells `account` as `oZakaunT` (অ্যাকাউন্ট).
        on = self.rig(mode=SMART).type("account")
        self.assertEqual(on.candidates[0], "অ্যাকাউন্ট")
        self.assertEqual(on.type("␣").doc, "অ্যাকাউন্ট ")

        off = self.rig(mode=SMART, autocorrect=False).type("account")
        self.assertNotIn("অ্যাকাউন্ট", off.candidates)
        self.assertEqual(off.type("␣").doc, "আচ্চউন্ত ")

    def test_phonetic_only_has_no_list(self):
        rig = self.rig(mode=PHONETIC_ONLY).type("sonar")
        self.assertEqual(rig.marked, "সনার")
        self.assertEqual(rig.candidates, [])
        rig.type("↓")
        self.assertEqual(rig.doc, "সনার")

    def test_escape_drops_the_word(self):
        rig = self.rig().type("ami␣sonar⎋")
        self.assertEqual(rig.doc, "আমি ")
        self.assertEqual(rig.marked, "")
        self.assertFalse(rig.composer.is_composing)

    def test_backspace_edits_the_word_then_the_app(self):
        self.assertEqual(self.rig().type("khi⌫a").finished, "খা")
        self.assertEqual(self.rig().type("ami␣⌫").finished, "আমি")

    def test_digits_are_bangla_outside_a_word(self):
        self.assertEqual(self.rig().type("2024␣").doc, "২০২৪ ")

    def test_punctuation_ends_a_word(self):
        self.assertEqual(self.rig().type("ki?").finished, "কি?")
        self.assertEqual(self.rig().type("ami.").finished, "আমি।")

    def test_emoji_only_by_explicit_pick_and_can_be_hidden(self):
        with_emoji = self.rig().type("hasi")
        self.assertFalse(contains_emoji(with_emoji.selected))
        without = self.rig(emoji=False).type("hasi")
        self.assertFalse(any(contains_emoji(candidate) for candidate in without.candidates))

    def test_readme_examples(self):
        self.assertEqual(self.rig().type("kotha␣dhorrmo␣prem").finished, "কথা ধর্ম প্রেম")
        self.assertIn("😄", self.rig().type("hasi").candidates)

    def test_smart_mode_never_commits_an_emoji_by_default(self):
        # riti offers ©️ for `st`.
        doc = self.rig(mode=SMART).type("st␣").doc
        self.assertFalse(contains_emoji(doc), doc)

    def test_colon_types_a_colon_by_default(self):
        self.assertEqual(self.rig().type("somoy:␣10").finished, "সময়: ১০")
        self.assertEqual(self.rig().type(":").finished, ":")
        # ঃ words still come from the list.
        self.assertIn("দুঃখ", self.rig().type("dukho").candidates)

    def test_colon_can_type_bisarga(self):
        self.assertEqual(self.rig(colon_is_bisarga=True).type("du:kho").finished, "দুঃখ")

    def test_display_only_options_keep_the_word(self):
        rig = self.rig().type("sonar")
        rig.composer.update(PhoneticOptions(mode=PHONETIC_FIRST, colon_is_bisarga=True,
                                            emoji=False))
        self.assertTrue(rig.composer.is_composing)

    def test_options_change_drops_the_word(self):
        rig = self.rig().type("sonar")
        rig.composer.update(PhoneticOptions(mode=SMART))
        self.assertFalse(rig.composer.is_composing)
        self.assertEqual(rig.type("sonar␣").doc, "সোনার ")


class EmojiTests(unittest.TestCase):
    """`contains_emoji` needs no riti: it is what keeps Space from committing an emoji."""

    def test_text_symbols_shown_as_emoji_count_as_emoji(self):
        self.assertTrue(contains_emoji("©️"))
        self.assertTrue(contains_emoji("😄"))
        self.assertFalse(contains_emoji("©"))
        self.assertFalse(contains_emoji("সোনার"))
        self.assertFalse(contains_emoji("১২#*"))


# Everything that drives a composer needs riti; the emoji test above does not.
PhoneticComposerTests = requires_riti(PhoneticComposerTests)


if __name__ == "__main__":
    unittest.main()
