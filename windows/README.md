# নুকতা বাংলা for Windows

The Bijoy layout and Avro-style phonetic typing as one Windows input method: the same keys, rules and
output as on macOS and Linux, from a port of the same engine, with the same settings. Windows 10 and 11,
64-bit and 32-bit; not Windows on Arm yet.

## Install
Run `NuktaBangla-Setup-<version>.exe` and accept the administrator prompt. It adds **নুকতা বাংলা** to
your keyboards, under Bangla (Bangladesh), adding Bangla to your languages if it isn't there. Switch to
it like any keyboard: **Win + Space**, or the language button in the taskbar.

Until the installer is code-signed, Windows SmartScreen warns about an unknown publisher: **More info →
Run anyway**.

Installing over a version that is in use (it is loaded in every app you type in) replaces it on the
next restart; Setup asks for one.

## Typing
Both layouts are in the one keyboard. While it is on, the taskbar shows **বি** (বিজয়) or **ফ**
(ফোনেটিক) next to the language button:

- **click** it to switch layout;
- **right-click** it for the menu: বিজয় লেআউট, ফোনেটিক, সেটিংস…, about.

The layout applies to every app and is remembered, as on macOS. Typing is as described in the main
[README](../README.md#typing-avro-451-style-the-default): the syllable or word still being typed is
underlined, and finished text goes into the document. One difference in phonetic typing: the whole word
is deleted with **Ctrl + Backspace** (⌥⌫ on a Mac).

Keys are read by their position on a US keyboard, so the layouts work whatever Windows keyboard sits
underneath. Ctrl and Alt shortcuts go to the app untouched (after finishing the word in progress).

## Settings
**সেটিংস…** in the menu (or **নুকতা বাংলা সেটিংস** in the Start menu) opens the same settings as on macOS,
applied straight away in every app. They are kept in `%APPDATA%\Nukta Bangla\settings.json`, with the
same keys and values as the macOS and Linux builds ([linux/README.md](../linux/README.md) lists them);
what phonetic typing learns is kept in the same folder.

## Uninstall
Settings → Apps → Installed apps → **নুকতা বাংলা** → Uninstall. Your settings and what phonetic typing
learnt stay in `%APPDATA%\Nukta Bangla`; delete that folder too for a clean slate.

## Building
On Windows, with [Rust](https://rustup.rs) (and the Visual Studio C++ build tools it asks for) and
[Inno Setup 6.3+](https://jrsoftware.org/isinfo.php):
```
powershell -ExecutionPolicy Bypass -File windows\build.ps1
```
It runs the tests, builds the 64-bit and 32-bit DLLs and the settings program, and writes
`build\windows\NuktaBangla-Setup-<version>.exe`. The `Windows` GitHub workflow does the same on every
push that touches `windows/` and keeps the installer as a download.

The engine and phonetic tests are plain Rust and run anywhere, a Mac included: `cargo test` in this
folder. They are the Swift and Python tests, case for case; add a case to all three.

## How it works
- `src/bijoy`, `src/phonetic`, `src/settings.rs`: ports of `Sources/NuktaEngine`,
  `Sources/NuktaPhonetic` and `Settings.swift` (and of their Python twins in `linux/`). riti is used as
  the Rust library it is, not through its C API.
- `src/tsf`: the TSF (Text Services Framework) text service, `nukta_bangla.dll`. Windows loads it into
  every app that takes text and creates one per thread. Keys arrive through TSF's key sink; the part
  still being typed is a TSF composition and finished text replaces it, as with IBus's preedit on Linux,
  so none of the macOS read-back-and-rewrite work is needed. A key the layout doesn't use while a word is
  in progress (Space, Enter, arrows, a shortcut) is taken, the word committed, and the key sent again
  for the app: TSF has no "finish, then pass on".
- Settings are shared through the settings file, which every copy of the DLL re-reads when it changes:
  that is how a layout picked in one app reaches all of them.
- Each app runs its own copy of riti, made the first time phonetic typing is used there. riti's file of
  learned picks (smart mode) and the phonetic-first pick memory are re-read before they are written, so
  apps don't overwrite each other's learning.
- Store apps run sandboxed and can't open a user's files unless allowed. The installer and the settings
  program give app containers read and write access to `%APPDATA%\Nukta Bangla`, so the layout setting
  and phonetic learning reach Store apps too. The menu can't start the settings program from inside a
  Store app; use it from any other app or the Start menu.
- Everything Windows calls into the DLL with runs under `catch_unwind`: the DLL lives inside other
  people's apps, and riti panics when it can't write a file.
- `src/bin/settings.rs`: `NuktaBanglaSettings.exe`, the settings window, and `--enable` / `--disable`,
  which the installer runs to add the keyboard to (and remove it from) the user's languages.
- `installer/nukta-bangla.iss`: the Inno Setup script. It installs `x64\nukta_bangla.dll` and
  `x86\nukta_bangla.dll` (32-bit apps load the text service too) and registers both with `regsvr32`.

Windows on Arm would need an Arm64X build of the DLL, which loads into both Arm64 and emulated x64 apps;
it isn't done yet, so the installer refuses to run there.

Code signing: the installer script has the `SignTool` lines ready, commented out, for when a certificate
is in place.
