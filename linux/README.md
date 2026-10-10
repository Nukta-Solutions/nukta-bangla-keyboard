# নুকতা বাংলা (Nukta Bangla) for Linux

The same two layouts as the macOS input method, as [IBus](https://github.com/ibus/ibus) engines — so
one keyboard works at the office (macOS) and at home (Linux):

- **নুকতা বাংলা** — the Bijoy layout. Typing rules, including the Avro 4.5.1 double-press kars, are
  identical to the macOS build; the table in the
  [top-level README](../README.md#typing-avro-451-style-the-default) applies here unchanged.
- **নুকতা বাংলা ফোনেটিক** — Avro-style phonetic typing with dictionary suggestions: `ami bangla
  likhchi` → আমি বাংলা লিখছি. Same engine (riti), same rules, same settings as the macOS
  [phonetic typing](../README.md#phonetic-typing).

Each is an input source of its own, so Super+Space switches between them like any other keyboard;
each engine's own menu (where the desktop shows IBus menus — GNOME puts it in the top bar) switches
to the other as the macOS **নু** menu does.

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

Phonetic typing also needs riti, OpenBangla's engine, as a compiled library (`libnukta_riti.so`). A
self-extracting installer built by `package.sh` carries it ready-built, so there is nothing to
install; from a clone, `install.sh` offers to build it when [Rust](https://rustup.rs) is there. The
Bijoy layout is pure Python and never needs it — without the library only Bijoy is installed, and
the phonetic input source is left out rather than offered and unable to type.

On KDE, Xfce and other non-GNOME desktops also make IBus the input method — `im-config -n ibus`
(Debian/Ubuntu) or `export GTK_IM_MODULE=ibus QT_IM_MODULE=ibus XMODIFIERS=@im=ibus` in your session
— and start `ibus-daemon -drx` if nothing starts it. GNOME runs IBus by itself.

## Install
```
linux/install.sh
```
It offers to build riti for phonetic typing (`--with-phonetic` to build without asking,
`--no-phonetic` to skip it), runs the engine tests, installs into `/usr/share` (asking for sudo once)
and restarts IBus. Then add the keyboard:

- **GNOME**: Settings → Keyboard → Input Sources → **+** → Bangla → **নুকতা বাংলা**
- **KDE**: System Settings → Keyboard → Input Method → IBus → Add Input Method → Bangla → **নুকতা বাংলা**
- **Any desktop**: `ibus-setup` → Input Method → Add → Bangla → **নুকতা বাংলা**

Both layouts are listed under Bangla — **নুকতা বাংলা** and **নুকতা বাংলা ফোনেটিক**. Add either or
both; **Super+Space** switches between whatever you added. If an engine is not listed, `ibus restart`,
or log out and back in. Run `linux/install.sh` again after any code change, and `linux/uninstall.sh`
to remove it (what phonetic typing learned stays in `~/.local/share/nukta-bangla`).

Without root, `linux/install.sh --user` installs into `~/.local`. IBus only scans `/usr/share/ibus/component`,
so that mode also needs `IBUS_COMPONENT_PATH=$HOME/.local/share/ibus/component` in
`~/.config/environment.d/nukta-bangla.conf` and a fresh login; the script prints the line.

## One-line install, and sharing it

```
linux/package.sh
```
builds two files in `build/`, neither of which needs git, this repository or a network:

| File | For |
|---|---|
| `nukta-bangla-<version>-linux.sh` | hand to someone — a single self-extracting installer |
| `nukta-bangla-<version>-linux.tar.gz` | the same tree, if they prefer a tarball |

Send the `.sh`. On the other machine the whole install is one line:

```
bash nukta-bangla-1.0.1-linux.sh
```

It unpacks itself to a temporary directory, offers to install IBus, the Python bindings and a Bangla
font with that distro's package manager (apt, dnf, pacman or zypper — it only names what is actually
missing), runs the engine tests, installs, and restarts IBus. riti travels inside it, so phonetic
typing works on a machine with no Rust. `--with-deps` installs those packages without asking, for a
scripted setup; `--user` installs into `~/.local` without root. Adding নুকতা বাংলা in the keyboard
settings is still a one-time manual step — IBus engines cannot add themselves to someone's
input-source list.

`package.sh --no-phonetic` leaves riti out, for a package of the Bijoy layout alone.

To see what a machine is missing without installing anything: `linux/install.sh --print-deps`.

From a clone, one line installs or updates in place:

```
git clone --depth 1 https://github.com/Nukta-Solutions/nukta-bangla-keyboard.git ~/nukta-bangla-keyboard && ~/nukta-bangla-keyboard/linux/install.sh
git -C ~/nukta-bangla-keyboard pull && ~/nukta-bangla-keyboard/linux/install.sh
```

### curl one-liner

`bootstrap.sh` is the whole install:

```
curl -fsSL https://raw.githubusercontent.com/Nukta-Solutions/nukta-bangla-keyboard/main/linux/bootstrap.sh | bash
```

It downloads the current source, checks the archive really is this project, hands the terminal back
to `install.sh` so it can still ask about missing packages (a piped script has no stdin of its own),
and installs. Every GitHub tarball URL redirects to `codeload.github.com`, which some networks block
or stall on, so each attempt is time-limited (`NUKTA_CONNECT_TIMEOUT`, 15s by default) and it falls
back to IPv4, then to `git clone` from github.com, and finally tells you to use the self-extracting
installer from Releases. Arguments pass through — `| bash -s -- --user` or `| bash -s -- --with-deps` — and
`NUKTA_REPO`, `NUKTA_REF` and `NUKTA_URL` point it at a fork, a tag or any other tarball.

It needs no git and no clone. For a machine with no network, or to install from a fork that is not
public, use the self-extracting installer above instead.

## Phonetic typing, and its settings

What the keys do while a word is being typed — Space and Enter commit the selected word, 1–9 pick
one, ↑ ↓ and Tab move the selection, Esc drops the word — is the
[macOS table](../README.md#phonetic-typing), with one Linux spelling: the whole word is deleted with
**Ctrl+⌫** or **Alt+⌫** (macOS uses ⌥⌫). IBus draws the suggestion list itself, so the desktop
places it and a click on a candidate commits it.

macOS has a settings window; here the same settings are a small JSON file, read again whenever it
changes, so an edit takes effect on the next focus change (click into another window and back).

    ~/.config/nukta-bangla/settings.json

```json
{
  "typingMode": "phoneticFirst",
  "autocorrect": true,
  "emoji": true,
  "englishWord": true,
  "colonIsBisarga": false,
  "popupDirection": "vertical"
}
```

| Key | Values | What it does |
|---|---|---|
| `typingMode` | `phoneticFirst` (default), `smart`, `phoneticOnly` | What Space commits: what you typed (`sonar` → সনার, a word you pick instead is selected next time), the dictionary's choice (সোনার), or transliteration with no list at all |
| `autocorrect` | `true` (default), `false` | Known misspellings and English words in Bangla (`account` → অ্যাকাউন্ট) |
| `emoji` | `true` (default), `false` | Emoji among the suggestions (`hasi` → 😄). Space never commits one unless you select it |
| `englishWord` | `true` (default), `false` | The typed roman word itself as a candidate |
| `colonIsBisarga` | `false` (default), `true` | `:` types a colon, or ঃ as in Avro. With a colon, ঃ words still come from the list (`dukho` → দুঃখ) |
| `popupDirection` | `vertical` (default), `horizontal` | The suggestion list as a column or a row; a row also lets ← → move the selection |

The file is yours to write — the engine only reads it. A missing file, a missing key or an
unrecognised value means that setting's default, and the rest of the file still applies; a file that
is not valid JSON means all the defaults. The names and values are the ones macOS stores in
UserDefaults, so one description covers both platforms.

What phonetic typing learns — your picks and riti's own selections — is kept in
`~/.local/share/nukta-bangla` (`~/Library/Application Support/Nukta Bangla` on macOS), in files of
the same names and formats: `phonetic-first-picks.json` and `phonetic-candidate-selection.json`.
Your own autocorrect entries go in `autocorrect.json` there, a flat JSON object of
`"typed": "replacement"`.

## How it works

`nukta_bangla/` is a port of the macOS sources, file for file.

Bijoy — a port of `Sources/NuktaEngine`: `keymap.py` (which key means what), `syllable.py` (one
syllable in Bijoy order, rendered into Unicode order) and `engine.py` (the state machine).
`ibus_engine.py` is the IBus front end, the counterpart of `Sources/NuktaInputMethod`, and carries
the layout menu both engines show.

Phonetic — a port of `Sources/NuktaPhonetic`: `riti.py` (riti's C API through `ctypes`, the
counterpart of `Riti.swift`) and `phonetic.py` (`PhoneticComposer` and the pick memory). Both
implement [docs/phonetic-spec.md](../docs/phonetic-spec.md), which is what keeps the two platforms
in step. `ibus_phonetic.py` is the IBus front end and `settings.py` reads the settings file.

Two things macOS needs and Linux does not: riti is linked into the macOS binary, while here the same
Rust crate is built as a shared library (`crate-type = ["staticlib", "cdylib"]`) and loaded at run
time, so the Python engine needs no compiler of its own; and IBus draws the suggestion list, so
`CandidatePanel.swift` and `CursorRect.swift` — a window placed at the text cursor, with all the
guessing that takes in Chrome and Electron — have no counterpart here at all.

Two differences from macOS, both because IBus gives us a real preedit:

- The syllable in progress sits in the **preedit** (underlined) until it is finished, so nothing
  already in the document is ever rewritten. The macOS `ClientWriter` dance — inserting text
  directly and rewriting the last few characters, with a marked-text fallback for apps that ignore
  rewrites — has no equivalent here, and neither do its per-application quirks.
- A kar or ্ that has no consonant to sit on yet is held back from the preedit, the same way the
  macOS build keeps it off screen (`Output.visible`, not `Output.display`): typing প্রা and then ে
  shows nothing extra until the next letter says where the ে belongs, rather than parking an orphan
  ে beside the finished word.
- Physical key codes are read from the event (`US_LAYOUT` in `ibus_engine.py`), so Bijoy works
  whatever XKB layout is underneath, exactly as the macOS build does with its key-code map.

The engine logic therefore exists twice, in Swift and in Python. What keeps the two honest is the
test corpus: `tests/test_engine.py` holds the same ~190 keystroke → Unicode cases as
`Tests/NuktaEngineTests/EngineTests.swift`. **Add a rule on one platform and add its case to both
test files.**

## Development
```
linux/build_riti.sh                              # libnukta_riti.so, for the phonetic tests (needs Rust)
python3 -m unittest discover -s linux/tests      # engine + key-event tests, no IBus needed
```
- `tests/test_engine.py` — the Bijoy corpus, shared with `Tests/NuktaEngineTests/EngineTests.swift`.
- `tests/test_phonetic.py` — the phonetic corpus, shared with
  `Tests/NuktaPhoneticTests/PhoneticComposerTests.swift`, against real riti. **A case added on one
  platform belongs in both files.**
- `tests/test_ibus_layer.py` and `tests/test_ibus_phonetic.py` — the IBus front ends against the
  stubs in `tests/fake_ibus.py`: preedit, commits, backspace, shortcut pass-through, the suggestion
  list, the settings file and the layout menu.

The two phonetic test files need `libnukta_riti.so` and are skipped without it, so the Bijoy tests
still run on a machine with no Rust. Swift compares strings as canonical equivalents and Python does
not, which is why `test_phonetic.py` compares text normalised — য় is one scalar in the Swift file and
two from riti.

After editing the engine's Python, reinstall with `./install.sh` rather than copying files over
`/usr/share/ibus-nukta-bangla`: the running process already has the old module in memory, and
`ibus restart` leaves that process alone. `install.sh` kills it so IBus respawns it; by hand it is
`pkill -f ibus-engine-nukta-bangla`.

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
- No settings window: the phonetic settings are the JSON file above, and the engine's menu switches
  layout but nothing else. There is nothing to configure for Bijoy — the Avro-style kar behaviour is
  always on.
- The layout menu depends on the desktop showing IBus property menus, which GNOME does and some
  desktops do not. Both layouts are input sources in their own right, so Super+Space always works.
- Phonetic typing needs the compiled riti library, so a machine without it (and without Rust) gets
  the Bijoy layout only.

## Licence
[MIT](../LICENSE), the same as the rest of the project — © 2026 Nukta Solutions. `nukta-bangla.xml`
and the engines' own metadata report `MIT` to IBus, which is what distro packagers read.

riti, which phonetic typing uses, is MPL-2.0 and stays MPL file by file. Wherever
`libnukta_riti.so` is installed its licence notices and source sit beside it, in
`$PREFIX/share/ibus-nukta-bangla/licenses/` (`MPL-2.0.txt`, `THIRD-PARTY-NOTICES.txt` and
`riti-source.tar.gz`) — the same files the macOS app carries. See
[the top-level README](../README.md#where-it-comes-from-and-the-licence).
