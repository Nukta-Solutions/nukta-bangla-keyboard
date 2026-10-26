//! নুকতা বাংলা for Windows: the Bijoy layout and Avro-style phonetic typing as one TSF text service.
//!
//! `bijoy`, `phonetic` and `settings` are plain Rust, ports of the macOS (Swift) and Linux (Python)
//! code, and are tested on any machine. `tsf` is the Windows part: the text service every app loads,
//! its taskbar menu and its suggestion list. `win` holds what the text service and the settings
//! program share (where the files live, adding the keyboard to a user's languages).

pub mod bijoy;
pub mod phonetic;
pub mod settings;

#[cfg(windows)]
pub mod gfx;

#[cfg(windows)]
pub mod logo;

#[cfg(windows)]
pub mod sound;

#[cfg(windows)]
pub mod win;

#[cfg(windows)]
mod tsf;
