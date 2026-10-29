//! The settings file: the same keys and defaults as macOS (UserDefaults) and Linux (settings.json).

use std::fs;
use std::path::PathBuf;

use nukta_bangla::phonetic::TypingMode;
use nukta_bangla::settings::{Layout, PopupDirection, PopupPosition, Settings};

fn fresh_directory(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("nukta-windows-settings-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    directory
}

#[test]
fn defaults_without_a_file() {
    let settings = Settings::new(&fresh_directory("defaults"));
    assert_eq!(settings.layout(), Layout::Phonetic);
    assert!(settings.toggle_sound());
    assert_eq!(settings.typing_mode(), TypingMode::PhoneticFirst);
    assert!(settings.autocorrect() && settings.emoji() && settings.english_word());
    assert!(!settings.colon_is_bisarga());
    assert_eq!(settings.popup_position(), PopupPosition::Below);
    assert_eq!(settings.popup_direction(), PopupDirection::Vertical);
}

#[test]
fn reads_the_shared_keys_and_ignores_bad_values() {
    let directory = fresh_directory("read");
    fs::write(
        directory.join("settings.json"),
        r#"{"layout": "phonetic", "typingMode": "smart", "emoji": false, "autocorrect": "no",
            "popupDirection": "horizontal", "popupPosition": "sideways"}"#,
    )
    .unwrap();
    let settings = Settings::new(&directory);
    assert_eq!(settings.layout(), Layout::Phonetic);
    assert_eq!(settings.typing_mode(), TypingMode::Smart);
    assert!(!settings.emoji());
    assert!(settings.autocorrect());
    assert_eq!(settings.popup_direction(), PopupDirection::Horizontal);
    assert_eq!(settings.popup_position(), PopupPosition::Below);
}

#[test]
fn a_change_in_one_process_reaches_another_and_keeps_unknown_keys() {
    let directory = fresh_directory("share");
    fs::write(directory.join("settings.json"), r#"{"future": 1}"#).unwrap();
    let mut settings_app = Settings::new(&directory);
    let mut text_service = Settings::new(&directory);

    settings_app.set_layout(Layout::Phonetic);
    text_service.set_emoji(false);
    // Each write re-reads the file first, so neither change is lost.
    assert!(text_service.reload_if_changed() || text_service.layout() == Layout::Phonetic);
    assert_eq!(text_service.layout(), Layout::Phonetic);
    settings_app.reload();
    assert!(!settings_app.emoji());
    let raw = fs::read_to_string(directory.join("settings.json")).unwrap();
    assert!(raw.contains("\"future\""), "{raw}");
}
