"""The key-event layer: physical keys → committed text and preedit.

Runs against the stubs in fake_ibus, so it needs no IBus and no desktop session. What it cannot
cover is IBus itself; linux/README.md has the manual check for that.
"""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import fake_ibus  # noqa: E402

IBus = fake_ibus.install()

from nukta_bangla.ibus_engine import NuktaBanglaEngine, US_LAYOUT  # noqa: E402

KEYCODE = {char: code for code, pair in US_LAYOUT.items() for char in pair}
SHIFTED = {pair[1] for pair in US_LAYOUT.values()}


def press(engine, char):
    """Types `char` as its physical key on a US keyboard."""
    keycode = KEYCODE[char]
    state = IBus.ModifierType.SHIFT_MASK if char in SHIFTED else 0
    return engine.do_process_key_event(ord(char.lower()), keycode, state)


class IBusLayerTests(unittest.TestCase):
    def setUp(self):
        self.engine = NuktaBanglaEngine()

    def type_keys(self, keys):
        for ch in keys:
            press(self.engine, ch)
        return self.engine

    def test_committed_text_and_preedit(self):
        e = self.type_keys("dj")
        self.assertEqual(e.preedit, "কি")       # still reorderable: preedit, not committed
        self.assertEqual(e.committed, "")
        self.assertTrue(e.preedit_visible)
        e.do_focus_out()
        self.assertEqual(e.committed, "কি")
        self.assertEqual(e.preedit, "")
        self.assertFalse(e.preedit_visible)

    def test_word_commits_syllable_by_syllable(self):
        e = self.type_keys("hfQVf")
        e.do_focus_out()
        self.assertEqual(e.committed, "বাংলা")

    def test_shifted_keys_use_the_us_layout(self):
        e = self.type_keys("gSLghA")               # ঊর্ধ্ব
        e.do_focus_out()
        self.assertEqual(e.committed, "ঊর্ধ্ব")

    def test_space_is_passed_to_the_application(self):
        e = self.engine
        self.type_keys("gfmd")
        handled = e.do_process_key_event(IBus.KEY_space, 57, 0)
        self.assertFalse(handled)                  # the application types the space itself
        self.assertEqual(e.committed, "আমি")
        self.assertEqual(e.preedit, "")

    def test_backspace_undoes_one_keystroke_then_goes_to_the_application(self):
        e = self.type_keys("dj")
        self.assertTrue(e.do_process_key_event(IBus.KEY_BackSpace, 14, 0))
        self.assertEqual(e.preedit, "ি")
        self.assertTrue(e.do_process_key_event(IBus.KEY_BackSpace, 14, 0))
        self.assertEqual(e.preedit, "")
        # Nothing of ours is left: the application deletes its own text.
        self.assertFalse(e.do_process_key_event(IBus.KEY_BackSpace, 14, 0))

    def test_shortcuts_pass_through_and_finish_the_syllable(self):
        e = self.type_keys("dj")
        handled = e.do_process_key_event(ord("a"), KEYCODE["a"],
                                         IBus.ModifierType.CONTROL_MASK)
        self.assertFalse(handled)                  # ⌃A selects all, as always
        self.assertEqual(e.committed, "কি")

    def test_key_release_is_ignored(self):
        e = self.engine
        self.assertFalse(e.do_process_key_event(ord("j"), KEYCODE["j"],
                                                IBus.ModifierType.RELEASE_MASK))
        self.assertEqual(e.preedit, "")

    def test_unknown_keycode_falls_back_to_the_key_value(self):
        e = self.engine
        self.assertTrue(e.do_process_key_event(ord("j"), 0, 0))   # no usable key code
        self.assertEqual(e.preedit, "ক")

    def test_non_bijoy_key_is_passed_through(self):
        e = self.engine
        self.assertFalse(e.do_process_key_event(ord("+"), 78, 0))
