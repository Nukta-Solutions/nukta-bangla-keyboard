# নুকতা বাংলা (Nukta Bangla) for macOS

The classic Bijoy keyboard layout as a native macOS input method, with Unicode output.

## Install
```
scripts/install.sh
```
Then go to System Settings → Keyboard → Input Sources → Edit… → + → Bangla → **নুকতা বাংলা**.
Run `scripts/install.sh` again after any code change. It also removes the old BijoyBangla.app; remove the old
Bijoy Bangla entry from Input Sources if it's still listed.

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

## Development
```
swift test          # engine tests (keystrokes → exact Unicode)
scripts/build.sh    # build/NuktaBangla.app
```
- `Sources/NuktaEngine`: key map and reordering logic (no AppKit).
- `Sources/NuktaInputMethod`: the InputMethodKit layer.
  - `ClientWriter` puts text straight into the app and rewrites the last few characters to reorder them.
  - Where an app can't rewrite text (terminals, Google Docs, Facebook, VS Code's chat box), it shows the
    syllable in progress as underlined (marked) text and inserts it once it's finished.
- `scripts/make_icon.swift` draws the নু icons: `Resources/icon.tiff` (menu bar) and, with `--app`,
  `Resources/AppIcon.icns` (Finder). Delete one and rebuild to redraw it.
- `scripts/build.sh` signs with your Apple Development certificate if you have one.
- Diagnostics: `/usr/bin/log show --last 1h --predicate 'subsystem == "com.asifmahmud.inputmethod.NuktaBangla"'`
  (use the full path; in zsh, plain `log` is a built-in command).
