//! Update checks: the counterpart of `Sources/NuktaInputMethod/Updates.swift` and
//! `linux/nukta_bangla/updates.py`, with the same rules.
//!
//! - The newest release *for Windows* comes from GitHub: of the recent published releases, the
//!   highest version that has `NuktaBangla-Setup.exe` attached. A release without it (one for
//!   macOS or Linux only) is never offered here. Its tag is the version, and the description's
//!   bullet lines (`- …` or `* …`, at most five) are what the dialog shows under "নতুন যা আছে".
//! - The top bar asks at most once a day, while `autoUpdateCheck` is on. The dialog then waits a
//!   day before asking again ("পরে"), and a skipped version is never offered again ("বাদ দিন").
//! - The settings window's "এখনই খুঁজুন" asks straight away and always answers.
//! - ডাউনলোড fetches the release's `NuktaBangla-Setup.exe` inside the dialog and runs it
//!   (`/SILENT`: no wizard pages; the installer starts the new top bar when it is done).
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
/// The recent releases, newest first; drafts are only listed for the repository's members.
pub const API_URL: &str = "https://api.github.com/repos/Nukta-Solutions/nukta-bangla-keyboard/releases?per_page=20";
/// The keyboard's page, which has every platform's installer.
pub const WEBSITE: &str = "https://www.nuktasolutions.com/nukta-bangla-keyboard";
/// The Windows installer's name in every release (CLAUDE.md: never renamed).
pub const INSTALLER_ASSET: &str = "NuktaBangla-Setup.exe";
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
    /// The release's Windows installer, if it has one.
    pub download: Option<String>,
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
    let download = value
        .get("assets")
        .and_then(Value::as_array)
        .and_then(|assets| assets.iter().find(|a| a.get("name").and_then(Value::as_str) == Some(INSTALLER_ASSET)))
        .and_then(|asset| asset.get("browser_download_url")?.as_str())
        .map(str::to_owned);
    Some(Release { version, notes, url, download })
}

/// GitHub's list of releases: the newest published one with the Windows installer, or None.
/// Drafts and pre-releases are passed over.
pub fn newest_for_windows(json: &[u8]) -> Option<Release> {
    let list: Vec<Value> = serde_json::from_slice(json).ok()?;
    list.iter()
        .filter(|r| !r.get("draft").and_then(Value::as_bool).unwrap_or(false))
        .filter(|r| !r.get("prerelease").and_then(Value::as_bool).unwrap_or(false))
        .filter_map(|r| parse_release(&serde_json::to_vec(r).ok()?))
        .filter(|r| r.download.is_some())
        .max_by(|a, b| {
            if is_newer(&a.version, &b.version) {
                std::cmp::Ordering::Greater
            } else if is_newer(&b.version, &a.version) {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
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
            download: text("latestDownload"),
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
            if let Some(download) = &release.download {
                value["latestDownload"] = download.clone().into();
            }
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

/// Asks GitHub for the newest release for Windows. Blocks for up to about 20 seconds: call it off
/// the UI thread.
#[cfg(windows)]
pub fn fetch_latest() -> Option<Release> {
    let body = get(API_URL, "application/vnd.github+json", 4 << 20, |_, _| {})?;
    newest_for_windows(&body)
}

/// Downloads the release's installer into the temporary folder and returns where it is.
/// `progress(received, total)` follows it. Blocks: call it off the UI thread.
#[cfg(windows)]
pub fn download_installer(release: &Release, progress: impl FnMut(u64, Option<u64>)) -> Option<PathBuf> {
    let url = release.download.as_deref()?;
    let data = get(url, "application/octet-stream", 200 << 20, progress)?;
    // A Windows program starts with "MZ": anything else (an error page) is not run.
    if !data.starts_with(b"MZ") {
        return None;
    }
    let path = installer_path(release);
    fs::write(&path, data).ok()?;
    Some(path)
}

/// Where `download_installer` puts a release's installer.
pub fn installer_path(release: &Release) -> PathBuf {
    std::env::temp_dir().join(format!("NuktaBangla-Setup-{}.exe", release.version))
}

/// An https GET, following redirects (GitHub sends downloads on to its file servers). None unless
/// the answer is 200 and at most `limit` bytes.
#[cfg(windows)]
fn get(url: &str, accept: &str, limit: usize, mut progress: impl FnMut(u64, Option<u64>)) -> Option<Vec<u8>> {
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

    let rest = url.strip_prefix("https://")?;
    let (host, path) = rest.split_once('/').map_or((rest, "/".to_owned()), |(h, p)| (h, format!("/{p}")));
    unsafe {
        let agent = crate::gfx::wide(&format!("NuktaBangla/{CURRENT} (Windows)"));
        // The system's proxy settings, as the browser uses.
        let session = Handle(WinHttpOpen(PCWSTR(agent.as_ptr()), WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, PCWSTR::null(), PCWSTR::null(), 0));
        if session.0.is_null() {
            return None;
        }
        let _ = WinHttpSetTimeouts(session.0, 5000, 5000, 5000, 15000);
        let host = crate::gfx::wide(host);
        let connection = Handle(WinHttpConnect(session.0, PCWSTR(host.as_ptr()), 443, 0));
        if connection.0.is_null() {
            return None;
        }
        let path = crate::gfx::wide(&path);
        let request = Handle(WinHttpOpenRequest(
            connection.0, w!("GET"), PCWSTR(path.as_ptr()), PCWSTR::null(), PCWSTR::null(), std::ptr::null(), WINHTTP_FLAG_SECURE,
        ));
        if request.0.is_null() {
            return None;
        }
        let headers: Vec<u16> = format!("Accept: {accept}\r\n").encode_utf16().collect();
        WinHttpSendRequest(request.0, Some(&headers), None, 0, 0, 0).ok()?;
        WinHttpReceiveResponse(request.0, std::ptr::null_mut()).ok()?;

        let number = |query: u32| {
            let mut value = 0u32;
            let mut size = std::mem::size_of::<u32>() as u32;
            WinHttpQueryHeaders(
                request.0, query | WINHTTP_QUERY_FLAG_NUMBER, PCWSTR::null(),
                Some(&mut value as *mut u32 as *mut _), &mut size, std::ptr::null_mut(),
            )
            .ok()
            .map(|_| value)
        };
        if number(WINHTTP_QUERY_STATUS_CODE)? != 200 {
            return None;
        }
        let total = number(WINHTTP_QUERY_CONTENT_LENGTH).map(u64::from);

        let mut body = Vec::with_capacity(total.unwrap_or(0).min(limit as u64) as usize);
        let mut buffer = vec![0u8; 64 * 1024];
        loop {
            let mut read = 0u32;
            WinHttpReadData(request.0, buffer.as_mut_ptr() as *mut _, buffer.len() as u32, &mut read).ok()?;
            if read == 0 {
                break;
            }
            body.extend_from_slice(&buffer[..read as usize]);
            if body.len() > limit {
                return None;
            }
            progress(body.len() as u64, total);
        }
        Some(body)
    }
}
