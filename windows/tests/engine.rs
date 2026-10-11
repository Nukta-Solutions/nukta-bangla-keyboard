//! The Bijoy engine against the same cases as Tests/NuktaEngineTests/EngineTests.swift and
//! linux/tests/test_engine.py. A case added there belongs here too.

use nukta_bangla::bijoy::Engine;

/// Plays keys the way the text service does: committed text goes into the document, a key that
/// isn't a Bijoy key finishes the syllable and is typed as itself. `⌫` means backspace.
fn type_keys(keys: &str) -> String {
    let mut engine = Engine::new();
    let mut doc = String::new();
    for ch in keys.chars() {
        if ch == '⌫' {
            if engine.backspace().is_none() {
                doc.pop();
            }
            continue;
        }
        match engine.process_character(ch) {
            Some(out) => doc += &out.committed,
            None => {
                doc += &engine.commit().committed;
                doc.push(ch);
            }
        }
    }
    doc + &engine.commit().committed
}

fn scalars(s: &str) -> String {
    s.chars().map(|c| format!("U+{:04X}", c as u32)).collect::<Vec<_>>().join(" ")
}

fn check(cases: &[(&str, &str)]) {
    let mut failures = Vec::new();
    for (keys, expected) in cases {
        let got = type_keys(keys);
        if got != *expected {
            failures.push(format!("{keys} → {got} ({}), expected {expected} ({})", scalars(&got), scalars(expected)));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn words() {
    check(&[
        // basic kar + reordering of pre-base kars
        ("j", "ক"), ("jf", "কা"), ("dj", "কি"), ("jD", "কী"), ("cj", "কে"), ("Cj", "কৈ"),
        ("cjf", "কো"), ("cjX", "কৌ"), ("js", "কু"), ("jS", "কূ"), ("ja", "কৃ"),
        // juktakkhor
        ("djgk", "ক্তি"), ("jgN", "ক্ষ"), ("ngkz", "স্ত্র"), ("cjgNkz", "ক্ষেত্র"),
        ("nQngjadk", "সংস্কৃতি"), ("gsugughV", "উজ্জ্বল"), ("ug", "জ্"), ("jgg", "ক্\u{200C}"),
        ("uIf", "জঞা"), ("ugIfb", "জ্ঞান"), ("dhugIfb", "বিজ্ঞান"), ("jaNgB", "কৃষ্ণ"),
        ("nmgmfb", "সম্মান"), ("LbZhfl", "ধন্যবাদ"), ("rvDjgNf", "পরীক্ষা"), ("cMzdB", "শ্রেণি"),
        ("cmgr", "ম্পে"),
        // phala
        ("ozfm", "গ্রাম"), ("dhlZfVW", "বিদ্যাল\u{09DF}"), ("hZfQj", "ব্যাংক"), ("gCjZ", "ঐক্য"),
        ("vZfh", "র\u{200D}\u{09CD}যাব"),
        // reph
        ("mA", "র্ম"), ("jmA", "কর্ম"), ("djA", "র্কি"), ("FKA", "অর্থ"), ("jkAarjgN", "কর্তৃপক্ষ"),
        ("gSLghA", "ঊর্ধ্ব"),
        // independent vowels
        ("gf", "আ"), ("gd", "ই"), ("gD", "ঈ"), ("gs", "উ"), ("gS", "ঊ"),
        ("ga", "ঋ"), ("gc", "এ"), ("gC", "ঐ"), ("gX", "ঔ"), ("F", "অ"), ("gx", "ও"), ("x", "\u{09CB}"),
        ("jgd", "কই"), ("gXNL", "ঔষধ"), ("gaB", "ঋণ"), ("gDl", "ঈদ"), ("gxcj", "ওকে"),
        // x after a consonant → ো
        ("jx", "কো"), ("jxb", "কোন"), ("Hfvx", "ভারো"), ("jx&", "কোঁ"), ("mAx", "র্মো"), ("ozx", "গ্রো"),
        ("jgx", "কও"), ("j gx", "ক ও"), ("j x", "ক \u{09CB}"), ("cjx", "কো"), ("jx⌫", "ক"),
        // X after a consonant → ৌ
        ("jX", "কৌ"), ("cjX", "কৌ"), ("mXn", "মৌস"), ("ozX", "গ্রৌ"),
        // signs
        ("hfQVf", "বাংলা"), ("yf&l", "চাঁদ"), ("cjf&", "কোঁ"), ("crX&Yfcbf", "পৌঁছানো"), ("j|", "কঃ"),
        ("iTf\\", "হঠাৎ"),
        // words and sentences
        ("gfmfv ncfbfv hfQVf", "আমার সোনার বাংলা"), ("hfQVfclM", "বাংলাদেশ"), ("mfbsN", "মানুষ"),
        ("HfcVfhfnf", "ভালোবাসা"), ("gfmd hfQVf dVdJ|", "আমি বাংলা লিখিঃ"),
        ("gfmd hfQVf dVdJG", "আমি বাংলা লিখি।"),
        // digits and symbols
        ("123", "১২৩"), ("$100", "৳১০০"), ("j, K.", "ক, থ."), ("gfmd .", "আমি ."),
        // F f → আ
        ("Ff", "আ"), ("Ffm", "আম"), ("FfmfV", "আমাল"), ("Ffmfv ncfbfv", "আমার সোনার"), ("Fj", "অক"),
        ("FF", "অঅ"), ("F ", "অ "),
        // a kar with nothing to attach to is kept, not lost
        ("f", "\u{09BE}"), ("d", "\u{09BF}"), ("d ", "\u{09BF} "), ("A", "র্"), ("Aj", "র্ক"),
    ]);
}

/// As in Avro 4.5.1's Bijoy layout: a kar pressed once waits for the next consonant (classic);
/// pressed twice right after a consonant, it attaches to that consonant.
#[test]
fn mixed_mode() {
    check(&[
        ("cicvm", "হেরেম"), ("icccvm", "হেরেম"), ("iccvccm", "হেরেম"),
        ("dndjm", "সিকিম"), ("ndddjm", "সিকিম"), ("nddjddm", "সিকিম"),
        ("CjCj", "কৈকৈ"), ("jCCjCC", "কৈকৈ"),
        ("dhuW", "বিজ\u{09DF}"), ("hdduW", "বিজ\u{09DF}"), ("hduW", "বজি\u{09DF}"),
        ("clciv", "দেহের"), ("cliccv", "দেহের"), ("lcciccv", "দেহের"),
        ("Ofcnv", "ঘাসের"), ("Ofnccv", "ঘাসের"),
        ("cjf", "কো"), ("jx", "কো"), ("jccf", "কো"), ("jcf", "কো"), ("cjx", "কো"),
        ("cjX", "কৌ"), ("jX", "কৌ"), ("jccX", "কৌ"),
        ("dj", "কি"), ("jdd", "কি"), ("jd", "কি"),
        ("cj", "কে"), ("jcc", "কে"), ("jc", "কে"),
        ("Cj", "কৈ"), ("jCC", "কৈ"),
        ("jcVu", "কলেজ"), ("jVccu", "কলেজ"), ("jccVu", "কেলজ"),
        ("mcb", "মনে"), ("mbcc", "মনে"), ("mbc", "মনে"),
        ("hfQVfclM", "বাংলাদেশ"), ("hfQVflccM", "বাংলাদেশ"),
        ("djgk", "ক্তি"), ("jgkdd", "ক্তি"), ("jgkd", "ক্তি"),
        ("cjgNkz", "ক্ষেত্র"), ("jgNcckz", "ক্ষেত্র"),
        ("cMzdB", "শ্রেণি"), ("MzccdB", "শ্রেণি"), ("MzccBdd", "শ্রেণি"),
        ("dhlZfVW", "বিদ্যাল\u{09DF}"), ("hddlZfVW", "বিদ্যাল\u{09DF}"),
        ("LbZhfl", "ধন্যবাদ"),
        ("jAd", "র্কি"), ("jAdd", "র্কি"), ("jddA", "র্কি"), ("djA", "র্কি"), ("vgjd", "র্কি"),
        ("mAx", "র্মো"), ("mxA", "র্মো"),
        ("gfmd", "আমি"), ("gfmdd", "আমি"),
        ("jgd", "কই"),
        ("jcc&", "কেঁ"), ("jc&", "কেঁ"),
        ("d", "\u{09BF}"), ("d ", "\u{09BF} "),
        ("jcc⌫", "ক"), ("jc⌫", "ক"), ("hd⌫u", "বজ"), ("hdd⌫u", "বজ"),
    ]);
}

#[test]
fn backspace_undoes_keystrokes() {
    check(&[
        ("djgk⌫⌫", "কি"),
        ("djA⌫", "কি"),
        ("cjf⌫", "কে"),
        ("jf⌫", "ক"),
        ("dj⌫j", "কি"),
        ("jf ⌫", "কা"), // committed text: app-style delete
        ("gf⌫", ""),
        ("F⌫", ""), // full vowel is committed; app deletes it
    ]);
}

#[test]
fn display_while_composing() {
    let mut e = Engine::new();
    assert_eq!(e.process_character('d').unwrap().display, "\u{09BF}");
    assert_eq!(e.process_character('j').unwrap().display, "কি");
    assert_eq!(e.process_character('g').unwrap().display, "ক্\u{09BF}");
    assert_eq!(e.process_character('k').unwrap().display, "ক্তি");
    let next = e.process_character('h').unwrap();
    assert_eq!(next.committed, "ক্তি");
    assert_eq!(next.display, "ব");
}

/// What goes on screen: a kar, ্ or অ with nothing to sit on stays off it until the next key.
#[test]
fn visible_holds_back_what_has_no_consonant_yet() {
    let mut e = Engine::new();
    assert_eq!(e.process_character('d').unwrap().visible, "");
    assert_eq!(e.process_character('j').unwrap().visible, "কি");
    assert_eq!(e.process_character('g').unwrap().visible, "কি");
    let mut e = Engine::new();
    e.process_character('j');
    let waiting = e.process_character('c').unwrap();
    assert_eq!((waiting.display.as_str(), waiting.visible.as_str()), ("ক", "ক"));
    assert_eq!(e.process_character('c').unwrap().visible, "কে");
}

#[test]
fn non_bijoy_keys_return_none() {
    let mut e = Engine::new();
    assert!(e.process_character(' ').is_none());
    assert!(e.process_character('!').is_none());
    assert!(e.process_character('\n').is_none());
}
