# নুকতা বাংলা (Nukta Bangla) for Linux

The same Bijoy layout as the macOS input method, as an [IBus](https://github.com/ibus/ibus) engine —
so one layout works at the office (macOS) and at home (Linux). Typing rules, including the Avro
4.5.1 double-press kars, are identical; the table in the [top-level README](../README.md#typing-avro-451-style-the-default)
applies here unchanged.

## Requirements

IBus and its Python bindings — present on GNOME and most desktops already:

| Distro | Install |
|---|---|
| Ubuntu / Debian / Mint | `sudo apt install ibus python3-gi` |
| Fedora | `sudo dnf install ibus python3-gobject` |
| Arch / Manjaro | `sudo pacman -S ibus python-gobject` |
| openSUSE | `sudo zypper install ibus python3-gobject` |

A Bangla font too, if the system has none: `fonts-beng` (Debian/Ubuntu), `google-noto-sans-bengali-fonts`
(Fedora), `noto-fonts` (Arch).

On KDE, Xfce and other non-GNOME desktops also make IBus the input method — `im-config -n ibus`
(Debian/Ubuntu) or `export GTK_IM_MODULE=ibus QT_IM_MODULE=ibus XMODIFIERS=@im=ibus` in your session
— and start `ibus-daemon -drx` if nothing starts it. GNOME runs IBus by itself.

## Install
```
linux/install.sh
```
It runs the engine tests, installs into `/usr/share` (asking for sudo once) and restarts IBus. Then
add the keyboard:

- **GNOME**: Settings → Keyboard → Input Sources → **+** → Bangla → **নুকতা বাংলা**
- **KDE**: System Settings → Keyboard → Input Method → IBus → Add Input Method → Bangla → **নুকতা বাংলা**
- **Any desktop**: `ibus-setup` → Input Method → Add → Bangla → **নুকতা বাংলা**

Switch between layouts with **Super+Space**. If the engine is not listed, `ibus restart`, or log out
and back in. Run `linux/install.sh` again after any code change, and `linux/uninstall.sh` to remove it.

Without root, `linux/install.sh --user` installs into `~/.local`. IBus only scans `/usr/share/ibus/component`,
so that mode also needs `IBUS_COMPONENT_PATH=$HOME/.local/share/ibus/component` in
`~/.config/environment.d/nukta-bangla.conf` and a fresh login; the script prints the line.

## How it works

`nukta_bangla/` is a port of `Sources/NuktaEngine`, file for file: `keymap.py` (which key means
what), `syllable.py` (one syllable in Bijoy order, rendered into Unicode order) and `engine.py` (the
state machine). `ibus_engine.py` is the IBus front end, the counterpart of
`Sources/NuktaInputMethod`.

Two differences from macOS, both because IBus gives us a real preedit:

- The syllable in progress sits in the **preedit** (underlined) until it is finished, so nothing
  already in the document is ever rewritten. The macOS `ClientWriter` dance — inserting text
  directly and rewriting the last few characters, with a marked-text fallback for apps that ignore
  rewrites — has no equivalent here, and neither do its per-application quirks.
- Physical key codes are read from the event (`US_LAYOUT` in `ibus_engine.py`), so Bijoy works
  whatever XKB layout is underneath, exactly as the macOS build does with its key-code map.

The engine logic therefore exists twice, in Swift and in Python. What keeps the two honest is the
test corpus: `tests/test_engine.py` holds the same ~190 keystroke → Unicode cases as
`Tests/NuktaEngineTests/EngineTests.swift`. **Add a rule on one platform and add its case to both
test files.**

## Development
```
python3 -m unittest discover -s linux/tests      # engine + key-event tests, no IBus needed
```
`tests/test_engine.py` is the shared corpus; `tests/test_ibus_layer.py` drives the IBus front end
against the stubs in `tests/fake_ibus.py` (preedit, commits, backspace, shortcut pass-through).

To try a change without installing, run the engine against the live bus:
```
linux/ibus-engine-nukta-bangla --standalone
```
It registers itself, switches IBus to it, and types in any application until you stop it with
Ctrl+C. Logs from an installed engine: `journalctl --user -f` (IBus starts the process, so stderr
goes to the journal).

## Known limits

- IBus only. Wayland and X11 both work through it, but a desktop using Fcitx5 needs a separate
  front end; the engine code would be reusable as-is.
- No input-method menu (the macOS **নু** menu with its About panel); there is nothing to configure
  yet, since the Avro-style kar behaviour is always on.
