//! The user's own shortcuts: a short roman code that types a longer text (`asw` → আসসালামু আলাইকুম).
//!
//! The first piece of the shared smart-typing engine (docs/smart-typing-plan.md): one copy of the
//! rules for every platform. Windows uses it as Rust; macOS and Linux through the C functions at the
//! bottom.
//!
//! Stored in `shortcuts.json` in the data directory, next to the pick memory: a JSON object, code →
//! text, the same on every platform. Several processes can share the file (on Windows every app
//! loads its own copy of the input method), so each change re-reads the file first and writes the
//! merged result, and a lookup re-reads it when another process has written it since.
//!
//! A shortcut is offered only when the whole code is typed: its text comes first in the list and
//! the plain transliteration second. The start of a code offers nothing, so ordinary typing is
//! never in the way.

use std::collections::BTreeMap;
use std::ffi::{c_char, CStr, CString};
use std::fs;
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::OnceLock;
use std::time::SystemTime;

pub const FILE_NAME: &str = "shortcuts.json";
pub const MIN_CODE: usize = 2;
pub const MAX_CODE: usize = 12;
/// Characters, not bytes.
pub const MAX_TEXT: usize = 300;

/// Why a shortcut couldn't be saved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutError {
    /// Not 2–12 English letters. Digits can't be in a code: while a word is being typed, 1–9 pick
    /// from the list.
    BadCode,
    EmptyText,
    TextTooLong,
    /// The file couldn't be written.
    NotSaved,
}

impl ShortcutError {
    /// The number the C functions return for it.
    pub fn code(self) -> i32 {
        match self {
            ShortcutError::BadCode => 1,
            ShortcutError::EmptyText => 2,
            ShortcutError::TextTooLong => 3,
            ShortcutError::NotSaved => 4,
        }
    }
}

/// A code as it is stored: trimmed and lowercase. None if it isn't 2–12 English letters.
pub fn normalize_code(code: &str) -> Option<String> {
    let code = code.trim();
    let length_ok = (MIN_CODE..=MAX_CODE).contains(&code.len());
    (length_ok && code.chars().all(|c| c.is_ascii_alphabetic())).then(|| code.to_ascii_lowercase())
}

/// A text as it is stored: trimmed, with line breaks and other control characters as spaces.
pub fn normalize_text(text: &str) -> Result<String, ShortcutError> {
    let text: String = text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    let text = text.trim();
    if text.is_empty() {
        Err(ShortcutError::EmptyText)
    } else if text.chars().count() > MAX_TEXT {
        Err(ShortcutError::TextTooLong)
    } else {
        Ok(text.to_owned())
    }
}

/// A dictionary word the code also spells (`ami` → আমি), if any: as a shortcut it would come first
/// every time that word is typed, so the settings name the word in a warning before saving it.
pub fn dictionary_word(code: &str) -> Option<String> {
    static DICTIONARY: OnceLock<upodesh::avro::Suggest> = OnceLock::new();
    let code = normalize_code(code)?;
    let mut words = DICTIONARY.get_or_init(upodesh::avro::Suggest::new).suggest(&code);
    // The dictionary's order changes from run to run: the word spelt exactly as typed, if it is
    // one (bari → বারি, not বাড়ি), else the first in a fixed order.
    let literal = okkhor::parser::Parser::new_phonetic().convert(&code);
    if words.contains(&literal) {
        return Some(literal);
    }
    words.sort();
    words.into_iter().next()
}

pub struct Shortcuts {
    file: PathBuf,
    map: BTreeMap<String, String>,
    /// The file as of our last read or write: see `modified`.
    stamp: Option<(SystemTime, u64)>,
}

impl Shortcuts {
    /// `directory`: the data directory. A missing or unreadable file is no shortcuts.
    pub fn new(directory: &Path) -> Self {
        let mut shortcuts = Shortcuts { file: directory.join(FILE_NAME), map: BTreeMap::new(), stamp: None };
        shortcuts.reload();
        shortcuts
    }

    /// The text for `typed` (roman, as typed) if it is a whole code; any case.
    pub fn text_for(&mut self, typed: &str) -> Option<String> {
        let code = normalize_code(typed)?;
        self.reload_if_changed();
        self.map.get(&code).cloned()
    }

    /// Every shortcut, code → text, ordered by code.
    pub fn all(&mut self) -> BTreeMap<String, String> {
        self.reload_if_changed();
        self.map.clone()
    }

    /// Adds a shortcut, or changes the text of an existing code. Returns the code as stored.
    pub fn set(&mut self, code: &str, text: &str) -> Result<String, ShortcutError> {
        let code = normalize_code(code).ok_or(ShortcutError::BadCode)?;
        let text = normalize_text(text)?;
        self.reload();
        if self.map.get(&code) != Some(&text) {
            self.map.insert(code.clone(), text);
            self.save()?;
        }
        Ok(code)
    }

    /// True if the code was there.
    pub fn remove(&mut self, code: &str) -> bool {
        let Some(code) = normalize_code(code) else { return false };
        self.reload();
        self.map.remove(&code).is_some() && self.save().is_ok()
    }

    fn reload_if_changed(&mut self) {
        if modified(&self.file) != self.stamp {
            self.reload();
        }
    }

    /// Entries that aren't a valid code and text (a file edited by hand) are left out.
    fn reload(&mut self) {
        self.stamp = modified(&self.file);
        self.map = fs::read(&self.file)
            .ok()
            .and_then(|data| serde_json::from_slice::<serde_json::Value>(&data).ok())
            .and_then(|value| match value {
                serde_json::Value::Object(map) => Some(
                    map.into_iter()
                        .filter_map(|(code, text)| {
                            Some((normalize_code(&code)?, normalize_text(text.as_str()?).ok()?))
                        })
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default();
    }

    /// Written to a temporary file and renamed over the old one, so a reader never sees half a file.
    fn save(&mut self) -> Result<(), ShortcutError> {
        let json = serde_json::to_vec_pretty(&self.map).map_err(|_| ShortcutError::NotSaved)?;
        let temporary = self.file.with_extension(format!("json.{}.tmp", std::process::id()));
        let saved = fs::write(&temporary, json).is_ok() && fs::rename(&temporary, &self.file).is_ok();
        if !saved {
            let _ = fs::remove_file(&temporary);
        }
        self.stamp = modified(&self.file);
        if saved { Ok(()) } else { Err(ShortcutError::NotSaved) }
    }
}

/// When the file was last written, and its size: two writes in the same clock tick still differ
/// in size, nearly always.
fn modified(file: &Path) -> Option<(SystemTime, u64)> {
    fs::metadata(file).and_then(|m| Ok((m.modified()?, m.len()))).ok()
}

// MARK: C functions, for macOS (Swift) and Linux (Python, ctypes)
//
// Strings are UTF-8 and NUL-terminated. A string returned by one of these belongs to the caller,
// who frees it with `nukta_string_free`.

/// # Safety
/// `text` must be NULL or a NUL-terminated string.
unsafe fn from_c<'a>(text: *const c_char) -> Option<&'a str> {
    if text.is_null() {
        return None;
    }
    CStr::from_ptr(text).to_str().ok()
}

fn to_c(text: String) -> *mut c_char {
    CString::new(text).map_or(ptr::null_mut(), CString::into_raw)
}

/// The shortcuts in `directory` (the data directory). Free with `nukta_shortcuts_free`.
///
/// # Safety
/// `directory` must be a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn nukta_shortcuts_new(directory: *const c_char) -> *mut Shortcuts {
    match from_c(directory) {
        Some(directory) => Box::into_raw(Box::new(Shortcuts::new(Path::new(directory)))),
        None => ptr::null_mut(),
    }
}

/// # Safety
/// `shortcuts` must be NULL or from `nukta_shortcuts_new`, and not used afterwards.
#[no_mangle]
pub unsafe extern "C" fn nukta_shortcuts_free(shortcuts: *mut Shortcuts) {
    if !shortcuts.is_null() {
        drop(Box::from_raw(shortcuts));
    }
}

/// The text for what was typed if it is a whole code, else NULL.
///
/// # Safety
/// `shortcuts` from `nukta_shortcuts_new`; `typed` a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn nukta_shortcuts_text(shortcuts: *mut Shortcuts, typed: *const c_char) -> *mut c_char {
    match (shortcuts.as_mut(), from_c(typed)) {
        (Some(shortcuts), Some(typed)) => shortcuts.text_for(typed).map_or(ptr::null_mut(), to_c),
        _ => ptr::null_mut(),
    }
}

/// Every shortcut as a JSON object, code → text.
///
/// # Safety
/// `shortcuts` from `nukta_shortcuts_new`.
#[no_mangle]
pub unsafe extern "C" fn nukta_shortcuts_json(shortcuts: *mut Shortcuts) -> *mut c_char {
    match shortcuts.as_mut() {
        Some(shortcuts) => to_c(serde_json::to_string(&shortcuts.all()).unwrap_or_else(|_| "{}".to_owned())),
        None => ptr::null_mut(),
    }
}

/// 0 when saved, else a `ShortcutError` number: 1 bad code, 2 empty text, 3 text too long, 4 not
/// saved.
///
/// # Safety
/// `shortcuts` from `nukta_shortcuts_new`; `code` and `text` NUL-terminated strings.
#[no_mangle]
pub unsafe extern "C" fn nukta_shortcuts_set(shortcuts: *mut Shortcuts, code: *const c_char, text: *const c_char) -> i32 {
    let Some(shortcuts) = shortcuts.as_mut() else { return ShortcutError::NotSaved.code() };
    let (Some(code), Some(text)) = (from_c(code), from_c(text)) else { return ShortcutError::BadCode.code() };
    match shortcuts.set(code, text) {
        Ok(_) => 0,
        Err(error) => error.code(),
    }
}

/// True if the code was there.
///
/// # Safety
/// `shortcuts` from `nukta_shortcuts_new`; `code` a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn nukta_shortcuts_remove(shortcuts: *mut Shortcuts, code: *const c_char) -> bool {
    match (shortcuts.as_mut(), from_c(code)) {
        (Some(shortcuts), Some(code)) => shortcuts.remove(code),
        _ => false,
    }
}

/// A dictionary word the code also spells, else NULL: the settings warn before saving it.
///
/// # Safety
/// `code` must be a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn nukta_dictionary_word(code: *const c_char) -> *mut c_char {
    from_c(code).and_then(dictionary_word).map_or(ptr::null_mut(), to_c)
}

/// Frees a string one of these functions returned.
///
/// # Safety
/// `text` must be NULL or a string returned by one of these functions, and not used afterwards.
#[no_mangle]
pub unsafe extern "C" fn nukta_string_free(text: *mut c_char) {
    if !text.is_null() {
        drop(CString::from_raw(text));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn fresh_directory() -> PathBuf {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!("nukta-shortcut-tests-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    #[test]
    fn codes_are_letters_only() {
        assert_eq!(normalize_code(" ASW "), Some("asw".to_owned()));
        assert_eq!(normalize_code("a"), None);
        assert_eq!(normalize_code("abcdefghijklm"), None);
        assert_eq!(normalize_code("as1"), None);
        assert_eq!(normalize_code("a s"), None);
        assert_eq!(normalize_code("আস"), None);
    }

    #[test]
    fn texts_are_one_trimmed_line() {
        assert_eq!(normalize_text("  ঠিক আছে\n"), Ok("ঠিক আছে".to_owned()));
        assert_eq!(normalize_text("এক\nদুই"), Ok("এক দুই".to_owned()));
        assert_eq!(normalize_text(" \n "), Err(ShortcutError::EmptyText));
        assert_eq!(normalize_text(&"ক".repeat(MAX_TEXT + 1)), Err(ShortcutError::TextTooLong));
    }

    #[test]
    fn only_the_whole_code_matches() {
        let directory = fresh_directory();
        let mut shortcuts = Shortcuts::new(&directory);
        assert_eq!(shortcuts.set("asw", "আসসালামু আলাইকুম"), Ok("asw".to_owned()));
        assert_eq!(shortcuts.text_for("asw").as_deref(), Some("আসসালামু আলাইকুম"));
        assert_eq!(shortcuts.text_for("ASW").as_deref(), Some("আসসালামু আলাইকুম"));
        assert_eq!(shortcuts.text_for("as"), None);
        assert_eq!(shortcuts.text_for("aswa"), None);
        assert_eq!(shortcuts.text_for("asw,"), None);
    }

    #[test]
    fn saved_for_other_processes_and_removed() {
        let directory = fresh_directory();
        let mut settings = Shortcuts::new(&directory);
        let mut typing = Shortcuts::new(&directory);
        settings.set("tnx", "অনেক ধন্যবাদ").unwrap();
        assert_eq!(typing.text_for("tnx").as_deref(), Some("অনেক ধন্যবাদ"));
        // A change made elsewhere is merged, not overwritten.
        typing.set("gm", "শুভ সকাল").unwrap();
        settings.set("gn", "শুভ রাত্রি").unwrap();
        assert_eq!(Shortcuts::new(&directory).all().len(), 3);
        assert!(settings.remove("TNX"));
        assert!(!settings.remove("tnx"));
        assert_eq!(typing.text_for("tnx"), None);
    }

    #[test]
    fn a_bad_file_is_no_shortcuts_and_bad_entries_are_left_out() {
        let directory = fresh_directory();
        fs::write(directory.join(FILE_NAME), "not json").unwrap();
        assert!(Shortcuts::new(&directory).all().is_empty());
        fs::write(directory.join(FILE_NAME), r#"{"ASW": "সালাম", "x1": "না", "ok": 5, "ty": "ধন্যবাদ"}"#).unwrap();
        let all = Shortcuts::new(&directory).all();
        assert_eq!(all.len(), 2);
        assert_eq!(all.get("asw").map(String::as_str), Some("সালাম"));
    }

    #[test]
    fn dictionary_words_are_spotted() {
        assert_eq!(dictionary_word("ami").as_deref(), Some("আমি"));
        assert!(dictionary_word("bangla").is_some());
        // অশ্ব: a warning, not a refusal.
        assert_eq!(dictionary_word("asw").as_deref(), Some("অশ্ব"));
        assert_eq!(dictionary_word("tnx"), None);
        assert_eq!(dictionary_word("x1"), None);
    }

    #[test]
    fn c_functions() {
        let directory = fresh_directory();
        let dir = CString::new(directory.to_string_lossy().into_owned()).unwrap();
        unsafe {
            let shortcuts = nukta_shortcuts_new(dir.as_ptr());
            assert!(!shortcuts.is_null());
            let (code, text) = (CString::new("asw").unwrap(), CString::new("আসসালামু আলাইকুম").unwrap());
            assert_eq!(nukta_shortcuts_set(shortcuts, code.as_ptr(), text.as_ptr()), 0);
            let bad = CString::new("a1").unwrap();
            assert_eq!(nukta_shortcuts_set(shortcuts, bad.as_ptr(), text.as_ptr()), 1);

            let found = nukta_shortcuts_text(shortcuts, code.as_ptr());
            assert_eq!(CStr::from_ptr(found).to_str().unwrap(), "আসসালামু আলাইকুম");
            nukta_string_free(found);
            let json = nukta_shortcuts_json(shortcuts);
            assert_eq!(CStr::from_ptr(json).to_str().unwrap(), r#"{"asw":"আসসালামু আলাইকুম"}"#);
            nukta_string_free(json);

            assert!(nukta_shortcuts_remove(shortcuts, code.as_ptr()));
            assert!(nukta_shortcuts_text(shortcuts, code.as_ptr()).is_null());
            let ami = CString::new("ami").unwrap();
            let word = nukta_dictionary_word(ami.as_ptr());
            assert_eq!(CStr::from_ptr(word).to_str().unwrap(), "আমি");
            nukta_string_free(word);
            let tnx = CString::new("tnx").unwrap();
            assert!(nukta_dictionary_word(tnx.as_ptr()).is_null());
            nukta_shortcuts_free(shortcuts);
        }
    }
}
