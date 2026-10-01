"""Keystrokes → exact Unicode, ported from Tests/NuktaEngineTests/EngineTests.swift.

The same corpus runs against the macOS engine, so a change that makes the two platforms type
differently fails here. Run with: python3 -m unittest discover -s linux/tests
"""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from nukta_bangla.engine import Engine  # noqa: E402


def type_keys(keys):
    """Types `keys` the way an application sees it. `⌫` means backspace."""
    engine = Engine()
    doc = ""
    for ch in keys:
        if ch == "⌫":
            if engine.backspace() is None and doc:
                doc = doc[:-1]
            continue
        out = engine.process_character(ch)
        if out is not None:
            doc += out.committed
        else:
            doc += engine.commit().committed + ch
    return doc + engine.commit().committed


def names(text):
    return " ".join("U+%04X" % ord(c) for c in text)


CASES = [
    # basic kar + reordering of pre-base kars
    ("j", "ক"),
    ("jf", "কা"),
    ("dj", "কি"),
    ("jD", "কী"),
    ("cj", "কে"),
    ("Cj", "কৈ"),
    ("cjf", "কো"),
    ("cjX", "কৌ"),
    ("js", "কু"),
    ("jS", "কূ"),
    ("ja", "কৃ"),
    # juktakkhor
    ("djgk", "ক্তি"),
    ("jgN", "ক্ষ"),
    ("ngkz", "স্ত্র"),
    ("cjgNkz", "ক্ষেত্র"),
    ("nQngjadk", "সংস্কৃতি"),
    ("gsugughV", "উজ্জ্বল"),
    ("ug", "জ্"),
    ("jgg", "ক্‌"),
    ("uIf", "জঞা"),
    ("ugIfb", "জ্ঞান"),
    ("dhugIfb", "বিজ্ঞান"),
    ("jaNgB", "কৃষ্ণ"),
    ("nmgmfb", "সম্মান"),
    ("LbZhfl", "ধন্যবাদ"),
    ("rvDjgNf", "পরীক্ষা"),
    ("cMzdB", "শ্রেণি"),
    ("cmgr", "ম্পে"),
    # phala
    ("ozfm", "গ্রাম"),
    ("dhlZfVW", "বিদ্যালয়"),
    ("hZfQj", "ব্যাংক"),
    ("gCjZ", "ঐক্য"),
    ("vZfh", "র‍্যাব"),
    # reph
    ("mA", "র্ম"),
    ("jmA", "কর্ম"),
    ("djA", "র্কি"),
    ("FKA", "অর্থ"),
    ("jkAarjgN", "কর্তৃপক্ষ"),
    ("gSLghA", "ঊর্ধ্ব"),
    # independent vowels
    ("gf", "আ"), ("gd", "ই"), ("gD", "ঈ"), ("gs", "উ"), ("gS", "ঊ"),
    ("ga", "ঋ"), ("gc", "এ"), ("gC", "ঐ"), ("gX", "ঔ"), ("F", "অ"), ("gx", "ও"), ("x", "ো"),
    ("jgd", "কই"),
    ("gXNL", "ঔষধ"),
    ("gaB", "ঋণ"),
    ("gDl", "ঈদ"),
    ("gxcj", "ওকে"),
    # x after a consonant → ো
    ("jx", "কো"),
    ("jxb", "কোন"),
    ("Hfvx", "ভারো"),
    ("jx&", "কোঁ"),
    ("mAx", "র্মো"),
    ("ozx", "গ্রো"),
    ("jgx", "কও"),
    ("j gx", "ক ও"),
    ("j x", "ক ো"),
    ("cjx", "কো"),
    ("jx⌫", "ক"),
    # X after a consonant → ৌ
    ("jX", "কৌ"),
    ("cjX", "কৌ"),
    ("mXn", "মৌস"),
    ("ozX", "গ্রৌ"),
    # signs
    ("hfQVf", "বাংলা"),
    ("yf&l", "চাঁদ"),
    ("cjf&", "কোঁ"),
    ("crX&Yfcbf", "পৌঁছানো"),
    ("j|", "কঃ"),
    ("iTf\\", "হঠাৎ"),
    # words and sentences
    ("gfmfv ncfbfv hfQVf", "আমার সোনার বাংলা"),
    ("hfQVfclM", "বাংলাদেশ"),
    ("mfbsN", "মানুষ"),
    ("HfcVfhfnf", "ভালোবাসা"),
    ("gfmd hfQVf dVdJ|", "আমি বাংলা লিখিঃ"),
    ("gfmd hfQVf dVdJG", "আমি বাংলা লিখি।"),
    # digits and symbols
    ("123", "১২৩"),
    ("$100", "৳১০০"),
    ("j, K.", "ক, থ."),
    ("gfmd .", "আমি ."),
    # F f → আ
    ("Ff", "আ"),
    ("Ffm", "আম"),
    ("FfmfV", "আমাল"),
    ("Ffmfv ncfbfv", "আমার সোনার"),
    ("Fj", "অক"),
    ("FF", "অঅ"),
    ("F ", "অ "),
    # a kar with nothing to attach to is kept, not lost
    ("f", "া"),
    ("d", "ি"),
    ("d ", "ি "),
    ("A", "র্"),
    ("Aj", "র্ক"),
]

# Mixed (default), as in Avro 4.5.1's Bijoy layout: a kar pressed once waits for the next
# consonant (classic); pressed twice right after a consonant, it attaches to that consonant.
MIXED_CASES = [
    # হেরেম, three ways
    ("cicvm", "হেরেম"),
    ("icccvm", "হেরেম"),
    ("iccvccm", "হেরেম"),
    # সিকিম, three ways
    ("dndjm", "সিকিম"),
    ("ndddjm", "সিকিম"),
    ("nddjddm", "সিকিম"),
    # কৈকৈ, two ways
    ("CjCj", "কৈকৈ"),
    ("jCCjCC", "কৈকৈ"),
    # বিজয়
    ("dhuW", "বিজয়"),
    ("hdduW", "বিজয়"),
    ("hduW", "বজিয়"),       # single press waits for জ
    # দেহের, typed classic or with double presses
    ("clciv", "দেহের"), ("cliccv", "দেহের"), ("lcciccv", "দেহের"),
    # ঘাসের
    ("Ofcnv", "ঘাসের"),
    ("Ofnccv", "ঘাসের"),
    # কো / কৌ / কি / কে / কৈ
    ("cjf", "কো"), ("jx", "কো"), ("jccf", "কো"), ("jcf", "কো"), ("cjx", "কো"),
    ("cjX", "কৌ"), ("jX", "কৌ"), ("jccX", "কৌ"),
    ("dj", "কি"), ("jdd", "কি"), ("jd", "কি"),
    ("cj", "কে"), ("jcc", "কে"), ("jc", "কে"),
    ("Cj", "কৈ"), ("jCC", "কৈ"),
    # single press mid-word waits for the next consonant, double press attaches
    ("jcVu", "কলেজ"), ("jVccu", "কলেজ"), ("jccVu", "কেলজ"),
    ("mcb", "মনে"), ("mbcc", "মনে"), ("mbc", "মনে"),
    ("hfQVfclM", "বাংলাদেশ"), ("hfQVflccM", "বাংলাদেশ"),
    # juktakkhor
    ("djgk", "ক্তি"), ("jgkdd", "ক্তি"), ("jgkd", "ক্তি"),
    ("cjgNkz", "ক্ষেত্র"), ("jgNcckz", "ক্ষেত্র"),
    ("cMzdB", "শ্রেণি"), ("MzccdB", "শ্রেণি"), ("MzccBdd", "শ্রেণি"),
    ("dhlZfVW", "বিদ্যালয়"), ("hddlZfVW", "বিদ্যালয়"),
    ("LbZhfl", "ধন্যবাদ"),
    # reph
    ("jAd", "র্কি"), ("jAdd", "র্কি"), ("jddA", "র্কি"), ("djA", "র্কি"), ("vgjd", "র্কি"),
    ("mAx", "র্মো"), ("mxA", "র্মো"),
    # other
    ("gfmd", "আমি"), ("gfmdd", "আমি"),
    ("jgd", "কই"),
    ("jcc&", "কেঁ"), ("jc&", "কেঁ"),
    ("d", "ি"),
    ("d ", "ি "),
    ("jcc⌫", "ক"),
    ("jc⌫", "ক"),
    ("hd⌫u", "বজ"),
    ("hdd⌫u", "বজ"),
]


class EngineTests(unittest.TestCase):
    def test_words(self):
        for keys, expected in CASES:
            got = type_keys(keys)
            self.assertEqual(names(got), names(expected), f"keys: {keys} → {got}, expected {expected}")

    def test_mixed_mode(self):
        for keys, expected in MIXED_CASES:
            got = type_keys(keys)
            self.assertEqual(names(got), names(expected), f"keys: {keys} → {got}, expected {expected}")

    def test_backspace_undoes_keystrokes(self):
        self.assertEqual(type_keys("djgk⌫⌫"), "কি")
        self.assertEqual(type_keys("djA⌫"), "কি")
        self.assertEqual(type_keys("cjf⌫"), "কে")
        self.assertEqual(type_keys("jf⌫"), "ক")
        self.assertEqual(type_keys("dj⌫j"), "কি")
        self.assertEqual(type_keys("jf ⌫"), "কা")   # committed text: app-style delete
        self.assertEqual(type_keys("gf⌫"), "")
        self.assertEqual(type_keys("F⌫"), "")       # full vowel is committed; app deletes it

    def test_display_while_composing(self):
        e = Engine()
        self.assertEqual(e.process_character("d").display, "ি")
        self.assertEqual(e.process_character("j").display, "কি")
        self.assertEqual(e.process_character("g").display, "ক্ি")
        self.assertEqual(e.process_character("k").display, "ক্তি")
        nxt = e.process_character("h")
        self.assertEqual(nxt.committed, "ক্তি")
        self.assertEqual(nxt.display, "ব")

    def test_non_bijoy_keys_return_none(self):
        e = Engine()
        self.assertIsNone(e.process_character(" "))
        self.assertIsNone(e.process_character("!"))
        self.assertIsNone(e.process_character("\n"))

    def test_commit_is_idempotent(self):
        e = Engine()
        e.process_character("j")
        self.assertEqual(e.commit().committed, "ক")
        self.assertEqual(e.commit().committed, "")
        self.assertFalse(e.is_composing)


if __name__ == "__main__":
    unittest.main()
