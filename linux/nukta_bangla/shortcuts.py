"""The user's own shortcuts: a short roman code that types a longer text (``asw`` → আসসালামু আলাইকুম).

The rules and the file live once, in riti-bridge (``riti-bridge/src/shortcuts.rs``), shared with
macOS and Windows; this is their C functions behind a Python interface. The file is
``shortcuts.json`` in the data directory, the same on every platform.
"""

import ctypes
import json
import os
import subprocess
import sys

from .riti import library

#: What `Shortcuts.set` returns: saved, or why not (the numbers riti-bridge's C functions return).
SAVED, BAD_CODE, EMPTY_TEXT, TEXT_TOO_LONG, NOT_SAVED = range(5)

MAX_CODE = 12
MAX_TEXT = 300


def _lib():
    """The library with the shortcut functions declared (once)."""
    lib = library()
    if not getattr(lib, "_nukta_shortcuts_declared", False):
        signatures = {
            "nukta_shortcuts_new": ([ctypes.c_char_p], ctypes.c_void_p),
            "nukta_shortcuts_free": ([ctypes.c_void_p], None),
            "nukta_shortcuts_text": ([ctypes.c_void_p, ctypes.c_char_p], ctypes.c_void_p),
            "nukta_shortcuts_json": ([ctypes.c_void_p], ctypes.c_void_p),
            "nukta_shortcuts_set": ([ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p], ctypes.c_int32),
            "nukta_shortcuts_remove": ([ctypes.c_void_p, ctypes.c_char_p], ctypes.c_bool),
            "nukta_dictionary_word": ([ctypes.c_char_p], ctypes.c_void_p),
            "nukta_string_free": ([ctypes.c_void_p], None),
        }
        for name, (argtypes, restype) in signatures.items():
            function = getattr(lib, name)
            function.argtypes = argtypes
            function.restype = restype
        lib._nukta_shortcuts_declared = True
    return lib


def _take(pointer):
    """A string one of the functions returned, as Python text; NULL is None. Freed here."""
    if not pointer:
        return None
    lib = _lib()
    try:
        return ctypes.string_at(pointer).decode("utf-8")
    finally:
        lib.nukta_string_free(pointer)


def dictionary_word(code):
    """A dictionary word the code also spells (``ami`` → আমি), else None: as a shortcut it would
    come first every time that word is typed, so the dialog warns before saving it."""
    return _take(_lib().nukta_dictionary_word(code.encode("utf-8")))


class Shortcuts:
    """The shortcuts in a data directory. Several processes share the file: each change re-reads
    it first, and a lookup re-reads it when another process has written it since."""

    def __init__(self, directory):
        self._lib = _lib()
        self._handle = self._lib.nukta_shortcuts_new(directory.encode("utf-8"))

    def text_for(self, typed):
        """The text for `typed` (roman, as typed) if it is a whole code; any case."""
        if not self._handle:
            return None
        return _take(self._lib.nukta_shortcuts_text(self._handle, typed.encode("utf-8")))

    def all(self):
        """Every shortcut, code → text, ordered by code."""
        if not self._handle:
            return {}
        return json.loads(_take(self._lib.nukta_shortcuts_json(self._handle)) or "{}")

    def set(self, code, text):
        """Adds a shortcut, or changes an existing code's text: `SAVED`, or why not."""
        if not self._handle:
            return NOT_SAVED
        return self._lib.nukta_shortcuts_set(self._handle, code.encode("utf-8"), text.encode("utf-8"))

    def remove(self, code):
        """True if the code was there."""
        return bool(self._handle) and self._lib.nukta_shortcuts_remove(self._handle, code.encode("utf-8"))

    def close(self):
        if self._handle:
            self._lib.nukta_shortcuts_free(self._handle)
            self._handle = None

    def __del__(self):
        try:
            self.close()
        except Exception:  # interpreter shutting down
            pass


def open_dialog():
    """Runs the shortcuts dialog (shortcuts_dialog.py) on its own, so IBus never waits for a
    window, and the engine never loads GTK."""
    package_parent = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    environment = dict(os.environ)
    environment["PYTHONPATH"] = os.pathsep.join(
        p for p in (package_parent, environment.get("PYTHONPATH")) if p)
    try:
        subprocess.Popen([sys.executable, "-m", "nukta_bangla.shortcuts_dialog"],
                         env=environment, cwd=package_parent, start_new_session=True,
                         stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    except OSError as error:
        print(f"নুকতা বাংলা: cannot open the shortcuts dialog — {error}", file=sys.stderr)
