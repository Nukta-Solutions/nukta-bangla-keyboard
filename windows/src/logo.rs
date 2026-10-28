//! The নুকতা বাংলা logo (`Resources/nu-icon.svg`) for Windows UI, drawn from the SVG's own path
//! data so it stays sharp at any size.
//!
//! The tray icon is a rounded square:
//! - on (Bangla): the brand gradient, `#D13A0E` → `#E87A1E`, with the নু glyph in white;
//! - off (English, the keyboard doing nothing): our black, `#0D0807`, with a faint edge so it shows
//!   on a dark taskbar, and EN in white.
//!
//! GDI draws without antialiasing, so the glyph is drawn eight times larger and scaled down here.

use windows::core::w;
use windows::Win32::Foundation::{COLORREF, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    BeginPath, CloseFigure, CreateBitmap, CreateCompatibleDC, CreateDIBSection, CreateFontW, CreatePen, CreateSolidBrush,
    DeleteDC, DeleteObject, DrawTextW, Ellipse, EndPath, FillPath, GdiFlush, LineTo, MoveToEx, PolyBezierTo, SelectObject,
    SetBkMode, SetTextColor, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DEFAULT_CHARSET, DIB_RGB_COLORS, DT_CENTER,
    DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, FF_DONTCARE, FONT_CLIP_PRECISION, FONT_OUTPUT_PRECISION,
    NONANTIALIASED_QUALITY, PS_NULL, TRANSPARENT,
};
use windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, HICON, ICONINFO};

/// The glyph নু from nu-icon.svg, in its own coordinates (drawn at 10, 4.989 in the 37 × 28 logo).
const GLYPH: &str = "M10.541 13.665L9.372 12.773C9.33 12.277 9.245 11.749 9.117 11.189C8.99 10.63 8.82 10.084 8.607 9.553C8.395 9.022 8.147 8.543 7.863 8.118C7.58 7.693 7.254 7.35 6.886 7.088C6.517 6.826 6.121 6.694 5.696 6.694C5.54 6.694 5.38 6.712 5.217 6.748C5.055 6.783 4.916 6.836 4.803 6.907L5.419 5.802C5.632 5.944 5.788 6.142 5.887 6.397C5.986 6.652 6.036 6.907 6.036 7.162C6.036 7.559 5.951 7.916 5.781 8.235C5.611 8.554 5.366 8.806 5.047 8.99C4.729 9.174 4.328 9.266 3.847 9.266C3.039 9.266 2.423 9.025 1.998 8.543C1.573 8.062 1.36 7.452 1.36 6.716C1.36 5.894 1.622 5.256 2.146 4.803C2.671 4.35 3.351 4.123 4.187 4.123C4.81 4.123 5.387 4.233 5.919 4.452C6.45 4.672 6.921 4.977 7.332 5.366C7.743 5.756 8.104 6.202 8.416 6.705C8.728 7.208 8.983 7.739 9.181 8.299C9.379 8.859 9.514 9.415 9.585 9.967L9.287 9.989C9.273 9.705 9.248 9.326 9.213 8.852C9.177 8.377 9.146 7.87 9.117 7.332C9.089 6.794 9.075 6.305 9.075 5.866L9.075 1.36L0.829 1.36L0 0L12.263 0L13.091 1.36L10.69 1.36L10.69 13.644L10.541 13.665ZM4.608 14.877C4.608 14.154 4.873 13.563 5.405 13.102C5.936 12.642 6.69 12.411 7.668 12.411C8.461 12.411 9.223 12.553 9.953 12.836C10.682 13.12 11.362 13.502 11.993 13.984C12.623 14.466 13.194 15.008 13.704 15.61C14.214 16.212 14.646 16.839 15 17.491L14.192 18.022C13.64 17.186 13.027 16.463 12.354 15.854C11.681 15.245 10.934 14.77 10.112 14.43C9.29 14.09 8.391 13.92 7.413 13.92C6.917 13.92 6.559 14.002 6.34 14.165C6.12 14.328 6.01 14.551 6.01 14.834C6.01 15.117 6.117 15.348 6.329 15.525C6.542 15.702 6.846 15.79 7.243 15.79C7.625 15.79 7.965 15.702 8.263 15.525C8.561 15.348 8.798 15.061 8.975 14.664C9.152 14.267 9.241 13.736 9.241 13.07L10.686 13.772C10.686 14.437 10.558 15.043 10.303 15.589C10.048 16.134 9.673 16.57 9.177 16.896C8.681 17.221 8.072 17.384 7.349 17.384C6.556 17.384 5.9 17.151 5.383 16.683C4.866 16.216 4.608 15.613 4.608 14.877Z";
/// Where the glyph sits in the 37 × 28 logo, and its dot (centre, radius), in logo coordinates.
const GLYPH_OFFSET: (f32, f32) = (10.0, 4.989);
const DOT: (f32, f32, f32) = (25.5, 17.489, 1.5);
/// The glyph and dot together, in logo coordinates: what the tray icon centres.
const GLYPH_BOX: (f32, f32, f32, f32) = (10.0, 4.989, 27.0, 23.011);

/// The brand gradient, from the SVG: `#D13A0E` at (1.761, 20.8) to `#E87A1E` at (35.665, 3.782).
const BRAND_FROM: (f32, f32, f32) = (0xD1 as f32, 0x3A as f32, 0x0E as f32);
const BRAND_TO: (f32, f32, f32) = (0xE8 as f32, 0x7A as f32, 0x1E as f32);
/// Our black, and the faint edge that keeps it visible on a dark taskbar.
const BLACK: (f32, f32, f32) = (0x0D as f32, 0x08 as f32, 0x07 as f32);
const EDGE: (f32, f32, f32) = (0x4A as f32, 0x41 as f32, 0x3F as f32);

/// How much larger the ink is drawn before it is scaled down.
const SUPERSAMPLE: i32 = 8;

/// The tray icon at `size` × `size` pixels: on (Bangla, নু on the brand gradient) or off (EN on
/// black). The caller owns the icon.
pub fn tray_icon(on: bool, size: i32) -> HICON {
    let size = size.max(16);
    let ink = if on { glyph_coverage(size) } else { text_coverage("EN", size) };
    let radius = size as f32 * 0.22;

    let mut pixels = vec![0u32; (size * size) as usize];
    for y in 0..size {
        for x in 0..size {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let shape = rounded_coverage(px, py, size as f32, radius, 0.0);
            if shape <= 0.0 {
                continue;
            }
            let background = if on {
                brand_color(px / size as f32, py / size as f32)
            } else {
                // Inside the one-pixel edge: black.
                let inner = rounded_coverage(px, py, size as f32, radius, 1.0);
                mix(EDGE, BLACK, inner)
            };
            let ink = ink[(y * size + x) as usize] * 0.9; // the SVG's white is 90% opaque
            let color = mix(background, (255.0, 255.0, 255.0), ink);
            let alpha = shape;
            let channel = |c: f32| (c * alpha).round().clamp(0.0, 255.0) as u32;
            // Premultiplied BGRA.
            pixels[(y * size + x) as usize] =
                ((alpha * 255.0).round() as u32) << 24 | channel(color.0) << 16 | channel(color.1) << 8 | channel(color.2);
        }
    }
    icon_from_pixels(&pixels, size)
}

fn mix(a: (f32, f32, f32), b: (f32, f32, f32), t: f32) -> (f32, f32, f32) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, a.2 + (b.2 - a.2) * t)
}

/// The SVG's gradient, at (u, v) in 0…1 across the icon (the logo's box stretched to a square).
fn brand_color(u: f32, v: f32) -> (f32, f32, f32) {
    let (x, y) = (u * 37.0, v * 28.0);
    let (x0, y0, x1, y1) = (1.761, 20.8, 35.665, 3.782);
    let (dx, dy) = (x1 - x0, y1 - y0);
    let t = (((x - x0) * dx + (y - y0) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    mix(BRAND_FROM, BRAND_TO, t)
}

/// How much of the pixel at (x, y) a rounded square `size` wide covers, shrunk by `inset`.
fn rounded_coverage(x: f32, y: f32, size: f32, radius: f32, inset: f32) -> f32 {
    let (low, high) = (inset, size - inset);
    let r = (radius - inset).max(0.5);
    let cx = x.clamp(low + r, high - r);
    let cy = y.clamp(low + r, high - r);
    let distance = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
    let edge = if x < low + r || x > high - r || y < low + r || y > high - r {
        r - distance
    } else {
        (x - low).min(high - x).min(y - low).min(high - y)
    };
    (edge + 0.5).clamp(0.0, 1.0)
}

// MARK: The ink: drawn large in white on black, then averaged down to coverage per pixel.

fn glyph_coverage(size: i32) -> Vec<f32> {
    let big = size * SUPERSAMPLE;
    // The glyph and its dot take 60% of the icon's height, centred.
    let (left, top, right, bottom) = GLYPH_BOX;
    let scale = big as f32 * 0.60 / (bottom - top);
    let offset_x = (big as f32 - (right - left) * scale) / 2.0 - left * scale;
    let offset_y = (big as f32 - (bottom - top) * scale) / 2.0 - top * scale;
    let at = |x: f32, y: f32| POINT {
        x: ((x + GLYPH_OFFSET.0) * scale + offset_x).round() as i32,
        y: ((y + GLYPH_OFFSET.1) * scale + offset_y).round() as i32,
    };
    let dot = (
        DOT.0 * scale + offset_x,
        DOT.1 * scale + offset_y,
        DOT.2 * scale,
    );
    draw_large(size, |dc| unsafe {
        let _ = BeginPath(dc);
        trace(dc, GLYPH, &at);
        let _ = Ellipse(
            dc,
            (dot.0 - dot.2).round() as i32,
            (dot.1 - dot.2).round() as i32,
            (dot.0 + dot.2).round() as i32,
            (dot.1 + dot.2).round() as i32,
        );
        let _ = EndPath(dc);
        let _ = FillPath(dc);
    })
}

fn text_coverage(text: &str, size: i32) -> Vec<f32> {
    let big = size * SUPERSAMPLE;
    draw_large(size, |dc| unsafe {
        let font = CreateFontW(
            -(big * 40 / 100), 0, 0, 0, 600, 0, 0, 0, DEFAULT_CHARSET, FONT_OUTPUT_PRECISION(0),
            FONT_CLIP_PRECISION(0), NONANTIALIASED_QUALITY, FF_DONTCARE.0 as u32, w!("Segoe UI"),
        );
        let old = SelectObject(dc, font.into());
        SetBkMode(dc, TRANSPARENT);
        SetTextColor(dc, COLORREF(0x00FF_FFFF));
        let mut text: Vec<u16> = text.encode_utf16().collect();
        let mut rect = RECT { left: 0, top: 0, right: big, bottom: big };
        DrawTextW(dc, &mut text, &mut rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(dc, old);
        let _ = DeleteObject(font.into());
    })
}

/// Runs `draw` on a white-ink, black-background canvas `SUPERSAMPLE` times the icon's size, and
/// returns each icon pixel's share of ink, 0…1.
fn draw_large(size: i32, draw: impl FnOnce(windows::Win32::Graphics::Gdi::HDC)) -> Vec<f32> {
    let big = size * SUPERSAMPLE;
    let mut coverage = vec![0f32; (size * size) as usize];
    unsafe {
        let info = bitmap_info(big);
        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let Ok(bitmap) = CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0) else {
            return coverage;
        };
        let dc = CreateCompatibleDC(None);
        let old_bitmap = SelectObject(dc, bitmap.into());
        let brush = CreateSolidBrush(COLORREF(0x00FF_FFFF));
        let pen = CreatePen(PS_NULL, 0, COLORREF(0));
        let old_brush = SelectObject(dc, brush.into());
        let old_pen = SelectObject(dc, pen.into());
        draw(dc);
        let _ = GdiFlush();

        let canvas = std::slice::from_raw_parts(bits as *const u32, (big * big) as usize);
        let samples = (SUPERSAMPLE * SUPERSAMPLE) as f32;
        for y in 0..size {
            for x in 0..size {
                let mut sum = 0u32;
                for sy in 0..SUPERSAMPLE {
                    let row = ((y * SUPERSAMPLE + sy) * big + x * SUPERSAMPLE) as usize;
                    for sx in 0..SUPERSAMPLE as usize {
                        sum += (canvas[row + sx] >> 16) & 0xFF;
                    }
                }
                coverage[(y * size + x) as usize] = sum as f32 / 255.0 / samples;
            }
        }

        SelectObject(dc, old_brush);
        SelectObject(dc, old_pen);
        SelectObject(dc, old_bitmap);
        let _ = DeleteObject(brush.into());
        let _ = DeleteObject(pen.into());
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(dc);
    }
    coverage
}

/// Follows SVG path data (absolute M, L, C and Z, all the logo uses) into the open GDI path.
fn trace(dc: windows::Win32::Graphics::Gdi::HDC, data: &str, at: &impl Fn(f32, f32) -> POINT) {
    let mut tokens = Vec::new();
    let mut number = String::new();
    for c in data.chars() {
        if c.is_ascii_alphabetic() {
            if !number.is_empty() {
                tokens.push(Token::Number(number.parse().unwrap_or(0.0)));
                number.clear();
            }
            tokens.push(Token::Command(c));
        } else if c == ' ' || c == ',' {
            if !number.is_empty() {
                tokens.push(Token::Number(number.parse().unwrap_or(0.0)));
                number.clear();
            }
        } else {
            number.push(c);
        }
    }
    if !number.is_empty() {
        tokens.push(Token::Number(number.parse().unwrap_or(0.0)));
    }

    let mut command = 'M';
    let mut values = Vec::new();
    let flush = |command: char, values: &mut Vec<f32>| unsafe {
        match command {
            'M' if values.len() >= 2 => {
                let p = at(values[0], values[1]);
                let _ = MoveToEx(dc, p.x, p.y, None);
            }
            'L' => {
                for pair in values.chunks_exact(2) {
                    let p = at(pair[0], pair[1]);
                    let _ = LineTo(dc, p.x, p.y);
                }
            }
            'C' => {
                let points: Vec<POINT> = values.chunks_exact(2).map(|pair| at(pair[0], pair[1])).collect();
                let whole = points.len() / 3 * 3;
                let _ = PolyBezierTo(dc, &points[..whole]);
            }
            'Z' => {
                let _ = CloseFigure(dc);
            }
            _ => {}
        }
        values.clear();
    };
    for token in tokens {
        match token {
            Token::Command(c) => {
                flush(command, &mut values);
                command = c.to_ascii_uppercase();
                if command == 'Z' {
                    flush('Z', &mut values);
                }
            }
            Token::Number(n) => values.push(n),
        }
    }
    flush(command, &mut values);
}

enum Token {
    Command(char),
    Number(f32),
}

fn bitmap_info(size: i32) -> BITMAPINFO {
    BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size,
            biHeight: -size, // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

/// An icon from premultiplied BGRA pixels.
fn icon_from_pixels(pixels: &[u32], size: i32) -> HICON {
    unsafe {
        let info = bitmap_info(size);
        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let Ok(color) = CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0) else {
            return HICON::default();
        };
        std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits as *mut u32, pixels.len());
        let mask = CreateBitmap(size, size, 1, 1, None);
        let icon_info = ICONINFO { fIcon: true.into(), xHotspot: 0, yHotspot: 0, hbmMask: mask, hbmColor: color };
        let icon = CreateIconIndirect(&icon_info).unwrap_or_default();
        let _ = DeleteObject(mask.into());
        let _ = DeleteObject(color.into());
        icon
    }
}
