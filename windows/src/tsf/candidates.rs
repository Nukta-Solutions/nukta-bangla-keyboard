//! The suggestion list shown under (or over) the word being typed: the counterpart of
//! `Sources/NuktaInputMethod/CandidatePanel.swift`. (IBus draws its own, so Linux has none.)
//!
//! A top-most popup that never takes focus: the app being typed in keeps it, even while the list is
//! clicked. It draws the typed text and up to nine candidates, as a column or a row, scrolling to
//! keep the selection in view, exactly as on macOS.

use std::cell::RefCell;
use std::rc::Rc;

use windows::core::{w, Result, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUNDSMALL};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreatePen, CreateSolidBrush,
    DeleteDC, DeleteObject, DrawTextW, EndPaint, FillRect, GetDC, GetDeviceCaps, GetMonitorInfoW, GetSysColor,
    GetSysColorBrush, InvalidateRect, MonitorFromRect, ReleaseDC, RoundRect, SelectObject, SetBkMode, SetTextColor,
    CLEARTYPE_QUALITY, COLOR_BTNSHADOW, COLOR_GRAYTEXT, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT, COLOR_WINDOW,
    COLOR_WINDOWTEXT, DEFAULT_CHARSET, DT_CALCRECT, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, FF_DONTCARE,
    FONT_CLIP_PRECISION, FONT_OUTPUT_PRECISION, FW_NORMAL, HDC, HFONT, HGDIOBJ, LOGPIXELSY, MONITORINFO,
    MONITOR_DEFAULTTONEAREST, PAINTSTRUCT, PS_SOLID, SRCCOPY, SYS_COLOR_INDEX, TRANSPARENT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, KillTimer, RegisterClassExW,
    SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow, CS_DROPSHADOW, CS_IME, GWLP_USERDATA, HWND_TOPMOST,
    MA_NOACTIVATE, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEACTIVATE,
    WM_NCDESTROY, WM_PAINT, WM_TIMER, WNDCLASSEXW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use super::module;
use crate::gfx::{Color, Gfx, Rect, Style};
use crate::settings::{PopupDirection, PopupPosition};

/// What the list tells the text service.
#[derive(Clone, Copy, Debug)]
pub(super) enum Event {
    /// A candidate was clicked (index into the whole list).
    Pick(usize),
    /// Time to ask the app again where the word is (it didn't know yet).
    Reposition,
}

const CLASS_NAME: PCWSTR = w!("NuktaBanglaCandidateList");
/// At most this many candidates show at once.
const PAGE_SIZE: usize = 9;
const RETRY_TIMER: usize = 1;
/// Asks this many times, this often, while the app hasn't laid the word out yet.
const RETRIES: u32 = 12;
const RETRY_INTERVAL_MS: u32 = 60;

pub(super) struct CandidateWindow {
    hwnd: HWND,
    inner: Rc<Inner>,
}

struct Inner {
    state: RefCell<State>,
    on_event: Box<dyn Fn(Event)>,
}

#[derive(Default)]
struct State {
    candidates: Vec<String>,
    auxiliary: String,
    selected: usize,
    horizontal: bool,
    /// Index of the first candidate shown.
    first: usize,
    /// Candidate under a mouse press not yet released.
    pressed: Option<usize>,
    retries_left: u32,
    /// Laid out by `arrange`: each shown candidate's index and rectangle (client coordinates).
    cells: Vec<(usize, RECT)>,
    auxiliary_rect: RECT,
    more_rect: RECT,
    size: (i32, i32),
    fonts: Option<Fonts>,
}

struct Fonts {
    word: HFONT,
    small: HFONT,
    /// DirectWrite, with the brand's Noto Serif Bengali for the words: GDI can't shape that font
    /// (its ু lands on the wrong letter), DirectWrite can. None if Direct2D won't start: the GDI
    /// fonts (Nirmala UI) are used then.
    gfx: Option<Gfx>,
    /// Pixels per 96-dpi unit, ×100.
    scale: i32,
}

impl Fonts {
    fn new() -> Fonts {
        let dpi = unsafe {
            let dc = GetDC(None);
            let dpi = GetDeviceCaps(Some(dc), LOGPIXELSY);
            ReleaseDC(None, dc);
            dpi
        };
        let scale = if dpi > 0 { dpi * 100 / 96 } else { 100 };
        // The brand's Noto Serif Bengali from the install folder (`fonts\`, beside `x64\` and
        // `x86\`), through DirectWrite.
        let dll = super::register::dll_path();
        let bundled = std::path::Path::new(&dll).parent().and_then(std::path::Path::parent).and_then(crate::win::bangla_font_in);
        let gfx = Gfx::new(bundled.as_deref()).ok();
        Fonts { word: font(w!("Nirmala UI"), 21 * scale / 100), small: font(w!("Segoe UI"), 14 * scale / 100), gfx, scale }
    }

    fn px(&self, value: i32) -> i32 {
        value * self.scale / 100
    }

    fn dpi(&self) -> f32 {
        self.scale as f32 * 96.0 / 100.0
    }

    /// A text's size in pixels: the word (big) or the small type.
    fn measure(&self, dc: HDC, text: &str, word: bool) -> (i32, i32) {
        match &self.gfx {
            Some(gfx) => {
                let style = Style::new(if word { WORD_SIZE } else { SMALL_SIZE });
                let text = if text.is_empty() { " " } else { text };
                let to_px = |dips: f32| (dips * self.scale as f32 / 100.0).ceil() as i32;
                (to_px(gfx.text_width(text, style)), to_px(gfx.text_height(text, style, 4000.0)))
            }
            None => measure(dc, if word { self.word } else { self.small }, text),
        }
    }
}

/// The type sizes, in DIPs.
const WORD_SIZE: f32 = 21.0;
const SMALL_SIZE: f32 = 14.0;

/// A system colour as the drawing code's colour.
fn system_color(index: SYS_COLOR_INDEX) -> Color {
    let bgr = unsafe { GetSysColor(index) };
    Color(((bgr & 0xFF) << 16) | (bgr & 0xFF00) | ((bgr >> 16) & 0xFF), 1.0)
}

impl Drop for Fonts {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.word.into());
            let _ = DeleteObject(self.small.into());
        }
    }
}

fn font(face: PCWSTR, height: i32) -> HFONT {
    unsafe {
        CreateFontW(
            -height, 0, 0, 0, FW_NORMAL.0 as i32, 0, 0, 0, DEFAULT_CHARSET, FONT_OUTPUT_PRECISION(0),
            FONT_CLIP_PRECISION(0), CLEARTYPE_QUALITY, FF_DONTCARE.0 as u32, face,
        )
    }
}

impl CandidateWindow {
    /// The window is created on the calling thread (the app's), hidden.
    pub(super) fn new(on_event: impl Fn(Event) + 'static) -> Result<CandidateWindow> {
        register_class();
        let inner = Rc::new(Inner { state: RefCell::new(State::default()), on_event: Box::new(on_event) });
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                CLASS_NAME,
                w!(""),
                WS_POPUP,
                0, 0, 1, 1,
                None,
                None,
                Some(module()),
                None,
            )?
        };
        unsafe {
            // The window holds its own reference, dropped in WM_NCDESTROY.
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Rc::into_raw(inner.clone()) as _);
            let corners = DWMWCP_ROUNDSMALL;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &corners as *const _ as *const _,
                std::mem::size_of_val(&corners) as u32,
            );
        }
        inner.state.borrow_mut().fonts = Some(Fonts::new());
        Ok(CandidateWindow { hwnd, inner })
    }

    pub(super) fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// Shows the list by `cursor` (the word's rectangle, screen coordinates). `exact` false: that is
    /// only a guess, and the window asks again shortly (`Event::Reposition`).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn show(
        &self,
        candidates: &[String],
        auxiliary: &str,
        selected: usize,
        cursor: RECT,
        exact: bool,
        position: PopupPosition,
        direction: PopupDirection,
    ) {
        if candidates.is_empty() {
            self.hide();
            return;
        }
        let size = {
            let mut state = self.inner.state.borrow_mut();
            if state.candidates != candidates {
                state.first = 0;
            }
            state.candidates = candidates.to_vec();
            state.auxiliary = auxiliary.to_owned();
            state.selected = selected.min(candidates.len() - 1);
            state.horizontal = direction == PopupDirection::Horizontal;
            state.pressed = None;
            // Scroll just enough to show the selection.
            if state.selected < state.first {
                state.first = state.selected;
            }
            if state.selected >= state.first + PAGE_SIZE {
                state.first = state.selected + 1 - PAGE_SIZE;
            }
            state.first = state.first.min(candidates.len().saturating_sub(PAGE_SIZE));
            arrange(&mut state, self.hwnd);
            state.size
        };
        self.place(size, cursor, position);
        if exact {
            self.stop_retrying();
        } else {
            self.inner.state.borrow_mut().retries_left = RETRIES;
            unsafe { SetTimer(Some(self.hwnd), RETRY_TIMER, RETRY_INTERVAL_MS, None) };
        }
    }

    /// Moves the shown list to the word's real position, once the app knows it.
    pub(super) fn move_to(&self, cursor: RECT, position: PopupPosition) {
        let size = self.inner.state.borrow().size;
        self.place(size, cursor, position);
        self.stop_retrying();
    }

    pub(super) fn stop_retrying(&self) {
        self.inner.state.borrow_mut().retries_left = 0;
        unsafe {
            let _ = KillTimer(Some(self.hwnd), RETRY_TIMER);
        }
    }

    pub(super) fn hide(&self) {
        self.stop_retrying();
        self.inner.state.borrow_mut().pressed = None;
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    /// Left edge at the word; under it (or over it) with a small gap, on the other side when the
    /// preferred one has no room, and always inside the work area of the word's monitor.
    fn place(&self, (width, height): (i32, i32), cursor: RECT, position: PopupPosition) {
        let gap = 4;
        let visible = unsafe {
            let monitor = MonitorFromRect(&cursor, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
            if GetMonitorInfoW(monitor, &mut info).as_bool() { info.rcWork } else { cursor }
        };
        let below = cursor.bottom + gap;
        let above = cursor.top - gap - height;
        let fits_below = below + height <= visible.bottom;
        let fits_above = above >= visible.top;
        let mut y = match position {
            PopupPosition::Below => if fits_below || !fits_above { below } else { above },
            PopupPosition::Above => if fits_above || !fits_below { above } else { below },
        };
        let x = cursor.left.max(visible.left).min(visible.right - width);
        y = y.max(visible.top).min(visible.bottom - height);
        unsafe {
            let _ = SetWindowPos(self.hwnd, Some(HWND_TOPMOST), x, y, width, height, SWP_NOACTIVATE | SWP_SHOWWINDOW);
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }
}

impl Drop for CandidateWindow {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

fn register_class() {
    let class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_IME | CS_DROPSHADOW,
        lpfnWndProc: Some(window_procedure),
        hInstance: module(),
        lpszClassName: CLASS_NAME,
        ..Default::default()
    };
    // Fails harmlessly once the class exists (a second thread, a second activation).
    unsafe { RegisterClassExW(&class) };
}

// MARK: Layout

/// Where more candidates exist beyond those shown, as a small arrow.
fn more_sign(state: &State) -> Option<&'static str> {
    let before = state.first > 0;
    let after = state.first + PAGE_SIZE < state.candidates.len();
    match (before, after, state.horizontal) {
        (false, false, _) => None,
        (true, true, true) => Some("◂▸"),
        (true, true, false) => Some("▴▾"),
        (true, false, true) => Some("◂"),
        (true, false, false) => Some("▴"),
        (false, true, true) => Some("▸"),
        (false, true, false) => Some("▾"),
    }
}

fn shown(state: &State) -> std::ops::Range<usize> {
    state.first..(state.first + PAGE_SIZE).min(state.candidates.len())
}

fn measure(dc: HDC, font: HFONT, text: &str) -> (i32, i32) {
    let mut text: Vec<u16> = text.encode_utf16().collect();
    if text.is_empty() {
        text.push(' ' as u16);
    }
    let mut rect = RECT::default();
    unsafe {
        let old = SelectObject(dc, font.into());
        DrawTextW(dc, &mut text, &mut rect, DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(dc, old);
    }
    (rect.right - rect.left, rect.bottom - rect.top)
}

fn arrange(state: &mut State, hwnd: HWND) {
    let Some(fonts) = state.fonts.as_ref() else { return };
    let dc = unsafe { GetDC(Some(hwnd)) };
    let padding = fonts.px(6);
    let cell_padding = (fonts.px(8), fonts.px(3));

    let auxiliary = fonts.measure(dc, &state.auxiliary, false);
    let more = more_sign(state).map_or((0, 0), |sign| fonts.measure(dc, sign, false));
    let auxiliary_rect = RECT {
        left: padding + cell_padding.0,
        top: padding,
        right: padding + cell_padding.0 + auxiliary.0,
        bottom: padding + auxiliary.1,
    };

    let sizes: Vec<(i32, i32)> = shown(state)
        .map(|index| {
            let number = fonts.measure(dc, &(index + 1).to_string(), false);
            let word = fonts.measure(dc, &state.candidates[index], true);
            (number.0 + fonts.px(4) + word.0 + 2 * cell_padding.0, number.1.max(word.1) + 2 * cell_padding.1)
        })
        .collect();
    unsafe { ReleaseDC(Some(hwnd), dc) };

    let row_height = sizes.iter().map(|s| s.1).max().unwrap_or(0);
    let top = auxiliary_rect.bottom + fonts.px(2);
    let mut cells = Vec::new();
    let (mut width, height);
    if state.horizontal {
        let mut x = padding;
        for (index, size) in shown(state).zip(&sizes) {
            cells.push((index, RECT { left: x, top, right: x + size.0, bottom: top + row_height }));
            x += size.0 + fonts.px(2);
        }
        width = x - fonts.px(2) + padding;
        height = top + row_height + padding;
    } else {
        let column = sizes.iter().map(|s| s.0).max().unwrap_or(0);
        let mut y = top;
        for index in shown(state) {
            cells.push((index, RECT { left: padding, top: y, right: padding + column, bottom: y + row_height }));
            y += row_height;
        }
        width = column + 2 * padding;
        height = y + padding;
    }

    // The typed text and the "more" sign share the top line.
    let top_line = auxiliary_rect.right + if more.0 > 0 { fonts.px(12) + more.0 } else { 0 } + cell_padding.0 + padding;
    width = width.max(top_line).max(fonts.px(60));
    state.more_rect = RECT {
        left: width - padding - cell_padding.0 - more.0,
        top: padding,
        right: width - padding - cell_padding.0,
        bottom: padding + more.1,
    };
    if !state.horizontal {
        for (_, rect) in &mut cells {
            rect.right = width - padding;
        }
    }
    state.auxiliary_rect = auxiliary_rect;
    state.cells = cells;
    state.size = (width, height);
}

// MARK: Drawing

fn paint(hwnd: HWND, state: &State) {
    let Some(fonts) = state.fonts.as_ref() else { return };
    if let Some(gfx) = &fonts.gfx {
        return paint_directwrite(hwnd, state, fonts, gfx);
    }
    unsafe {
        let mut ps = PAINTSTRUCT::default();
        let screen = BeginPaint(hwnd, &mut ps);
        let (width, height) = state.size;
        // Drawn off screen, then copied in one go: no flicker while typing.
        let dc = CreateCompatibleDC(Some(screen));
        let bitmap = CreateCompatibleBitmap(screen, width, height);
        let old_bitmap = SelectObject(dc, bitmap.into());

        let all = RECT { left: 0, top: 0, right: width, bottom: height };
        FillRect(dc, &all, GetSysColorBrush(COLOR_WINDOW));
        frame(dc, all, COLORREF(GetSysColor(COLOR_BTNSHADOW)));
        SetBkMode(dc, TRANSPARENT);

        draw_text(dc, fonts.small, &state.auxiliary, state.auxiliary_rect, COLORREF(GetSysColor(COLOR_GRAYTEXT)));
        if let Some(sign) = more_sign(state) {
            draw_text(dc, fonts.small, sign, state.more_rect, COLORREF(GetSysColor(COLOR_GRAYTEXT)));
        }

        let highlighted = state.pressed.unwrap_or(state.selected);
        for (index, rect) in &state.cells {
            let is_highlighted = *index == highlighted;
            if is_highlighted {
                rounded(dc, *rect, COLORREF(GetSysColor(COLOR_HIGHLIGHT)), fonts.px(8));
            }
            let (number_color, word_color) = if is_highlighted {
                (COLORREF(GetSysColor(COLOR_HIGHLIGHTTEXT)), COLORREF(GetSysColor(COLOR_HIGHLIGHTTEXT)))
            } else {
                (COLORREF(GetSysColor(COLOR_GRAYTEXT)), COLORREF(GetSysColor(COLOR_WINDOWTEXT)))
            };
            let number = (index + 1).to_string();
            let number_width = measure(dc, fonts.small, &number).0;
            let mut text_rect = *rect;
            text_rect.left += fonts.px(8);
            draw_text(dc, fonts.small, &number, text_rect, number_color);
            text_rect.left += number_width + fonts.px(4);
            draw_text(dc, fonts.word, &state.candidates[*index], text_rect, word_color);
        }

        let _ = BitBlt(screen, 0, 0, width, height, Some(dc), 0, 0, SRCCOPY);
        SelectObject(dc, old_bitmap);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(dc);
        let _ = EndPaint(hwnd, &ps);
    }
}

/// The same as the GDI drawing below, with DirectWrite: the words in Noto Serif Bengali.
fn paint_directwrite(hwnd: HWND, state: &State, fonts: &Fonts, gfx: &Gfx) {
    unsafe {
        let mut ps = PAINTSTRUCT::default();
        let screen = BeginPaint(hwnd, &mut ps);
        let (width, height) = state.size;
        let dip = |px: i32| px as f32 * 100.0 / fonts.scale as f32;
        let rect = |r: RECT| Rect { left: dip(r.left), top: dip(r.top), right: dip(r.right), bottom: dip(r.bottom) };
        gfx.paint(screen, width, height, fonts.dpi(), |canvas| {
            let all = Rect::new(0.0, 0.0, dip(width), dip(height));
            canvas.fill_rect(all, system_color(COLOR_WINDOW));
            canvas.stroke_round(all, 0.0, system_color(COLOR_BTNSHADOW));
            let grey = system_color(COLOR_GRAYTEXT);
            canvas.text(&state.auxiliary, rect(state.auxiliary_rect), Style::new(SMALL_SIZE), grey);
            if let Some(sign) = more_sign(state) {
                canvas.text(sign, rect(state.more_rect), Style::new(SMALL_SIZE), grey);
            }
            let highlighted = state.pressed.unwrap_or(state.selected);
            for (index, cell) in &state.cells {
                let is_highlighted = *index == highlighted;
                let cell = rect(*cell);
                if is_highlighted {
                    canvas.fill_round(cell, 4.0, system_color(COLOR_HIGHLIGHT));
                }
                let (number_color, word_color) = if is_highlighted {
                    (system_color(COLOR_HIGHLIGHTTEXT), system_color(COLOR_HIGHLIGHTTEXT))
                } else {
                    (grey, system_color(COLOR_WINDOWTEXT))
                };
                let number = (index + 1).to_string();
                let number_width = gfx.text_width(&number, Style::new(SMALL_SIZE));
                let mut text = Rect { left: cell.left + 8.0, ..cell };
                canvas.text(&number, text, Style::new(SMALL_SIZE), number_color);
                text.left += number_width + 4.0;
                canvas.text(&state.candidates[*index], text, Style::new(WORD_SIZE), word_color);
            }
        });
        let _ = EndPaint(hwnd, &ps);
    }
}

fn draw_text(dc: HDC, font: HFONT, text: &str, rect: RECT, color: COLORREF) {
    let mut text: Vec<u16> = text.encode_utf16().collect();
    let mut rect = rect;
    unsafe {
        let old = SelectObject(dc, font.into());
        SetTextColor(dc, color);
        DrawTextW(dc, &mut text, &mut rect, DT_SINGLELINE | DT_NOPREFIX | DT_VCENTER);
        SelectObject(dc, old);
    }
}

fn rounded(dc: HDC, rect: RECT, color: COLORREF, radius: i32) {
    unsafe {
        let brush = CreateSolidBrush(color);
        let pen = CreatePen(PS_SOLID, 1, color);
        let old_brush = SelectObject(dc, brush.into());
        let old_pen = SelectObject(dc, pen.into());
        let _ = RoundRect(dc, rect.left, rect.top, rect.right, rect.bottom, radius, radius);
        SelectObject(dc, old_brush);
        SelectObject(dc, old_pen);
        let _ = DeleteObject(HGDIOBJ::from(brush));
        let _ = DeleteObject(HGDIOBJ::from(pen));
    }
}

/// A one-pixel border.
fn frame(dc: HDC, rect: RECT, color: COLORREF) {
    unsafe {
        let brush = CreateSolidBrush(color);
        for edge in [
            RECT { bottom: rect.top + 1, ..rect },
            RECT { top: rect.bottom - 1, ..rect },
            RECT { right: rect.left + 1, ..rect },
            RECT { left: rect.right - 1, ..rect },
        ] {
            FillRect(dc, &edge, brush);
        }
        let _ = DeleteObject(HGDIOBJ::from(brush));
    }
}

// MARK: Window procedure

fn hit(state: &State, lparam: LPARAM) -> Option<usize> {
    let point = POINT { x: (lparam.0 & 0xFFFF) as i16 as i32, y: ((lparam.0 >> 16) & 0xFFFF) as i16 as i32 };
    state
        .cells
        .iter()
        .find(|(_, r)| point.x >= r.left && point.x < r.right && point.y >= r.top && point.y < r.bottom)
        .map(|(index, _)| *index)
}

unsafe extern "system" fn window_procedure(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let pointer = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const Inner;
    if pointer.is_null() {
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }
    if message == WM_NCDESTROY {
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            drop(Rc::from_raw(pointer));
            return DefWindowProcW(hwnd, message, wparam, lparam);
        }
    }
    // A reference of our own: the event handler may hide or rebuild the list.
    let inner = unsafe {
        Rc::increment_strong_count(pointer);
        Rc::from_raw(pointer)
    };
    // Nothing may unwind into Windows from here.
    let handled = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handle(hwnd, &inner, message, lparam)));
    match handled {
        Ok(Some(result)) => result,
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn handle(hwnd: HWND, inner: &Inner, message: u32, lparam: LPARAM) -> Option<LRESULT> {
    match message {
        WM_MOUSEACTIVATE => Some(LRESULT(MA_NOACTIVATE as isize)),
        WM_PAINT => {
            paint(hwnd, &inner.state.borrow());
            Some(LRESULT(0))
        }
        WM_LBUTTONDOWN => {
            let mut state = inner.state.borrow_mut();
            state.pressed = hit(&state, lparam);
            unsafe {
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            Some(LRESULT(0))
        }
        WM_LBUTTONUP => {
            let picked = {
                let mut state = inner.state.borrow_mut();
                let pressed = state.pressed.take();
                unsafe {
                    let _ = InvalidateRect(Some(hwnd), None, false);
                }
                hit(&state, lparam).filter(|&index| Some(index) == pressed)
            };
            if let Some(index) = picked {
                (inner.on_event)(Event::Pick(index));
            }
            Some(LRESULT(0))
        }
        WM_TIMER => {
            let ask = {
                let mut state = inner.state.borrow_mut();
                if state.retries_left > 0 {
                    state.retries_left -= 1;
                    true
                } else {
                    false
                }
            };
            if ask {
                (inner.on_event)(Event::Reposition);
            } else {
                unsafe {
                    let _ = KillTimer(Some(hwnd), RETRY_TIMER);
                }
            }
            Some(LRESULT(0))
        }
        _ => None,
    }
}
