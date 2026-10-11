//! NuktaBanglaSettings.exe: সেটিংস, opened from the taskbar indicator's menu. The counterpart of
//! `Sources/NuktaInputMethod/SettingsWindow.swift`: the same controls, and every change is saved
//! (to settings.json) and taken up by every app straight away.
//!
//! The installer also runs it to add the keyboard to the user's languages and to remove it:
//!
//! ```text
//! NuktaBanglaSettings.exe            the settings window
//! NuktaBanglaSettings.exe --about    the about box
//! NuktaBanglaSettings.exe --enable   add নুকতা বাংলা to this user's keyboards
//! NuktaBanglaSettings.exe --disable  remove it
//! ```

#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(not(windows))]
fn main() {
    eprintln!("NuktaBanglaSettings is the Windows settings program; it does nothing elsewhere.");
}

#[cfg(windows)]
fn main() {
    std::process::exit(app::main());
}

#[cfg(windows)]
mod app {
    use std::cell::RefCell;

    use nukta_bangla::phonetic::TypingMode;
    use nukta_bangla::settings::{Layout, PopupDirection, PopupPosition, Settings};
    use nukta_bangla::win;
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        CreateFontW, DeleteObject, DrawTextW, GetDC, GetSysColor, GetSysColorBrush, ReleaseDC, SelectObject,
        SetBkMode, SetTextColor, CLEARTYPE_QUALITY, COLOR_GRAYTEXT, COLOR_WINDOW, COLOR_WINDOWTEXT, DEFAULT_CHARSET,
        DT_CALCRECT, DT_NOPREFIX, DT_WORDBREAK, FF_DONTCARE, FONT_CLIP_PRECISION, FONT_OUTPUT_PRECISION, FW_BOLD,
        FW_NORMAL, HBRUSH, HDC, HFONT, TRANSPARENT,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::SystemServices::{SS_CENTERIMAGE, SS_ETCHEDHORZ};
    use windows::Win32::UI::Controls::{BST_CHECKED, BST_UNCHECKED};
    use windows::Win32::UI::HiDpi::{GetDpiForSystem, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_SYSTEM_AWARE};
    use windows::Win32::UI::WindowsAndMessaging::*;

    const CLASS_NAME: PCWSTR = w!("NuktaBanglaSettings");
    const TITLE: &str = "নুকতা বাংলা — সেটিংস";
    /// Client width, in 96-dpi pixels.
    const WIDTH: i32 = 480;
    const MARGIN: i32 = 24;

    const ID_LAYOUT: i32 = 100;
    const ID_MODE: i32 = 110; // + the mode's index
    const ID_COLON: i32 = 120;
    const ID_AUTOCORRECT: i32 = 130;
    const ID_EMOJI: i32 = 131;
    const ID_ENGLISH_WORD: i32 = 132;
    const ID_POSITION: i32 = 140;
    const ID_DIRECTION: i32 = 141;
    /// Re-reads the settings file now and then: the layout can change from the taskbar meanwhile.
    const WATCH_TIMER: usize = 1;

    const LAYOUTS: [(Layout, &str); 2] = [(Layout::Bijoy, "বিজয় লেআউট"), (Layout::Phonetic, "ফোনেটিক")];
    const MODES: [(TypingMode, &str, &str); 3] = [
        (TypingMode::PhoneticFirst, "ফোনেটিক আগে (প্রস্তাবিত)",
         "যেমন বানান লিখবেন ঠিক তেমন বাংলা বসবে; তালিকা থেকে অন্য শব্দ বেছে নিলে পরের বার সেটাই আগে আসবে।"),
        (TypingMode::Smart, "স্মার্ট",
         "অভিধান আর অটোকারেক্ট সবচেয়ে মানানসই শব্দ বেছে দেবে (sonar → সোনার)।"),
        (TypingMode::PhoneticOnly, "শুধু ফোনেটিক",
         "সাজেশন তালিকা ছাড়া, অক্ষর ধরে ধরে লেখা।"),
    ];
    const COLON_CHOICES: [(bool, &str); 2] = [(false, "কোলন  :"), (true, "বিসর্গ  ঃ")];
    const POSITIONS: [(PopupPosition, &str); 2] = [(PopupPosition::Below, "লেখার নিচে"), (PopupPosition::Above, "লেখার উপরে")];
    const DIRECTIONS: [(PopupDirection, &str); 2] =
        [(PopupDirection::Vertical, "উপর-নিচে তালিকা"), (PopupDirection::Horizontal, "পাশাপাশি এক সারি")];

    pub fn main() -> i32 {
        let argument = std::env::args().nth(1).unwrap_or_default();
        match argument.as_str() {
            "--enable" => {
                win::prepare_data_directory();
                if win::enable_for_user(true).is_ok() { 0 } else { 1 }
            }
            "--disable" => {
                if win::enable_for_user(false).is_ok() { 0 } else { 1 }
            }
            "--about" => {
                about();
                0
            }
            _ => run_window(),
        }
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }

    fn about() {
        let text = wide(&format!(
            "নুকতা বাংলা\nসংস্করণ {}\n\nবিজয় লেআউট আর ফোনেটিক টাইপিং, ইউনিকোডে।\n\nকৃতজ্ঞতা: riti\n© 2026 Nukta Solutions",
            env!("CARGO_PKG_VERSION")
        ));
        unsafe {
            MessageBoxW(None, PCWSTR(text.as_ptr()), w!("নুকতা বাংলা সম্পর্কে"), MB_OK | MB_ICONINFORMATION);
        }
    }

    // MARK: The window

    struct Controls {
        layout: HWND,
        modes: Vec<HWND>,
        colon: HWND,
        autocorrect: HWND,
        emoji: HWND,
        english_word: HWND,
        position: HWND,
        direction: HWND,
        /// Small grey text.
        notes: Vec<HWND>,
    }

    struct App {
        settings: Settings,
        controls: Controls,
        fonts: Vec<HFONT>,
    }

    thread_local! {
        static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    }

    fn run_window() -> i32 {
        unsafe {
            // One window: a second start brings the first forward.
            if let Ok(existing) = FindWindowW(CLASS_NAME, None) {
                let _ = ShowWindow(existing, SW_RESTORE);
                let _ = SetForegroundWindow(existing);
                return 0;
            }
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE);
            let instance = GetModuleHandleW(None).unwrap_or_default();
            let class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(window_procedure),
                hInstance: instance.into(),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                hbrBackground: GetSysColorBrush(COLOR_WINDOW),
                lpszClassName: CLASS_NAME,
                ..Default::default()
            };
            RegisterClassExW(&class);

            let title = wide(TITLE);
            let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
            let Ok(window) = CreateWindowExW(
                WINDOW_EX_STYLE(0), CLASS_NAME, PCWSTR(title.as_ptr()), style,
                CW_USEDEFAULT, CW_USEDEFAULT, 100, 100, None, None, Some(instance.into()), None,
            ) else {
                return 1;
            };

            let directory = win::prepare_data_directory();
            let scale = GetDpiForSystem().max(96) as i32;
            let (controls, fonts, height) = build(window, scale);
            APP.with(|app| *app.borrow_mut() = Some(App { settings: Settings::new(&directory), controls, fonts }));
            refresh();

            // Size the window around its content, centred on the screen.
            let px = |v: i32| v * scale / 96;
            let mut frame = RECT { left: 0, top: 0, right: px(WIDTH), bottom: height };
            let _ = AdjustWindowRectEx(&mut frame, style, false, WINDOW_EX_STYLE(0));
            let (width, height) = (frame.right - frame.left, frame.bottom - frame.top);
            let x = (GetSystemMetrics(SM_CXSCREEN) - width) / 2;
            let y = (GetSystemMetrics(SM_CYSCREEN) - height) / 3;
            let _ = SetWindowPos(window, None, x, y, width, height, SWP_NOZORDER);
            let _ = ShowWindow(window, SW_SHOW);
            SetTimer(Some(window), WATCH_TIMER, 1000, None);

            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).as_bool() {
                // Tab and the arrow keys move between controls, as in a dialog.
                if !IsDialogMessageW(window, &message).as_bool() {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            APP.with(|app| {
                if let Some(app) = app.borrow_mut().take() {
                    for font in app.fonts {
                        let _ = DeleteObject(font.into());
                    }
                }
            });
            0
        }
    }

    fn font(height: i32, weight: i32) -> HFONT {
        unsafe {
            CreateFontW(
                -height, 0, 0, 0, weight, 0, 0, 0, DEFAULT_CHARSET, FONT_OUTPUT_PRECISION(0),
                FONT_CLIP_PRECISION(0), CLEARTYPE_QUALITY, FF_DONTCARE.0 as u32, w!("Nirmala UI"),
            )
        }
    }

    /// Lays out the controls top to bottom. Returns them, the fonts to free, and the content height.
    fn build(window: HWND, scale: i32) -> (Controls, Vec<HFONT>, i32) {
        let px = |v: i32| v * scale / 96;
        let normal = font(px(14), FW_NORMAL.0 as i32);
        let bold = font(px(14), FW_BOLD.0 as i32);
        let small = font(px(12), FW_NORMAL.0 as i32);
        let content = px(WIDTH - 2 * MARGIN);
        let left = px(MARGIN);
        let mut y = px(20);
        let mut notes = Vec::new();

        let make = |class: PCWSTR, text: &str, style: WINDOW_STYLE, x: i32, y: i32, width: i32, height: i32, id: i32, font: HFONT| unsafe {
            let text = wide(text);
            let control = CreateWindowExW(
                WINDOW_EX_STYLE(0), class, PCWSTR(text.as_ptr()), WS_CHILD | WS_VISIBLE | style,
                x, y, width, height, Some(window), Some(HMENU(id as isize as *mut _)), None, None,
            )
            .unwrap_or_default();
            SendMessageW(control, WM_SETFONT, Some(WPARAM(font.0 as usize)), Some(LPARAM(1)));
            control
        };
        let text_height = |text: &str, font: HFONT, width: i32| unsafe {
            let dc: HDC = GetDC(Some(window));
            let old = SelectObject(dc, font.into());
            let mut text: Vec<u16> = text.encode_utf16().collect();
            let mut rect = RECT { left: 0, top: 0, right: width, bottom: 0 };
            DrawTextW(dc, &mut text, &mut rect, DT_CALCRECT | DT_WORDBREAK | DT_NOPREFIX);
            SelectObject(dc, old);
            ReleaseDC(Some(window), dc);
            rect.bottom - rect.top
        };
        let row_height = px(28);

        let header = |text: &str, y: &mut i32| {
            let height = text_height(text, bold, content);
            make(w!("STATIC"), text, WINDOW_STYLE(0), left, *y, content, height, -1, bold);
            *y += height + px(8);
        };
        let note = |text: &str, indent: i32, y: &mut i32, notes: &mut Vec<HWND>| {
            let height = text_height(text, small, content - indent);
            notes.push(make(w!("STATIC"), text, WINDOW_STYLE(0), left + indent, *y, content - indent, height, -1, small));
            *y += height + px(10);
        };
        let separator = |y: &mut i32| {
            *y += px(4);
            make(w!("STATIC"), "", WINDOW_STYLE(SS_ETCHEDHORZ.0), left, *y, content, px(2), -1, normal);
            *y += px(14);
        };
        // A label and a drop-down list on one row.
        let row = |label: &str, choices: &[&str], id: i32, y: &mut i32| {
            let label_width = px(150);
            make(w!("STATIC"), label, WINDOW_STYLE(SS_CENTERIMAGE.0), left, *y, label_width, row_height, -1, normal);
            let combo = make(
                w!("COMBOBOX"), "", WINDOW_STYLE(CBS_DROPDOWNLIST as u32) | WS_TABSTOP | WS_VSCROLL,
                left + label_width, *y, px(200), px(200), id, normal,
            );
            for choice in choices {
                let choice = wide(choice);
                unsafe { SendMessageW(combo, CB_ADDSTRING, None, Some(LPARAM(choice.as_ptr() as isize))) };
            }
            *y += row_height + px(8);
            combo
        };
        let check = |text: &str, id: i32, y: &mut i32| {
            let control = make(w!("BUTTON"), text, WINDOW_STYLE(BS_AUTOCHECKBOX as u32) | WS_TABSTOP, left, *y, content, px(24), id, normal);
            *y += px(28);
            control
        };

        header("কিবোর্ড", &mut y);
        let layout = row("লেআউট:", &LAYOUTS.map(|l| l.1), ID_LAYOUT, &mut y);
        note("টাস্কবারের বি / ফ আইকনে ক্লিক করেও বদলানো যায়।", 0, &mut y, &mut notes);
        separator(&mut y);

        header("ফোনেটিক টাইপিং", &mut y);
        let mut modes = Vec::new();
        for (index, (_, title, detail)) in MODES.iter().enumerate() {
            let group = if index == 0 { WS_GROUP | WS_TABSTOP } else { WINDOW_STYLE(0) };
            modes.push(make(
                w!("BUTTON"), title, WINDOW_STYLE(BS_AUTORADIOBUTTON as u32) | group,
                left, y, content, px(24), ID_MODE + index as i32, normal,
            ));
            y += px(24);
            note(detail, px(20), &mut y, &mut notes);
        }
        let colon = row("কোলন (:) চাপলে:", &COLON_CHOICES.map(|c| c.1), ID_COLON, &mut y);
        note("কোলন বেছে নিলেও ঃ-ওয়ালা শব্দ তালিকায় পাবেন (dukho → দুঃখ)।", 0, &mut y, &mut notes);
        separator(&mut y);

        header("সাজেশন", &mut y);
        let autocorrect = check("অটোকারেক্ট: চেনা ভুল বানান ঠিক করে দেখাও", ID_AUTOCORRECT, &mut y);
        let emoji = check("ইমোজি সাজেশন দেখাও (হাসি → 😄)", ID_EMOJI, &mut y);
        let english_word = check("টাইপ করা ইংরেজি শব্দটিও তালিকায় রাখো", ID_ENGLISH_WORD, &mut y);
        y += px(4);
        let position = row("তালিকা দেখাও:", &POSITIONS.map(|p| p.1), ID_POSITION, &mut y);
        let direction = row("সাজানো:", &DIRECTIONS.map(|d| d.1), ID_DIRECTION, &mut y);
        note(
            "Space বা Enter: বাছাই করা শব্দ বসবে · ↑↓ বা Tab: বাছাই বদলাও · ১–৯: সরাসরি বাছাই · Esc: বাদ দাও · Ctrl+Backspace: পুরো শব্দ মোছো",
            0, &mut y, &mut notes,
        );
        y += px(14);

        let controls = Controls { layout, modes, colon, autocorrect, emoji, english_word, position, direction, notes };
        (controls, vec![normal, bold, small], y)
    }

    // MARK: State

    fn select(combo: HWND, index: usize) {
        unsafe { SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(index)), None) };
    }

    fn selection(combo: HWND) -> usize {
        unsafe { SendMessageW(combo, CB_GETCURSEL, None, None) }.0.max(0) as usize
    }

    fn set_checked(button: HWND, on: bool) {
        let state = if on { BST_CHECKED } else { BST_UNCHECKED };
        unsafe { SendMessageW(button, BM_SETCHECK, Some(WPARAM(state.0 as usize)), None) };
    }

    fn is_checked(button: HWND) -> bool {
        unsafe { SendMessageW(button, BM_GETCHECK, None, None) }.0 as u32 == BST_CHECKED.0
    }

    fn enable(control: HWND, on: bool) {
        unsafe {
            let _ = windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow(control, on);
        }
    }

    /// Shows the saved settings.
    fn refresh() {
        APP.with(|app| {
            let app = app.borrow();
            let Some(app) = app.as_ref() else { return };
            let (s, c) = (&app.settings, &app.controls);
            select(c.layout, LAYOUTS.iter().position(|l| l.0 == s.layout()).unwrap_or(0));
            let mode = s.typing_mode();
            for (index, button) in c.modes.iter().enumerate() {
                set_checked(*button, MODES[index].0 == mode);
            }
            set_checked(c.autocorrect, s.autocorrect());
            set_checked(c.emoji, s.emoji());
            set_checked(c.english_word, s.english_word());
            select(c.colon, COLON_CHOICES.iter().position(|x| x.0 == s.colon_is_bisarga()).unwrap_or(0));
            select(c.position, POSITIONS.iter().position(|p| p.0 == s.popup_position()).unwrap_or(0));
            select(c.direction, DIRECTIONS.iter().position(|d| d.0 == s.popup_direction()).unwrap_or(0));

            // Bijoy uses none of these, and phonetic-only has no list.
            let phonetic = s.layout() == Layout::Phonetic;
            for control in c.modes.iter().chain([&c.colon]) {
                enable(*control, phonetic);
            }
            let suggestions = phonetic && mode != TypingMode::PhoneticOnly;
            for control in [c.autocorrect, c.emoji, c.english_word, c.position, c.direction] {
                enable(control, suggestions);
            }
        });
    }

    fn changed(id: i32) {
        APP.with(|app| {
            let mut app = app.borrow_mut();
            let Some(app) = app.as_mut() else { return };
            let (s, c) = (&mut app.settings, &app.controls);
            match id {
                ID_LAYOUT => s.set_layout(LAYOUTS[selection(c.layout).min(1)].0),
                ID_COLON => s.set_colon_is_bisarga(COLON_CHOICES[selection(c.colon).min(1)].0),
                ID_AUTOCORRECT => s.set_autocorrect(is_checked(c.autocorrect)),
                ID_EMOJI => s.set_emoji(is_checked(c.emoji)),
                ID_ENGLISH_WORD => s.set_english_word(is_checked(c.english_word)),
                ID_POSITION => s.set_popup_position(POSITIONS[selection(c.position).min(1)].0),
                ID_DIRECTION => s.set_popup_direction(DIRECTIONS[selection(c.direction).min(1)].0),
                id if (ID_MODE..ID_MODE + MODES.len() as i32).contains(&id) => {
                    s.set_typing_mode(MODES[(id - ID_MODE) as usize].0)
                }
                _ => {}
            }
        });
        refresh();
    }

    fn is_note(control: HWND) -> bool {
        APP.with(|app| app.borrow().as_ref().is_some_and(|app| app.controls.notes.contains(&control)))
    }

    unsafe extern "system" fn window_procedure(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match message {
            WM_COMMAND => {
                let id = (wparam.0 & 0xFFFF) as i32;
                let notification = ((wparam.0 >> 16) & 0xFFFF) as u32;
                if notification == CBN_SELCHANGE || notification == BN_CLICKED {
                    changed(id);
                }
                LRESULT(0)
            }
            WM_CTLCOLORSTATIC => {
                let dc = HDC(wparam.0 as *mut _);
                let control = HWND(lparam.0 as *mut _);
                unsafe {
                    SetBkMode(dc, TRANSPARENT);
                    let color = if is_note(control) { COLOR_GRAYTEXT } else { COLOR_WINDOWTEXT };
                    SetTextColor(dc, COLORREF(GetSysColor(color)));
                    let brush: HBRUSH = GetSysColorBrush(COLOR_WINDOW);
                    LRESULT(brush.0 as isize)
                }
            }
            WM_TIMER => {
                let changed = APP.with(|app| app.borrow_mut().as_mut().is_some_and(|app| app.settings.reload_if_changed()));
                if changed {
                    refresh();
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                unsafe { PostQuitMessage(0) };
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
        }
    }
}
