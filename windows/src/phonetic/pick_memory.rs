//! Phonetic-first's memory: a port of `Sources/NuktaPhonetic/PickMemory.swift`.
//!
//! For a typed word, the list entry the user picked instead of the plain transliteration, so it is
//! selected next time. Stored as a JSON object, typed word → picked word, in
//! `phonetic-first-picks.json`: the same name and format as macOS and Linux.
//!
//! On Windows every app loads its own copy of the input method, so several processes share this
//! file. Each change re-reads the file first and writes the merged result, and a lookup re-reads it
//! when another process has written it since.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub const FILE_NAME: &str = "phonetic-first-picks.json";

/// What riti puts around a word (`sonar,`, `(ami)`…), so `sonar` and `sonar,` share a pick:
/// riti's `META` (riti-bridge/riti/src/utility.rs), plus the quotes its smart quoting writes.
const PUNCTUATION: &str = "-]~!@#%&*()_=+[{}'\";<>/?|.,।“”‘’";

pub struct PickMemory {
    file: PathBuf,
    picks: BTreeMap<String, String>,
    /// When the file was last read or written by us.
    stamp: Option<SystemTime>,
}

impl PickMemory {
    /// A missing or unreadable file is an empty memory.
    pub fn new(directory: &Path) -> Self {
        let mut memory = PickMemory { file: directory.join(FILE_NAME), picks: BTreeMap::new(), stamp: None };
        memory.reload();
        memory
    }

    /// The remembered pick for `typed` (roman, as typed), trimmed of punctuation.
    pub fn pick(&mut self, typed: &str) -> Option<String> {
        let key = trim(typed);
        if key.is_empty() {
            return None;
        }
        self.reload_if_changed();
        self.picks.get(key).cloned()
    }

    pub fn remember(&mut self, picked: &str, typed: &str) {
        let (key, value) = (trim(typed), trim(picked));
        if key.is_empty() || value.is_empty() {
            return;
        }
        self.reload();
        if self.picks.get(key).map(String::as_str) == Some(value) {
            return;
        }
        self.picks.insert(key.to_owned(), value.to_owned());
        self.save();
    }

    pub fn forget(&mut self, typed: &str) {
        self.reload();
        if self.picks.remove(trim(typed)).is_some() {
            self.save();
        }
    }

    fn reload_if_changed(&mut self) {
        if modified(&self.file) != self.stamp {
            self.reload();
        }
    }

    fn reload(&mut self) {
        self.stamp = modified(&self.file);
        self.picks = fs::read(&self.file)
            .ok()
            .and_then(|data| serde_json::from_slice::<serde_json::Value>(&data).ok())
            .and_then(|value| match value {
                serde_json::Value::Object(map) => Some(
                    map.into_iter()
                        .filter_map(|(k, v)| v.as_str().map(|v| (k, v.to_owned())))
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default();
    }

    /// Written to a temporary file and renamed over the old one, so a reader never sees half a file.
    fn save(&mut self) {
        let Ok(json) = serde_json::to_vec(&self.picks) else { return };
        let temporary = self.file.with_extension(format!("json.{}.tmp", std::process::id()));
        if fs::write(&temporary, json).is_ok() && fs::rename(&temporary, &self.file).is_err() {
            let _ = fs::remove_file(&temporary);
        }
        self.stamp = modified(&self.file);
    }
}

pub fn trim(text: &str) -> &str {
    text.trim_matches(|c| PUNCTUATION.contains(c))
}

fn modified(file: &Path) -> Option<SystemTime> {
    fs::metadata(file).and_then(|m| m.modified()).ok()
}
