//! Phonetic typing: a port of `Sources/NuktaPhonetic/PhoneticComposer.swift` (and of
//! `linux/nukta_bangla/phonetic.py`). Pure logic, no Windows APIs: the TSF layer shows `Output`
//! and `candidates`.
//!
//! Keep the three copies in step: a rule changed here is changed in Swift and Python too.

pub mod pick_memory;
pub mod riti;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use pick_memory::PickMemory;
use riti::{Riti, RitiSuggestion};

/// How phonetic typing picks the word that Space commits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TypingMode {
    /// What was typed, transliterated; a word picked from the list instead is preferred next time.
    #[default]
    PhoneticFirst,
    /// riti's own choice: dictionary, autocorrect and past picks.
    Smart,
    /// Transliteration only, no list.
    PhoneticOnly,
}

impl TypingMode {
    pub const ALL: [TypingMode; 3] = [TypingMode::PhoneticFirst, TypingMode::Smart, TypingMode::PhoneticOnly];

    /// The settings file's name for the mode: the same as macOS and Linux.
    pub fn name(self) -> &'static str {
        match self {
            TypingMode::PhoneticFirst => "phoneticFirst",
            TypingMode::Smart => "smart",
            TypingMode::PhoneticOnly => "phoneticOnly",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.name() == name)
    }
}

/// The settings phonetic typing depends on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhoneticOptions {
    pub mode: TypingMode,
    pub autocorrect: bool,
    pub emoji: bool,
    /// List the typed roman word itself.
    pub english_word: bool,
    /// False: ":" types a colon. True: it types ঃ, as in Avro.
    pub colon_is_bisarga: bool,
}

impl Default for PhoneticOptions {
    fn default() -> Self {
        PhoneticOptions { mode: TypingMode::PhoneticFirst, autocorrect: true, emoji: true, english_word: true, colon_is_bisarga: false }
    }
}

/// One keystroke, as the TSF layer reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// A printable key, as on a US layout.
    Character(char, bool),
    /// Backspace, or Ctrl+Backspace for the whole word.
    Backspace { whole_word: bool },
    Enter,
    Escape,
    Space,
    Tab { backward: bool },
    Up,
    Down,
    Left,
    Right,
}

/// What one key does to the app's text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    /// Final text to insert, replacing the composition.
    pub insert: String,
    /// Composition (underlined) text to show afterwards; "" = none.
    pub marked: String,
    /// False: the app must also handle the key (space, arrows…).
    pub handled: bool,
}

impl Default for Output {
    fn default() -> Self {
        Output { insert: String::new(), marked: String::new(), handled: true }
    }
}

impl Output {
    fn insert(text: String) -> Self {
        Output { insert: text, ..Output::default() }
    }

    fn marked(text: String) -> Self {
        Output { marked: text, ..Output::default() }
    }

    fn unhandled() -> Self {
        Output { handled: false, ..Output::default() }
    }
}

/// Keys riti handles as punctuation: typed after the user moved the selection, riti keeps the
/// selection instead of picking its own default (riti-bridge/riti/src/phonetic/method.rs).
const SELECTION_KEEPERS: &str = ".?!,:;-_)}]'\"";

/// Turns keys into Bangla with riti, and keeps the suggestion list for the word in progress.
///
/// riti contexts are expensive, and each one rewrites riti's file of learned picks from its own
/// copy, so each process keeps one composer.
pub struct PhoneticComposer {
    options: PhoneticOptions,
    /// ← → also move the selection (the list is a row).
    pub horizontal_navigation: bool,
    /// The list to show; empty = no list.
    candidates: Vec<String>,
    selected_index: usize,
    /// What was typed (roman), shown above the list.
    auxiliary: String,

    directory: PathBuf,
    riti: Riti,
    /// Phonetic-first only: a context with suggestions off, fed the same keys, whose (lonely) answer
    /// is the literal transliteration. riti's list contains it but doesn't say which entry it is.
    literal_riti: Option<Riti>,
    picks: PickMemory,
    /// riti's file of learned picks as of our contexts' creation or our own last write to it.
    selection_stamp: Option<SystemTime>,

    /// The word when riti gives no list (phonetic-only mode, a lone punctuation mark).
    lonely: String,
    /// For each candidate, its index in riti's own list (the order differs after filtering and
    /// moving the literal up).
    riti_indices: Vec<usize>,
    /// Index in `candidates` of the literal transliteration, if it is there.
    literal_index: Option<usize>,
    /// The literal transliteration of what was typed, from `literal_riti`.
    literal: String,
    /// The user moved the selection since the last key that went to riti.
    selection_moved: bool,
}

impl PhoneticComposer {
    /// `directory`: where riti and the pick memory keep what they learn; created if missing.
    pub fn new(options: PhoneticOptions, directory: &Path) -> Self {
        let _ = fs::create_dir_all(directory);
        let (riti, literal_riti) = make_contexts(&options, directory);
        PhoneticComposer {
            horizontal_navigation: false,
            candidates: Vec::new(),
            selected_index: 0,
            auxiliary: String::new(),
            directory: directory.to_owned(),
            riti,
            literal_riti,
            picks: PickMemory::new(directory),
            selection_stamp: selection_modified(directory),
            lonely: String::new(),
            riti_indices: Vec::new(),
            literal_index: None,
            literal: String::new(),
            selection_moved: false,
            options,
        }
    }

    pub fn options(&self) -> &PhoneticOptions {
        &self.options
    }

    pub fn candidates(&self) -> &[String] {
        &self.candidates
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn auxiliary(&self) -> &str {
        &self.auxiliary
    }

    /// A word is in progress.
    pub fn is_composing(&self) -> bool {
        !self.candidates.is_empty() || !self.lonely.is_empty()
    }

    /// A change to the mode, autocorrect or the English word rebuilds riti and drops the word in
    /// progress. Emoji and the colon apply from the next key.
    pub fn update(&mut self, options: PhoneticOptions) {
        let rebuild = options.mode != self.options.mode
            || options.autocorrect != self.options.autocorrect
            || options.english_word != self.options.english_word;
        self.options = options;
        if rebuild {
            self.rebuild();
        }
    }

    /// Another process (another app typing phonetically) has rewritten riti's learned picks since
    /// this one read them: rebuild riti so it learns on top of them rather than overwriting them.
    /// Only between words; the TSF layer calls it when an app gets the keyboard.
    pub fn reload_if_learned_elsewhere(&mut self) {
        if !self.is_composing() && selection_modified(&self.directory) != self.selection_stamp {
            self.rebuild();
        }
    }

    pub fn handle(&mut self, key: Key) -> Output {
        match key {
            Key::Character(character, shift) => self.type_character(character, shift),
            Key::Backspace { whole_word } => self.backspace(whole_word),
            Key::Enter => {
                if !self.is_composing() {
                    return Output::unhandled();
                }
                self.commit()
            }
            Key::Escape => {
                if !self.is_composing() {
                    return Output::unhandled();
                }
                self.discard();
                Output::default()
            }
            Key::Space => {
                if !self.is_composing() {
                    return Output::unhandled();
                }
                let mut out = self.commit();
                out.handled = false;
                out
            }
            Key::Tab { backward } => self.move_selection(if backward { -1 } else { 1 }),
            Key::Down => self.move_selection(1),
            Key::Up => self.move_selection(-1),
            Key::Left | Key::Right => {
                if self.horizontal_navigation {
                    return self.move_selection(if key == Key::Left { -1 } else { 1 });
                }
                self.commit_and_pass()
            }
        }
    }

    /// Commits the selected word (a click elsewhere, a switch to another app).
    pub fn commit(&mut self) -> Output {
        if !self.is_composing() {
            return Output::default();
        }
        if self.candidates.is_empty() {
            let text = std::mem::take(&mut self.lonely);
            self.end_word(None);
            return Output::insert(text);
        }
        self.commit_at(self.selected_index)
    }

    /// Commits `candidates[index]` (a click in the list).
    pub fn commit_at(&mut self, index: usize) -> Output {
        if index >= self.candidates.len() {
            return if self.is_composing() { Output::marked(self.marked()) } else { Output::default() };
        }
        let text = self.candidates[index].clone();
        if self.options.mode == TypingMode::PhoneticFirst {
            if let Some(literal_index) = self.literal_index {
                if index == literal_index || pick_memory::trim(&text) == pick_memory::trim(&self.literal) {
                    self.picks.forget(&self.auxiliary);
                } else if !contains_emoji(&text) {
                    self.picks.remember(&text, &self.auxiliary);
                }
            }
        }
        let learning = (self.options.mode == TypingMode::Smart).then(|| self.riti_indices[index]);
        self.end_word(learning);
        Output::insert(text)
    }

    /// Drops the word in progress, with no output.
    pub fn discard(&mut self) {
        self.end_word(None);
    }

    // MARK: Keys

    fn type_character(&mut self, character: char, shift: bool) -> Output {
        if character == ':' && !self.options.colon_is_bisarga {
            let mut text = self.commit().insert;
            text.push(':');
            return Output::insert(text);
        }

        if let Some(digit) = character.to_digit(10).filter(|_| character.is_ascii()) {
            if !self.is_composing() {
                return Output::insert(bangla_digit(digit).to_string());
            }
            if (1..=9).contains(&digit) {
                let digit = digit as usize;
                if !self.candidates.is_empty() && digit <= self.candidates.len() {
                    return self.commit_at(digit - 1);
                }
                if self.candidates.is_empty() {
                    let mut text = self.commit().insert;
                    text.push(bangla_digit(digit as u32));
                    return Output::insert(text);
                }
            }
        }

        let Some(code) = riti::keycode(character) else {
            return self.commit_and_pass();
        };

        // After the user moved the selection, punctuation keeps it: riti is told which entry it was.
        let keep_selection =
            self.selection_moved && !self.candidates.is_empty() && SELECTION_KEEPERS.contains(character);
        if !keep_selection {
            self.selection_moved = false;
        }
        let selection = if keep_selection { self.riti_indices[self.selected_index] } else { 0 };

        // riti only reports what was typed with a list; this keeps it for a lonely answer too.
        self.auxiliary.push(character);
        let suggestion = self.riti.key(code, shift, selection);
        if let Some(literal_riti) = &self.literal_riti {
            if let RitiSuggestion::Lonely(text) = literal_riti.key(code, shift, 0) {
                self.literal = text;
            }
        }
        self.show(suggestion, keep_selection);

        if !self.riti.is_composing() {
            // riti ended the word itself: what it produced is final.
            let text = self.marked();
            self.end_word(None);
            return Output::insert(text);
        }
        Output::marked(self.marked())
    }

    fn backspace(&mut self, whole_word: bool) -> Output {
        if !self.is_composing() {
            return Output::unhandled();
        }
        self.selection_moved = false;
        if whole_word {
            self.auxiliary.clear();
        } else {
            self.auxiliary.pop();
        }
        let suggestion = self.riti.backspace(whole_word);
        if let Some(literal_riti) = &self.literal_riti {
            self.literal = match literal_riti.backspace(whole_word) {
                RitiSuggestion::Lonely(text) => text,
                _ => String::new(),
            };
        }
        if suggestion == RitiSuggestion::Empty || !self.riti.is_composing() {
            self.end_word(None);
            return Output::default();
        }
        self.show(suggestion, false);
        Output::marked(self.marked())
    }

    /// Up/down, Tab: move the selection, wrapping. With no list, the word is committed and the key
    /// goes to the app.
    fn move_selection(&mut self, step: isize) -> Output {
        if !self.is_composing() {
            return Output::unhandled();
        }
        if self.candidates.is_empty() {
            return self.commit_and_pass();
        }
        let count = self.candidates.len() as isize;
        self.selected_index = ((self.selected_index as isize + step + count) % count) as usize;
        self.selection_moved = true;
        Output::marked(self.marked())
    }

    fn commit_and_pass(&mut self) -> Output {
        let mut out = self.commit();
        out.handled = false;
        out
    }

    // MARK: State

    /// The composition text: the selected candidate, or the lonely text when there is no list.
    fn marked(&self) -> String {
        self.candidates.get(self.selected_index).cloned().unwrap_or_else(|| self.lonely.clone())
    }

    /// Builds the list from riti's answer.
    fn show(&mut self, suggestion: RitiSuggestion, kept_selection: bool) {
        match suggestion {
            RitiSuggestion::Empty => self.clear_list(),
            RitiSuggestion::Lonely(text) => {
                self.clear_list();
                self.lonely = text;
            }
            RitiSuggestion::List { entries, auxiliary, selection } => {
                self.lonely.clear();
                self.auxiliary = auxiliary;
                self.build_list(entries, selection, kept_selection);
            }
        }
    }

    fn build_list(&mut self, entries: Vec<String>, riti_selection: usize, kept_selection: bool) {
        let mut items: Vec<(String, usize)> = entries.into_iter().enumerate().map(|(i, t)| (t, i)).collect();
        if !self.options.emoji {
            let plain: Vec<_> = items.iter().filter(|(t, _)| !contains_emoji(t)).cloned().collect();
            if !plain.is_empty() {
                items = plain;
            }
        }

        self.literal_index = None;
        if self.options.mode == TypingMode::PhoneticFirst && !self.literal.is_empty() {
            let target = straightened(&self.literal);
            if let Some(found) = items.iter().position(|(t, _)| straightened(t) == target) {
                let item = items.remove(found);
                items.insert(0, item);
                self.literal_index = Some(0);
            }
        }

        self.candidates = items.iter().map(|(t, _)| t.clone()).collect();
        self.riti_indices = items.iter().map(|(_, i)| *i).collect();
        if self.candidates.is_empty() {
            self.selected_index = 0;
            return;
        }
        let riti_default = self.riti_indices.iter().position(|&i| i == riti_selection).unwrap_or(0);

        let mut selection = if kept_selection || self.options.mode != TypingMode::PhoneticFirst {
            riti_default
        } else if let Some(literal_index) = self.literal_index {
            let mut selection = literal_index;
            if let Some(pick) = self.picks.pick(&self.auxiliary) {
                if let Some(picked) = self.candidates.iter().position(|c| pick_memory::trim(c) == pick) {
                    selection = picked;
                }
            }
            selection
        } else {
            riti_default
        };

        // Space must never commit an emoji nobody chose. A typed emoticon (`:)`) is exempt.
        let typed_letter = self.auxiliary.chars().next().is_some_and(char::is_alphabetic);
        if !kept_selection && typed_letter && contains_emoji(&self.candidates[selection]) {
            if let Some(plain) = self.candidates.iter().position(|c| !contains_emoji(c)) {
                selection = plain;
            }
        }
        self.selected_index = selection;
    }

    fn clear_list(&mut self) {
        self.candidates.clear();
        self.riti_indices.clear();
        self.literal_index = None;
        self.selected_index = 0;
    }

    /// Ends the word in both contexts and resets. `learning`: riti index of the pick for riti to
    /// learn (smart mode).
    fn end_word(&mut self, learning: Option<usize>) {
        match learning {
            Some(index) => {
                self.riti.committed(index);
                // riti may have rewritten its file of learned picks: that is our own change.
                self.selection_stamp = selection_modified(&self.directory);
            }
            None => self.riti.finish(),
        }
        if let Some(literal_riti) = &self.literal_riti {
            literal_riti.finish();
        }
        self.clear_list();
        self.lonely.clear();
        self.auxiliary.clear();
        self.literal.clear();
        self.selection_moved = false;
    }

    fn rebuild(&mut self) {
        self.discard();
        self.literal_riti = None;
        let (riti, literal_riti) = make_contexts(&self.options, &self.directory);
        self.riti = riti;
        self.literal_riti = literal_riti;
        self.selection_stamp = selection_modified(&self.directory);
    }
}

// MARK: Helpers

fn make_contexts(options: &PhoneticOptions, directory: &Path) -> (Riti, Option<Riti>) {
    let main = Riti::new(directory, options.mode != TypingMode::PhoneticOnly, options.english_word, options.autocorrect);
    let literal = (options.mode == TypingMode::PhoneticFirst).then(|| Riti::new(directory, false, false, false));
    (main, literal)
}

fn selection_modified(directory: &Path) -> Option<SystemTime> {
    fs::metadata(directory.join(riti::SELECTION_FILE)).and_then(|m| m.modified()).ok()
}

/// riti's list has curly quotes where the literal transliteration has straight ones.
fn straightened(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '“' | '”' => '"',
            '‘' | '’' => '\'',
            other => other,
        })
        .collect()
}

fn bangla_digit(digit: u32) -> char {
    char::from_u32(0x09E6 + digit).unwrap_or('০')
}

/// Turns a text symbol like © into ©️.
const VARIATION_SELECTOR_16: u32 = 0xFE0F;

/// Scalars that count as emoji: `Emoji=Yes` from Unicode's emoji-data, keeping only U+2000 and
/// above: the same table as linux/nukta_bangla/phonetic.py. Below U+2000 nothing is
/// Emoji_Presentation, which is why digits, `#`, `*` and a plain © are not emoji here; only ©
/// followed by U+FE0F is.
const EMOJI_RANGES: &[(u32, u32)] = &[
    (0x203C, 0x203C), (0x2049, 0x2049), (0x2122, 0x2122), (0x2139, 0x2139),
    (0x2194, 0x2199), (0x21A9, 0x21AA), (0x231A, 0x231B), (0x2328, 0x2328),
    (0x23CF, 0x23CF), (0x23E9, 0x23F3), (0x23F8, 0x23FA), (0x24C2, 0x24C2),
    (0x25AA, 0x25AB), (0x25B6, 0x25B6), (0x25C0, 0x25C0), (0x25FB, 0x25FE),
    (0x2600, 0x2604), (0x260E, 0x260E), (0x2611, 0x2611), (0x2614, 0x2615),
    (0x2618, 0x2618), (0x261D, 0x261D), (0x2620, 0x2620), (0x2622, 0x2623),
    (0x2626, 0x2626), (0x262A, 0x262A), (0x262E, 0x262F), (0x2638, 0x263A),
    (0x2640, 0x2640), (0x2642, 0x2642), (0x2648, 0x2653), (0x265F, 0x2660),
    (0x2663, 0x2663), (0x2665, 0x2666), (0x2668, 0x2668), (0x267B, 0x267B),
    (0x267E, 0x267F), (0x2692, 0x2697), (0x2699, 0x2699), (0x269B, 0x269C),
    (0x26A0, 0x26A1), (0x26A7, 0x26A7), (0x26AA, 0x26AB), (0x26B0, 0x26B1),
    (0x26BD, 0x26BE), (0x26C4, 0x26C5), (0x26C8, 0x26C8), (0x26CE, 0x26CF),
    (0x26D1, 0x26D1), (0x26D3, 0x26D4), (0x26E9, 0x26EA), (0x26F0, 0x26F5),
    (0x26F7, 0x26FA), (0x26FD, 0x26FD), (0x2702, 0x2702), (0x2705, 0x2705),
    (0x2708, 0x270D), (0x270F, 0x270F), (0x2712, 0x2712), (0x2714, 0x2714),
    (0x2716, 0x2716), (0x271D, 0x271D), (0x2721, 0x2721), (0x2728, 0x2728),
    (0x2733, 0x2734), (0x2744, 0x2744), (0x2747, 0x2747), (0x274C, 0x274C),
    (0x274E, 0x274E), (0x2753, 0x2755), (0x2757, 0x2757), (0x2763, 0x2764),
    (0x2795, 0x2797), (0x27A1, 0x27A1), (0x27B0, 0x27B0), (0x27BF, 0x27BF),
    (0x2934, 0x2935), (0x2B05, 0x2B07), (0x2B1B, 0x2B1C), (0x2B50, 0x2B50),
    (0x2B55, 0x2B55), (0x3030, 0x3030), (0x303D, 0x303D), (0x3297, 0x3297),
    (0x3299, 0x3299),
    (0x1F004, 0x1F004), (0x1F0CF, 0x1F0CF), (0x1F170, 0x1F171), (0x1F17E, 0x1F17F),
    (0x1F18E, 0x1F18E), (0x1F191, 0x1F19A), (0x1F1E6, 0x1F1FF), (0x1F201, 0x1F202),
    (0x1F21A, 0x1F21A), (0x1F22F, 0x1F22F), (0x1F232, 0x1F23A), (0x1F250, 0x1F251),
    (0x1F300, 0x1F321), (0x1F324, 0x1F393), (0x1F396, 0x1F397), (0x1F399, 0x1F39B),
    (0x1F39E, 0x1F3F0), (0x1F3F3, 0x1F3F5), (0x1F3F7, 0x1F4FD), (0x1F4FF, 0x1F53D),
    (0x1F549, 0x1F54E), (0x1F550, 0x1F567), (0x1F56F, 0x1F570), (0x1F573, 0x1F57A),
    (0x1F587, 0x1F587), (0x1F58A, 0x1F58D), (0x1F590, 0x1F590), (0x1F595, 0x1F596),
    (0x1F5A4, 0x1F5A5), (0x1F5A8, 0x1F5A8), (0x1F5B1, 0x1F5B2), (0x1F5BC, 0x1F5BC),
    (0x1F5C2, 0x1F5C4), (0x1F5D1, 0x1F5D3), (0x1F5DC, 0x1F5DE), (0x1F5E1, 0x1F5E1),
    (0x1F5E3, 0x1F5E3), (0x1F5E8, 0x1F5E8), (0x1F5EF, 0x1F5EF), (0x1F5F3, 0x1F5F3),
    (0x1F5FA, 0x1F64F), (0x1F680, 0x1F6C5), (0x1F6CB, 0x1F6D2), (0x1F6D5, 0x1F6D7),
    (0x1F6DC, 0x1F6E5), (0x1F6E9, 0x1F6E9), (0x1F6EB, 0x1F6EC), (0x1F6F0, 0x1F6F0),
    (0x1F6F3, 0x1F6FC), (0x1F7E0, 0x1F7EB), (0x1F7F0, 0x1F7F0), (0x1F90C, 0x1F93A),
    (0x1F93C, 0x1F945), (0x1F947, 0x1F9FF), (0x1FA70, 0x1FA7C), (0x1FA80, 0x1FA88),
    (0x1FA90, 0x1FABD), (0x1FABF, 0x1FAC5), (0x1FACE, 0x1FADB), (0x1FAE0, 0x1FAE8),
    (0x1FAF0, 0x1FAF8),
];

/// Any emoji, including a text symbol made to show as one (© + U+FE0F = ©️).
pub fn contains_emoji(text: &str) -> bool {
    text.chars().any(|c| {
        let code = c as u32;
        code == VARIATION_SELECTOR_16
            || (code >= 0x2000 && EMOJI_RANGES.iter().any(|&(start, end)| (start..=end).contains(&code)))
    })
}
