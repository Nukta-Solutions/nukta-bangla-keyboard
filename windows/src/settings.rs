//! User settings, kept in a JSON file: the Windows counterpart of
//! `Sources/NuktaInputMethod/Settings.swift` (UserDefaults) and `linux/nukta_bangla/settings.py`.
//!
//! ```text
//! %APPDATA%\Nukta Bangla\settings.json
//! ```
//!
//! The keys are the ones macOS stores, with the same names and values. The settings program writes
//! the file and the input method, loaded separately into every app, re-reads it whenever it changes:
//! the layout picked in one app applies to all of them, as on macOS.
//!
//! What riti and the pick memory learn lives in the same directory.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde_json::{Map, Value};

use crate::phonetic::{PhoneticOptions, TypingMode};

pub const FILE_NAME: &str = "settings.json";

/// The keyboard layout, shared by every app and kept across restarts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Layout {
    #[default]
    Bijoy,
    Phonetic,
}

impl Layout {
    pub fn name(self) -> &'static str {
        match self {
            Layout::Bijoy => "bijoy",
            Layout::Phonetic => "phonetic",
        }
    }

    pub fn toggled(self) -> Layout {
        match self {
            Layout::Bijoy => Layout::Phonetic,
            Layout::Phonetic => Layout::Bijoy,
        }
    }
}

/// Where the suggestion list opens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PopupPosition {
    #[default]
    Below,
    Above,
}

/// How the suggestions are laid out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PopupDirection {
    #[default]
    Vertical,
    Horizontal,
}

/// The settings file, re-read when it changes on disk. A missing or unreadable file means the
/// defaults; a key with a wrong value falls back to its default and the rest still applies.
pub struct Settings {
    path: PathBuf,
    values: Map<String, Value>,
    stamp: Option<SystemTime>,
}

impl Settings {
    pub fn new(directory: &Path) -> Self {
        let mut settings = Settings { path: directory.join(FILE_NAME), values: Map::new(), stamp: None };
        settings.reload();
        settings
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn reload(&mut self) {
        self.stamp = modified(&self.path);
        self.values = read(&self.path);
    }

    /// Re-reads the file if it has been written since the last read. True if it was.
    pub fn reload_if_changed(&mut self) -> bool {
        if modified(&self.path) == self.stamp {
            return false;
        }
        self.reload();
        true
    }

    // MARK: The settings themselves

    pub fn layout(&self) -> Layout {
        match self.string("layout") {
            Some("phonetic") => Layout::Phonetic,
            _ => Layout::Bijoy,
        }
    }

    pub fn typing_mode(&self) -> TypingMode {
        self.string("typingMode").and_then(TypingMode::from_name).unwrap_or_default()
    }

    /// Switches are on unless turned off.
    pub fn autocorrect(&self) -> bool {
        self.flag("autocorrect", true)
    }

    pub fn emoji(&self) -> bool {
        self.flag("emoji", true)
    }

    pub fn english_word(&self) -> bool {
        self.flag("englishWord", true)
    }

    pub fn colon_is_bisarga(&self) -> bool {
        self.flag("colonIsBisarga", false)
    }

    pub fn popup_position(&self) -> PopupPosition {
        match self.string("popupPosition") {
            Some("above") => PopupPosition::Above,
            _ => PopupPosition::Below,
        }
    }

    pub fn popup_direction(&self) -> PopupDirection {
        match self.string("popupDirection") {
            Some("horizontal") => PopupDirection::Horizontal,
            _ => PopupDirection::Vertical,
        }
    }

    pub fn phonetic_options(&self) -> PhoneticOptions {
        PhoneticOptions {
            mode: self.typing_mode(),
            autocorrect: self.autocorrect(),
            emoji: self.emoji(),
            english_word: self.english_word(),
            colon_is_bisarga: self.colon_is_bisarga(),
        }
    }

    // MARK: Changing them

    pub fn set_layout(&mut self, layout: Layout) {
        self.set("layout", layout.name().into());
    }

    pub fn set_typing_mode(&mut self, mode: TypingMode) {
        self.set("typingMode", mode.name().into());
    }

    pub fn set_autocorrect(&mut self, on: bool) {
        self.set("autocorrect", on.into());
    }

    pub fn set_emoji(&mut self, on: bool) {
        self.set("emoji", on.into());
    }

    pub fn set_english_word(&mut self, on: bool) {
        self.set("englishWord", on.into());
    }

    pub fn set_colon_is_bisarga(&mut self, on: bool) {
        self.set("colonIsBisarga", on.into());
    }

    pub fn set_popup_position(&mut self, position: PopupPosition) {
        let name = match position {
            PopupPosition::Below => "below",
            PopupPosition::Above => "above",
        };
        self.set("popupPosition", name.into());
    }

    pub fn set_popup_direction(&mut self, direction: PopupDirection) {
        let name = match direction {
            PopupDirection::Vertical => "vertical",
            PopupDirection::Horizontal => "horizontal",
        };
        self.set("popupDirection", name.into());
    }

    /// Writes one key. The file is re-read first, so a change made meanwhile in another process
    /// (the layout, picked from the taskbar in another app) is kept, and so are keys this version
    /// doesn't know.
    fn set(&mut self, key: &str, value: Value) {
        self.values = read(&self.path);
        self.values.insert(key.to_owned(), value);
        if let Some(directory) = self.path.parent() {
            let _ = fs::create_dir_all(directory);
        }
        if let Ok(json) = serde_json::to_vec_pretty(&self.values) {
            let temporary = self.path.with_extension(format!("json.{}.tmp", std::process::id()));
            if fs::write(&temporary, json).is_ok() && fs::rename(&temporary, &self.path).is_err() {
                let _ = fs::remove_file(&temporary);
            }
        }
        self.stamp = modified(&self.path);
    }

    // MARK: Helpers

    fn string(&self, key: &str) -> Option<&str> {
        self.values.get(key).and_then(Value::as_str)
    }

    fn flag(&self, key: &str, default: bool) -> bool {
        self.values.get(key).and_then(Value::as_bool).unwrap_or(default)
    }
}

fn read(path: &Path) -> Map<String, Value> {
    match fs::read(path).ok().and_then(|data| serde_json::from_slice(&data).ok()) {
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}
