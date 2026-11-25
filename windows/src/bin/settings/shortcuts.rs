//! নিজের শর্টকাট: the dialog where the user's shortcuts are added, changed and removed
//! (`NuktaBanglaSettings.exe --shortcuts`, opened by সাজান… in the settings). Its design is the
//! dialog in `windows/design/settings-preview.html`.
//!
//! The rules (what a code may be, the file every app reads) live in riti-bridge
//! (`nukta_riti::shortcuts`); this is only the window. Like the rest of the settings it is drawn with
//! Direct2D, except the two text boxes, which are Windows' own edit controls: typing Bangla in them
//! goes through নুকতা বাংলা itself. The code box is marked as English-only, so the text service
//! leaves its keys alone whatever layout is on.

use std::cell::Cell;

use nukta_bangla::gfx::{wide, Align, Gfx, Icon, Rect, Style};
use nukta_bangla::win;
use nukta_riti::shortcuts::{dictionary_word, normalize_code, normalize_text, ShortcutError};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateFontW, CreateSolidBrush, SetBkColor, SetTextColor, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH,
    CLEARTYPE_QUALITY, HBRUSH, HDC, HFONT, OUT_DEFAULT_PRECIS,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{EM_LIMITTEXT, EM_SETCUEBANNER, EM_SETSEL};
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GetWindowTextLengthW, GetWindowTextW, SendMessageW, SetWindowPos, SetWindowTextW,
    EN_CHANGE, ES_AUTOHSCROLL, ES_LOWERCASE, HMENU, IDCANCEL, IDOK, SWP_NOACTIVATE,
    SWP_NOZORDER, WINDOW_EX_STYLE, WINDOW_STYLE, WM_SETFONT, WS_CHILD, WS_TABSTOP, WS_VISIBLE,
};

use super::{card, refresh, with_app, Action, Ctx, BORDER, BOX, BOX_HOVER, DIM, FAINT, INK, MODE_ON, ORANGE, PAD};

pub(super) const WIDTH: f32 = 460.0;

const CODE_ID: i32 = 101;
const TEXT_ID: i32 = 102;
/// The text boxes' frames, and the text inside them.
const BOX_HEIGHT: f32 = 36.0;
const EDIT_HEIGHT: f32 = 24.0;
const CODE_WIDTH: f32 = 96.0;
const ROW_HEIGHT: f32 = 38.0;
const WARN: nukta_bangla::gfx::Color = nukta_bangla::gfx::Color(0xF0A24E, 1.0);

const X: Icon = Icon { view: 24.0, paths: &["M18 6 6 18", "m6 6 12 12"] };

/// The dialog's state, kept in the settings program's `App`.
pub(super) struct Dialog {
    code: HWND,
    text: HWND,
    /// The edit controls' background (WM_CTLCOLOREDIT).
    brush: HBRUSH,
    /// Every shortcut, ordered by code.
    list: Vec<(String, String)>,
    /// The code whose text is being changed (picked in the list, or typed).
    editing: Option<String>,
    /// A code the user was warned about: pressing the button again saves it.
    pending: Option<String>,
    warning: Option<String>,
    /// Where the two boxes were drawn, in content DIPs: the edit controls go inside them.
    code_box: Cell<Rect>,
    text_box: Cell<Rect>,
}

impl Dialog {
    /// The two edit controls inside `window`, at `dpi`.
    pub(super) fn new(window: HWND, dpi: f32, list: Vec<(String, String)>) -> Dialog {
        let px = |v: f32| (v * dpi / 96.0).round() as i32;
        unsafe {
            let instance = GetModuleHandleW(None).unwrap_or_default();
            let edit = |id: i32, extra: i32| {
                let style = WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE((ES_AUTOHSCROLL | extra) as u32);
                CreateWindowExW(
                    WINDOW_EX_STYLE(0), w!("EDIT"), PCWSTR::null(), style, 0, 0, 10, 10, Some(window),
                    Some(HMENU(id as isize as *mut _)), Some(instance.into()), None,
                )
                .unwrap_or_default()
            };
            let code = edit(CODE_ID, ES_LOWERCASE);
            let text = edit(TEXT_ID, 0);
            // GDI draws the boxes, and it mis-shapes Noto Serif Bengali: Windows' own fonts here.
            let font = |face: PCWSTR| {
                CreateFontW(
                    -px(14.0), 0, 0, 0, 400, 0, 0, 0, DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
                    CLEARTYPE_QUALITY, DEFAULT_PITCH.0 as u32, face,
                )
            };
            set_font(code, font(w!("Consolas")));
            set_font(text, font(w!("Nirmala UI")));
            SendMessageW(code, EM_LIMITTEXT, Some(WPARAM(nukta_riti::shortcuts::MAX_CODE)), None);
            SendMessageW(text, EM_LIMITTEXT, Some(WPARAM(nukta_riti::shortcuts::MAX_TEXT)), None);
            let cue = |edit: HWND, text: &str| {
                let text = wide(text);
                SendMessageW(edit, EM_SETCUEBANNER, Some(WPARAM(1)), Some(LPARAM(text.as_ptr() as isize)));
            };
            cue(code, "কোড: asw");
            cue(text, "পুরো লেখা");
            win::mark_latin_only(code);
            Dialog {
                code,
                text,
                brush: CreateSolidBrush(bgr(BOX)),
                list,
                editing: None,
                pending: None,
                warning: None,
                code_box: Cell::new(Rect::new(0.0, 0.0, 0.0, 0.0)),
                text_box: Cell::new(Rect::new(0.0, 0.0, 0.0, 0.0)),
            }
        }
    }

    fn label(&self) -> &'static str {
        if self.pending.is_some() {
            "তবুও যোগ করুন"
        } else if self.editing.is_some() {
            "বদলান"
        } else {
            "যোগ করুন"
        }
    }
}

fn set_font(edit: HWND, font: HFONT) {
    unsafe {
        SendMessageW(edit, WM_SETFONT, Some(WPARAM(font.0 as usize)), Some(LPARAM(1)));
    }
}

/// A colour as GDI wants it (0x00BBGGRR).
fn bgr(color: nukta_bangla::gfx::Color) -> COLORREF {
    let rgb = color.0;
    COLORREF(((rgb & 0xFF) << 16) | (rgb & 0xFF00) | ((rgb >> 16) & 0xFF))
}

fn window_text(window: HWND) -> String {
    unsafe {
        let mut buffer = vec![0u16; GetWindowTextLengthW(window).max(0) as usize + 1];
        let length = GetWindowTextW(window, &mut buffer).max(0) as usize;
        String::from_utf16_lossy(&buffer[..length])
    }
}

fn set_text(window: HWND, text: &str) {
    let text = wide(text);
    unsafe {
        let _ = SetWindowTextW(window, PCWSTR(text.as_ptr()));
        // The caret at the end.
        SendMessageW(window, EM_SETSEL, Some(WPARAM(usize::MAX >> 1)), Some(LPARAM(-1)));
    }
}

/// `text` cut to fit `width` in `style`, with … at the end if it was cut.
fn fitted(gfx: &Gfx, text: &str, style: Style, width: f32) -> String {
    if gfx.text_width(text, style) <= width {
        return text.to_owned();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let cut: String = chars.iter().collect::<String>() + "…";
        if gfx.text_width(&cut, style) <= width {
            return cut;
        }
    }
    "…".to_owned()
}

// MARK: Drawing

pub(super) fn draw(ctx: &mut Ctx, d: &Dialog) -> f32 {
    let width = WIDTH - 2.0 * PAD;
    let mut y = 22.0;
    ctx.text("নিজের শর্টকাট", Rect::new(PAD, y - 2.0, width, 30.0), Style::new(19.0).weight(700), INK);
    y += 32.0;
    let lead = "কোডটা পুরো লিখলে লেখাটা তালিকার প্রথমে আসে, সাধারণ শব্দটা থাকে দ্বিতীয়তে। কোডের শুধু শুরুটা মিললে কিছু আসে না।";
    let lead_style = Style::new(12.5).wrap();
    let height = ctx.height(lead, lead_style, width);
    ctx.text(lead, Rect::new(PAD, y, width, height), lead_style, DIM);
    y += height + 16.0;

    // The code, the text, and the button.
    let label = d.label();
    let button_width = ctx.gfx.text_width(label, Style::new(13.0).weight(600)) + 32.0;
    let code_box = Rect::new(PAD, y, CODE_WIDTH, BOX_HEIGHT);
    let text_left = PAD + CODE_WIDTH + 8.0;
    let text_box = Rect::new(text_left, y, WIDTH - PAD - button_width - 8.0 - text_left, BOX_HEIGHT);
    for frame in [code_box, text_box] {
        ctx.fill(frame, 6.0, BOX);
        ctx.outline(frame, 6.0, BORDER);
    }
    d.code_box.set(code_box);
    d.text_box.set(text_box);
    ctx.primary(Rect::new(WIDTH - PAD - button_width, y, button_width, BOX_HEIGHT), label, Action::SaveShortcut, true);
    y += BOX_HEIGHT;

    if let Some(warning) = &d.warning {
        y += 10.0;
        let style = Style::new(12.5).wrap();
        let height = ctx.height(warning, style, width);
        ctx.text(warning, Rect::new(PAD, y, width, height), style, WARN);
        y += height;
    }
    y += 14.0;

    // The list.
    if d.list.is_empty() {
        let height = 70.0;
        card(ctx, PAD, y, width, height);
        let empty = "এখনো কোনো শর্টকাট নেই।\nযেমন: asw → আসসালামু আলাইকুম";
        let style = Style::new(13.0).align(Align::Center).wrap();
        let text_height = ctx.height(empty, style, width - 28.0);
        ctx.text(empty, Rect::new(PAD + 14.0, y + (height - text_height) / 2.0, width - 28.0, text_height), style, DIM);
        y += height;
    } else {
        let height = d.list.len() as f32 * ROW_HEIGHT;
        card(ctx, PAD, y, width, height);
        let text_style = Style::new(14.0);
        for (index, (code, text)) in d.list.iter().enumerate() {
            let row = Rect::new(PAD, y + index as f32 * ROW_HEIGHT, width, ROW_HEIGHT);
            if index > 0 {
                ctx.fill(Rect::new(row.left + 1.0, row.top, width - 2.0, 1.0), 0.0, BORDER);
            }
            let hovered = ctx.hit(row, Action::EditShortcut(index));
            let x_rect = Rect::new(row.right - 6.0 - 26.0, row.top + 6.0, 26.0, 26.0);
            let x_hovered = ctx.hit(x_rect, Action::RemoveShortcut(index));
            if d.editing.as_deref() == Some(code.as_str()) {
                ctx.fill(row.inset(1.0), 9.0, MODE_ON);
            } else if hovered || x_hovered {
                ctx.fill(row.inset(1.0), 9.0, BOX);
            }
            ctx.text(code, Rect::new(row.left + 12.0, row.top, CODE_WIDTH - 4.0, ROW_HEIGHT), Style::new(13.0), ORANGE);
            let text_left = row.left + 12.0 + CODE_WIDTH + 8.0;
            let text_width = x_rect.left - 8.0 - text_left;
            let shown = fitted(ctx.gfx, text, text_style, text_width);
            ctx.text(&shown, Rect::new(text_left, row.top, text_width, ROW_HEIGHT), text_style, INK);
            if x_hovered {
                ctx.fill(x_rect, 6.0, BOX_HOVER);
            }
            ctx.icon(&X, x_rect.left + 6.0, x_rect.top + 6.0, 14.0, if x_hovered { INK } else { DIM });
        }
        y += height;
    }
    y += 14.0;

    let hint = "কোড হবে ২ থেকে ১২টা ইংরেজি অক্ষর, সংখ্যা ছাড়া (লেখার সময় সংখ্যা চাপলে তালিকা থেকে শব্দ বাছাই হয়)। কোডের ঘরে নুকতা নিজে থেকেই ইংরেজি লেখে। কোনো সারিতে ক্লিক করলে সেটা বদলানো যায়।";
    let hint_style = Style::new(12.5).wrap();
    let height = ctx.height(hint, hint_style, width);
    ctx.text(hint, Rect::new(PAD, y, width, height), hint_style, DIM);
    y += height + 14.0;
    ctx.text("পরিবর্তন সাথে সাথে সব অ্যাপে চালু হয়", Rect::new(0.0, y, WIDTH, 18.0), Style::new(12.0).align(Align::Center), FAINT);
    y + 18.0 + 18.0
}

/// Puts the edit controls inside the boxes drawn for them, after a layout pass or a scroll.
pub(super) fn place() {
    let Some(places) = with_app(|app| {
        let d = app.dialog.as_ref()?;
        let px = |v: f32| (v * app.dpi / 96.0).round() as i32;
        let inside = |frame: Rect| {
            let top = frame.top + (BOX_HEIGHT - EDIT_HEIGHT) / 2.0 - app.scroll;
            (px(frame.left + 9.0), px(top), px(frame.width() - 18.0), px(EDIT_HEIGHT))
        };
        Some([(d.code, inside(d.code_box.get())), (d.text, inside(d.text_box.get()))])
    })
    .flatten() else {
        return;
    };
    for (edit, (x, y, width, height)) in places {
        unsafe {
            let _ = SetWindowPos(edit, None, x, y, width, height, SWP_NOZORDER | SWP_NOACTIVATE);
        }
    }
}

/// The edit controls' colours: WM_CTLCOLOREDIT.
pub(super) fn colors(dc: HDC) -> Option<LRESULT> {
    let brush = with_app(|app| app.dialog.as_ref().map(|d| d.brush)).flatten()?;
    unsafe {
        SetTextColor(dc, bgr(nukta_bangla::gfx::Color(0xEBEBEB, 1.0)));
        SetBkColor(dc, bgr(BOX));
    }
    Some(LRESULT(brush.0 as isize))
}

pub(super) fn focus_code() {
    if let Some(code) = with_app(|app| app.dialog.as_ref().map(|d| d.code)).flatten() {
        unsafe {
            let _ = SetFocus(Some(code));
        }
    }
}

// MARK: Changes

/// WM_COMMAND: Enter in a box saves (IDOK), Esc closes (IDCANCEL), and typing resets the warning.
pub(super) fn command(window: HWND, wparam: WPARAM) {
    let id = (wparam.0 & 0xFFFF) as i32;
    let notification = ((wparam.0 >> 16) & 0xFFFF) as u32;
    match id {
        id if id == IDOK.0 => save(),
        id if id == IDCANCEL.0 => unsafe {
            let _ = DestroyWindow(window);
        },
        CODE_ID if notification == EN_CHANGE => code_changed(),
        TEXT_ID if notification == EN_CHANGE && warning_dropped() => refresh(),
        _ => {}
    }
}

/// Typing again drops the warning (and the "again to save" it was waiting for). True if there was one.
fn warning_dropped() -> bool {
    with_app(|app| app.dialog.as_mut().is_some_and(|d| d.pending.take().is_some() | d.warning.take().is_some())).unwrap_or(false)
}

/// Only English letters stay in the code box (pasted text too). A code that exists already is
/// being changed: the button says বদলান.
fn code_changed() {
    let Some(code) = with_app(|app| app.dialog.as_ref().map(|d| d.code)).flatten() else { return };
    let typed = window_text(code);
    let kept: String = typed.chars().filter(char::is_ascii_alphabetic).map(|c| c.to_ascii_lowercase()).collect();
    if kept != typed {
        // Comes back here through EN_CHANGE.
        return set_text(code, &kept);
    }
    with_app(|app| {
        if let Some(d) = app.dialog.as_mut() {
            d.editing = d.list.iter().any(|(c, _)| *c == kept).then_some(kept);
            d.pending = None;
            d.warning = None;
        }
    });
    refresh();
}

/// যোগ করুন / বদলান / তবুও যোগ করুন.
pub(super) fn save() {
    let Some((code_box, text_box)) = with_app(|app| app.dialog.as_ref().map(|d| (d.code, d.text))).flatten() else { return };
    let (typed_code, typed_text) = (window_text(code_box), window_text(text_box));
    let warn = |warning: String, pending: Option<String>| {
        with_app(|app| {
            if let Some(d) = app.dialog.as_mut() {
                d.warning = Some(warning);
                d.pending = pending;
            }
        });
        refresh();
    };
    let Some(code) = normalize_code(&typed_code) else {
        return warn("কোড হবে ২ থেকে ১২টা ইংরেজি অক্ষর, সংখ্যা ছাড়া।".to_owned(), None);
    };
    let text = match normalize_text(&typed_text) {
        Ok(text) => text,
        Err(ShortcutError::TextTooLong) => return warn("লেখাটা ৩০০ অক্ষরের বেশি হতে পারবে না।".to_owned(), None),
        Err(_) => return warn("পুরো লেখাটা দিন।".to_owned(), None),
    };
    let (exists, warned) = with_app(|app| {
        let d = app.dialog.as_ref();
        (
            d.is_some_and(|d| d.list.iter().any(|(c, _)| *c == code)),
            d.is_some_and(|d| d.pending.as_deref() == Some(code.as_str())),
        )
    })
    .unwrap_or((false, false));
    if !exists && !warned {
        if let Some(word) = dictionary_word(&code) {
            return warn(
                format!("{code} দিয়ে অভিধানের শব্দ «{word}»-ও লেখা হয়। শর্টকাট করলে {word} লিখতে গেলে আগে শর্টকাট আসবে। তবুও রাখতে আবার চাপুন।"),
                Some(code),
            );
        }
    }
    let saved = with_app(|app| app.shortcuts.set(&code, &text).is_ok()).unwrap_or(false);
    if !saved {
        return warn("সেভ করা গেল না। আবার চেষ্টা করুন।".to_owned(), None);
    }
    after_change(true);
}

/// A row was clicked: its code and text go into the boxes for changing.
pub(super) fn edit(index: usize) {
    let Some((code_box, text_box, row)) =
        with_app(|app| app.dialog.as_ref().and_then(|d| Some((d.code, d.text, d.list.get(index)?.clone())))).flatten()
    else {
        return;
    };
    set_text(code_box, &row.0);
    set_text(text_box, &row.1);
    unsafe {
        let _ = SetFocus(Some(text_box));
    }
}

/// The × on a row.
pub(super) fn remove(index: usize) {
    let cleared = with_app(|app| {
        let code = app.dialog.as_ref()?.list.get(index)?.0.clone();
        app.shortcuts.remove(&code);
        Some(app.dialog.as_ref()?.editing.as_deref() == Some(code.as_str()))
    })
    .flatten();
    if let Some(cleared) = cleared {
        after_change(cleared);
    }
}

/// The list is read again; `clear` empties the boxes and puts the caret in the code box.
fn after_change(clear: bool) {
    let boxes = with_app(|app| {
        let list: Vec<_> = app.shortcuts.all().into_iter().collect();
        let d = app.dialog.as_mut()?;
        d.list = list;
        d.pending = None;
        d.warning = None;
        if clear {
            d.editing = None;
        }
        Some((d.code, d.text))
    })
    .flatten();
    if let (Some((code, text)), true) = (boxes, clear) {
        set_text(text, "");
        set_text(code, "");
        focus_code();
    }
    refresh();
}
