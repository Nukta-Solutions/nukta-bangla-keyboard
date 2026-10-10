"""riti (Avro Phonetic) behind a Python interface — a port of ``Sources/NuktaPhonetic/Riti.swift``.

riti is OpenBangla's phonetic engine, a Rust library with a C API. The macOS app links it
statically; here the engine is Python, so it is loaded at run time from
``libnukta_riti.so`` (built by ``linux/build_riti.sh``) with ctypes.

riti panics — and so would take the whole engine process down — if it is used the wrong way, so
every call is made the way riti expects: a lonely suggestion is never asked for list details, and
everything riti hands out is freed here.
"""

import ctypes
import json
import os
import time

MODIFIER_SHIFT = 1 << 0

#: riti's learned picks and the user's autocorrect entries, inside the user directory.
USER_FILES = ("phonetic-candidate-selection.json", "autocorrect.json")


class RitiUnavailable(Exception):
    """libnukta_riti.so is not installed, so phonetic typing cannot run.

    The Bijoy layout never raises this: it is pure Python.
    """


def _candidate_paths():
    """Where libnukta_riti.so may be, best first.

    An installed engine has it in ``lib/`` beside ``nukta_bangla/``; a clone has it where
    ``linux/build_riti.sh`` puts it. ``NUKTA_RITI_LIB`` overrides both, for trying a build.
    """
    override = os.environ.get("NUKTA_RITI_LIB")
    if override:
        return [override]
    here = os.path.dirname(os.path.abspath(__file__))
    return [
        os.path.join(here, os.pardir, "lib", "libnukta_riti.so"),      # installed, and packages
        os.path.join(here, os.pardir, os.pardir, "riti-bridge", "lib", "libnukta_riti.so"),  # clone
        "libnukta_riti.so",                                            # on the loader's path
    ]


_library = None


def library():
    """The loaded library, or raises `RitiUnavailable`. Loaded once per process."""
    global _library
    if _library is None:
        _library = _load()
    return _library


def available():
    """True when phonetic typing can run on this machine."""
    try:
        library()
        return True
    except RitiUnavailable:
        return False


def _load():
    tried = []
    for path in _candidate_paths():
        try:
            return _declare(ctypes.CDLL(path))
        except OSError as error:
            tried.append(f"{path}: {error}")
    raise RitiUnavailable(
        "libnukta_riti.so not found — phonetic typing needs it. Build it with "
        "linux/build_riti.sh (needs Rust), or install a package that carries it.\n  "
        + "\n  ".join(tried))


def _declare(lib):
    """Gives ctypes the signatures from riti-bridge/riti/include/riti.h.

    Pointers are `c_void_p`, and the strings riti returns are `c_void_p` too rather than
    `c_char_p`: ctypes turns a `c_char_p` result into Python bytes and loses the pointer, which
    `riti_string_free` still needs.
    """
    signatures = {
        "riti_config_new": ([], ctypes.c_void_p),
        "riti_config_free": ([ctypes.c_void_p], None),
        "riti_config_set_layout_file": ([ctypes.c_void_p, ctypes.c_char_p], ctypes.c_bool),
        "riti_config_set_user_dir": ([ctypes.c_void_p, ctypes.c_char_p], ctypes.c_bool),
        "riti_config_set_phonetic_suggestion": ([ctypes.c_void_p, ctypes.c_bool], None),
        "riti_config_set_suggestion_include_english": ([ctypes.c_void_p, ctypes.c_bool], None),
        "riti_config_set_autocorrect": ([ctypes.c_void_p, ctypes.c_bool], None),
        "riti_context_new_with_config": ([ctypes.c_void_p], ctypes.c_void_p),
        "riti_context_free": ([ctypes.c_void_p], None),
        "riti_get_suggestion_for_key": ([ctypes.c_void_p, ctypes.c_uint16, ctypes.c_uint8,
                                         ctypes.c_uint8], ctypes.c_void_p),
        "riti_context_backspace_event": ([ctypes.c_void_p, ctypes.c_bool], ctypes.c_void_p),
        "riti_context_ongoing_input_session": ([ctypes.c_void_p], ctypes.c_bool),
        "riti_context_finish_input_session": ([ctypes.c_void_p], None),
        "riti_context_candidate_committed": ([ctypes.c_void_p, ctypes.c_size_t], None),
        "riti_suggestion_free": ([ctypes.c_void_p], None),
        "riti_suggestion_is_empty": ([ctypes.c_void_p], ctypes.c_bool),
        "riti_suggestion_is_lonely": ([ctypes.c_void_p], ctypes.c_bool),
        "riti_suggestion_get_lonely_suggestion": ([ctypes.c_void_p], ctypes.c_void_p),
        "riti_suggestion_get_suggestion": ([ctypes.c_void_p, ctypes.c_size_t], ctypes.c_void_p),
        "riti_suggestion_get_auxiliary_text": ([ctypes.c_void_p], ctypes.c_void_p),
        "riti_suggestion_previously_selected_index": ([ctypes.c_void_p], ctypes.c_size_t),
        "riti_suggestion_get_length": ([ctypes.c_void_p], ctypes.c_size_t),
        "riti_string_free": ([ctypes.c_void_p], None),
        # The bridge's own function (riti-bridge/src/lib.rs), not riti's.
        "nukta_riti_keycode": ([ctypes.c_uint32], ctypes.c_uint16),
    }
    for name, (argtypes, restype) in signatures.items():
        function = getattr(lib, name)          # AttributeError here = an old or wrong library
        function.argtypes = argtypes
        function.restype = restype
    return lib


def keycode(character):
    """riti's keycode for a character typed on a US layout, or None if riti has no key for it."""
    if len(character) != 1:
        return None
    code = library().nukta_riti_keycode(ord(character))
    return code or None


class Suggestion:
    """One riti answer, copied out of riti so nothing points into its memory afterwards.

    ``kind`` is ``"empty"``, ``"lonely"`` (a single string and no list: phonetic-only mode, or a
    lone punctuation mark) or ``"list"``. ``selection`` is riti's default pick — a past choice, or
    the selection passed in for punctuation.
    """

    __slots__ = ("kind", "text", "entries", "auxiliary", "selection")

    def __init__(self, kind, text="", entries=(), auxiliary="", selection=0):
        self.kind = kind
        self.text = text
        self.entries = list(entries)
        self.auxiliary = auxiliary
        self.selection = selection

    @property
    def is_empty(self):
        return self.kind == "empty"

    @property
    def is_lonely(self):
        return self.kind == "lonely"

    @property
    def is_list(self):
        return self.kind == "list"

    def __eq__(self, other):
        return (isinstance(other, Suggestion)
                and (self.kind, self.text, self.entries, self.auxiliary, self.selection)
                == (other.kind, other.text, other.entries, other.auxiliary, other.selection))

    def __repr__(self):
        if self.kind == "empty":
            return "Suggestion(empty)"
        if self.kind == "lonely":
            return f"Suggestion(lonely={self.text!r})"
        return (f"Suggestion(list={self.entries!r}, auxiliary={self.auxiliary!r}, "
                f"selection={self.selection})")


EMPTY = Suggestion("empty")


class RitiContext:
    """A riti context for Avro Phonetic.

    ``directory`` must exist; riti reads and writes its user files there. ``suggestions`` false
    gives lonely transliterations only.
    """

    def __init__(self, directory, suggestions, english_word, autocorrect):
        self._lib = library()
        self._context = None
        set_aside_bad_files(directory)
        config = self._lib.riti_config_new()
        if not config:
            raise RitiUnavailable("riti would not make a configuration")
        try:
            self._lib.riti_config_set_layout_file(config, b"avro_phonetic")
            self._lib.riti_config_set_user_dir(config, os.fsencode(directory))
            self._lib.riti_config_set_phonetic_suggestion(config, suggestions)
            self._lib.riti_config_set_suggestion_include_english(config, english_word)
            self._lib.riti_config_set_autocorrect(config, autocorrect)
            # The context keeps its own copy of the config.
            self._context = self._lib.riti_context_new_with_config(config)
        finally:
            self._lib.riti_config_free(config)
        if not self._context:
            raise RitiUnavailable("riti would not make a context")

    def key(self, code, shift=False, selection=0):
        """Feeds one key.

        ``selection`` is the riti index of the entry the user has selected; riti keeps it as the
        default when the key is a punctuation mark.
        """
        modifier = MODIFIER_SHIFT if shift else 0
        index = min(max(selection, 0), 255)
        return self._take(self._lib.riti_get_suggestion_for_key(
            self._context, code, modifier, index))

    def backspace(self, whole_word):
        """Deletes the last letter, or the whole word. An empty result means the word is gone."""
        return self._take(self._lib.riti_context_backspace_event(self._context, whole_word))

    @property
    def is_composing(self):
        return bool(self._lib.riti_context_ongoing_input_session(self._context))

    def committed(self, index):
        """Ends the word, letting riti learn that `index` was picked."""
        self._lib.riti_context_candidate_committed(self._context, max(index, 0))

    def finish(self):
        """Ends the word without telling riti what was picked."""
        self._lib.riti_context_finish_input_session(self._context)

    def close(self):
        if self._context:
            self._lib.riti_context_free(self._context)
            self._context = None

    def __del__(self):
        try:
            self.close()
        except Exception:       # interpreter shutdown: the library may be gone already
            pass

    # MARK: Helpers

    def _take(self, pointer):
        """Copies a suggestion into Python values and frees it."""
        if not pointer:
            return EMPTY
        try:
            if self._lib.riti_suggestion_is_empty(pointer):
                return EMPTY
            if self._lib.riti_suggestion_is_lonely(pointer):
                return Suggestion("lonely",
                                  text=self._string(self._lib.riti_suggestion_get_lonely_suggestion(pointer)))
            count = self._lib.riti_suggestion_get_length(pointer)
            entries = [self._string(self._lib.riti_suggestion_get_suggestion(pointer, i))
                       for i in range(count)]
            return Suggestion(
                "list", entries=entries,
                auxiliary=self._string(self._lib.riti_suggestion_get_auxiliary_text(pointer)),
                selection=self._lib.riti_suggestion_previously_selected_index(pointer))
        finally:
            self._lib.riti_suggestion_free(pointer)

    def _string(self, pointer):
        """Copies a string riti returned and frees it."""
        if not pointer:
            return ""
        try:
            return ctypes.cast(pointer, ctypes.c_char_p).value.decode("utf-8", "replace")
        finally:
            self._lib.riti_string_free(pointer)


def set_aside_bad_files(directory):
    """riti aborts the process if a user file isn't a flat JSON object of strings.

    A file like that (half-written, edited by hand) is renamed, not deleted, so nothing the user
    made is lost.
    """
    for name in USER_FILES:
        path = os.path.join(directory, name)
        if not os.path.exists(path) or _is_flat_string_object(path):
            continue
        try:
            os.rename(path, f"{path}.bad-{int(time.time())}")
        except OSError:
            pass


def _is_flat_string_object(path):
    try:
        with open(path, "rb") as handle:
            data = handle.read()
        # riti's JSON reader rejects a byte order mark.
        if data.startswith(b"\xef\xbb\xbf"):
            return False
        loaded = json.loads(data)
    except (OSError, ValueError):
        return False
    return (isinstance(loaded, dict)
            and all(isinstance(key, str) and isinstance(value, str)
                    for key, value in loaded.items()))
