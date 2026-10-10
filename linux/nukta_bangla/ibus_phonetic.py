"""The IBus layer for phonetic typing — the counterpart of ``Sources/NuktaInputMethod/Phonetic.swift``.

IBus draws the suggestion list itself (its lookup table), so there is no equivalent of the macOS
``CandidatePanel`` and ``CursorRect``: the desktop places the list and reports clicks on it.

One composer serves the whole process, as on macOS: riti is large, and every context rewrites
riti's file of learned picks from its own copy. Each application gets its own engine object, but
only one types at a time — the `owner`.
"""

import sys

import gi

gi.require_version("IBus", "1.0")
from gi.repository import IBus  # noqa: E402

from . import ibus_engine  # noqa: E402
from .phonetic import Key, PhoneticComposer  # noqa: E402
from .riti import RitiUnavailable  # noqa: E402
from .settings import Settings, data_directory  # noqa: E402

#: Candidates shown at once, and so the numbers 1–9 that pick them.
PAGE_SIZE = 9


class PhoneticSession:
    """The one composer, its settings, and which engine object is typing with it."""

    _shared = None
    _unavailable = None

    @classmethod
    def shared(cls):
        """The session, or None when riti is missing — phonetic typing cannot run then.

        Built on first use, so the Bijoy layout never loads riti.
        """
        if cls._shared is None and cls._unavailable is None:
            try:
                cls._shared = cls()
            except RitiUnavailable as error:
                cls._unavailable = error
                # IBus has nowhere to show a message; the journal is where an engine complains.
                print(f"নুকতা বাংলা: phonetic typing is unavailable — {error}", file=sys.stderr)
        return cls._shared

    def __init__(self):
        self.settings = Settings()
        self.composer = PhoneticComposer(self.settings.phonetic_options(), data_directory())
        self.composer.horizontal_navigation = self.settings.horizontal_popup
        self.owner = None

    def claim(self, engine):
        """Makes `engine` the one typing. A word left in progress by the previous owner is
        committed to its own application first."""
        if self.owner is engine:
            return
        if self.owner is not None:
            self.owner.phonetic_taken_over()
        self.composer.discard()
        self.owner = engine

    def release(self, engine):
        if self.owner is engine:
            self.owner = None

    def refresh_settings(self):
        """Re-reads the settings file. True when the word in progress was dropped with it."""
        if not self.settings.reload_if_changed():
            return False
        self.composer.horizontal_navigation = self.settings.horizontal_popup
        options = self.settings.phonetic_options()
        if options == self.composer.options:
            return False
        was_composing = self.composer.is_composing
        self.composer.update(options)
        return was_composing and not self.composer.is_composing


class NuktaBanglaPhoneticEngine(ibus_engine.LayoutMenu, IBus.Engine):
    """Avro Phonetic: roman letters in, a list of Bangla words to choose from."""

    __gtype_name__ = "NuktaBanglaPhoneticEngine"
    engine_name = "nukta-bangla-phonetic"

    def __init__(self):
        super().__init__()
        self._preedit = ""
        self._lookup_visible = False

    # MARK: key handling

    def do_process_key_event(self, keyval, keycode, state):
        if state & IBus.ModifierType.RELEASE_MASK:
            return False
        if keyval in ibus_engine.MODIFIER_KEYVALS:
            return False       # a modifier alone: leave the word exactly as it is

        session = PhoneticSession.shared()
        if session is None:
            return False       # no riti: every key belongs to the application
        session.claim(self)
        composer = session.composer

        # Shortcuts go to the application; Ctrl+⌫ and Alt+⌫ (delete the whole word) are ours,
        # as ⌥⌫ is on macOS.
        whole_word = (keyval == IBus.KEY_BackSpace
                      and bool(state & (IBus.ModifierType.CONTROL_MASK
                                        | IBus.ModifierType.MOD1_MASK)))
        if state & ibus_engine.SHORTCUT_MASK and not whole_word:
            self._apply(composer.commit())
            return False

        key = self._key(keyval, keycode, state, whole_word)
        if key is None:
            # F-keys, Home, Delete…: finish the word and let the application have the key.
            self._apply(composer.commit())
            return False

        out = composer.handle(key)
        self._apply(out)
        return out.handled

    @classmethod
    def _key(cls, keyval, keycode, state, whole_word):
        """The composer key this event stands for, or None for a key phonetic typing ignores."""
        shift = bool(state & IBus.ModifierType.SHIFT_MASK)
        special = {
            IBus.KEY_Return: Key.ENTER, IBus.KEY_KP_Enter: Key.ENTER,
            IBus.KEY_Escape: Key.ESCAPE,
            IBus.KEY_space: Key.SPACE, IBus.KEY_KP_Space: Key.SPACE,
            IBus.KEY_Up: Key.UP, IBus.KEY_KP_Up: Key.UP,
            IBus.KEY_Down: Key.DOWN, IBus.KEY_KP_Down: Key.DOWN,
            IBus.KEY_Left: Key.LEFT, IBus.KEY_KP_Left: Key.LEFT,
            IBus.KEY_Right: Key.RIGHT, IBus.KEY_KP_Right: Key.RIGHT,
        }
        if keyval == IBus.KEY_BackSpace:
            return Key.backspace(whole_word=whole_word)
        if keyval in (IBus.KEY_Tab, IBus.KEY_ISO_Left_Tab, IBus.KEY_KP_Tab):
            return Key.tab(backward=shift or keyval == IBus.KEY_ISO_Left_Tab)
        if keyval in special:
            return special[keyval]
        character = ibus_engine.us_character(keyval, keycode, state)
        return None if character is None else Key.printable(character, shift=shift)

    # MARK: the suggestion list

    def do_candidate_clicked(self, index, button, state):
        """A candidate was clicked. `index` counts from the top of the page on screen."""
        session = PhoneticSession.shared()
        if session is None:
            return
        composer = session.composer
        page = (composer.selected_index // PAGE_SIZE) * PAGE_SIZE
        self._apply(composer.commit_at(page + index))

    def do_cursor_up(self):
        self._move(-1)

    def do_cursor_down(self):
        self._move(1)

    def do_page_up(self):
        self._move(-PAGE_SIZE)

    def do_page_down(self):
        self._move(PAGE_SIZE)

    def _move(self, step):
        session = PhoneticSession.shared()
        if session is None or not session.composer.candidates:
            return
        composer = session.composer
        if abs(step) == 1:
            self._apply(composer.handle(Key.DOWN if step > 0 else Key.UP))
            return
        # A whole page (the panel's own page buttons): the composer only knows single steps, and
        # a page is clamped to the ends of the list rather than wrapping around it.
        target = min(max(composer.selected_index + step, 0), len(composer.candidates) - 1)
        key = Key.DOWN if target > composer.selected_index else Key.UP
        out = None
        for _ in range(abs(target - composer.selected_index)):
            out = composer.handle(key)
        if out is not None:
            self._apply(out)

    # MARK: focus and state

    def do_focus_in(self):
        session = PhoneticSession.shared()
        if session is None:
            return
        # Nothing can be in flight for a context that is only now becoming active.
        session.claim(self)
        session.refresh_settings()
        session.composer.discard()
        self._clear()
        self.register_properties(self.layout_properties())

    def do_focus_in_id(self, object_path, client):  # ibus 1.5.27+
        self.do_focus_in()

    def do_property_activate(self, name, state):
        # In the engine's own body, not the mixin's: see `ibus_engine.LayoutMenu`.
        self.switch_layout(name, state)

    def do_focus_out(self):
        self._flush()

    def do_focus_out_id(self, object_path):  # ibus 1.5.27+
        self._flush()

    def do_reset(self):
        session = PhoneticSession.shared()
        if session is not None:
            session.composer.discard()
        self._clear()

    def do_enable(self):
        self.do_focus_in()

    def do_disable(self):
        self._flush()
        session = PhoneticSession.shared()
        if session is not None:
            session.release(self)

    def phonetic_taken_over(self):
        """Another application took over phonetic typing while this one had a word in progress."""
        session = PhoneticSession.shared()
        if session is not None and session.composer.is_composing:
            self._apply(session.composer.commit())

    # MARK: output

    def _flush(self):
        """Finishes the word in progress, as a click elsewhere or an application switch does."""
        session = PhoneticSession.shared()
        if session is not None and session.owner is self:
            self._apply(session.composer.commit())
        self._clear()

    def _apply(self, out):
        """Puts a result into the application: committed text, preedit, suggestion list."""
        if out.insert:
            self.commit_text(IBus.Text.new_from_string(out.insert))
        self._set_preedit(out.marked)
        self._show_candidates()

    def _set_preedit(self, text):
        self._preedit = text
        ibus_text = IBus.Text.new_from_string(text)
        if text:
            ibus_text.append_attribute(IBus.AttrType.UNDERLINE, IBus.AttrUnderline.SINGLE,
                                       0, len(text))
        self.update_preedit_text(ibus_text, len(text), bool(text))

    def _show_candidates(self):
        session = PhoneticSession.shared()
        composer = None if session is None else session.composer
        if composer is None or not composer.candidates:
            self.update_auxiliary_text(IBus.Text.new_from_string(""), False)
            if self._lookup_visible:
                self.hide_lookup_table()
                self._lookup_visible = False
            return

        table = IBus.LookupTable.new(PAGE_SIZE, 0, True, True)
        table.set_orientation(IBus.Orientation.HORIZONTAL if session.settings.horizontal_popup
                              else IBus.Orientation.VERTICAL)
        for candidate in composer.candidates:
            table.append_candidate(IBus.Text.new_from_string(candidate))
        table.set_cursor_pos(composer.selected_index)
        self.update_lookup_table(table, True)
        self._lookup_visible = True
        # What was typed, above the list, the way the macOS panel shows it.
        self.update_auxiliary_text(IBus.Text.new_from_string(composer.auxiliary),
                                   bool(composer.auxiliary))

    def _clear(self):
        self._set_preedit("")
        self.update_auxiliary_text(IBus.Text.new_from_string(""), False)
        if self._lookup_visible:
            self.hide_lookup_table()
            self._lookup_visible = False
