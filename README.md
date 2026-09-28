# Bijoy Bangla for macOS

Classic Bijoy keyboard as a native macOS input method, with Unicode output.

## Install
```
scripts/install.sh
```
Then go to System Settings → Keyboard → Input Sources → Edit… → + → Bangla → **Bijoy Bangla**.
Run `scripts/install.sh` again after any code change.

## Typing (classic Bijoy order)
| You type | You get |
|---|---|
| `d j` | কি (ি ে ৈ go **before** the consonant) |
| `c j f` or `j x` | কো |
| `c j X` or `j X` | কৌ |
| `d j g k` | ক্তি (`g` = link ্ for juktakkhor) |
| `j g N` | ক্ষ |
| `m A` | র্ম (reph typed **after** the consonant) |
| `o z` / `h Z` | গ্র / ব্য (`z` ্র, `Z` ্য) |
| `g f`, `g d`, `g s`, `g c` … | আ ই উ এ … (`g` + kar = full vowel) |
| `j g d` | কই |
| `g g` | visible hasanta (ক্‌) |
| `Q`, `\|`, `&`, `\`, `$`, `G` | ং ঃ ঁ ৎ ৳ । |

Backspace undoes the last key of the syllable you're still typing.

### Kar after consonant mode
From the ব input menu, choose **Kar after consonant** to type ি ে ৈ after the consonant:
`j c` → কে, `j d` → কি, `j g k d` → ক্তি, `j V c u` → কলেজ. Switch back with **Classic Bijoy**.
Your choice is remembered. `j x` → কো and `j X` → কৌ work in both modes.

## Development
```
swift test          # engine tests (keystrokes → exact Unicode)
scripts/build.sh    # build/BijoyBangla.app
```
- `Sources/BijoyEngine`: key map and reordering logic (no AppKit).
- `Sources/BijoyInputMethod`: the InputMethodKit layer.
  - `ClientWriter` puts text straight into the app and rewrites the last few characters to reorder them.
  - In terminals, which can't rewrite text, it falls back to underlined marked text.
