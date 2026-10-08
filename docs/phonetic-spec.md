# Phonetic typing: behaviour spec

What the phonetic half of Nukta Bangla must do: the riti wrapper, the composer that turns keys into text,
the memory of picked words, the suggestion list window, and the riti keycode lookup. It describes behaviour
and interfaces only; the code is written from this, the README and the tests.

Everything here is Nukta Solutions' own code under the repo's MIT licence. The one third-party part is riti
(`riti-bridge/riti`, MPL-2.0), used as a library through its C API (`riti-bridge/riti/include/riti.h`).

## 1. Keycode lookup (Rust, `riti-bridge/src/lib.rs`)

The static library `nukta_riti` re-exports riti's C API (link the crate in with `extern crate riti;`) and adds:

    uint16_t nukta_riti_keycode(uint32_t ch);   // declared in Sources/CRiti/shim.h

It returns riti's keycode (`riti::keycodes`, see `riti-bridge/riti/src/keycodes.rs`) for a character typed on
a US keyboard, or 0 if riti has no key for it. Covered: `a`–`z`; `A`–`Z` (riti's `*_SHIFT` codes); `0`–`9`;
and `` ` ~ ! @ # $ % ^ & * ( ) - _ = + [ ] { } \ | ; : ' " , . < > / ? ``. Everything else returns 0.

## 2. Public Swift API (`Sources/NuktaPhonetic`, module `NuktaPhonetic`)

These names are used by the app and the tests, so they are fixed. Everything else is internal.

```
public enum TypingMode: String, CaseIterable { case phoneticFirst, smart, phoneticOnly }

public struct PhoneticOptions: Equatable {
    public var mode: TypingMode = .phoneticFirst
    public var autocorrect = true
    public var emoji = true
    public var englishWord = true      // list the typed roman word itself
    public var colonIsBisarga = false  // false: ":" types a colon
    public init()
}

public final class PhoneticComposer {
    public enum Key: Equatable {
        case character(Character, shift: Bool)   // printable key, as on a US layout
        case backspace(wholeWord: Bool)          // ⌫, or ⌥⌫
        case enter, escape, space
        case tab(backward: Bool)
        case up, down, left, right
    }
    public struct Output: Equatable {
        public var insert = ""      // final text to insert, replacing the marked text
        public var marked = ""      // marked (underlined) text to show afterwards; "" = none
        public var handled = true   // false: the app must also handle the key (space, arrows…)
    }
    public init(options: PhoneticOptions, directory: URL)  // directory: where learned data is kept; create it
    public private(set) var options: PhoneticOptions
    public var horizontalNavigation: Bool               // ← → also move the selection
    public private(set) var candidates: [String]        // the list to show; empty = no list
    public private(set) var selectedIndex: Int
    public private(set) var auxiliary: String           // what was typed (roman), shown above the list
    public var isComposing: Bool                        // a word is in progress
    public func update(options: PhoneticOptions)
    public func handle(_ key: Key) -> Output
    public func commit() -> Output                      // commit the selected word (click elsewhere, app switch)
    public func commit(at index: Int) -> Output         // commit candidates[index] (a click in the list)
    public func discard()                               // drop the word in progress, no output
    static func containsEmoji(_ text: String) -> Bool   // internal; used by tests
}
```

## 3. riti, as the wrapper must use it

- One configuration per context: layout `"avro_phonetic"`, the user directory, phonetic suggestions on (off in
  phonetic-only mode), include-English per `englishWord`, and `riti_config_set_autocorrect` per `autocorrect`
  (a Nukta patch to riti, see `riti-bridge/riti.patch`).
- riti carries its dictionary inside the library; it only reads and writes the user directory:
  `phonetic-candidate-selection.json` (what it learned) and an optional `autocorrect.json`. riti aborts the
  process if either is not a flat JSON object of strings, so before creating a context move a bad file aside
  (rename it, don't delete it).
- A suggestion is either *lonely* (one string, no list: phonetic-only mode, or a lone punctuation mark) or a
  list. On a lonely suggestion, riti panics if asked for its length, auxiliary text or previously selected
  index: never ask. Every suggestion must be freed (`riti_suggestion_free`), every returned string too
  (`riti_string_free`). Copy what you need into Swift values straight away.
- `riti_get_suggestion_for_key(ctx, key, modifier, selection)`: `modifier` is `MODIFIER_SHIFT` when Shift is
  down. riti keeps the `selection` index passed in when the key is a punctuation mark, otherwise it computes
  its own.
- `riti_context_candidate_committed(ctx, index)` ends the session and lets riti learn the pick. Only smart mode
  calls it; other modes just finish the session (`riti_context_finish_input_session`).
- A list contains, among dictionary words, the literal transliteration of what was typed, but riti doesn't say
  which entry that is. Phonetic-first needs it: get it from a second context with suggestions off, fed exactly
  the same keys and backspaces, and ended whenever the main session ends.
- riti's list uses curly quotes (“ ” ‘ ’) where the literal transliteration has straight ones; compare with
  quotes straightened.
- riti can rank an emoji (matched by English or Bangla name) above dictionary words.
- Contexts are expensive and each one rewrites riti's selection file from its own copy: the app keeps one
  composer per process.

## 4. Composer behaviour

**Marked text** is always the selected candidate, or the lonely text when there is no list.

**Keys while a word is in progress** (not composing: every key below passes through, `handled = false`, unless
said otherwise):

| Key | Behaviour |
|---|---|
| Enter | commit the selected word; the key is consumed (no new line) |
| Esc | drop the word, clear the marked text; consumed |
| ⌫ / ⌥⌫ | riti backspace (last letter / whole word). If the word is now empty: clear. Otherwise rebuild the list with a fresh default selection |
| Space | commit the selected word, then let the app type the space |
| Tab, ⇧Tab, ↓, ↑ | move the selection forward/back, wrapping. With no list (phonetic-only): commit and pass the key through |
| ← → | as Tab when `horizontalNavigation`, else commit and pass through |
| `:` with `colonIsBisarga == false` | commit the word (if any), then insert `:`; consumed, also when not composing |
| ASCII digit, not composing | insert the Bangla digit (০–৯); consumed |
| `1`–`9`, list showing, digit ≤ list size | commit that candidate (1 = first) |
| `1`–`9`, no list (phonetic-only) | commit the word, then insert the Bangla digit |
| other digits, other characters riti knows | feed to riti |
| characters riti doesn't know | commit (if composing) and pass through |

**After feeding a key:** if riti's session is still going, rebuild the list. If riti ended the session itself,
insert what it produced (the lonely text, or the default-selected candidate) and reset.

**Building the list** from riti's suggestion:
1. Start from riti's order. With `emoji` off, drop emoji entries, unless that would leave nothing.
2. Phonetic-first: move the literal transliteration to the top.
3. Default selection:
   - smart: riti's previously selected index (where that entry ended up);
   - phonetic-first: the top (the literal), or the remembered pick for this typed word if it is in the list;
   - after a punctuation key typed while the user had moved the selection (see below): riti's preserved index.
4. Space must never commit an emoji by default: if the typed word starts with a letter and the default lands
   on an emoji, move it to the first non-emoji. (Typed emoticons like `:)` are exempt.)

**Selection kept across punctuation:** if the user moved the selection, and the next key is one of
`. ? ! , : ; - _ ) } ] ' "`, pass riti the riti index of the selected entry so riti keeps it. Any other key
forgets that the user moved the selection.

**Committing a list entry:** insert it. Phonetic-first, when the literal was in the list: remember the pick
(below). Smart: tell riti (it learns). Then reset all state.

**`containsEmoji`:** true when any scalar has emoji presentation, or is an emoji at U+2000 or above, or is
U+FE0F (which turns a text symbol like © into ©️). False for digits, `#`, `*`, plain ©, Bangla text.

**Options:** a change of `mode`, `autocorrect` or `englishWord` rebuilds riti and drops the word in progress.
`emoji` and `colonIsBisarga` apply from the next key, and keep the word.

## 5. Pick memory (phonetic-first)

A JSON object in `<directory>/phonetic-first-picks.json` (keep this name and format: installed copies already
have it): typed word → picked word, both trimmed of the punctuation riti puts around words
(`` - ] ~ ! @ # % & * ( ) _ = + [ { } ' " ; < > / ? | . , `` and `।“”‘’`). When the user commits an entry other
than the literal, store it (unless it contains an emoji). When they commit the literal, forget the entry.
Save in the background; a corrupt or missing file means an empty memory.

## 6. Suggestion list window (`Sources/NuktaInputMethod/CandidatePanel.swift`)

```
final class CandidatePanel {
    var onSelect: ((Int) -> Void)?          // a candidate was clicked
    func show(candidates: [String], auxiliary: String, selected: Int, cursor: NSRect,
              position: PopupPosition, direction: PopupDirection)   // enums in Settings.swift
    func hide()
}
```

- A borderless panel that never takes focus from the app being typed in, floats above normal windows, works
  on every Space and over full-screen apps, and has a shadow and rounded corners.
- Top line: the typed roman text, small and secondary. Then up to 9 candidates at a time, each with its
  number (1-based position in the whole list) and the word in the 16 pt system font. The selected one is
  highlighted with the system selection colour. If there are more than 9, scroll so the selection stays
  visible, and show a small sign that more exist. System colours, so it follows light and dark mode.
- `direction`: a column (`vertical`) or one row (`horizontal`). The window sizes itself to its content.
- Mouse: pressing on a candidate highlights it; releasing on the same one calls `onSelect`.
- Placement: left edge at the cursor rect's left; `below` (4 pt gap under the rect) or `above` (4 pt gap over
  it). If the preferred side has no room on that screen, use the other side. Always keep the window inside
  the visible frame of the screen the cursor is on.

## 7. Cursor position (`Sources/NuktaInputMethod/CursorRect.swift`)

```
enum CursorRect { static func of(_ client: Client) -> NSRect }   // called after the marked text is set
```

The screen rectangle of the word being typed, ideally of its start. Apps answer `firstRect(forCharacterRange:)`
and `attributes(forCharacterIndex:lineHeightRectangle:)` with varying reliability; Chrome and Electron
sometimes return garbage (uninitialised tiny values, the screen corner, zero height, points on no screen).
Try the reliable sources first, reject garbage, remember the last good answer and use it when everything
fails, and as a last resort use the mouse pointer position.
