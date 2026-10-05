# নুকতা বাংলা (Nukta Bangla)

The classic Bijoy keyboard layout as a native input method with Unicode output: an InputMethodKit
input method on macOS, an IBus engine on Linux. Same keys, same rules, same output on both.

macOS adds Avro-style phonetic typing with dictionary suggestions. Pick the layout from the নু menu in
the menu bar: **বিজয় লেআউট** or **ফোনেটিক**. The choice applies to every app and is remembered;
**সেটিংস…** in the same menu has the rest. (Linux types Bijoy only — see [linux/README.md](linux/README.md).)

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
or, without cloning it at all:
```
curl -fsSL https://raw.githubusercontent.com/Nukta-Solutions/nukta-bangla-keyboard/main/linux/bootstrap.sh | bash
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

## Phonetic typing
Type Bangla in roman letters with Avro Phonetic's rules: `ami bangla likhchi` → আমি বাংলা লিখছি. The word shows
underlined in Bangla as you type, with a list of suggestions from a 150k-word dictionary under it.

| Key | While a word is being typed |
|---|---|
| Space | commits the selected word, then types the space |
| Enter | commits the selected word (no new line) |
| ↑ ↓ · Tab / ⇧Tab | move the selection (← → too when the list is a row) |
| 1–9 | commit that candidate (outside a word, digits type ০–৯) |
| Esc | drops the word |
| ⌫ · ⌥⌫ | delete the last letter · the whole word |
| punctuation (`?` `,` …) | commits the word, then types it (`.` is ।) |

**Settings** (নু menu → সেটিংস…), applied straight away; greyed out while বিজয় is the layout:
- **Typing mode**
  - *Phonetic first* (default): what you typed, transliterated, is selected (`sonar` → সনার). The
    dictionary's words are one key away, and a word you pick instead is selected next time.
  - *Smart*: the dictionary, autocorrect and your past picks choose (`sonar` → সোনার).
  - *Phonetic only*: transliteration with no list.
- **Autocorrect**: known misspellings and English words in Bangla (`account` → অ্যাকাউন্ট).
- **Emoji** suggestions (`hasi` → 😄). Space never commits an emoji unless you select it.
- **The typed English word** as a candidate.
- **`:`** types a colon (default), or ঃ as in Avro. With colon, ঃ words come from the list
  (`dukho` → দুঃখ).
- **The list** below or above the text, as a column or a row.

| You type | You get |
|---|---|
| `kh` `gh` `ch` `jh` `th` `dh` `ph` `bh` `sh` | খ ঘ ছ ঝ থ ধ ফ ভ শ |
| `T` `Th` `D` `Dh` `N` `Sh` `R` | ট ঠ ড ঢ ণ ষ ড় |
| `a` `i` `I`/`ee` `u` `U`/`oo` `e` `O` `OI` `OU` `rri` | া ি ী ু ূ ে ো ৈ ৌ ৃ (full vowels at a word's start) |
| `o` | inherent vowel after a consonant (`kotha` → কথা); অ at a word's start |
| `ng` `Ng` `NG` ``` t`` ``` `^` | ং ঙ ঞ ৎ ঁ |
| `rr` before a consonant · `r` / `y` / `w` after one | reph (`dhorrmo` → ধর্ম) · ্র ্য ্ব (`prem` → প্রেম) |

What it learns (picks, riti's selections) is kept in `~/Library/Application Support/Nukta Bangla`.

### Where it comes from, and the licence
Phonetic typing is [riti](https://github.com/OpenBangla/riti), OpenBangla's engine (Rust), with key handling,
the suggestion list and the pick memory adapted from [Lekho](https://github.com/ARahim3/Lekho). Both are under
the Mozilla Public License 2.0, which applies file by file:
- MPL files: everything in `riti-bridge/`, `Sources/CRiti`, `Sources/NuktaPhonetic`, and
  `Sources/NuktaInputMethod/CandidatePanel.swift` and `CursorRect.swift`. Keep their headers, and keep them
  MPL when you change them. The rest of the app isn't MPL.
- `riti-bridge/riti` is riti at `9afef32` with [riti-bridge/riti.patch](riti-bridge/riti.patch) applied (the
  autocorrect switch). To update riti, copy a newer revision in and reapply the patch.
- `scripts/build.sh` puts the MPL source (`MPL-source.zip`), `MPL-2.0.txt` and the Rust crates' licences
  (`THIRD-PARTY-NOTICES.txt`, from `scripts/third_party_notices.sh`) in the app; the About panel and
  INSTALL.txt point there. Rerun `scripts/third_party_notices.sh` after changing `riti-bridge/Cargo.lock`.

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
Needs Rust for riti: `curl https://sh.rustup.rs -sSf | sh` (once).
```
scripts/build_riti.sh   # riti-bridge/lib/libnukta_riti.a; once before swift build/test, again after changing riti
swift test              # Bijoy and phonetic tests (keystrokes → exact Unicode, real riti)
scripts/build.sh        # build/NuktaBangla.app (builds riti too)
```
- `Sources/NuktaEngine`: Bijoy key map and reordering logic (no AppKit).
- `Sources/NuktaPhonetic`: riti wrapper and `PhoneticComposer`, which turns keys into text to commit,
  marked text and the suggestion list (no AppKit, tested in `NuktaPhoneticTests`).
- `riti-bridge`: the Rust static library, riti plus a keycode lookup. `Sources/CRiti` is its C module.
- `linux/`: the Linux port — the Bijoy engine again in Python plus the IBus front end, needing no Swift
  or Rust (`python3 -m unittest discover -s linux/tests`). The keystroke corpus in
  `linux/tests/test_engine.py` mirrors `Tests/NuktaEngineTests/EngineTests.swift`: a new Bijoy typing
  rule needs its case in both.
- `Sources/NuktaInputMethod`: the InputMethodKit layer. `Phonetic` holds the one composer and suggestion
  list the whole process shares; `SettingsWindow` and `Settings` (UserDefaults) are the settings.
  - `ClientWriter` puts text straight into the app and rewrites the last few characters to reorder them.
  - Where an app can't rewrite text (terminals, Google Docs, Facebook, VS Code's chat box), it shows the
    syllable in progress as underlined (marked) text and inserts it once it's finished.
- `scripts/make_icon.swift` draws the নু icons: `Resources/icon.tiff` (menu bar) and, with `--app`,
  `Resources/AppIcon.icns` (Finder). Delete one and rebuild to redraw it.
- `scripts/build.sh` signs with your Apple Development certificate if you have one.
- Diagnostics: `/usr/bin/log show --last 1h --predicate 'subsystem == "com.asifmahmud.inputmethod.NuktaBangla"'`
  (use the full path; in zsh, plain `log` is a built-in command).

## Licence
[MIT](LICENSE) — © 2026 Nukta Solutions — for this keyboard's own code, including the Linux port. Use
it, change it, ship it, including in commercial work; just keep the copyright and permission notice.

Phonetic typing is built on MPL-2.0 code, which stays MPL file by file: see
[Where it comes from, and the licence](#where-it-comes-from-and-the-licence) for exactly which files.
The Bijoy layout itself is a long-standing convention, not something this project claims.
