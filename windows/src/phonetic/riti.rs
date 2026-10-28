//! riti (Avro Phonetic) behind a small interface: the counterpart of
//! `Sources/NuktaPhonetic/Riti.swift` and `linux/nukta_bangla/riti.py`.
//!
//! macOS and Linux reach riti through its C API; here the input method is Rust too, so riti is
//! used as the Rust library it is. riti panics on some failures (an unwritable or malformed user
//! file), so the TSF layer runs every composer call under `catch_unwind`; this module only avoids
//! the panics it can see coming.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use riti::config::Config;
use riti::context::RitiContext;
use riti::suggestion::Suggestion;

/// riti's learned picks and the user's autocorrect entries, inside the user directory.
pub const USER_FILES: [&str; 2] = ["phonetic-candidate-selection.json", "autocorrect.json"];
/// The file riti rewrites when it learns a pick (smart mode).
pub const SELECTION_FILE: &str = USER_FILES[0];

const MODIFIER_SHIFT: u8 = 1 << 0;

/// One riti answer, copied out of riti.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RitiSuggestion {
    Empty,
    /// A single string and no list: phonetic-only mode, or a lone punctuation mark.
    Lonely(String),
    /// `selection`: riti's default pick (a past choice, or the selection passed in for punctuation).
    List { entries: Vec<String>, auxiliary: String, selection: usize },
}

pub struct Riti {
    context: RitiContext,
}

impl Riti {
    /// riti reads and writes its user files in `directory`. Where that can't be reached (a Store app
    /// the directory isn't shared with), the temporary directory stands in: riti panics when it
    /// can't write its file of learned picks, and a fresh start beats no phonetic typing.
    /// `suggestions` false gives lonely transliterations only.
    pub fn new(directory: &Path, suggestions: bool, english_word: bool, autocorrect: bool) -> Self {
        let fallback = std::env::temp_dir();
        let directory = if directory.is_dir() { directory } else { fallback.as_path() };
        set_aside_bad_files(directory);
        let mut config = Config::default();
        config.set_layout_file_path("avro_phonetic");
        config.set_user_dir(&directory.to_string_lossy());
        config.set_phonetic_suggestion(suggestions);
        config.set_suggestion_include_english(english_word);
        config.set_autocorrect(autocorrect);
        Riti { context: RitiContext::new_with_config(&config) }
    }

    /// Feeds one key. `selection` is the riti index of the entry the user has selected; riti keeps
    /// it as the default when the key is a punctuation mark.
    pub fn key(&self, code: u16, shift: bool, selection: usize) -> RitiSuggestion {
        let modifier = if shift { MODIFIER_SHIFT } else { 0 };
        let index = selection.min(u8::MAX as usize) as u8;
        take(self.context.get_suggestion_for_key(code, modifier, index))
    }

    /// Deletes the last letter, or the whole word. An empty result means the word is gone.
    pub fn backspace(&self, whole_word: bool) -> RitiSuggestion {
        take(self.context.backspace_event(whole_word))
    }

    pub fn is_composing(&self) -> bool {
        self.context.ongoing_input_session()
    }

    /// Ends the word, letting riti learn that `index` was picked.
    pub fn committed(&self, index: usize) {
        self.context.candidate_committed(index);
    }

    /// Ends the word without telling riti what was picked.
    pub fn finish(&self) {
        self.context.finish_input_session();
    }
}

/// riti's keycode for a character on a US layout, or None if riti has no key for it. The lookup
/// lives in riti-bridge, shared with macOS and Linux.
pub fn keycode(character: char) -> Option<u16> {
    match nukta_riti::nukta_riti_keycode(character as u32) {
        0 => None,
        code => Some(code),
    }
}

fn take(suggestion: Suggestion) -> RitiSuggestion {
    if suggestion.is_empty() {
        RitiSuggestion::Empty
    } else if suggestion.is_lonely() {
        RitiSuggestion::Lonely(suggestion.get_lonely_suggestion().to_owned())
    } else {
        RitiSuggestion::List {
            entries: suggestion.get_suggestions().to_vec(),
            auxiliary: suggestion.get_auxiliary_text().to_owned(),
            selection: suggestion.previously_selected_index(),
        }
    }
}

/// riti panics if a user file isn't a flat JSON object of strings. A file like that (half-written,
/// edited by hand) is renamed, not deleted, so nothing the user made is lost.
fn set_aside_bad_files(directory: &Path) {
    for name in USER_FILES {
        let file = directory.join(name);
        if !file.exists() || is_flat_string_object(&file) {
            continue;
        }
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let _ = fs::rename(&file, directory.join(format!("{name}.bad-{stamp}")));
    }
}

fn is_flat_string_object(file: &Path) -> bool {
    let Ok(data) = fs::read(file) else { return false };
    // riti's JSON reader rejects a byte order mark.
    if data.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return false;
    }
    match serde_json::from_slice::<serde_json::Value>(&data) {
        Ok(serde_json::Value::Object(map)) => map.values().all(|v| v.is_string()),
        _ => false,
    }
}
