# নুকতা বাংলা (Nukta Bangla)

The classic Bijoy keyboard layout as a native input method, with Unicode output: an InputMethodKit
input method on macOS, an IBus engine on Linux. Same keys, same rules, same output on both.

## Install on macOS
```
scripts/install.sh
```
Then go to System Settings → Keyboard → Input Sources → Edit… → + → Bangla → **নুকতা বাংলা**.
Run `scripts/install.sh` again after any code change. It also removes the old BijoyBangla.app; remove the old
Bijoy Bangla entry from Input Sources if it's still listed.

## Install on Linux
```
linux/install.sh
```
or, once this repository is public, without cloning it at all:
```
curl -fsSL https://raw.githubusercontent.com/Nukta-Solutions/nukta-bangla-keyboard/master/linux/bootstrap.sh | bash
```
Then add it under Settings → Keyboard → Input Sources → **+** → Bangla → **নুকতা বাংলা** (GNOME) or with
`ibus-setup` elsewhere, and switch with Super+Space. It needs IBus and `python3-gi`;
[linux/README.md](linux/README.md) has the per-distro packages, the non-GNOME desktop setup, the
no-root install and how the Linux port works.

## Typing (Avro 4.5.1 style: the default)
Classic Bijoy order works as always. As in Avro, a pre-base kar (ি ে ৈ) **pressed twice** right after a
consonant attaches to that consonant. The first press shows nothing; the second shows the kar.

| You type | You get |
|---|---|
| `c i c v m` · `i c c c v m` · `i c c v c c m` | হেরেম |
| `d n d j m` · `n d d d j m` · `n d d j d d m` | সিকিম |
| `C j C j` · `j C C j C C` | কৈকৈ |
| `d h u W` · `h d d u W` | বিজয় |
| `c j f` · `j c c f` · `j x` | কো |
| `c j X` · `j c c X` · `j X` | কৌ |
| `d j g k` · `j g k d d` | ক্তি (`g` = link ্ for juktakkhor) |
| `j g N` | ক্ষ |
| `m A` · `v g m` | র্ম (reph after the consonant, or র + g) |
| `o z` / `h Z` | গ্র / ব্য (`z` ্র, `Z` ্য) |
| `g f`, `g d`, `g s`, `g c`, `g x`, `g X` | আ ই উ এ ও ঔ (`g` + kar = full vowel); `F` অ, `F f` আ |
| `j g d` | কই |
| `g g` | visible hasanta (ক্‌) |
| `Q`, `\|`, `&`, `\`, `$`, `G` | ং ঃ ঁ ৎ ৳ । |

A kar pressed **once** always waits for the next consonant: `j c V u` → কলেজ, `h d u W` → বজিয়.

Backspace undoes the last key of the syllable you're still typing.

## Sharing with friends
```
scripts/package.sh         # macOS: build/NuktaBangla-<version>.pkg + build/INSTALL.txt
linux/package.sh           # Linux: build/nukta-bangla-<version>-linux.sh (one self-extracting file)
```
On Linux the whole install is then `bash nukta-bangla-<version>-linux.sh` — it carries the engine,
offers to install IBus and a Bangla font, and needs neither git nor this repository.

Send both macOS files. The .pkg runs on Apple Silicon and Intel Macs (macOS 13+) and installs into
`/Library/Input Methods`. It isn't notarized, so the first open needs System Settings → Privacy & Security →
Open Anyway; [INSTALL.txt](INSTALL.txt) walks through it in Bangla. Bump `CFBundleShortVersionString` in
Info.plist before sharing a new version.

## Development
```
swift test          # engine tests (keystrokes → exact Unicode)
scripts/build.sh    # build/NuktaBangla.app
```
- `Sources/NuktaEngine`: key map and reordering logic (no AppKit).
- `linux/`: the Linux port — the same engine in Python plus the IBus front end
  (`python3 -m unittest discover -s linux/tests`). The keystroke corpus in `linux/tests/test_engine.py`
  mirrors `Tests/NuktaEngineTests/EngineTests.swift`: a new typing rule needs its case in both.
- `Sources/NuktaInputMethod`: the InputMethodKit layer.
  - `ClientWriter` puts text straight into the app and rewrites the last few characters to reorder them.
  - Where an app can't rewrite text (terminals, Google Docs, Facebook, VS Code's chat box), it shows the
    syllable in progress as underlined (marked) text and inserts it once it's finished.
- `scripts/make_icon.swift` draws the নু icons: `Resources/icon.tiff` (menu bar) and, with `--app`,
  `Resources/AppIcon.icns` (Finder). Delete one and rebuild to redraw it.
- `scripts/build.sh` signs with your Apple Development certificate if you have one.
- Diagnostics: `/usr/bin/log show --last 1h --predicate 'subsystem == "com.asifmahmud.inputmethod.NuktaBangla"'`
  (use the full path; in zsh, plain `log` is a built-in command).

## Licence
[MIT](LICENSE) — © 2026 Nukta Solutions. Use it, change it, ship it, including in commercial work;
just keep the copyright and permission notice. The licence covers this keyboard's own code: the
Bijoy layout itself is a long-standing convention, not something this project claims.
