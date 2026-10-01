"""The IBus layer: turns key events into Nukta Bangla text.

This is the Linux counterpart of ``Sources/NuktaInputMethod`` on macOS. IBus gives us a real
preedit (the underlined syllable in progress), so the syllable can be reordered freely while it is
being typed and only final text ever reaches the application — none of the rewrite-the-last-few-
characters work the macOS ``ClientWriter`` has to do is needed here.
"""

import gi

gi.require_version("IBus", "1.0")
from gi.repository import GLib, IBus  # noqa: E402

from . import keymap  # noqa: E402
from .engine import Engine  # noqa: E402

# Physical key → character on a US layout, so Bijoy works whatever XKB layout is underneath
# (the same reason the macOS build maps key codes rather than characters). Keys are evdev
# key codes, which is what IBus passes as `keycode`.
US_LAYOUT = {
    16: ("q", "Q"), 17: ("w", "W"), 18: ("e", "E"), 19: ("r", "R"), 20: ("t", "T"),
    21: ("y", "Y"), 22: ("u", "U"), 23: ("i", "I"), 24: ("o", "O"), 25: ("p", "P"),
    30: ("a", "A"), 31: ("s", "S"), 32: ("d", "D"), 33: ("f", "F"), 34: ("g", "G"),
    35: ("h", "H"), 36: ("j", "J"), 37: ("k", "K"), 38: ("l", "L"),
    44: ("z", "Z"), 45: ("x", "X"), 46: ("c", "C"), 47: ("v", "V"), 48: ("b", "B"),
    49: ("n", "N"), 50: ("m", "M"),
    2: ("1", "!"), 3: ("2", "@"), 4: ("3", "#"), 5: ("4", "$"), 6: ("5", "%"),
    7: ("6", "^"), 8: ("7", "&"), 9: ("8", "*"), 10: ("9", "("), 11: ("0", ")"),
    43: ("\\", "|"),
}

# Modifiers that mean the keystroke is a shortcut (⌃C, Alt-Tab, Super…), not typing.
SHORTCUT_MASK = (IBus.ModifierType.CONTROL_MASK
                 | IBus.ModifierType.MOD1_MASK      # Alt
                 | IBus.ModifierType.MOD4_MASK      # Super
                 | IBus.ModifierType.MOD5_MASK      # AltGr
                 | IBus.ModifierType.HYPER_MASK
                 | IBus.ModifierType.META_MASK)


class NuktaBanglaEngine(IBus.Engine):
    __gtype_name__ = "NuktaBanglaEngine"

    def __init__(self):
        super().__init__()
        self._engine = Engine()
        self._preedit = ""

    # MARK: key handling

    def do_process_key_event(self, keyval, keycode, state):
        if state & IBus.ModifierType.RELEASE_MASK:
            return False

        if state & SHORTCUT_MASK:
            self._flush()
            return False

        if keyval == IBus.KEY_BackSpace:
            out = self._engine.backspace()
            if out is None:
                return False  # nothing of ours under the cursor: let the application delete
            self._show(out)
            return True

        character = self._character(keyval, keycode, state)
        if character is None or keymap.key(character) is None:
            # Space, Enter, Tab, arrows, punctuation…: finish the syllable, let the app have the key.
            self._flush()
            return False

        self._show(self._engine.process_character(character))
        return True

    @staticmethod
    def _character(keyval, keycode, state):
        """The Bijoy key character this event stands for, or None.

        The physical key wins, so the layout underneath does not matter; the key value is the
        fallback for events that carry no usable key code (virtual keyboards, xdotool, remote
        sessions). Only Shift picks the shifted key — Caps Lock is ignored, as on macOS.
        """
        shift = bool(state & IBus.ModifierType.SHIFT_MASK)
        pair = US_LAYOUT.get(keycode)
        if pair is not None:
            return pair[1] if shift else pair[0]
        unicode_point = IBus.keyval_to_unicode(keyval)
        return unicode_point if unicode_point and len(unicode_point) == 1 else None

    # MARK: focus and state

    def do_focus_in(self):
        self._engine.reset()
        self._clear_preedit()

    def do_focus_in_id(self, object_path, client):  # ibus 1.5.27+
        self.do_focus_in()

    def do_focus_out(self):
        self._flush()

    def do_focus_out_id(self, object_path):  # ibus 1.5.27+
        self._flush()

    def do_reset(self):
        self._engine.reset()
        self._clear_preedit()

    def do_disable(self):
        self._flush()

    def do_enable(self):
        self._engine.reset()
        self._clear_preedit()

    # MARK: output

    def _show(self, out):
        """Puts finished text into the application and the syllable in progress in the preedit.

        The preedit shows `visible`, not `display`: a kar or ্ with no consonant to sit on yet
        (``প্রা`` then ে, waiting to see which letter it belongs to) is held back, exactly as the
        macOS build holds it back, instead of showing an orphan ``ে`` beside the last word. The key
        is not lost — it reappears as soon as the next letter decides where it goes.
        """
        if out.committed:
            self.commit_text(IBus.Text.new_from_string(out.committed))
        self._set_preedit(out.visible)

    def _flush(self):
        """Finishes the syllable in progress, as a click or a non-Bijoy key does."""
        out = self._engine.commit()
        if out.committed:
            self.commit_text(IBus.Text.new_from_string(out.committed))
        self._clear_preedit()

    def _set_preedit(self, text):
        self._preedit = text
        ibus_text = IBus.Text.new_from_string(text)
        if text:
            ibus_text.append_attribute(IBus.AttrType.UNDERLINE, IBus.AttrUnderline.SINGLE,
                                       0, len(text))
        self.update_preedit_text(ibus_text, len(text), bool(text))

    def _clear_preedit(self):
        self._preedit = ""
        self.update_preedit_text(IBus.Text.new_from_string(""), 0, False)


def run(standalone=False):
    """Registers the engine with IBus and runs until the bus goes away.

    `standalone` (``--standalone``) registers the component at run time, for testing the engine
    without installing its XML; IBus itself starts us with ``--ibus``.
    """
    IBus.init()
    bus = IBus.Bus()
    loop = GLib.MainLoop()
    bus.connect("disconnected", lambda *_: loop.quit())

    factory = IBus.Factory.new(bus.get_connection())
    factory.add_engine("nukta-bangla", NuktaBanglaEngine.__gtype__)

    if standalone:
        component = IBus.Component.new(
            "org.freedesktop.IBus.NuktaBangla", "নুকতা বাংলা (Nukta Bangla)", _version(),
            "© 2026 Nukta Solutions", "Nukta Solutions",
            "https://github.com/Nukta-Solutions/nukta-bangla-keyboard", "", "nukta-bangla")
        component.add_engine(IBus.EngineDesc.new(
            "nukta-bangla", "নুকতা বাংলা (Nukta Bangla)",
            "Bijoy Bangla keyboard layout, Unicode output", "bn",
            "© 2026 Nukta Solutions", "Nukta Solutions", "", "us"))
        bus.register_component(component)
        bus.set_global_engine_async("nukta-bangla", -1, None, None, None)
    else:
        bus.request_name("org.freedesktop.IBus.NuktaBangla", 0)

    loop.run()


def _version():
    from . import __version__
    return __version__
