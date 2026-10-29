//! Update checks: the same rules as macOS (Updates.swift) and Linux (updates.py).

use std::fs;
use std::path::PathBuf;

use nukta_bangla::update::{bangla_digits, is_newer, notes_from, parse_release, plain, version_parts, State, DAY, MAX_NOTES};

fn fresh_directory(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("nukta-windows-update-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    directory
}

#[test]
fn compares_versions_by_number() {
    assert!(is_newer("1.0.3", "1.0.2"));
    assert!(is_newer("v1.0.10", "1.0.9"));
    assert!(is_newer("1.1", "1.0.9"));
    assert!(is_newer("2.0.0", "1.9.9"));
    assert!(!is_newer("1.0.2", "1.0.2"));
    assert!(!is_newer("1.0", "1.0.0"));
    assert!(!is_newer("1.0.1", "1.0.2"));
    assert!(!is_newer("nonsense", "1.0.0"));
    assert_eq!(version_parts("v1.2.0-beta"), vec![1, 2, 0]);
    assert!(version_parts("1.x").is_empty());
}

#[test]
fn reads_a_github_release() {
    let json = r#"{
        "tag_name": "v1.0.3",
        "html_url": "https://github.com/Nukta-Solutions/nukta-bangla-keyboard/releases/tag/v1.0.3",
        "body": "Intro paragraph.\r\n\r\n- ফোনেটিক সাজেশন আরও দ্রুত\r\n* বিজয়ে যুক্তাক্ষর ঠিক\r\n-not a bullet\r\n## Install\r\n- \r\n"
    }"#.as_bytes();
    let release = parse_release(json).unwrap();
    assert_eq!(release.version, "1.0.3");
    assert_eq!(release.notes, vec!["ফোনেটিক সাজেশন আরও দ্রুত", "বিজয়ে যুক্তাক্ষর ঠিক"]);
    assert!(release.url.ends_with("/v1.0.3"));
    assert!(parse_release(br#"{"message": "Not Found"}"#).is_none());
    assert!(parse_release(br#"{"tag_name": "latest"}"#).is_none());
    assert_eq!(release.download, None, "no installer attached");
}

#[test]
fn finds_the_windows_installer_among_the_assets() {
    let json = br#"{"tag_name": "v1.0.4", "assets": [
        {"name": "nukta-bangla-linux.sh", "browser_download_url": "https://example.com/linux.sh"},
        {"name": "NuktaBangla-Setup.exe", "browser_download_url": "https://example.com/Setup.exe"}
    ]}"#;
    let release = parse_release(json).unwrap();
    assert_eq!(release.download.as_deref(), Some("https://example.com/Setup.exe"));

    // It is kept in update.json with the rest.
    let directory = fresh_directory("download");
    let mut state = State::load(&directory);
    state.record(release, 1);
    state.save();
    let again = State::load(&directory).latest.unwrap();
    assert_eq!(again.download.as_deref(), Some("https://example.com/Setup.exe"));
}

#[test]
fn shows_at_most_five_notes() {
    let body: String = (1..=8).map(|n| format!("- note {n}\n")).collect();
    assert_eq!(notes_from(&body).len(), MAX_NOTES);
    assert!(notes_from("No bullets here.").is_empty());
}

#[test]
fn notes_lose_their_markdown() {
    assert_eq!(plain("**Phonetic typing** (macOS): `ami` → আমি"), "Phonetic typing (macOS): ami → আমি");
    assert_eq!(plain("See [the guide](https://example.com/guide) and [this]"), "See the guide and [this]");
    assert_eq!(plain("__bold__ [a](b)[c](d)"), "bold ac");
    assert_eq!(plain("[x] y [a](b)"), "[x] y a");
}

#[test]
fn writes_versions_in_bangla_digits() {
    assert_eq!(bangla_digits("1.0.3"), "১.০.৩");
}

#[test]
fn checks_once_a_day() {
    let directory = fresh_directory("due");
    let mut state = State::load(&directory);
    assert!(state.check_due(1_000_000));
    state.record(parse_release(br#"{"tag_name": "v1.0.3"}"#).unwrap(), 1_000_000);
    assert!(!state.check_due(1_000_000 + DAY - 1));
    assert!(state.check_due(1_000_000 + DAY));
    // A clock set back doesn't stop checks for good.
    assert!(state.check_due(10));
}

#[test]
fn offers_a_new_version_until_skipped_and_waits_a_day_after_showing() {
    let directory = fresh_directory("pending");
    let mut state = State::load(&directory);
    let now = 5_000_000;
    assert!(state.pending("1.0.2", now).is_none());

    state.record(parse_release(br#"{"tag_name": "v1.0.3"}"#).unwrap(), now);
    assert_eq!(state.pending("1.0.2", now).unwrap().version, "1.0.3");
    assert!(state.pending("1.0.3", now).is_none(), "already up to date");

    state.mark_shown(now);
    assert!(state.pending("1.0.2", now + DAY - 1).is_none());
    assert!(state.pending("1.0.2", now + DAY).is_some());

    state.skip("1.0.3");
    assert!(state.pending("1.0.2", now + 2 * DAY).is_none());
    // The next release is offered again.
    state.record(parse_release(br#"{"tag_name": "v1.0.4"}"#).unwrap(), now + 2 * DAY);
    assert!(state.pending("1.0.2", now + 2 * DAY).is_some());
}

#[test]
fn keeps_its_state_in_update_json() {
    let directory = fresh_directory("save");
    let mut state = State::load(&directory);
    let json = r#"{"tag_name": "v1.0.3", "html_url": "https://example.com/r", "body": "- one\n- দুই"}"#.as_bytes();
    state.record(parse_release(json).unwrap(), 42);
    state.skip("1.0.1");
    state.mark_shown(100);
    state.save();

    let again = State::load(&directory);
    assert_eq!(again.last_check, 42);
    assert_eq!(again.remind_after, 100 + DAY);
    assert_eq!(again.skipped.as_deref(), Some("1.0.1"));
    let latest = again.latest.unwrap();
    assert_eq!(latest.version, "1.0.3");
    assert_eq!(latest.notes, vec!["one", "দুই"]);
    assert_eq!(latest.url, "https://example.com/r");

    // A broken file is no state at all, not a crash.
    fs::write(directory.join("update.json"), b"{not json").unwrap();
    assert!(State::load(&directory).latest.is_none());
}

/// Asks the real GitHub: `cargo test --test update -- --ignored`.
#[cfg(windows)]
#[test]
#[ignore = "needs the network"]
fn fetches_the_latest_release_from_github() {
    let release = nukta_bangla::update::fetch_latest().expect("GitHub answered");
    assert!(!version_parts(&release.version).is_empty());
    assert!(release.url.starts_with("https://github.com/"));
    println!("{release:?}");
}

/// Downloads the real latest installer: `cargo test --test update -- --ignored`.
#[cfg(windows)]
#[test]
#[ignore = "needs the network"]
fn downloads_the_latest_installer_from_github() {
    let release = nukta_bangla::update::fetch_latest().expect("GitHub answered");
    let mut steps = 0;
    let path = nukta_bangla::update::download_installer(&release, |_, total| {
        assert!(total.is_some(), "GitHub says how big it is");
        steps += 1;
    })
    .expect("downloaded");
    let data = fs::read(&path).unwrap();
    assert!(data.starts_with(b"MZ") && data.len() > 1 << 20);
    assert!(steps > 1);
    println!("{} ({} bytes)", path.display(), data.len());
    let _ = fs::remove_file(path);
}
