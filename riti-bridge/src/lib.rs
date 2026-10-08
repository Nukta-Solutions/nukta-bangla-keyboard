//! The static library the app links: riti's C API, plus a keycode lookup so Swift doesn't need to
//! know riti's key numbering.

// riti's C functions live in a private module of the riti crate. Naming the crate here links it in,
// and its `#[no_mangle]` functions end up in this static library as they are.
extern crate riti;

use riti::keycodes::*;

/// riti's keycode for a character typed on a US keyboard, or 0 if riti has no key for it.
///
/// Uppercase letters have their own codes in riti (`VC_A_SHIFT`…), and shifted symbols (`!`, `:`…)
/// are keys of their own, so the character alone is enough.
#[no_mangle]
pub extern "C" fn nukta_riti_keycode(ch: u32) -> u16 {
    match char::from_u32(ch) {
        Some(ch) => keycode(ch),
        None => 0,
    }
}

fn keycode(ch: char) -> u16 {
    match ch {
        'a'..='z' => VC_A + (ch as u16 - 'a' as u16),
        'A'..='Z' => VC_A_SHIFT + (ch as u16 - 'A' as u16),
        // VC_1…VC_9 are consecutive, VC_0 comes after them (as on the keyboard).
        '1'..='9' => VC_1 + (ch as u16 - '1' as u16),
        '0' => VC_0,
        '`' => VC_GRAVE,
        '~' => VC_TILDE,
        '!' => VC_EXCLAIM,
        '@' => VC_AT,
        '#' => VC_HASH,
        '$' => VC_DOLLAR,
        '%' => VC_PERCENT,
        '^' => VC_CIRCUM,
        '&' => VC_AMPERSAND,
        '*' => VC_ASTERISK,
        '(' => VC_PAREN_LEFT,
        ')' => VC_PAREN_RIGHT,
        '-' => VC_MINUS,
        '_' => VC_UNDERSCORE,
        '=' => VC_EQUALS,
        '+' => VC_PLUS,
        '[' => VC_BRACKET_LEFT,
        ']' => VC_BRACKET_RIGHT,
        '{' => VC_BRACE_LEFT,
        '}' => VC_BRACE_RIGHT,
        '\\' => VC_BACK_SLASH,
        '|' => VC_BAR,
        ';' => VC_SEMICOLON,
        ':' => VC_COLON,
        '\'' => VC_APOSTROPHE,
        '"' => VC_QUOTE,
        ',' => VC_COMMA,
        '.' => VC_PERIOD,
        '<' => VC_LESS,
        '>' => VC_GREATER,
        '/' => VC_SLASH,
        '?' => VC_QUESTION,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_digits_and_symbols() {
        assert_eq!(nukta_riti_keycode('a' as u32), VC_A);
        assert_eq!(nukta_riti_keycode('z' as u32), VC_Z);
        assert_eq!(nukta_riti_keycode('Z' as u32), VC_Z_SHIFT);
        assert_eq!(nukta_riti_keycode('9' as u32), VC_9);
        assert_eq!(nukta_riti_keycode('0' as u32), VC_0);
        assert_eq!(nukta_riti_keycode('?' as u32), VC_QUESTION);
        assert_eq!(nukta_riti_keycode(' ' as u32), 0);
        assert_eq!(nukta_riti_keycode('ক' as u32), 0);
        assert_eq!(nukta_riti_keycode(0xD800), 0);
    }
}
