"""A stand-in for the `gi` IBus bindings, so the key-event layer can be tested anywhere.

It implements only what nukta_bangla.ibus_engine touches: the modifier and key constants, text
objects, and an IBus.Engine base class that records what the engine committed and what it put in
the preedit.
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


class Engine:
    """Base class standing in for IBus.Engine; collects output instead of talking to a bus."""

    __gtype__ = object()

    def __init__(self):
        self.committed = ""
        self.preedit = ""
        self.preedit_visible = False

    def commit_text(self, text):
        self.committed += text.text

    def update_preedit_text(self, text, cursor, visible):
        self.preedit = text.text
        self.preedit_visible = visible

    def hide_preedit_text(self):
        self.preedit_visible = False


def keyval_to_unicode(keyval):
    return chr(keyval) if 0x20 <= keyval < 0x7F else ""


def install():
    """Puts the stubs in sys.modules; call before importing nukta_bangla.ibus_engine."""
    ibus = types.SimpleNamespace(
        ModifierType=ModifierType, AttrType=AttrType, AttrUnderline=AttrUnderline,
        Text=Text, Engine=Engine, keyval_to_unicode=keyval_to_unicode,
        KEY_BackSpace=0xFF08, KEY_Return=0xFF0D, KEY_space=0x20,
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
