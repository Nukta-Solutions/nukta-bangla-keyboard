"""The IBus layer: turns key events into Nukta Bangla text.

This is the Linux counterpart of ``Sources/NuktaInputMethod`` on macOS. IBus gives us a real
preedit (the underlined syllable in progress), so the syllable can be reordered freely while it is
being typed and only final text ever reaches the application — none of the rewrite-the-last-few-
characters work the macOS ``ClientWriter`` has to do is needed here.

Bijoy lives here; phonetic typing is ``ibus_phonetic.py``, registered as a second engine so a
Linux user switches layouts with Super+Space like any other input source. Both engines also carry
the layout menu below, the counterpart of the macOS **নু** menu.
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

def us_character(keyval, keycode, state):
    """The character this key event stands for on a US layout, or None.

    Both layouts read keys this way. The physical key wins, so the XKB layout underneath does not
    matter; the key value is the fallback for events that carry no usable key code (virtual
    keyboards, xdotool, remote sessions). Only Shift picks the shifted key — Caps Lock is ignored,
    as on macOS.
    """
    shift = bool(state & IBus.ModifierType.SHIFT_MASK)
    pair = US_LAYOUT.get(keycode)
    if pair is not None:
        return pair[1] if shift else pair[0]
    unicode_point = IBus.keyval_to_unicode(keyval)
    return unicode_point if unicode_point and len(unicode_point) == 1 else None


# A modifier pressed on its own. IBus sends these as key events too, and they must change
# nothing: Shift on its way to a capital letter (V for ল, B for ণ) used to finish the syllable
# and push a pending kar out before its consonant — প্রাণে came out as প্রােণ.
MODIFIER_KEYVALS = frozenset((
    0xFFE1, 0xFFE2,          # Shift_L, Shift_R
    0xFFE3, 0xFFE4,          # Control_L, Control_R
    0xFFE5, 0xFFE6,          # Caps_Lock, Shift_Lock
    0xFFE7, 0xFFE8,          # Meta_L, Meta_R
    0xFFE9, 0xFFEA,          # Alt_L, Alt_R
    0xFFEB, 0xFFEC,          # Super_L, Super_R
    0xFFED, 0xFFEE,          # Hyper_L, Hyper_R
    0xFF7E, 0xFF7F,          # Mode_switch, Num_Lock
    0xFF14,                  # Scroll_Lock
    0xFE03,                  # ISO_Level3_Shift (AltGr)
))

# Modifiers that mean the keystroke is a shortcut (⌃C, Alt-Tab, Super…), not typing.
SHORTCUT_MASK = (IBus.ModifierType.CONTROL_MASK
                 | IBus.ModifierType.MOD1_MASK      # Alt
                 | IBus.ModifierType.MOD4_MASK      # Super
                 | IBus.ModifierType.MOD5_MASK      # AltGr
                 | IBus.ModifierType.HYPER_MASK
                 | IBus.ModifierType.META_MASK)


#: The two engines, in the order the layout menu lists them. Each is an input source of its own
#: in the desktop's keyboard settings; the menu switches between them without going there.
LAYOUTS = (
    ("nukta-bangla", "বিজয় লেআউট"),
    ("nukta-bangla-phonetic", "ফোনেটিক"),
)

#: The bus this process is connected to, set by `run`. Switching layout from the menu asks IBus to
#: change the global engine, which only the bus can do.
_bus = None


class LayoutMenu:
    """The layout menu both engines show, the counterpart of the macOS **নু** menu.

    IBus calls it a property list. Desktops differ in how much of one they display — GNOME shows
    it in the top bar, some desktops not at all — so it is a convenience, never the only way:
    both layouts are input sources in their own right.

    Each engine declares its own ``do_property_activate`` and calls `switch_layout` from it:
    PyGObject hooks up a ``do_*`` method only where it is written in the class's own body, not one
    inherited from a plain mixin like this one, and a vfunc it does not hook up is never called.
    """

    #: The IBus engine name of the layout this class is, set by each engine.
    engine_name = ""

    def layout_properties(self):
        # The icon and tooltip are empty strings rather than None: the bindings take no None here.
        blank = IBus.Text.new_from_string("")
        items = IBus.PropList()
        for name, label in LAYOUTS:
            items.append(IBus.Property.new(
                f"layout:{name}", IBus.PropType.RADIO, IBus.Text.new_from_string(label),
                "", blank, True, True,
                IBus.PropState.CHECKED if name == self.engine_name else IBus.PropState.UNCHECKED,
                None))
        menu = IBus.Property.new("layout", IBus.PropType.MENU,
                                 IBus.Text.new_from_string("লেআউট"), "",
                                 IBus.Text.new_from_string("নুকতা বাংলা: কীবোর্ড লেআউট"),
                                 True, True, IBus.PropState.UNCHECKED, items)
        properties = IBus.PropList()
        properties.append(menu)
        return properties

    def switch_layout(self, name, state):
        """A menu item was chosen: switch to that layout, or ignore anything else."""
        if not name.startswith("layout:"):
            return
        target = name[len("layout:"):]
        if target == self.engine_name or _bus is None:
            return
        self._flush()           # finish the word with the layout it was typed in
        _bus.set_global_engine_async(target, -1, None, None, None)


class NuktaBanglaEngine(LayoutMenu, IBus.Engine):
    __gtype_name__ = "NuktaBanglaEngine"
    engine_name = "nukta-bangla"

    def __init__(self):
        super().__init__()
        self._engine = Engine()
        self._preedit = ""

    # MARK: key handling

    def do_process_key_event(self, keyval, keycode, state):
        if state & IBus.ModifierType.RELEASE_MASK:
            return False

        if keyval in MODIFIER_KEYVALS:
            return False       # a modifier alone: leave the syllable exactly as it is

        if state & SHORTCUT_MASK:
            self._flush()
            return False

        if keyval == IBus.KEY_BackSpace:
            out = self._engine.backspace()
            if out is None:
                return False  # nothing of ours under the cursor: let the application delete
            self._show(out)
            return True

        character = us_character(keyval, keycode, state)
        if character is None or keymap.key(character) is None:
            # Space, Enter, Tab, arrows, punctuation…: finish the syllable, let the app have the key.
            self._flush()
            return False

        self._show(self._engine.process_character(character))
        return True

    # MARK: focus and state

    def do_focus_in(self):
        self._engine.reset()
        self._clear_preedit()
        self.register_properties(self.layout_properties())

    def do_property_activate(self, name, state):
        self.switch_layout(name, state)

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
    """Registers the engines with IBus and runs until the bus goes away.

    `standalone` (``--standalone``) registers the component at run time, for testing the engines
    without installing their XML; IBus itself starts us with ``--ibus``.
    """
    global _bus
    IBus.init()
    bus = IBus.Bus()
    _bus = bus
    loop = GLib.MainLoop()
    bus.connect("disconnected", lambda *_: loop.quit())

    # Imported here, not at the top: it loads riti, and the Bijoy engine must not need it.
    from .ibus_phonetic import NuktaBanglaPhoneticEngine

    factory = IBus.Factory.new(bus.get_connection())
    factory.add_engine("nukta-bangla", NuktaBanglaEngine.__gtype__)
    factory.add_engine("nukta-bangla-phonetic", NuktaBanglaPhoneticEngine.__gtype__)

    if standalone:
        component = IBus.Component.new(
            "org.freedesktop.IBus.NuktaBangla", "নুকতা বাংলা (Nukta Bangla)", _version(),
            "MIT", "Nukta Solutions",
            "https://github.com/Nukta-Solutions/nukta-bangla-keyboard", "", "nukta-bangla")
        for name, longname, description in (
            # The longnames are what desktops show; keep them the same as nukta-bangla.xml.in.
            ("nukta-bangla", "নুকতা বাংলা - বিজয়",
             "Bijoy Bangla keyboard layout, Unicode output"),
            ("nukta-bangla-phonetic", "নুকতা বাংলা - ফোনেটিক",
             "Avro Phonetic: type Bangla with roman letters"),
        ):
            component.add_engine(IBus.EngineDesc.new(
                name, longname, description, "bn", "MIT", "Nukta Solutions", "", "us"))
        bus.register_component(component)
        bus.set_global_engine_async("nukta-bangla", -1, None, None, None)
    else:
        bus.request_name("org.freedesktop.IBus.NuktaBangla", 0)

    loop.run()


def _version():
    from . import __version__
    return __version__
