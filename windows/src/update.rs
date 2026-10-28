//! Update checks: the counterpart of `Sources/NuktaInputMethod/Updates.swift` and
//! `linux/nukta_bangla/updates.py`, with the same rules.
//!
//! - The latest release comes from GitHub (`/releases/latest`): its tag is the version, and the
//!   description's bullet lines (`- …` or `* …`, at most five) are what the dialog shows under
//!   "নতুন যা আছে".
//! - The top bar asks at most once a day, while `autoUpdateCheck` is on. The dialog then waits a
//!   day before asking again ("পরে"), and a skipped version is never offered again ("বাদ দিন").
//! - The settings window's "এখনই খুঁজুন" asks straight away and always answers.
//!
//! What was found and what the user chose live beside the settings, in their own file, so the
//! daily check doesn't make every app re-read settings.json:
//!
//! ```text
//! %APPDATA%\Nukta Bangla\update.json
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

pub const STATE_FILE: &str = "update.json";
pub const API_HOST: &str = "api.github.com";
pub const API_PATH: &str = "/repos/Nukta-Solutions/nukta-bangla-keyboard/releases/latest";
/// Where ডাউনলোড goes: the keyboard's page, which has every platform's installer.
pub const WEBSITE: &str = "https://www.nuktasolutions.com/nukta-bangla-keyboard";
/// How long between automatic checks, and how long "পরে" waits.
pub const DAY: u64 = 24 * 60 * 60;
/// Bullet lines shown in the dialog; the rest are on the release page.
pub const MAX_NOTES: usize = 5;

/// This build's version.
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// A published release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    /// Without the tag's "v": 1.0.3.
    pub version: String,
    pub notes: Vec<String>,
    /// The release's page on GitHub ("সব পরিবর্তন দেখুন").
    pub url: String,
}

/// GitHub's answer for the latest release, or None if it isn't one.
pub fn parse_release(json: &[u8]) -> Option<Release> {
    let value: Value = serde_json::from_slice(json).ok()?;
    let tag = value.get("tag_name")?.as_str()?;
    let version = tag.trim().trim_start_matches(['v', 'V']).to_owned();
    if version_parts(&version).is_empty() {
        return None;
    }
    let notes = notes_from(value.get("body").and_then(Value::as_str).unwrap_or(""));
    let url = value.get("html_url").and_then(Value::as_str).unwrap_or(WEBSITE).to_owned();
    Some(Release { version, notes, url })
}

/// The bullet lines of a release description, without their bullets or Markdown.
pub fn notes_from(body: &str) -> Vec<String> {
    body.lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = plain(line.strip_prefix("- ").or_else(|| line.strip_prefix("* "))?);
            (!rest.is_empty()).then_some(rest)
        })
        .take(MAX_NOTES)
        .collect()
}

/// A line of Markdown as plain text: **bold**, __bold__ and `code` lose their marks, and
/// [a link](url) keeps only its text.
pub fn plain(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(start) = rest.find('[') {
        let after = &rest[start + 1..];
        // [text](url): no "]" in the text, no ")" in the url.
        let link = after
            .find(']')
            .filter(|&close| after[close + 1..].starts_with('('))
            .and_then(|close| after[close + 2..].find(')').map(|end| (close, close + 2 + end)));
        match link {
            Some((middle, end)) => {
                out.push_str(&rest[..start]);
                out.push_str(&after[..middle]);
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[..=start]);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out.replace("**", "").replace("__", "").replace('`', "").trim().to_owned()
}

/// 1.0.3 → [1, 0, 3]. Anything after a "-" or "+" (1.1.0-beta) is ignored; so is a leading "v".
pub fn version_parts(version: &str) -> Vec<u64> {
    let core = version.trim().trim_start_matches(['v', 'V']);
    let core = core.split(['-', '+']).next().unwrap_or("");
    let mut parts = Vec::new();
    for part in core.split('.') {
        match part.parse() {
            Ok(number) => parts.push(number),
            Err(_) => return Vec::new(),
        }
    }
    parts
}

/// True if `candidate` is a later version than `current` (1.0.10 is later than 1.0.9).
pub fn is_newer(candidate: &str, current: &str) -> bool {
    let (mut a, mut b) = (version_parts(candidate), version_parts(current));
    if a.is_empty() || b.is_empty() {
        return false;
    }
    let length = a.len().max(b.len());
    a.resize(length, 0);
    b.resize(length, 0);
    a > b
}

/// A version in Bangla digits (১.০.৩).
pub fn bangla_digits(text: &str) -> String {
    text.chars().map(|c| c.to_digit(10).map_or(c, |d| char::from_u32(0x09E6 + d).unwrap_or(c))).collect()
}

/// Seconds since 1970.
pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// What the last check found and what the user chose, kept in update.json.
#[derive(Debug)]
pub struct State {
    path: PathBuf,
    /// When GitHub last answered.
    pub last_check: u64,
    pub latest: Option<Release>,
    /// "এই সংস্করণ বাদ দিন".
    pub skipped: Option<String>,
    /// The dialog is not shown again before this ("পরে", or it was shown a moment ago).
    pub remind_after: u64,
}

impl State {
    /// Reads the file; missing or unreadable means nothing found yet.
    pub fn load(directory: &Path) -> State {
        let path = directory.join(STATE_FILE);
        let value: Value = fs::read(&path).ok().and_then(|data| serde_json::from_slice(&data).ok()).unwrap_or(Value::Null);
        let number = |key: &str| value.get(key).and_then(Value::as_u64).unwrap_or(0);
        let text = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
        let latest = text("latestVersion").map(|version| Release {
            version,
            notes: value
                .get("latestNotes")
                .and_then(Value::as_array)
                .map(|notes| notes.iter().filter_map(Value::as_str).map(str::to_owned).collect())
                .unwrap_or_default(),
            url: text("latestUrl").unwrap_or_else(|| WEBSITE.to_owned()),
        });
        State { path, last_check: number("lastCheck"), latest, skipped: text("skippedVersion"), remind_after: number("remindAfter") }
    }

    pub fn save(&self) {
        let mut value = json!({
            "lastCheck": self.last_check,
            "remindAfter": self.remind_after,
        });
        if let Some(release) = &self.latest {
            value["latestVersion"] = release.version.clone().into();
            value["latestNotes"] = release.notes.clone().into();
            value["latestUrl"] = release.url.clone().into();
        }
        if let Some(skipped) = &self.skipped {
            value["skippedVersion"] = skipped.clone().into();
        }
        if let Some(directory) = self.path.parent() {
            let _ = fs::create_dir_all(directory);
        }
        let Ok(data) = serde_json::to_vec_pretty(&value) else { return };
        let temporary = self.path.with_extension(format!("json.{}.tmp", std::process::id()));
        if fs::write(&temporary, data).is_ok() && fs::rename(&temporary, &self.path).is_err() {
            let _ = fs::remove_file(&temporary);
        }
    }

    /// A day has passed since GitHub last answered.
    pub fn check_due(&self, now: u64) -> bool {
        now.saturating_sub(self.last_check) >= DAY || self.last_check > now
    }

    /// GitHub answered.
    pub fn record(&mut self, release: Release, now: u64) {
        self.last_check = now;
        self.latest = Some(release);
    }

    /// The release worth telling the user about, if any: newer than `current`, not skipped, and
    /// not put off.
    pub fn pending(&self, current: &str, now: u64) -> Option<&Release> {
        let release = self.latest.as_ref()?;
        let wanted = is_newer(&release.version, current)
            && self.skipped.as_deref() != Some(release.version.as_str())
            && now >= self.remind_after;
        wanted.then_some(release)
    }

    /// The dialog is on screen: not again for a day, whatever is clicked.
    pub fn mark_shown(&mut self, now: u64) {
        self.remind_after = now + DAY;
    }

    pub fn skip(&mut self, version: &str) {
        self.skipped = Some(version.to_owned());
    }
}

/// Asks GitHub for the latest release. Blocks for up to about 20 seconds: call it off the UI thread.
#[cfg(windows)]
pub fn fetch_latest() -> Option<Release> {
    use windows::core::{w, PCWSTR};
    use windows::Win32::Networking::WinHttp::*;

    struct Handle(*mut core::ffi::c_void);
    impl Drop for Handle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    let _ = WinHttpCloseHandle(self.0);
                }
            }
        }
    }

    unsafe {
        let agent = crate::gfx::wide(&format!("NuktaBangla/{CURRENT} (Windows)"));
        // The system's proxy settings, as the browser uses.
        let session = Handle(WinHttpOpen(PCWSTR(agent.as_ptr()), WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, PCWSTR::null(), PCWSTR::null(), 0));
        if session.0.is_null() {
            return None;
        }
        let _ = WinHttpSetTimeouts(session.0, 5000, 5000, 5000, 10000);
        let host = crate::gfx::wide(API_HOST);
        let connection = Handle(WinHttpConnect(session.0, PCWSTR(host.as_ptr()), 443, 0));
        if connection.0.is_null() {
            return None;
        }
        let path = crate::gfx::wide(API_PATH);
        let request = Handle(WinHttpOpenRequest(
            connection.0, w!("GET"), PCWSTR(path.as_ptr()), PCWSTR::null(), PCWSTR::null(), std::ptr::null(), WINHTTP_FLAG_SECURE,
        ));
        if request.0.is_null() {
            return None;
        }
        let headers: Vec<u16> = "Accept: application/vnd.github+json\r\n".encode_utf16().collect();
        WinHttpSendRequest(request.0, Some(&headers), None, 0, 0, 0).ok()?;
        WinHttpReceiveResponse(request.0, std::ptr::null_mut()).ok()?;

        let mut status = 0u32;
        let mut size = std::mem::size_of::<u32>() as u32;
        WinHttpQueryHeaders(
            request.0, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER, PCWSTR::null(),
            Some(&mut status as *mut u32 as *mut _), &mut size, std::ptr::null_mut(),
        )
        .ok()?;
        if status != 200 {
            return None;
        }

        let mut body = Vec::new();
        let mut buffer = [0u8; 8192];
        loop {
            let mut read = 0u32;
            WinHttpReadData(request.0, buffer.as_mut_ptr() as *mut _, buffer.len() as u32, &mut read).ok()?;
            if read == 0 {
                break;
            }
            body.extend_from_slice(&buffer[..read as usize]);
            // A release description is a few kilobytes; anything huge isn't what we asked for.
            if body.len() > 1 << 20 {
                return None;
            }
        }
        parse_release(&body)
    }
}
