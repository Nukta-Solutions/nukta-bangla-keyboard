"""The phonetic key-event layer: physical keys → committed text, preedit and suggestion list.

Runs against the stubs in fake_ibus (no IBus, no desktop session) but against real riti, so it is
skipped when libnukta_riti.so is not built. What it cannot cover is IBus itself and the panel it
draws; linux/README.md has the manual check for those.
"""

import os
import shutil
import sys
import tempfile
import unittest
import uuid

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import fake_ibus  # noqa: E402

IBus = fake_ibus.install()

from nukta_bangla import ibus_engine, riti  # noqa: E402
from nukta_bangla.ibus_phonetic import NuktaBanglaPhoneticEngine, PhoneticSession  # noqa: E402

KEYCODE = {char: code for code, pair in ibus_engine.US_LAYOUT.items() for char in pair}
SHIFTED = {pair[1] for pair in ibus_engine.US_LAYOUT.values()}
SHIFT_KEYVAL = 0xFFE1   # Shift_L
SHIFT_KEYCODE = 42
SPACE_KEYCODE = 57


def press(engine, char):
    """Types `char` as its physical key on a US keyboard, Shift press included."""
    keycode = KEYCODE[char]
    if char in SHIFTED:
        engine.do_process_key_event(SHIFT_KEYVAL, SHIFT_KEYCODE, 0)
        return engine.do_process_key_event(ord(char.lower()), keycode,
                                           IBus.ModifierType.SHIFT_MASK)
    return engine.do_process_key_event(ord(char.lower()), keycode, 0)


@unittest.skipUnless(riti.available(), "libnukta_riti.so is not built (linux/build_riti.sh)")
class PhoneticLayerTests(unittest.TestCase):
    def setUp(self):
        # Settings and learned picks go to a directory of this test's own, and the composer is a
        # process-wide singleton, so it is dropped between tests.
        self.home = os.path.join(tempfile.gettempdir(), f"nukta-ibus-tests-{uuid.uuid4()}")
        os.makedirs(self.home)
        self._environment = {key: os.environ.get(key)
                             for key in ("XDG_CONFIG_HOME", "XDG_DATA_HOME")}
        os.environ["XDG_CONFIG_HOME"] = os.path.join(self.home, "config")
        os.environ["XDG_DATA_HOME"] = os.path.join(self.home, "data")
        PhoneticSession._shared = None
        PhoneticSession._unavailable = None
        self.engine = NuktaBanglaPhoneticEngine()

    def tearDown(self):
        PhoneticSession._shared = None
        for key, value in self._environment.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value
        shutil.rmtree(self.home, ignore_errors=True)

    def type_keys(self, keys):
        for char in keys:
            press(self.engine, char)
        return self.engine

    def write_settings(self, text):
        directory = os.environ["XDG_CONFIG_HOME"] + "/nukta-bangla"
        os.makedirs(directory, exist_ok=True)
        with open(os.path.join(directory, "settings.json"), "w", encoding="utf-8") as handle:
            handle.write(text)

    # MARK: typing

    def test_the_word_is_in_the_preedit_with_the_list_beside_it(self):
        e = self.type_keys("sonar")
        self.assertEqual(e.preedit, "সনার")          # phonetic-first: what was typed
        self.assertEqual(e.committed, "")
        self.assertTrue(e.preedit_visible)
        self.assertIn("সোনার", e.candidates)
        self.assertEqual(e.selected, "সনার")
        self.assertEqual(e.auxiliary, "sonar")       # the roman word, above the list

    def test_space_commits_the_selected_word_and_goes_to_the_application(self):
        e = self.type_keys("ami")
        handled = e.do_process_key_event(IBus.KEY_space, SPACE_KEYCODE, 0)
        self.assertFalse(handled)                    # the application types the space itself
        self.assertEqual(e.committed, "আমি")
        self.assertEqual(e.preedit, "")
        self.assertEqual(e.candidates, [])

    def test_a_number_key_picks_a_candidate(self):
        e = self.type_keys("sonar")
        press(e, str(e.candidates.index("সোনার") + 1))
        self.assertEqual(e.committed, "সোনার")
        self.assertEqual(e.candidates, [])

    def test_enter_commits_without_a_new_line(self):
        e = self.type_keys("ami")
        self.assertTrue(e.do_process_key_event(IBus.KEY_Return, 28, 0))
        self.assertEqual(e.committed, "আমি")

    def test_escape_drops_the_word(self):
        e = self.type_keys("ami")
        self.assertTrue(e.do_process_key_event(IBus.KEY_Escape, 1, 0))
        self.assertEqual(e.committed, "")
        self.assertEqual(e.preedit, "")
        self.assertEqual(e.candidates, [])

    def test_arrow_keys_move_the_selection(self):
        e = self.type_keys("sonar")
        first = e.selected
        self.assertTrue(e.do_process_key_event(IBus.KEY_Down, 108, 0))
        self.assertNotEqual(e.selected, first)
        self.assertEqual(e.preedit, e.selected)      # the preedit is the selected candidate

    def test_backspace_edits_the_word_then_goes_to_the_application(self):
        e = self.type_keys("khi")
        # One roman letter at a time, as riti counts them: khi → kh → k → nothing.
        self.assertTrue(e.do_process_key_event(IBus.KEY_BackSpace, 14, 0))
        self.assertEqual(e.preedit, "খ")
        self.assertTrue(e.do_process_key_event(IBus.KEY_BackSpace, 14, 0))
        self.assertEqual(e.preedit, "ক")
        self.assertTrue(e.do_process_key_event(IBus.KEY_BackSpace, 14, 0))
        self.assertEqual(e.preedit, "")
        # Nothing of ours is left: the application deletes its own text.
        self.assertFalse(e.do_process_key_event(IBus.KEY_BackSpace, 14, 0))

    def test_ctrl_backspace_deletes_the_whole_word(self):
        e = self.type_keys("sonar")
        self.assertTrue(e.do_process_key_event(IBus.KEY_BackSpace, 14,
                                               IBus.ModifierType.CONTROL_MASK))
        self.assertEqual(e.preedit, "")
        self.assertEqual(e.committed, "")
        self.assertEqual(e.candidates, [])

    def test_shortcuts_pass_through_and_commit_the_word(self):
        e = self.type_keys("ami")
        handled = e.do_process_key_event(ord("a"), KEYCODE["a"],
                                         IBus.ModifierType.CONTROL_MASK)
        self.assertFalse(handled)                    # ⌃A selects all, as always
        self.assertEqual(e.committed, "আমি")

    def test_a_modifier_on_its_own_leaves_the_word_alone(self):
        e = self.type_keys("ami")
        self.assertFalse(e.do_process_key_event(SHIFT_KEYVAL, SHIFT_KEYCODE, 0))
        self.assertEqual(e.committed, "")
        self.assertEqual(e.preedit, "আমি")

    def test_key_release_is_ignored(self):
        e = self.engine
        self.assertFalse(e.do_process_key_event(ord("a"), KEYCODE["a"],
                                                IBus.ModifierType.RELEASE_MASK))
        self.assertEqual(e.preedit, "")

    def test_a_key_phonetic_typing_does_not_use_finishes_the_word(self):
        e = self.type_keys("ami")
        self.assertFalse(e.do_process_key_event(IBus.KEY_Home, 102, 0))
        self.assertEqual(e.committed, "আমি")

    def test_focus_out_commits_and_clears(self):
        e = self.type_keys("ami")
        e.do_focus_out()
        self.assertEqual(e.committed, "আমি")
        self.assertEqual(e.preedit, "")
        self.assertFalse(e.lookup_visible)

    def test_another_context_taking_over_commits_to_the_first_one(self):
        first = self.type_keys("ami")
        second = NuktaBanglaPhoneticEngine()
        second.do_focus_in()                         # the user clicked into another application
        self.assertEqual(first.committed, "আমি")     # the word went to the application it was typed in
        self.assertEqual(second.committed, "")

    # MARK: the list IBus draws

    def test_clicking_a_candidate_commits_it(self):
        e = self.type_keys("sonar")
        index = e.candidates.index("সোনার")
        e.do_candidate_clicked(index, 1, 0)
        self.assertEqual(e.committed, "সোনার")

    def test_the_list_is_laid_out_the_way_the_settings_ask(self):
        self.write_settings('{"popupDirection": "horizontal"}')
        self.engine.do_focus_in()
        e = self.type_keys("sonar")
        self.assertEqual(e.lookup_table.orientation, IBus.Orientation.HORIZONTAL)
        # A row: ← → move the selection too.
        first = e.selected
        self.assertTrue(e.do_process_key_event(IBus.KEY_Right, 106, 0))
        self.assertNotEqual(e.selected, first)

    def test_phonetic_only_mode_has_no_list(self):
        self.write_settings('{"typingMode": "phoneticOnly"}')
        self.engine.do_focus_in()
        e = self.type_keys("sonar")
        self.assertEqual(e.preedit, "সনার")
        self.assertEqual(e.candidates, [])

    def test_settings_are_re_read_when_the_file_changes(self):
        self.engine.do_focus_in()
        self.write_settings('{"typingMode": "smart"}')
        self.engine.do_focus_in()                    # the next focus change picks it up
        e = self.type_keys("sonar")
        self.assertEqual(e.selected, "সোনার")        # smart mode: riti's own choice

    # MARK: the layout menu

    def test_the_layout_menu_marks_the_layout_in_use(self):
        self.engine.do_focus_in()
        menu = self.engine.properties.properties[0]
        self.assertEqual(menu.type, IBus.PropType.MENU)
        states = {item.key: item.state for item in menu.sub_properties.properties}
        self.assertEqual(states["layout:nukta-bangla-phonetic"], IBus.PropState.CHECKED)
        self.assertEqual(states["layout:nukta-bangla"], IBus.PropState.UNCHECKED)

    def test_picking_the_other_layout_commits_the_word_and_switches(self):
        switched = []

        class FakeBus:
            def set_global_engine_async(self, name, *rest):
                switched.append(name)

        e = self.type_keys("ami")
        ibus_engine._bus = FakeBus()
        try:
            e.do_property_activate("layout:nukta-bangla", IBus.PropState.CHECKED)
        finally:
            ibus_engine._bus = None
        self.assertEqual(switched, ["nukta-bangla"])
        self.assertEqual(e.committed, "আমি")         # finished with the layout it was typed in


if __name__ == "__main__":
    unittest.main()
