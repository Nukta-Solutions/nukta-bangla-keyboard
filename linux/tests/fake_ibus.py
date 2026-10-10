"""A stand-in for the `gi` IBus bindings, so the key-event layer can be tested anywhere.

It implements only what nukta_bangla.ibus_engine and nukta_bangla.ibus_phonetic touch: the
modifier and key constants, text, property and lookup-table objects, and an IBus.Engine base class
that records what the engine committed, what it put in the preedit, and the suggestion list it
showed.
"""

import sys
import types


class ModifierType:
    SHIFT_MASK = 1 << 0
    LOCK_MASK = 1 << 1
    CONTROL_MASK = 1 << 2
    MOD1_MASK = 1 << 3
    MOD4_MASK = 1 << 6
    MOD5_MASK = 1 << 7
    RELEASE_MASK = 1 << 30
    HYPER_MASK = 1 << 27
    META_MASK = 1 << 28


class PropType:
    NORMAL = 0
    TOGGLE = 1
    RADIO = 2
    MENU = 3
    SEPARATOR = 4


class PropState:
    UNCHECKED = 0
    CHECKED = 1
    INCONSISTENT = 2


class Orientation:
    HORIZONTAL = 0
    VERTICAL = 1
    SYSTEM = 2


class AttrType:
    UNDERLINE = 1


class AttrUnderline:
    SINGLE = 1


class Text:
    def __init__(self, text):
        self.text = text
        self.attributes = []

    @classmethod
    def new_from_string(cls, text):
        return cls(text)

    def append_attribute(self, *args):
        self.attributes.append(args)


class Property:
    """One item of the layout menu."""

    def __init__(self, key, type_, label, icon, tooltip, sensitive, visible, state, prop_list):
        self.key = key
        self.type = type_
        self.label = label
        self.state = state
        self.sub_properties = prop_list

    @classmethod
    def new(cls, key, type_, label, icon, tooltip, sensitive, visible, state, prop_list):
        return cls(key, type_, label, icon, tooltip, sensitive, visible, state, prop_list)


class PropList:
    def __init__(self):
        self.properties = []

    def append(self, prop):
        self.properties.append(prop)


class LookupTable:
    """The suggestion list IBus draws, standing in for IBus.LookupTable."""

    def __init__(self, page_size, cursor_pos, cursor_visible, round_):
        self.page_size = page_size
        self.cursor_pos = cursor_pos
        self.candidates = []
        self.orientation = Orientation.VERTICAL

    @classmethod
    def new(cls, page_size, cursor_pos, cursor_visible, round_):
        return cls(page_size, cursor_pos, cursor_visible, round_)

    def append_candidate(self, text):
        self.candidates.append(text.text)

    def set_cursor_pos(self, position):
        self.cursor_pos = position

    def set_orientation(self, orientation):
        self.orientation = orientation


class Engine:
    """Base class standing in for IBus.Engine; collects output instead of talking to a bus."""

    __gtype__ = object()

    def __init__(self):
        self.committed = ""
        self.preedit = ""
        self.preedit_visible = False
        self.auxiliary = ""
        self.auxiliary_visible = False
        self.lookup_table = None
        self.lookup_visible = False
        self.properties = None

    def commit_text(self, text):
        self.committed += text.text

    def update_preedit_text(self, text, cursor, visible):
        self.preedit = text.text
        self.preedit_visible = visible

    def hide_preedit_text(self):
        self.preedit_visible = False

    def update_auxiliary_text(self, text, visible):
        self.auxiliary = text.text
        self.auxiliary_visible = visible

    def hide_auxiliary_text(self):
        self.auxiliary_visible = False

    def update_lookup_table(self, table, visible):
        self.lookup_table = table
        self.lookup_visible = visible

    def hide_lookup_table(self):
        self.lookup_visible = False

    def register_properties(self, properties):
        self.properties = properties

    @property
    def candidates(self):
        """The suggestions on screen, or [] when no list is showing."""
        if not self.lookup_visible or self.lookup_table is None:
            return []
        return list(self.lookup_table.candidates)

    @property
    def selected(self):
        return None if not self.candidates else self.candidates[self.lookup_table.cursor_pos]


def keyval_to_unicode(keyval):
    return chr(keyval) if 0x20 <= keyval < 0x7F else ""


def install():
    """Puts the stubs in sys.modules; call before importing nukta_bangla.ibus_engine."""
    ibus = types.SimpleNamespace(
        ModifierType=ModifierType, AttrType=AttrType, AttrUnderline=AttrUnderline,
        PropType=PropType, PropState=PropState, Orientation=Orientation,
        Property=Property, PropList=PropList, LookupTable=LookupTable,
        Text=Text, Engine=Engine, keyval_to_unicode=keyval_to_unicode,
        KEY_BackSpace=0xFF08, KEY_Return=0xFF0D, KEY_space=0x20, KEY_Escape=0xFF1B,
        KEY_Tab=0xFF09, KEY_ISO_Left_Tab=0xFE20, KEY_KP_Tab=0xFF89, KEY_KP_Enter=0xFF8D,
        KEY_KP_Space=0xFF80, KEY_Delete=0xFFFF, KEY_Home=0xFF50,
        KEY_Left=0xFF51, KEY_Up=0xFF52, KEY_Right=0xFF53, KEY_Down=0xFF54,
        KEY_KP_Left=0xFF96, KEY_KP_Up=0xFF97, KEY_KP_Right=0xFF98, KEY_KP_Down=0xFF99,
        init=lambda: None,
    )
    repository = types.ModuleType("gi.repository")
    repository.IBus = ibus
    repository.GLib = types.SimpleNamespace(MainLoop=lambda: None)
    gi = types.ModuleType("gi")
    gi.require_version = lambda *args: None
    gi.repository = repository
    sys.modules["gi"] = gi
    sys.modules["gi.repository"] = repository
    return ibus
