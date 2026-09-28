# Bijoy Bangla for macOS

Classic Bijoy keyboard as a native macOS input method, with Unicode output.

## Install
```
scripts/install.sh
```
Then go to System Settings → Keyboard → Input Sources → Edit… → + → Bangla → **Bijoy Bangla**.
Run `scripts/install.sh` again after any code change.

## Typing (mixed, Avro style: the default)
Both the classic Bijoy order and the kar-after-consonant order work, with no switching.

| You type | You get |
|---|---|
| `d j` or `j d` | কি |
| `c j` or `j c` | কে |
| `c j f`, `j c f` or `j x` | কো |
| `c j X`, `j c X` or `j X` | কৌ |
| `d h u W` or `h d u W` | বিজয় |
| `d j g k` or `j g k d` | ক্তি (`g` = link ্ for juktakkhor) |
| `j g N` | ক্ষ |
| `m A` or `v g m` | র্ম (reph after the consonant, or র + g) |
| `o z` / `h Z` | গ্র / ব্য (`z` ্র, `Z` ্য) |
| `g f`, `g d`, `g s`, `g c`, `g x`, `g X` | আ ই উ এ ও ঔ (`g` + kar = full vowel); `F` অ, `F f` আ |
| `j g d` | কই |
| `g g` | visible hasanta (ক্‌) |
| `Q`, `\|`, `&`, `\`, `$`, `G` or `.` | ং ঃ ঁ ৎ ৳ । |

**The one rule to know:** ি ে ৈ typed right after a consonant with no kar belong to that consonant.
Anywhere else (word start, after a space or another kar), they wait for the next consonant.
So কলেজ is `j V c u`, and `j c V u` gives কেলজ.

Backspace undoes the last key of the syllable you're still typing.

### Strict classic mode
Choose **Classic Bijoy, strict** from the ব input menu if you want ি ে ৈ to always wait for the next consonant
(`j c V u` → কলেজ). Switch back with **Mixed, Avro style**. Your choice is remembered.

## Development
```
swift test          # engine tests (keystrokes → exact Unicode)
scripts/build.sh    # build/BijoyBangla.app
```
- `Sources/BijoyEngine`: key map and reordering logic (no AppKit).
- `Sources/BijoyInputMethod`: the InputMethodKit layer.
  - `ClientWriter` puts text straight into the app and rewrites the last few characters to reorder them.
  - In terminals, which can't rewrite text, it falls back to underlined marked text.
