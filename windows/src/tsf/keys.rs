//! Physical key → character on a US layout, so both layouts work whatever Windows keyboard layout
//! is underneath: the same reason the macOS build maps key codes and the Linux build evdev codes.
//! Windows scan codes for the main block are the evdev codes linux/nukta_bangla/ibus_engine.py uses.

use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, VIRTUAL_KEY, VK_BACK, VK_CAPITAL, VK_CONTROL, VK_DOWN, VK_ESCAPE, VK_F12, VK_LCONTROL, VK_LEFT,
    VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_NUMLOCK, VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU,
    VK_RSHIFT, VK_RWIN, VK_SCROLL, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
};

use crate::phonetic::Key;

/// Scan code → (unshifted, shifted) character on a US layout.
fn us_character(scan: u32) -> Option<(char, char)> {
    Some(match scan {
        0x10 => ('q', 'Q'), 0x11 => ('w', 'W'), 0x12 => ('e', 'E'), 0x13 => ('r', 'R'), 0x14 => ('t', 'T'),
        0x15 => ('y', 'Y'), 0x16 => ('u', 'U'), 0x17 => ('i', 'I'), 0x18 => ('o', 'O'), 0x19 => ('p', 'P'),
        0x1E => ('a', 'A'), 0x1F => ('s', 'S'), 0x20 => ('d', 'D'), 0x21 => ('f', 'F'), 0x22 => ('g', 'G'),
        0x23 => ('h', 'H'), 0x24 => ('j', 'J'), 0x25 => ('k', 'K'), 0x26 => ('l', 'L'),
        0x2C => ('z', 'Z'), 0x2D => ('x', 'X'), 0x2E => ('c', 'C'), 0x2F => ('v', 'V'), 0x30 => ('b', 'B'),
        0x31 => ('n', 'N'), 0x32 => ('m', 'M'),
        0x02 => ('1', '!'), 0x03 => ('2', '@'), 0x04 => ('3', '#'), 0x05 => ('4', '$'), 0x06 => ('5', '%'),
        0x07 => ('6', '^'), 0x08 => ('7', '&'), 0x09 => ('8', '*'), 0x0A => ('9', '('), 0x0B => ('0', ')'),
        0x0C => ('-', '_'), 0x0D => ('=', '+'), 0x1A => ('[', '{'), 0x1B => (']', '}'), 0x2B => ('\\', '|'),
        0x27 => (';', ':'), 0x28 => ('\'', '"'), 0x29 => ('`', '~'), 0x33 => (',', '<'), 0x34 => ('.', '>'),
        0x35 => ('/', '?'),
        // The extra key left of Z on ISO keyboards is \ | there.
        0x56 => ('\\', '|'),
        _ => return None,
    })
}

/// For events with no scan code (on-screen keyboards, remote sessions, automation): the virtual
/// key, read as a US layout would.
fn us_character_for_virtual_key(vk: u16) -> Option<(char, char)> {
    const SHIFTED_DIGITS: [char; 10] = [')', '!', '@', '#', '$', '%', '^', '&', '*', '('];
    Some(match vk {
        0x41..=0x5A => {
            let lower = char::from(vk as u8 + 32);
            (lower, lower.to_ascii_uppercase())
        }
        0x30..=0x39 => (char::from(vk as u8), SHIFTED_DIGITS[(vk - 0x30) as usize]),
        0xBA => (';', ':'), 0xBB => ('=', '+'), 0xBC => (',', '<'), 0xBD => ('-', '_'), 0xBE => ('.', '>'),
        0xBF => ('/', '?'), 0xC0 => ('`', '~'), 0xDB => ('[', '{'), 0xDC => ('\\', '|'), 0xDD => (']', '}'),
        0xDE => ('\'', '"'),
        _ => return None,
    })
}

/// One key-down event, read once: what it is, and which modifiers are held.
#[derive(Clone, Copy, Debug)]
pub(super) struct KeyEvent {
    pub vk: u16,
    pub scan: u32,
    pub extended: bool,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl KeyEvent {
    pub fn new(wparam: WPARAM, lparam: LPARAM) -> Self {
        let bits = lparam.0 as u32;
        KeyEvent {
            vk: wparam.0 as u16,
            scan: (bits >> 16) & 0xFF,
            extended: bits & (1 << 24) != 0,
            shift: is_down(VK_SHIFT),
            ctrl: is_down(VK_CONTROL),
            alt: is_down(VK_MENU),
        }
    }

    /// A modifier on its own. It must change nothing: Shift on its way to a capital letter (V for ল)
    /// must not finish the syllable (the Linux build learnt this the hard way: প্রাণে came out as
    /// প্রােণ).
    pub fn is_modifier(&self) -> bool {
        [
            VK_SHIFT, VK_LSHIFT, VK_RSHIFT, VK_CONTROL, VK_LCONTROL, VK_RCONTROL, VK_MENU, VK_LMENU, VK_RMENU,
            VK_LWIN, VK_RWIN, VK_CAPITAL, VK_NUMLOCK, VK_SCROLL,
        ]
        .contains(&VIRTUAL_KEY(self.vk))
    }

    /// Ctrl or Alt held (AltGr is both): a shortcut, not typing.
    pub fn is_shortcut(&self) -> bool {
        self.ctrl || self.alt
    }

    /// The character on a US layout. Only Shift picks the shifted key; Caps Lock is ignored, as on
    /// macOS and Linux.
    pub fn character(&self) -> Option<char> {
        let pair = if self.scan != 0 {
            if self.extended {
                return None; // the numeric keypad's Enter and /, arrows…: not typing keys here
            }
            us_character(self.scan)
        } else {
            us_character_for_virtual_key(self.vk)
        }?;
        Some(if self.shift { pair.1 } else { pair.0 })
    }

    /// F12 alone: English ⇄ Bangla, as in Avro.
    pub fn is_toggle(&self) -> bool {
        self.vk == VK_F12.0 && !self.shift && !self.ctrl && !self.alt
    }

    pub fn is_backspace(&self) -> bool {
        self.vk == VK_BACK.0
    }

    /// The key as phonetic typing sees it, or None for keys it has no use for.
    pub fn phonetic_key(&self) -> Option<Key> {
        Some(match VIRTUAL_KEY(self.vk) {
            VK_RETURN => Key::Enter,
            VK_ESCAPE => Key::Escape,
            // Ctrl+Backspace deletes the whole word, as ⌥⌫ does on macOS.
            VK_BACK => Key::Backspace { whole_word: self.ctrl },
            VK_SPACE => Key::Space,
            VK_TAB => Key::Tab { backward: self.shift },
            VK_LEFT => Key::Left,
            VK_RIGHT => Key::Right,
            VK_DOWN => Key::Down,
            VK_UP => Key::Up,
            _ => return self.character().map(|c| Key::Character(c, self.shift)),
        })
    }
}

fn is_down(vk: VIRTUAL_KEY) -> bool {
    let state = unsafe { GetKeyState(vk.0 as i32) };
    state < 0
}
