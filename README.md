# নুকতা বাংলা (Nukta Bangla)

The classic Bijoy keyboard layout as a native input method with Unicode output: an InputMethodKit
input method on macOS, an IBus engine on Linux, a TSF text service on Windows. Same keys, same rules,
same output on all three.

All three also do Avro-style phonetic typing with dictionary suggestions, from the same engine and
with the same settings. On macOS, pick the layout from the নু menu in the menu bar: **বিজয় লেআউট** or
**ফোনেটিক** — the choice applies to every app and is remembered, and **সেটিংস…** in the same menu has
the rest. On Linux the two layouts are separate input sources, switched with Super+Space, and the
settings are a small JSON file — see [linux/README.md](linux/README.md). On Windows both are in one
keyboard: click the **বি** / **ফ** indicator in the taskbar to switch, right-click it for the menu — see
[windows/README.md](windows/README.md).

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
Then add it under Settings → Keyboard → Input Sources → **+** → Bangla → **নুকতা বাংলা - বিজয়** or
**নুকতা বাংলা - ফোনেটিক** — in GNOME, or with `ibus-setup` elsewhere — and switch with
Super+Space. It needs IBus and `python3-gi`, and phonetic typing needs riti as a compiled library,
which the self-extracting installer carries and `install.sh` offers to build from a clone.
[linux/README.md](linux/README.md) has the per-distro packages, the phonetic settings file, the
non-GNOME desktop setup, the no-root install and how the Linux port works.

## Install on Windows
Run `NuktaBangla-Setup.exe` (built by `windows\build.ps1`, or downloaded from the `Windows`
GitHub workflow's run). It adds **নুকতা বাংলা** to your keyboards; switch to it with Win+Space.
Windows 10 and 11, 64-bit and 32-bit. [windows/README.md](windows/README.md) has the details and how the
Windows port works.

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

What it learns (picks, riti's selections) is kept in `~/Library/Application Support/Nukta Bangla`
(`~/.local/share/nukta-bangla` on Linux), in the same files, so a copied directory works on either.
Linux has the same settings in `~/.config/nukta-bangla/settings.json`
([linux/README.md](linux/README.md#phonetic-typing-and-its-settings)), and deletes the whole word
with Ctrl+⌫ or Alt+⌫ rather than ⌥⌫.

### Where it comes from, and the licence
Phonetic typing uses [riti](https://github.com/OpenBangla/riti), OpenBangla's engine (Rust), as a library.
riti is under the Mozilla Public License 2.0, which applies file by file:
- MPL files: `riti-bridge/riti` (riti itself) and `riti-bridge/riti.patch`. Keep them MPL when you change
  them. Everything else, including the riti wrapper, the suggestion list and the bridge in
  `riti-bridge/src`, is this project's own MIT code; [docs/phonetic-spec.md](docs/phonetic-spec.md)
  describes how it behaves.
- `riti-bridge/riti` is riti at `9afef32` with [riti-bridge/riti.patch](riti-bridge/riti.patch) applied (the
  autocorrect switch). To update riti, copy a newer revision in and reapply the patch.
- `scripts/build.sh` puts the MPL source (`MPL-source.zip`), `MPL-2.0.txt` and the Rust crates' licences
  (`THIRD-PARTY-NOTICES.txt`, from `scripts/third_party_notices.sh`) in the app; the About panel and
  INSTALL.txt point there. Rerun `scripts/third_party_notices.sh` after changing `riti-bridge/Cargo.lock`.
- On Linux the same notices and `riti-source.tar.gz` are installed beside `libnukta_riti.so`, in
  `$PREFIX/share/ibus-nukta-bangla/licenses/`, by `linux/install.sh`.

## Sharing with friends
```
scripts/package.sh         # macOS: build/NuktaBangla-<version>.dmg (the pkg + INSTALL.txt)
linux/package.sh           # Linux: build/nukta-bangla-<version>-linux.sh (one self-extracting file)
```
On Linux the whole install is then `bash nukta-bangla-<version>-linux.sh` — it carries the engine,
offers to install IBus and a Bangla font, and needs neither git nor this repository.

On macOS, send the .dmg. It holds the installer (`Install Nukta Bangla.pkg`) and
[INSTALL.txt](INSTALL.txt), which walks through it in Bangla; the .pkg is also left in `build/` on its own.
It runs on Apple Silicon and Intel Macs (macOS 13+) and installs into `/Library/Input Methods`. An input
method can't be dragged into place like an app, so the .dmg carries the pkg rather than the app. Bump
`CFBundleShortVersionString` (and `CFBundleVersion`) in Info.plist before sharing a new version.

`package.sh` signs the app and the .pkg with Nukta Solutions' Developer ID certificates, notarizes the .pkg
and the .dmg and staples their tickets, so it installs with no Gatekeeper warning. It needs, in your keychain:
- the **Developer ID Application** and **Developer ID Installer** certificates
  (Xcode → Settings → Accounts → Manage Certificates);
- a notarytool profile named `nukta`, created once with an app-specific password from account.apple.com:
  `xcrun notarytool store-credentials nukta --apple-id <Apple ID> --team-id <Team ID>`.

`scripts/package.sh --no-notarize` makes a signed but un-notarized .pkg and .dmg for a quick test.

On Windows, `windows\build.ps1` makes `build\windows\NuktaBangla-Setup.exe` (see
[windows/README.md](windows/README.md)); the Windows workflow builds the same file on every push.

## Releasing
The website and README link to the newest release with fixed names, so **every release must carry all
three files, named exactly**:

| Platform | Asset | Permanent link |
|---|---|---|
| macOS | `NuktaBangla.dmg` | https://github.com/Nukta-Solutions/nukta-bangla-keyboard/releases/latest/download/NuktaBangla.dmg |
| Windows | `NuktaBangla-Setup.exe` | https://github.com/Nukta-Solutions/nukta-bangla-keyboard/releases/latest/download/NuktaBangla-Setup.exe |
| Linux | `nukta-bangla-linux.sh` | https://github.com/Nukta-Solutions/nukta-bangla-keyboard/releases/latest/download/nukta-bangla-linux.sh |

A release missing one of them breaks that platform's link until the next release, so never publish
one without all three.

Pushing a `v*` tag runs [.github/workflows/release.yml](.github/workflows/release.yml):
1. Checks the tag is `v` + `CFBundleShortVersionString`, and that `linux/nukta_bangla/__init__.py` and
   `windows/Cargo.toml` say the same version, and creates a draft release (or uses one you made by hand
   with notes).
2. On macOS, if the Apple secrets below are set, runs `scripts/package.sh` with the Developer ID
   certificates and a notarytool profile set up from them, checks the DMG is signed, notarized and
   stapled, and uploads it as `NuktaBangla.dmg`. Without the secrets this job is skipped (see below).
3. On Ubuntu 22.04, builds riti, runs the Linux tests and `linux/package.sh`, and uploads
   `nukta-bangla-linux.sh`.
4. On Windows, runs `windows\build.ps1` (tests, both DLLs, the installer) and uploads
   `NuktaBangla-Setup.exe`.
5. Checks all three files are attached, publishes the release, then sets the version and sha256 in
   [Nukta-Solutions/homebrew-tap](https://github.com/Nukta-Solutions/homebrew-tap)'s cask (skipped
   until `HOMEBREW_TAP_TOKEN` is set).

If any build, signing or notarization fails, the run fails and the release stays a draft with nothing
public. Fix it and rerun the failed jobs (`gh run rerun <id> --failed`).

```
# bump CFBundleShortVersionString and CFBundleVersion in Resources/Info.plist, __version__ in
# linux/nukta_bangla/__init__.py and version in windows/Cargo.toml; merge that to main, then:
git switch main && git pull
git tag v1.0.2 && git push origin v1.0.2
gh run watch
```

**Signing the Mac build on your own Mac instead.** Until the Apple secrets are set, the workflow builds
Windows and Linux into a draft and skips macOS. Then, on a Mac with both Developer ID certificates in
the keychain and the `nukta` notarytool profile (see above):
```
git fetch --tags && git switch --detach v1.0.2
scripts/release-mac.sh v1.0.2      # package.sh, upload NuktaBangla.dmg, publish once all three are there
git switch -
```
The Homebrew cask is only updated by the workflow; with this route, bump it in
[homebrew-tap](https://github.com/Nukta-Solutions/homebrew-tap) by hand.

To build the Mac release in CI instead, set these repository secrets (Settings → Secrets and variables →
Actions):

| Secret | Value |
|---|---|
| `DEVELOPER_ID_APPLICATION_P12` | base64 of a .p12 export of the Developer ID Application certificate *with its private key* |
| `DEVELOPER_ID_APPLICATION_P12_PASSWORD` | the password set on that export |
| `DEVELOPER_ID_INSTALLER_P12` | the same for the Developer ID Installer certificate |
| `DEVELOPER_ID_INSTALLER_P12_PASSWORD` | its export password |
| `APP_STORE_CONNECT_API_KEY_P8` | the contents of an App Store Connect API key (`AuthKey_XXXXXXXXXX.p8`) |
| `APP_STORE_CONNECT_KEY_ID` | that key's ID (the `XXXXXXXXXX`) |
| `APP_STORE_CONNECT_ISSUER_ID` | the Issuer ID shown above the keys list |
| `HOMEBREW_TAP_TOKEN` | optional: a fine-grained token with Contents: read and write on `Nukta-Solutions/homebrew-tap` only |

To export a certificate: Keychain Access → login → My Certificates → right-click the certificate (it
must show a key under it) → Export… → .p12, with a password. Then
`base64 -i app.p12 | gh secret set DEVELOPER_ID_APPLICATION_P12` and delete the .p12. The API key comes from
App Store Connect → Users and Access → Integrations → App Store Connect API → Team Keys → **+**, role
Developer; it can be downloaded once. `gh secret set APP_STORE_CONNECT_API_KEY_P8 < AuthKey_XXXXXXXXXX.p8`.

### The update notice
Every installed copy asks GitHub for the latest release once a day (macOS: when you switch to the
keyboard; Windows: the top bar; Linux: when a layout gets the focus) and, if its tag is a newer version
than the installed one, shows the update dialog. So:
- The tag must match the versions in Info.plist, `windows/Cargo.toml` and
  `linux/nukta_bangla/__init__.py` (the release workflow checks this): a copy whose version is behind
  the tag keeps being told to update.
- In the release description, the lines starting with `- ` (at most five) are the dialog's
  "নতুন যা আছে", shown as you write them, so write them in Bangla. Everything else (paragraphs,
  headings, install steps) is only on the GitHub page. No bullet lines: the dialog has no notes box.
- **ডাউনলোড** opens <https://www.nuktasolutions.com/nukta-bangla-keyboard>, so link the new release there.
- Drafts and pre-releases are never offered: a release only reaches users once it is published.

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
- `riti-bridge`: the Rust library, riti plus a keycode lookup. Built as a static library for the macOS
  app (`Sources/CRiti` is its C module) and as a shared one for Linux, which loads it from Python.
- `linux/`: the Linux port — both engines again in Python plus their IBus front ends, needing no Swift
  (`python3 -m unittest discover -s linux/tests`). The keystroke corpora in `linux/tests/test_engine.py`
  and `linux/tests/test_phonetic.py` mirror `Tests/NuktaEngineTests/EngineTests.swift` and
  `Tests/NuktaPhoneticTests/PhoneticComposerTests.swift`: a new typing rule needs its case in both
  platforms' files. Only the phonetic half needs Rust, for `linux/build_riti.sh`.
- `Sources/NuktaInputMethod`: the InputMethodKit layer. `Phonetic` holds the one composer and suggestion
  list the whole process shares; `SettingsWindow` and `Settings` (UserDefaults) are the settings.
  - `ClientWriter` puts text straight into the app and rewrites the last few characters to reorder them.
  - Where an app can't rewrite text (terminals, Google Docs, Facebook, VS Code's chat box), it shows the
    syllable in progress as underlined (marked) text and inserts it once it's finished.
- `scripts/make_icon.swift` draws the নু icons: `Resources/icon.tiff` (menu bar) and, with `--app`,
  `Resources/AppIcon.icns` (Finder). Delete one and rebuild to redraw it.
- `scripts/build.sh` signs with your Apple Development (or Developer ID) certificate if you have one;
  `--release` (used by `package.sh`) signs with Developer ID and the hardened runtime for distribution.
- Diagnostics: `/usr/bin/log show --last 1h --predicate 'subsystem == "com.nuktasolutions.inputmethod.NuktaBangla"'`
  (use the full path; in zsh, plain `log` is a built-in command).

## Licence
[MIT](LICENSE) — © 2026 Nukta Solutions — for this keyboard's own code, including the Linux port. Use
it, change it, ship it, including in commercial work; just keep the copyright and permission notice.

Phonetic typing uses riti, which is MPL-2.0 and stays MPL file by file: see
[Where it comes from, and the licence](#where-it-comes-from-and-the-licence).
The Bijoy layout itself is a long-standing convention, not something this project claims.
