"""User settings, kept in a JSON file — the Linux counterpart of
``Sources/NuktaInputMethod/Settings.swift``, which uses UserDefaults.

    ~/.config/nukta-bangla/settings.json

The keys are the ones the macOS build stores, with the same names and values, so one description of
the settings covers both platforms (linux/README.md lists them). The file is read when the engine
starts and re-read whenever it changes, so an edit takes effect on the next focus change — there is
no settings window on Linux.

What riti and the pick memory learn lives in ``~/.local/share/nukta-bangla``.
"""

import json
import os

#: Settings file keys → their defaults. A key that is missing, or holds a value of the wrong type
#: or an unknown mode, falls back to its default; the rest of the file still applies.
DEFAULTS = {
    "typingMode": "phoneticFirst",      # phoneticFirst | smart | phoneticOnly
    "autocorrect": True,
    "emoji": True,
    "englishWord": True,
    "colonIsBisarga": False,
    "popupDirection": "vertical",       # vertical | horizontal (the suggestion list)
}

_MODES = ("phoneticFirst", "smart", "phoneticOnly")
_DIRECTIONS = ("vertical", "horizontal")


def config_directory():
    base = os.environ.get("XDG_CONFIG_HOME") or os.path.join(os.path.expanduser("~"), ".config")
    return os.path.join(base, "nukta-bangla")


def config_file():
    return os.path.join(config_directory(), "settings.json")


def data_directory():
    """Where riti and the pick memory keep what they learn; created by the composer."""
    base = os.environ.get("XDG_DATA_HOME") or os.path.join(os.path.expanduser("~"),
                                                           ".local", "share")
    return os.path.join(base, "nukta-bangla")


class Settings:
    """The settings file, re-read when it changes on disk."""

    def __init__(self, path=None):
        self._path = path or config_file()
        self._values = dict(DEFAULTS)
        self._stamp = None
        self.reload()

    @property
    def path(self):
        return self._path

    def reload(self):
        """Reads the file. A missing or unreadable file means the defaults."""
        self._stamp = self._mtime()
        values = dict(DEFAULTS)
        values.update(self._read())
        self._values = values

    def reload_if_changed(self):
        """Re-reads the file if it has been written since the last read. True if anything changed."""
        if self._mtime() == self._stamp:
            return False
        before = dict(self._values)
        self.reload()
        return self._values != before

    # MARK: The settings themselves

    @property
    def typing_mode(self):
        return self._choice("typingMode", _MODES)

    @property
    def autocorrect(self):
        return self._flag("autocorrect")

    @property
    def emoji(self):
        return self._flag("emoji")

    @property
    def english_word(self):
        return self._flag("englishWord")

    @property
    def colon_is_bisarga(self):
        return self._flag("colonIsBisarga")

    @property
    def popup_direction(self):
        return self._choice("popupDirection", _DIRECTIONS)

    @property
    def horizontal_popup(self):
        return self.popup_direction == "horizontal"

    def phonetic_options(self):
        """These settings as the composer's options."""
        from .phonetic import PhoneticOptions
        return PhoneticOptions(mode=self.typing_mode, autocorrect=self.autocorrect,
                               emoji=self.emoji, english_word=self.english_word,
                               colon_is_bisarga=self.colon_is_bisarga)

    # MARK: Helpers

    def _read(self):
        try:
            with open(self._path, "rb") as handle:
                loaded = json.load(handle)
        except (OSError, ValueError):
            return {}
        return loaded if isinstance(loaded, dict) else {}

    def _mtime(self):
        try:
            return os.stat(self._path).st_mtime_ns
        except OSError:
            return None

    def _flag(self, key):
        value = self._values.get(key)
        return value if isinstance(value, bool) else DEFAULTS[key]

    def _choice(self, key, allowed):
        value = self._values.get(key)
        return value if value in allowed else DEFAULTS[key]
