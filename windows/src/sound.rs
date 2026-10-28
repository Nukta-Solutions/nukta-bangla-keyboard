//! The beep played when নুকতা বাংলা is switched on or off (F12, the tray icon, the bar): one short
//! soft beep, higher for Bangla, lower for English. Made here as a WAV in memory, so there is no
//! sound file to install and nothing to find at run time.

use std::f32::consts::TAU;
use std::sync::OnceLock;

use windows::core::PCWSTR;
use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};

const RATE: u32 = 44_100;

/// Plays the on (Bangla) or off (English) beep, without waiting for it.
pub fn play_toggle(on: bool) {
    static ON: OnceLock<Vec<u8>> = OnceLock::new();
    static OFF: OnceLock<Vec<u8>> = OnceLock::new();
    // C6 for on, G5 for off: a fifth apart, so the two are told apart without thinking.
    let wav = if on { ON.get_or_init(|| beep(1046.5)) } else { OFF.get_or_init(|| beep(784.0)) };
    // The buffer is static, so it outlives the asynchronous playback.
    unsafe {
        let _ = PlaySoundW(PCWSTR(wav.as_ptr() as *const u16), None, SND_MEMORY | SND_ASYNC | SND_NODEFAULT);
    }
}

/// One short beep: a sine with a touch of its octave, 70 ms, with a 3 ms fade-in and a 25 ms
/// fade-out so it never clicks, as a 16-bit mono WAV file.
fn beep(frequency: f32) -> Vec<u8> {
    const LENGTH: f32 = 0.070; // seconds
    const FADE_IN: f32 = 0.003;
    const FADE_OUT: f32 = 0.025;
    const VOLUME: f32 = 0.25;
    let total = (LENGTH * RATE as f32) as usize;
    let samples: Vec<f32> = (0..total)
        .map(|n| {
            let t = n as f32 / RATE as f32;
            let envelope = (t / FADE_IN).min(1.0) * ((LENGTH - t) / FADE_OUT).clamp(0.0, 1.0);
            let tone = (TAU * frequency * t).sin() + 0.15 * (TAU * frequency * 2.0 * t).sin();
            tone * envelope * VOLUME
        })
        .collect();

    let data: Vec<u8> = samples
        .iter()
        .flat_map(|s| ((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).to_le_bytes())
        .collect();
    let mut wav = Vec::with_capacity(44 + data.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&RATE.to_le_bytes());
    wav.extend_from_slice(&(RATE * 2).to_le_bytes()); // bytes per second
    wav.extend_from_slice(&2u16.to_le_bytes()); // block align
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
    wav.extend_from_slice(&data);
    wav
}
