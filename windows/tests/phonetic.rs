//! The phonetic composer against the same cases as Tests/NuktaPhoneticTests/PhoneticComposerTests.swift
//! and linux/tests/test_phonetic.py. A case added there belongs here too.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use nukta_bangla::phonetic::{contains_emoji, Key, Output, PhoneticComposer, PhoneticOptions, TypingMode};

fn fresh_directory() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("nukta-windows-tests-{}-{n}", std::process::id()))
}

/// Plays keys into a composer the way the text service does, keeping the app's text: committed
/// text, plus the composition still being typed.
struct Rig {
    composer: PhoneticComposer,
    doc: String,
    marked: String,
}

impl Rig {
    fn new() -> Self {
        Self::with(|_| {})
    }

    fn with(configure: impl FnOnce(&mut PhoneticOptions)) -> Self {
        Self::in_directory(configure, fresh_directory())
    }

    fn in_directory(configure: impl FnOnce(&mut PhoneticOptions), directory: PathBuf) -> Self {
        let mut options = PhoneticOptions::default();
        configure(&mut options);
        Rig { composer: PhoneticComposer::new(options, &directory), doc: String::new(), marked: String::new() }
    }

    fn apply(&mut self, out: Output, app_text: &str) {
        self.doc += &out.insert;
        self.marked = out.marked;
        if !out.handled {
            self.doc += app_text;
        }
    }

    /// Letters, digits and punctuation; `␣` space, `⏎` Enter, `⎋` Escape, `⌫` backspace,
    /// `↓`/`↑` move the selection.
    fn type_keys(&mut self, keys: &str) -> &mut Self {
        for ch in keys.chars() {
            match ch {
                '␣' => { let out = self.composer.handle(Key::Space); self.apply(out, " ") }
                '⏎' => { let out = self.composer.handle(Key::Enter); self.apply(out, "\n") }
                '⎋' => { let out = self.composer.handle(Key::Escape); self.apply(out, "") }
                '⌫' => {
                    let out = self.composer.handle(Key::Backspace { whole_word: false });
                    if out.handled { self.apply(out, "") } else { self.doc.pop(); }
                }
                '↓' => { let out = self.composer.handle(Key::Down); self.apply(out, "") }
                '↑' => { let out = self.composer.handle(Key::Up); self.apply(out, "") }
                _ => {
                    let out = self.composer.handle(Key::Character(ch, ch.is_uppercase()));
                    self.apply(out, &ch.to_string());
                }
            }
        }
        self
    }

    /// Everything on screen once the word in progress is committed.
    fn finished(&mut self) -> String {
        let out = self.composer.commit();
        self.apply(out, "");
        self.doc.clone()
    }

    fn candidates(&self) -> &[String] {
        self.composer.candidates()
    }

    fn index_of(&self, word: &str) -> usize {
        self.candidates().iter().position(|c| c == word).unwrap_or_else(|| panic!("{word} not in {:?}", self.candidates()))
    }
}

#[test]
fn phonetic_first_commits_the_transliteration() {
    assert_eq!(Rig::new().type_keys("ami␣banglay␣likhchi").finished(), "আমি বাংলা\u{09DF} লিখছি");
    // Without dictionary help, `sonar` is সনার, exactly as typed.
    assert_eq!(Rig::new().type_keys("sonar␣").doc, "সনার ");
}

#[test]
fn marked_text_follows_the_word() {
    let mut rig = Rig::new();
    rig.type_keys("kh");
    assert_eq!(rig.marked, "খ");
    assert_eq!(rig.doc, "");
    rig.type_keys("i");
    assert_eq!(rig.marked, "খি");
    assert_eq!(rig.candidates()[0], "খি");
    assert_eq!(rig.composer.auxiliary(), "khi");
}

#[test]
fn list_offers_dictionary_words() {
    let mut rig = Rig::new();
    rig.type_keys("sonar");
    assert_eq!(rig.candidates()[0], "সনার");
    assert!(rig.candidates().iter().any(|c| c == "সোনার"), "{:?}", rig.candidates());
    assert_eq!(rig.composer.selected_index(), 0);
}

#[test]
fn number_key_picks_a_candidate() {
    let mut rig = Rig::new();
    rig.type_keys("sonar");
    let index = rig.index_of("সোনার");
    rig.type_keys(&(index + 1).to_string());
    assert_eq!(rig.doc, "সোনার");
    assert!(!rig.composer.is_composing());
}

#[test]
fn arrows_move_the_selection_and_enter_commits_without_newline() {
    let mut rig = Rig::new();
    rig.type_keys("sonar↓");
    assert_eq!(rig.composer.selected_index(), 1);
    let second = rig.candidates()[1].clone();
    assert_eq!(rig.marked, second);
    rig.type_keys("↑↓⏎");
    assert_eq!(rig.doc, second);
}

#[test]
fn phonetic_first_remembers_a_pick() {
    let directory = fresh_directory();
    let mut rig = Rig::in_directory(|_| {}, directory.clone());
    rig.type_keys("sonar");
    let index = rig.index_of("সোনার");
    rig.type_keys(&(index + 1).to_string());
    rig.type_keys("␣sonar␣");
    assert_eq!(rig.doc, "সোনার সোনার ");

    // Another app (another process, here another composer on the same directory) sees the pick.
    assert_eq!(Rig::in_directory(|_| {}, directory).type_keys("sonar␣").doc, "সোনার ");

    // Picking the plain transliteration again forgets it.
    rig.type_keys("sonar1␣sonar␣");
    assert_eq!(rig.doc, "সোনার সোনার সনার সনার ");
}

#[test]
fn smart_mode_commits_the_dictionary_word() {
    assert_eq!(Rig::with(|o| o.mode = TypingMode::Smart).type_keys("sonar␣").doc, "সোনার ");
}

#[test]
fn autocorrect_switch() {
    // riti's autocorrect list spells `account` as `oZakaunT` (অ্যাকাউন্ট).
    let mut on = Rig::with(|o| o.mode = TypingMode::Smart);
    on.type_keys("account");
    assert_eq!(on.candidates()[0], "অ্যাকাউন্ট");
    assert_eq!(on.type_keys("␣").doc, "অ্যাকাউন্ট ");

    let mut off = Rig::with(|o| {
        o.mode = TypingMode::Smart;
        o.autocorrect = false;
    });
    off.type_keys("account");
    assert!(!off.candidates().iter().any(|c| c == "অ্যাকাউন্ট"), "{:?}", off.candidates());
    assert_eq!(off.type_keys("␣").doc, "আচ্চউন্ত ");
}

#[test]
fn phonetic_only_has_no_list() {
    let mut rig = Rig::with(|o| o.mode = TypingMode::PhoneticOnly);
    rig.type_keys("sonar");
    assert_eq!(rig.marked, "সনার");
    assert!(rig.candidates().is_empty());
    rig.type_keys("↓");
    assert_eq!(rig.doc, "সনার");
}

#[test]
fn escape_drops_the_word() {
    let mut rig = Rig::new();
    rig.type_keys("ami␣sonar⎋");
    assert_eq!(rig.doc, "আমি ");
    assert_eq!(rig.marked, "");
    assert!(!rig.composer.is_composing());
}

#[test]
fn backspace_edits_the_word_then_the_app() {
    assert_eq!(Rig::new().type_keys("khi⌫a").finished(), "খা");
    assert_eq!(Rig::new().type_keys("ami␣⌫").finished(), "আমি");
}

#[test]
fn digits_are_bangla_outside_a_word() {
    assert_eq!(Rig::new().type_keys("2024␣").doc, "২০২৪ ");
}

#[test]
fn punctuation_ends_a_word() {
    assert_eq!(Rig::new().type_keys("ki?").finished(), "কি?");
    assert_eq!(Rig::new().type_keys("ami.").finished(), "আমি।");
}

#[test]
fn emoji_only_by_explicit_pick_and_can_be_hidden() {
    let mut with = Rig::new();
    with.type_keys("hasi");
    assert!(!contains_emoji(&with.candidates()[with.composer.selected_index()]));
    let mut without = Rig::with(|o| o.emoji = false);
    without.type_keys("hasi");
    assert!(!without.candidates().iter().any(|c| contains_emoji(c)));
}

/// The examples in README.md.
#[test]
fn readme_examples() {
    assert_eq!(Rig::new().type_keys("kotha␣dhorrmo␣prem").finished(), "কথা ধর্ম প্রেম");
    assert!(Rig::new().type_keys("hasi").candidates().iter().any(|c| c == "😄"));
}

#[test]
fn text_symbols_shown_as_emoji_count_as_emoji() {
    assert!(contains_emoji("©️"));
    assert!(contains_emoji("😄"));
    assert!(!contains_emoji("©"));
    assert!(!contains_emoji("সোনার"));
    assert!(!contains_emoji("১২#*"));
}

#[test]
fn smart_mode_never_commits_an_emoji_by_default() {
    // riti offers ©️ for `st`.
    let doc = Rig::with(|o| o.mode = TypingMode::Smart).type_keys("st␣").doc.clone();
    assert!(!contains_emoji(&doc), "{doc}");
}

#[test]
fn colon_types_a_colon_by_default() {
    assert_eq!(Rig::new().type_keys("somoy:␣10").finished(), "সম\u{09DF}: ১০");
    assert_eq!(Rig::new().type_keys(":").finished(), ":");
    // ঃ words still come from the list.
    assert!(Rig::new().type_keys("dukho").candidates().iter().any(|c| c == "দুঃখ"));
}

#[test]
fn colon_can_type_bisarga() {
    assert_eq!(Rig::with(|o| o.colon_is_bisarga = true).type_keys("du:kho").finished(), "দুঃখ");
}

#[test]
fn display_only_options_keep_the_word() {
    let mut rig = Rig::new();
    rig.type_keys("sonar");
    let mut options = rig.composer.options().clone();
    options.colon_is_bisarga = true;
    options.emoji = false;
    rig.composer.update(options);
    assert!(rig.composer.is_composing());
}

#[test]
fn options_change_drops_the_word() {
    let mut rig = Rig::new();
    rig.type_keys("sonar");
    let mut options = rig.composer.options().clone();
    options.mode = TypingMode::Smart;
    rig.composer.update(options);
    assert!(!rig.composer.is_composing());
    assert_eq!(rig.type_keys("sonar␣").doc, "সোনার ");
}

/// Smart mode: what one app's riti learns, another app's riti picks up once it has the keyboard
/// again, instead of overwriting it.
#[test]
fn learning_in_another_process_is_picked_up() {
    let directory = fresh_directory();
    let smart = |o: &mut PhoneticOptions| o.mode = TypingMode::Smart;
    let mut first = Rig::in_directory(smart, directory.clone());
    let mut second = Rig::in_directory(smart, directory);

    first.type_keys("sonar");
    let other = (0..first.candidates().len()).find(|&i| i != first.composer.selected_index()).unwrap();
    let picked = first.candidates()[other].clone();
    first.type_keys(&(other + 1).to_string());

    second.composer.reload_if_learned_elsewhere();
    second.type_keys("sonar");
    assert_eq!(second.marked, picked);
}
