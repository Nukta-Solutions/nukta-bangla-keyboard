# Smart phonetic typing: plan

Decided on 2026-11-23: these features will be built. No code yet. They will live in the shared phonetic
engine, so the same features reach macOS, Windows, Linux and the future mobile keyboard.

## Features

| # | Feature | Setting | Status |
|---|---|---|---|
| 1 | Rank suggestions by the previous word (`বই pore` → পড়ে, `জামা pore` → পরে) | none, always on | new |
| 2 | Learn new words: a word not in the dictionary, committed twice, becomes a suggestion; can be removed | none (plus a "forget what it learned" button) | new |
| 3 | Spelling tolerance: improve what exists, add typos | none | improvement |
| 4 | Next-word suggestions after a word is committed | on/off: on for mobile, off for desktop by default | new |
| 5 | Own shortcuts (`asw` → আসসালামু আলাইকুম) | a list in settings | new |
| 6 | Control over learned data: delete, export, import | buttons in settings | new |

Dropped on 2026-11-23: grammar and standard-spelling suggestions (প্রমিত বানান, e.g. দিবে → দেবে). Not to be built.

### 1. Rank by the previous word
- Score = how common the word is + how often it follows the previous word (from a bundled table of word pairs)
  + the user's own pairs (learned as they type).
- The pair table is built from a Bangla text corpus and bundled in the program (a few MB). The corpus and its
  licence are still to be chosen.

### 2. Learn new words
- A transliterated word that isn't in the dictionary is counted on each commit; at 2 commits it becomes a
  suggestion. The user's word pairs (for 1 and 4) are learned too.
- A learned word can be removed (for example Shift+Del on the candidate). There's also a "forget what it
  learned" button.
- Stored in a new file next to the existing pick memory, in the same data directory, with the same format
  on every platform so a copied directory works anywhere. Nothing leaves the device.
  - Windows `%APPDATA%\Nukta Bangla`, macOS `~/Library/Application Support/Nukta Bangla`,
    Linux `~/.local/share/nukta-bangla`, mobile: the app's private storage.

### 3. Spelling tolerance (improvement, no new setting)
- Already there: the dictionary search (upodesh, used by riti) matches every reading of each roman letter:
  ি/ী, ু/ূ, ন/ণ, শ/ষ/স, জ/য, র/ড়/ঢ়. Smart mode selects that word, and phonetic-first mode keeps it one key away.
- To add: typos in the roman text (one letter missing, extra or swapped: `bnagla` → বাংলা), ranked below
  exact matches, and better ordering of spelling-matched words.

### 4. Next-word suggestions
- Uses the same pair table and user pairs as 1. Tap or a number key inserts the word.

### 5. Own shortcuts
- The user adds pairs in settings: a roman code and the text it stands for (any length, more than one word,
  punctuation allowed).
- When the typed roman text equals a code, the expansion is the first suggestion and the normal transliteration
  is the second, so the plain word is one key away. A code that is only the start of what's typed shows nothing.
- Adding a code that is also a dictionary word (e.g. `ami`) shows a warning.
- Codes are 2–12 English letters, no digits (while a word is typed, 1–9 pick from the list); any case.
- Stored in `shortcuts.json` in the data directory (code → text), apart from what is learned, so wiping the
  learned data keeps them; export/import (6) carries both.
- Built (2026-11-25): the engine in `riti-bridge/src/shortcuts.rs` (with C functions for macOS and Linux), the
  Windows list, and the Windows settings: a card with সাজান…, which opens the shortcuts dialog
  (`NuktaBanglaSettings.exe --shortcuts`, `windows/src/bin/settings/shortcuts.rs`). The code box is marked
  English-only and the text service leaves its keys alone (`win::mark_latin_only`). Still to do: macOS and Linux.

### 6. Control over learned data
- Settings show what has been learned: the number of learned words, word pairs, picks and shortcuts.
- Delete: one item (Shift+Del on a candidate, or from a list in settings) or everything at once
  ("forget what it learned"). Shortcuts are deleted separately, so wiping what it learned doesn't lose them.
- Export: one file the user saves wherever they choose, holding learned words, word pairs, picks (the existing
  pick memory and riti's selections) and shortcuts, with a format version. The same file works on every platform,
  including mobile.
- Import: merges into what's there (counts are added, shortcuts with the same code ask which to keep).
- Nothing leaves the device: no sync and no network. The exported file goes only where the user puts it.

## Architecture
- All logic lives in our own MIT code in `riti-bridge` (not in riti's MPL files, where possible), behind the
  existing FFI, so every platform layer (Swift, Rust/TSF, Python/IBus, mobile) only shows the results.
- The dictionary is already bundled: upodesh, about 159k words. New bundled data: the word-pair table.
- No network: everything runs on the device.

## Prototype
https://claude.ai/artifact/R7ZBAbneACCuF759wN6XVo (private; a small sample dictionary). Its grammar card (৫) was
made before grammar was dropped; it isn't part of the plan.
