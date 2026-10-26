//! NuktaBanglaSettings.exe: সেটিংস and the About box, opened from the top bar and the tray menu. The
//! counterpart of `Sources/NuktaInputMethod/SettingsWindow.swift`: the same settings, and every change
//! is saved (to settings.json) and taken up by every app straight away.
//!
//! Both windows follow the approved designs (`windows/design/settings-preview.html`,
//! `about-preview.html`): dark, drawn with Direct2D (`nukta_bangla::gfx`), Bangla in Noto Serif
//! Bengali. Settings has two columns: the layout and typing mode on the left, suggestions and
//! shortcuts on the right. With বিজয় only the layout shows, across the whole width: Bijoy uses
//! none of the rest.
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

    use nukta_bangla::gfx::{wide, Align, Canvas, Color, Gfx, Icon, Rect, Style};
    use nukta_bangla::phonetic::TypingMode;
    use nukta_bangla::settings::{Layout, PopupDirection, PopupPosition, Settings};
    use nukta_bangla::win;
    use windows::core::{w, BOOL, PCWSTR};
    use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};
    use windows::Win32::Graphics::Gdi::{BeginPaint, EndPaint, InvalidateRect, PAINTSTRUCT};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::HiDpi::{GetDpiForSystem, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_SYSTEM_AWARE};
    use windows::Win32::UI::Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT, VK_ESCAPE};
    use windows::Win32::UI::WindowsAndMessaging::*;

    const SETTINGS_CLASS: PCWSTR = w!("NuktaBanglaSettings");
    const ABOUT_CLASS: PCWSTR = w!("NuktaBanglaAbout");
    /// Re-reads the settings file now and then: the layout can change from the top bar meanwhile.
    const WATCH_TIMER: usize = 1;
    const WM_MOUSELEAVE: u32 = 0x02A3;

    // MARK: The design (settings-preview.html), in DIPs.

    const SETTINGS_WIDTH: f32 = 860.0;
    const ABOUT_WIDTH: f32 = 400.0;
    const PAD: f32 = 24.0;
    const GAP: f32 = 16.0;
    const CARD_PAD: f32 = 16.0;

    const BG: Color = Color(0x0D0807, 1.0);
    const CARD: Color = Color(0x161110, 1.0);
    const BOX: Color = Color(0x241C1A, 1.0);
    const BOX_HOVER: Color = Color(0x2B2220, 1.0);
    const BORDER: Color = Color(0x2D2827, 1.0);
    const INK: Color = Color(0xFFFFFF, 0.92);
    const DIM: Color = Color(0xFFFFFF, 0.56);
    const FAINT: Color = Color(0xFFFFFF, 0.32);
    const ORANGE: Color = Color(0xE87A1E, 1.0);
    const MODE_ON: Color = Color(0x2A1A13, 1.0);
    const TOGGLE_OFF: Color = Color(0x3A3230, 1.0);
    const WHITE: Color = Color(0xFFFFFF, 1.0);

    const KEYBOARD: Icon = Icon {
        view: 24.0,
        paths: &[
            "M10 8h.01", "M12 12h.01", "M14 8h.01", "M16 12h.01", "M18 8h.01", "M6 8h.01", "M7 16h10", "M8 12h.01",
            "M4 4h16a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2z",
        ],
    };
    const TYPE: Icon = Icon { view: 24.0, paths: &["M12 4v16", "M4 7V5a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v2", "M9 20h6"] };
    const LIST: Icon = Icon {
        view: 24.0,
        paths: &["M3 12h.01", "M3 18h.01", "M3 6h.01", "M8 12h13", "M8 18h13", "M8 6h13"],
    };
    const COMMAND: Icon = Icon {
        view: 24.0,
        paths: &["M15 6v12a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3V6a3 3 0 1 0-3 3h12a3 3 0 1 0-3-3"],
    };

    const MODES: [(TypingMode, &str, &str); 3] = [
        (TypingMode::PhoneticFirst, "ফোনেটিক আগে",
         "যেমন বানান লিখবেন ঠিক তেমন বাংলা বসবে। তালিকা থেকে অন্য শব্দ বেছে নিলে পরের বার সেটাই আগে আসবে।"),
        (TypingMode::Smart, "স্মার্ট", "অভিধান আর অটোকারেক্ট সবচেয়ে মানানসই শব্দ বেছে দেবে (sonar → সোনার)।"),
        (TypingMode::PhoneticOnly, "শুধু ফোনেটিক", "সাজেশন তালিকা ছাড়া, অক্ষর ধরে ধরে লেখা।"),
    ];

    /// What a click on a part of the settings window does.
    #[derive(Clone, Copy, PartialEq, Debug)]
    enum Action {
        Layout(Layout),
        Mode(TypingMode),
        Colon(bool),
        Autocorrect,
        Emoji,
        EnglishWord,
        Position(PopupPosition),
        Direction(PopupDirection),
    }

    #[derive(Clone, Copy, PartialEq)]
    enum Screen {
        Settings,
        About,
    }

    struct App {
        window: HWND,
        screen: Screen,
        gfx: Gfx,
        settings: Settings,
        dpi: f32,
        hover: Option<Action>,
        /// Clickable areas from the last drawing, in DIPs.
        hits: Vec<(Rect, Action)>,
        /// The content height the window was last sized to, in DIPs.
        height: f32,
    }

    thread_local! {
        static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    }

    fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
        APP.with(|app| app.try_borrow_mut().ok().and_then(|mut app| app.as_mut().map(f)))
    }

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
            "--about" => run_window(Screen::About),
            _ => run_window(Screen::Settings),
        }
    }

    /// The version in Bangla digits (১.০.১).
    fn bangla_version() -> String {
        env!("CARGO_PKG_VERSION")
            .chars()
            .map(|c| c.to_digit(10).map_or(c, |d| char::from_u32(0x09E6 + d).unwrap_or(c)))
            .collect()
    }

    // MARK: The window

    fn run_window(screen: Screen) -> i32 {
        let (class_name, title, width) = match screen {
            Screen::Settings => (SETTINGS_CLASS, "নুকতা বাংলা — সেটিংস", SETTINGS_WIDTH),
            Screen::About => (ABOUT_CLASS, "নুকতা বাংলা সম্পর্কে", ABOUT_WIDTH),
        };
        unsafe {
            // One window of each: a second start brings the first forward.
            if let Ok(existing) = FindWindowW(class_name, None) {
                let _ = ShowWindow(existing, SW_RESTORE);
                let _ = SetForegroundWindow(existing);
                return 0;
            }
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE);
            let Ok(gfx) = Gfx::new(win::bangla_font_for_program().as_deref()) else { return 1 };
            let instance = GetModuleHandleW(None).unwrap_or_default();
            let class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(window_procedure),
                hInstance: instance.into(),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                // The logo, from the program's icon resource 1 (installer/nukta-bangla.rc).
                hIcon: LoadIconW(Some(instance.into()), PCWSTR(1 as *const u16)).unwrap_or_default(),
                lpszClassName: class_name,
                ..Default::default()
            };
            RegisterClassExW(&class);

            let title = wide(title);
            let style = match screen {
                Screen::Settings => WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
                Screen::About => WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
            };
            let Ok(window) = CreateWindowExW(
                WINDOW_EX_STYLE(0), class_name, PCWSTR(title.as_ptr()), style,
                CW_USEDEFAULT, CW_USEDEFAULT, 100, 100, None, None, Some(instance.into()), None,
            ) else {
                return 1;
            };
            dark_title_bar(window);

            let dpi = GetDpiForSystem().max(96) as f32;
            let settings = Settings::new(&win::prepare_data_directory());
            APP.with(|app| {
                *app.borrow_mut() = Some(App { window, screen, gfx, settings, dpi, hover: None, hits: Vec::new(), height: 0.0 })
            });

            // Sized around the content, centred on the screen.
            let height = content_height();
            resize(window, width, height, dpi, true);
            let _ = ShowWindow(window, SW_SHOW);
            if screen == Screen::Settings {
                SetTimer(Some(window), WATCH_TIMER, 1000, None);
            }

            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            0
        }
    }

    /// Windows draws the title bar dark, in the window's colours (Windows 11; Windows 10 shows its
    /// own dark title bar).
    fn dark_title_bar(window: HWND) {
        unsafe {
            let on = BOOL(1);
            let _ = DwmSetWindowAttribute(window, DWMWINDOWATTRIBUTE(20), &on as *const BOOL as *const _, 4);
            let caption = COLORREF(0x0012_131A); // #1A1312, as BGR
            let _ = DwmSetWindowAttribute(window, DWMWINDOWATTRIBUTE(35), &caption as *const COLORREF as *const _, 4);
            let text = COLORREF(0x00C8_C8C8);
            let _ = DwmSetWindowAttribute(window, DWMWINDOWATTRIBUTE(36), &text as *const COLORREF as *const _, 4);
        }
    }

    /// Sizes the window's client area to `width` × `height` DIPs; `center` also centres it.
    fn resize(window: HWND, width: f32, height: f32, dpi: f32, center: bool) {
        unsafe {
            let style = WINDOW_STYLE(GetWindowLongW(window, GWL_STYLE) as u32);
            let mut frame = RECT { left: 0, top: 0, right: (width * dpi / 96.0).round() as i32, bottom: (height * dpi / 96.0).round() as i32 };
            let _ = AdjustWindowRectEx(&mut frame, style, false, WINDOW_EX_STYLE(0));
            let (w, h) = (frame.right - frame.left, frame.bottom - frame.top);
            if center {
                let x = (GetSystemMetrics(SM_CXSCREEN) - w) / 2;
                let y = ((GetSystemMetrics(SM_CYSCREEN) - h) / 3).max(0);
                let _ = SetWindowPos(window, None, x, y, w, h, SWP_NOZORDER);
            } else {
                let _ = SetWindowPos(window, None, 0, 0, w, h, SWP_NOZORDER | SWP_NOMOVE | SWP_NOACTIVATE);
            }
        }
    }

    /// The content's height now, from a layout pass without drawing.
    fn content_height() -> f32 {
        with_app(|app| {
            let mut ctx = Ctx { canvas: None, gfx: &app.gfx, hover: None, hits: Vec::new() };
            let height = match app.screen {
                Screen::Settings => draw_settings(&mut ctx, &app.settings),
                Screen::About => draw_about(&mut ctx),
            };
            app.height = height;
            height
        })
        .unwrap_or(400.0)
    }

    /// After a change: the window follows the content's height (বিজয় ⇄ ফোনেটিক), and redraws.
    fn refresh() {
        let Some((window, old_height, dpi)) = with_app(|app| (app.window, app.height, app.dpi)) else { return };
        let height = content_height();
        if (height - old_height).abs() > 0.5 {
            resize(window, SETTINGS_WIDTH, height, dpi, false);
        }
        unsafe {
            let _ = InvalidateRect(Some(window), None, false);
        }
    }

    // MARK: Drawing, and where the clicks go

    /// One pass over the layout: draws when there is a canvas, and records the clickable parts.
    struct Ctx<'a, 'b> {
        canvas: Option<&'a Canvas<'b>>,
        gfx: &'a Gfx,
        hover: Option<Action>,
        hits: Vec<(Rect, Action)>,
    }

    impl Ctx<'_, '_> {
        fn fill(&self, rect: Rect, radius: f32, color: Color) {
            if let Some(c) = self.canvas {
                c.fill_round(rect, radius, color);
            }
        }

        fn brand(&self, rect: Rect, radius: f32) {
            if let Some(c) = self.canvas {
                c.fill_round_brand(rect, radius);
            }
        }

        fn outline(&self, rect: Rect, radius: f32, color: Color) {
            if let Some(c) = self.canvas {
                c.stroke_round(rect, radius, color);
            }
        }

        fn text(&self, text: &str, rect: Rect, style: Style, color: Color) {
            if let Some(c) = self.canvas {
                c.text(text, rect, style, color);
            }
        }

        fn icon(&self, icon: &Icon, x: f32, y: f32, size: f32, color: Color) {
            if let Some(c) = self.canvas {
                c.icon_stroke(icon, x, y, size, color, 1.5 * size / 24.0);
            }
        }

        /// Height of wrapped text.
        fn height(&self, text: &str, style: Style, width: f32) -> f32 {
            self.gfx.text_height(text, style, width)
        }

        /// A clickable part; true when the pointer is over it.
        fn hit(&mut self, rect: Rect, action: Action) -> bool {
            self.hits.push((rect, action));
            self.hover == Some(action)
        }

        /// A card's heading, with its icon. Returns the y below it.
        fn heading(&self, icon: &Icon, text: &str, x: f32, y: f32, width: f32) -> f32 {
            self.icon(icon, x, y + 2.0, 16.0, Color(0xFFFFFF, 0.75));
            self.text(text, Rect::new(x + 24.0, y - 2.0, width - 24.0, 24.0), Style::new(14.5).weight(600), INK);
            y + 22.0 + 12.0
        }

        /// A segmented control: one choice of several. `big` adds a second line under each.
        fn segments(&mut self, rect: Rect, choices: &[(&str, Option<&str>, Action, bool)]) {
            self.fill(rect, 8.0, BOX);
            self.outline(rect, 8.0, BORDER);
            let inner = rect.inset(3.0);
            let each = (inner.width() - 3.0 * (choices.len() as f32 - 1.0)) / choices.len() as f32;
            for (index, (label, sub, action, on)) in choices.iter().enumerate() {
                let part = Rect::new(inner.left + index as f32 * (each + 3.0), inner.top, each, inner.height());
                let hovered = self.hit(part, *action);
                if *on {
                    self.brand(part, 6.0);
                } else if hovered {
                    self.fill(part, 6.0, BOX_HOVER);
                }
                let color = if *on { WHITE } else { INK };
                match sub {
                    Some(sub) => {
                        let top = part.top + (part.height() - 42.0) / 2.0;
                        self.text(label, Rect::new(part.left, top, part.width(), 24.0), Style::new(15.0).weight(if *on { 600 } else { 400 }).align(Align::Center), color);
                        self.text(sub, Rect::new(part.left, top + 23.0, part.width(), 18.0), Style::new(11.5).align(Align::Center), Color(0xFFFFFF, 0.72));
                    }
                    None => {
                        let size = if part.height() > 32.0 { 14.0 } else { 13.0 };
                        self.text(label, part, Style::new(size).weight(if *on { 600 } else { 400 }).align(Align::Center), color);
                    }
                }
            }
        }

        /// An on/off switch, 40 × 22, at its top-left corner.
        fn toggle(&mut self, x: f32, y: f32, on: bool, action: Action) {
            let rect = Rect::new(x, y, 40.0, 22.0);
            self.hit(rect, action);
            if on {
                self.brand(rect, 11.0);
            } else {
                self.fill(rect, 11.0, TOGGLE_OFF);
            }
            let knob_x = if on { x + 29.0 } else { x + 11.0 };
            if let Some(c) = self.canvas {
                c.circle(knob_x, y + 11.0, 8.0, WHITE, 0.0);
            }
        }

        /// A key cap; returns its width.
        fn key(&self, label: &str, x: f32, y: f32) -> f32 {
            let width = self.gfx.text_width(label, Style::new(12.0)) + 14.0;
            let rect = Rect::new(x, y, width, 22.0);
            self.fill(rect, 5.0, BOX);
            self.outline(rect, 5.0, BORDER);
            // The thicker bottom edge of a key.
            self.fill(Rect::new(x + 2.0, y + 21.0, width - 4.0, 1.0), 0.0, BORDER);
            self.text(label, rect, Style::new(12.0).align(Align::Center), INK);
            width
        }
    }

    fn draw_settings(ctx: &mut Ctx, s: &Settings) -> f32 {
        let phonetic = s.layout() == Layout::Phonetic;
        let mut y = 22.0;

        // The head: logo, name, version.
        if let Some(c) = ctx.canvas {
            c.logo(PAD, y, 49.0);
        }
        ctx.text("নুকতা বাংলা", Rect::new(PAD + 63.0, y - 4.0, 400.0, 30.0), Style::new(21.0).weight(700), INK);
        ctx.text(&format!("সেটিংস · সংস্করণ {}", bangla_version()), Rect::new(PAD + 63.0, y + 24.0, 400.0, 18.0), Style::new(13.0), DIM);
        y += 37.0 + 20.0;

        let full = SETTINGS_WIDTH - 2.0 * PAD;
        let column = (full - GAP) / 2.0;
        let left_width = if phonetic { column } else { full };

        // Left: the layout.
        let mut left_y = layout_card(ctx, s, PAD, y, left_width);
        let mut right_y = y;
        if phonetic {
            left_y = typing_card(ctx, s, PAD, left_y + GAP, column);
            let right = PAD + column + GAP;
            right_y = suggestions_card(ctx, s, right, y, column);
            right_y = shortcuts_card(ctx, right, right_y + GAP, column);
        }
        let bottom = left_y.max(right_y) + 14.0;
        ctx.text("পরিবর্তন সাথে সাথে সব অ্যাপে চালু হয়", Rect::new(0.0, bottom, SETTINGS_WIDTH, 18.0), Style::new(12.0).align(Align::Center), FAINT);
        bottom + 18.0 + 18.0
    }

    /// A card's background and border, drawn once its height is known.
    fn card(ctx: &Ctx, x: f32, y: f32, width: f32, height: f32) {
        let rect = Rect::new(x, y, width, height);
        ctx.fill(rect, 10.0, CARD);
        ctx.outline(rect, 10.0, BORDER);
    }

    /// Draws a card around `content`: a measuring pass first finds its height.
    fn with_card(ctx: &mut Ctx, x: f32, y: f32, width: f32, content: impl Fn(&mut Ctx) -> f32) -> f32 {
        let mut measure = Ctx { canvas: None, gfx: ctx.gfx, hover: None, hits: Vec::new() };
        let bottom = content(&mut measure) + CARD_PAD;
        card(ctx, x, y, width, bottom - y);
        content(ctx);
        bottom
    }

    fn layout_card(ctx: &mut Ctx, s: &Settings, x: f32, y: f32, width: f32) -> f32 {
        let layout = s.layout();
        with_card(ctx, x, y, width, |ctx| {
            let inner = x + CARD_PAD;
            let inner_width = width - 2.0 * CARD_PAD;
            let mut y = ctx.heading(&KEYBOARD, "কিবোর্ড লেআউট", inner, y + CARD_PAD, inner_width);
            ctx.segments(
                Rect::new(inner, y, inner_width, 54.0),
                &[
                    ("বিজয়", Some("চেনা বিজয় লেআউট"), Action::Layout(Layout::Bijoy), layout == Layout::Bijoy),
                    ("ফোনেটিক", Some("ami → আমি"), Action::Layout(Layout::Phonetic), layout == Layout::Phonetic),
                ],
            );
            y += 54.0 + 10.0;
            let hint = "F12 চাপলে বা টপ বারের নু লোগোতে ক্লিক করলে বাংলা চালু বা বন্ধ হয়।";
            let style = Style::new(12.5).wrap();
            let height = ctx.height(hint, style, inner_width);
            ctx.text(hint, Rect::new(inner, y, inner_width, height), style, DIM);
            y + height
        })
    }

    fn typing_card(ctx: &mut Ctx, s: &Settings, x: f32, y: f32, width: f32) -> f32 {
        let mode = s.typing_mode();
        let colon = s.colon_is_bisarga();
        with_card(ctx, x, y, width, |ctx| {
            let inner = x + CARD_PAD;
            let inner_width = width - 2.0 * CARD_PAD;
            let mut y = ctx.heading(&TYPE, "টাইপিং মোড", inner, y + CARD_PAD, inner_width);
            for (index, (value, title, detail)) in MODES.iter().enumerate() {
                let text_left = inner + 14.0 + 16.0 + 12.0;
                let text_width = inner_width - (text_left - inner) - 14.0;
                let detail_style = Style::new(12.5).wrap();
                let detail_height = ctx.height(detail, detail_style, text_width);
                let rect = Rect::new(inner, y, inner_width, 12.0 + 22.0 + 3.0 + detail_height + 12.0);
                let on = mode == *value;
                let hovered = ctx.hit(rect, Action::Mode(*value));
                ctx.fill(rect, 8.0, if on { MODE_ON } else if hovered { BOX_HOVER } else { BOX });
                ctx.outline(rect, 8.0, if on { ORANGE } else { BORDER });
                // The radio button.
                if let Some(c) = ctx.canvas {
                    let (cx, cy) = (inner + 14.0 + 8.0, y + 12.0 + 11.0);
                    c.circle(cx, cy, 7.25, if on { ORANGE } else { FAINT }, 1.5);
                    if on {
                        c.circle(cx, cy, 4.0, ORANGE, 0.0);
                    }
                }
                let title_style = Style::new(14.5).weight(600);
                ctx.text(title, Rect::new(text_left, y + 10.0, text_width, 24.0), title_style, INK);
                if index == 0 {
                    let after = ctx.gfx.text_width(title, title_style) + 8.0;
                    ctx.text("প্রস্তাবিত", Rect::new(text_left + after, y + 11.0, 120.0, 24.0), Style::new(12.0), ORANGE);
                }
                ctx.text(detail, Rect::new(text_left, y + 12.0 + 22.0 + 3.0, text_width, detail_height), detail_style, DIM);
                y = rect.bottom + 8.0;
            }

            // The colon.
            y += 4.0;
            ctx.fill(Rect::new(inner, y, inner_width, 1.0), 0.0, BORDER);
            y += 13.0;
            let seg_width = 190.0;
            let label_width = inner_width - seg_width - 16.0;
            let detail = "কোলন বেছে নিলেও বিসর্গ-ওয়ালা শব্দ তালিকায় পাবেন (dukho → দুঃখ)।";
            let detail_style = Style::new(12.5).wrap();
            let detail_height = ctx.height(detail, detail_style, label_width);
            ctx.text("কোলন (:) চাপলে", Rect::new(inner, y, label_width, 22.0), Style::new(14.0), INK);
            ctx.text(detail, Rect::new(inner, y + 24.0, label_width, detail_height), detail_style, DIM);
            let block = 24.0 + detail_height;
            ctx.segments(
                Rect::new(inner + inner_width - seg_width, y + (block - 36.0) / 2.0, seg_width, 36.0),
                &[("কোলন  :", None, Action::Colon(false), !colon), ("বিসর্গ  দুঃ", None, Action::Colon(true), colon)],
            );
            y + block
        })
    }

    fn suggestions_card(ctx: &mut Ctx, s: &Settings, x: f32, y: f32, width: f32) -> f32 {
        // Phonetic-only has no list: these don't apply to it.
        let enabled = s.typing_mode() != TypingMode::PhoneticOnly;
        let toggles = [
            ("অটোকারেক্ট", "চেনা ভুল বানান ঠিক করে দেখাও", s.autocorrect(), Action::Autocorrect),
            ("ইমোজি", "হাসি → 😄", s.emoji(), Action::Emoji),
            ("ইংরেজি শব্দ", "টাইপ করা ইংরেজি শব্দটিও তালিকায় রাখো", s.english_word(), Action::EnglishWord),
        ];
        let position = s.popup_position();
        let direction = s.popup_direction();
        let bottom = with_card(ctx, x, y, width, |ctx| {
            let inner = x + CARD_PAD;
            let inner_width = width - 2.0 * CARD_PAD;
            let mut y = ctx.heading(&LIST, "সাজেশন", inner, y + CARD_PAD, inner_width);
            let hits_before = ctx.hits.len();
            for (index, (title, detail, on, action)) in toggles.iter().enumerate() {
                if index > 0 {
                    ctx.fill(Rect::new(inner, y, inner_width, 1.0), 0.0, BORDER);
                    y += 10.0;
                }
                ctx.text(title, Rect::new(inner, y, inner_width - 56.0, 22.0), Style::new(14.0), INK);
                ctx.text(detail, Rect::new(inner, y + 22.0, inner_width - 56.0, 20.0), Style::new(12.5), DIM);
                ctx.toggle(inner + inner_width - 40.0, y + 10.0, *on, *action);
                y += 42.0 + 10.0;
            }
            let seg_width = 190.0;
            let rows: [(&str, [(&str, Option<&str>, Action, bool); 2]); 2] = [
                ("তালিকার জায়গা", [
                    ("লেখার নিচে", None, Action::Position(PopupPosition::Below), position == PopupPosition::Below),
                    ("লেখার উপরে", None, Action::Position(PopupPosition::Above), position == PopupPosition::Above),
                ]),
                ("সাজানো", [
                    ("উপর-নিচে", None, Action::Direction(PopupDirection::Vertical), direction == PopupDirection::Vertical),
                    ("পাশাপাশি", None, Action::Direction(PopupDirection::Horizontal), direction == PopupDirection::Horizontal),
                ]),
            ];
            for (label, choices) in rows {
                ctx.fill(Rect::new(inner, y, inner_width, 1.0), 0.0, BORDER);
                y += 10.0;
                ctx.text(label, Rect::new(inner, y, inner_width - seg_width - 16.0, 36.0), Style::new(14.0), INK);
                ctx.segments(Rect::new(inner + inner_width - seg_width, y, seg_width, 36.0), &choices);
                y += 36.0 + 10.0;
            }
            if !enabled {
                // Shown, but not for changing.
                ctx.hits.truncate(hits_before);
            }
            y - 10.0
        });
        if !enabled {
            ctx.fill(Rect::new(x, y, width, bottom - y), 10.0, Color(0x161110, 0.62));
        }
        bottom
    }

    fn shortcuts_card(ctx: &mut Ctx, x: f32, y: f32, width: f32) -> f32 {
        // Keys, with "বা" / "থেকে" / "+" between them.
        type Row = (&'static [&'static str], &'static str);
        const ALWAYS: [Row; 1] = [(&["F12"], "বাংলা চালু বা বন্ধ")];
        const TYPING: [Row; 6] = [
            (&["Space"], "বাছাই করা শব্দটা বসবে, সাথে একটা স্পেস"),
            (&["Enter"], "বাছাই করা শব্দটা বসবে, নতুন লাইন হবে না"),
            (&["↑", "↓", "বা", "Tab"], "তালিকায় অন্য শব্দ বাছাই"),
            (&["1", "থেকে", "9"], "তালিকার ওই নম্বরের শব্দটা সরাসরি বসবে"),
            (&["Esc"], "শব্দটা বাতিল, কিছুই বসবে না"),
            (&["Ctrl", "+", "Backspace"], "লিখতে থাকা পুরো শব্দটা একবারে মুছবে"),
        ];
        with_card(ctx, x, y, width, |ctx| {
            let inner = x + CARD_PAD;
            let inner_width = width - 2.0 * CARD_PAD;
            let mut y = ctx.heading(&COMMAND, "শর্টকাট", inner, y + CARD_PAD, inner_width);
            let keys_width = 150.0;
            for (heading, rows) in [("সবসময়", &ALWAYS[..]), ("ফোনেটিকে লেখার সময়, সাজেশন তালিকা খোলা থাকলে", &TYPING[..])] {
                ctx.text(heading, Rect::new(inner, y - 2.0, inner_width, 18.0), Style::new(12.0).weight(600), ORANGE);
                y += 18.0 + 8.0;
                for (keys, meaning) in rows {
                    // Right-aligned in the keys column: measure first.
                    let joiners = ["বা", "থেকে", "+"];
                    let widths: Vec<f32> = keys
                        .iter()
                        .map(|k| {
                            if joiners.contains(k) {
                                ctx.gfx.text_width(k, Style::new(11.5)) + 6.0
                            } else {
                                ctx.gfx.text_width(k, Style::new(12.0)) + 14.0
                            }
                        })
                        .collect();
                    let total: f32 = widths.iter().sum::<f32>() + 4.0 * (keys.len() as f32 - 1.0);
                    let mut kx = inner + keys_width - total;
                    for (key, w) in keys.iter().zip(&widths) {
                        if joiners.contains(key) {
                            ctx.text(key, Rect::new(kx, y, *w, 22.0), Style::new(11.5).align(Align::Center), DIM);
                        } else {
                            ctx.key(key, kx, y);
                        }
                        kx += w + 4.0;
                    }
                    ctx.text(meaning, Rect::new(inner + keys_width + 14.0, y, inner_width - keys_width - 14.0, 22.0), Style::new(13.0), Color(0xFFFFFF, 0.78));
                    y += 22.0 + 9.0;
                }
                y += 6.0;
            }
            y - 15.0
        })
    }

    // MARK: About (about-preview.html)

    fn draw_about(ctx: &mut Ctx) -> f32 {
        let w = ABOUT_WIDTH;
        let center = w / 2.0;
        let mut y = 28.0 + 26.0;
        if let Some(c) = ctx.canvas {
            c.glow(center, y + 35.0, 95.0, Color(0xE87A1E, 0.28));
            c.logo(center - 46.0, y, 92.0);
        }
        y += 70.0 + 16.0;
        ctx.text("নুকতা বাংলা", Rect::new(0.0, y, w, 34.0), Style::new(24.0).weight(700).align(Align::Center), INK);
        y += 34.0;
        ctx.text(&format!("সংস্করণ {}", bangla_version()), Rect::new(0.0, y, w, 20.0), Style::new(13.0).align(Align::Center), DIM);
        y += 20.0 + 14.0;
        let tagline = "বিজয় লেআউট আর ফোনেটিক টাইপিং, ইউনিকোডে। সব অ্যাপে, একই নিয়মে।";
        let style = Style::new(14.0).align(Align::Center).wrap();
        let height = ctx.height(tagline, style, 300.0);
        ctx.text(tagline, Rect::new(center - 150.0, y, 300.0, height), style, INK);
        y += height + 12.0;

        // Windows · macOS · Linux.
        let chips = ["Windows", "macOS", "Linux"];
        let widths: Vec<f32> = chips.iter().map(|c| ctx.gfx.text_width(c, Style::new(12.0)) + 20.0).collect();
        let total: f32 = widths.iter().sum::<f32>() + 6.0 * (chips.len() as f32 - 1.0);
        let mut x = center - total / 2.0;
        for (chip, width) in chips.iter().zip(&widths) {
            let rect = Rect::new(x, y, *width, 24.0);
            ctx.fill(rect, 12.0, BOX);
            ctx.outline(rect, 12.0, BORDER);
            ctx.text(chip, rect, Style::new(12.0).align(Align::Center), INK);
            x += width + 6.0;
        }
        y += 24.0 + 22.0;

        // Credits.
        let left = 28.0;
        let card_width = w - 2.0 * left;
        let credits = [
            ("ফোনেটিক ইঞ্জিন", "riti · OpenBangla", "Mozilla Public License 2.0"),
            ("বাংলা ফন্ট", "Noto Serif Bengali · Google", "SIL Open Font License 1.1"),
        ];
        let card_height = 12.0 + 20.0 + credits.len() as f32 * 40.0 + 6.0;
        card(ctx, left, y, card_width, card_height);
        let inner = left + 14.0;
        let inner_width = card_width - 28.0;
        ctx.text("কৃতজ্ঞতা", Rect::new(inner, y + 10.0, inner_width, 20.0), Style::new(12.0).weight(600), ORANGE);
        let mut row = y + 12.0 + 20.0;
        for (index, (what, who, licence)) in credits.iter().enumerate() {
            if index > 0 {
                ctx.fill(Rect::new(inner, row, inner_width, 1.0), 0.0, BORDER);
            }
            ctx.text(what, Rect::new(inner, row + 4.0, inner_width, 20.0), Style::new(13.0), DIM);
            ctx.text(who, Rect::new(inner, row + 4.0, inner_width, 20.0), Style::new(13.0).align(Align::Right), INK);
            ctx.text(licence, Rect::new(inner, row + 22.0, inner_width, 16.0), Style::new(11.5).align(Align::Right), FAINT);
            row += 40.0;
        }
        y += card_height + 18.0;
        ctx.text("© ২০২৬ Nukta Solutions", Rect::new(0.0, y, w, 18.0), Style::new(12.0).align(Align::Center), FAINT);
        y + 18.0 + 24.0
    }

    // MARK: Changes

    fn act(action: Action) {
        with_app(|app| {
            let s = &mut app.settings;
            match action {
                Action::Layout(layout) => s.set_layout(layout),
                Action::Mode(mode) => s.set_typing_mode(mode),
                Action::Colon(bisarga) => s.set_colon_is_bisarga(bisarga),
                Action::Autocorrect => {
                    let on = s.autocorrect();
                    s.set_autocorrect(!on)
                }
                Action::Emoji => {
                    let on = s.emoji();
                    s.set_emoji(!on)
                }
                Action::EnglishWord => {
                    let on = s.english_word();
                    s.set_english_word(!on)
                }
                Action::Position(position) => s.set_popup_position(position),
                Action::Direction(direction) => s.set_popup_direction(direction),
            }
        });
        refresh();
    }

    fn action_at(lparam: LPARAM) -> Option<Action> {
        let x = (lparam.0 & 0xFFFF) as i16 as f32;
        let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;
        with_app(|app| {
            let (x, y) = (x * 96.0 / app.dpi, y * 96.0 / app.dpi);
            app.hits.iter().rev().find(|(rect, _)| rect.contains(x, y)).map(|(_, action)| *action)
        })
        .flatten()
    }

    fn set_hover(action: Option<Action>) {
        let changed = with_app(|app| std::mem::replace(&mut app.hover, action) != action).unwrap_or(false);
        if changed {
            if let Some(window) = with_app(|app| app.window) {
                unsafe {
                    let _ = InvalidateRect(Some(window), None, false);
                }
            }
        }
    }

    unsafe extern "system" fn window_procedure(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match message {
            WM_PAINT => {
                let mut paint = PAINTSTRUCT::default();
                let dc = unsafe { BeginPaint(window, &mut paint) };
                let mut client = RECT::default();
                unsafe {
                    let _ = GetClientRect(window, &mut client);
                }
                with_app(|app| {
                    let mut hits = Vec::new();
                    app.gfx.paint(dc, client.right, client.bottom, app.dpi, |canvas| {
                        let (w, h) = (client.right as f32 * 96.0 / app.dpi, client.bottom as f32 * 96.0 / app.dpi);
                        canvas.fill_rect(Rect::new(0.0, 0.0, w, h), BG);
                        let mut ctx = Ctx { canvas: Some(canvas), gfx: &app.gfx, hover: app.hover, hits: Vec::new() };
                        match app.screen {
                            Screen::Settings => {
                                draw_settings(&mut ctx, &app.settings);
                            }
                            Screen::About => {
                                draw_about(&mut ctx);
                            }
                        }
                        hits = ctx.hits;
                    });
                    app.hits = hits;
                });
                unsafe {
                    let _ = EndPaint(window, &paint);
                }
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            WM_MOUSEMOVE => {
                set_hover(action_at(lparam));
                let mut track = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: window,
                    dwHoverTime: 0,
                };
                unsafe {
                    let _ = TrackMouseEvent(&mut track);
                }
                LRESULT(0)
            }
            WM_MOUSELEAVE => {
                set_hover(None);
                LRESULT(0)
            }
            WM_SETCURSOR => {
                // A hand over what can be clicked.
                let over = with_app(|app| app.hover.is_some()).unwrap_or(false);
                if over && (lparam.0 & 0xFFFF) as u32 == HTCLIENT {
                    unsafe {
                        SetCursor(LoadCursorW(None, IDC_HAND).ok());
                    }
                    return LRESULT(1);
                }
                unsafe { DefWindowProcW(window, message, wparam, lparam) }
            }
            WM_LBUTTONUP => {
                if let Some(action) = action_at(lparam) {
                    act(action);
                }
                LRESULT(0)
            }
            WM_KEYDOWN if wparam.0 as u16 == VK_ESCAPE.0 => {
                // Esc closes the About box.
                if with_app(|app| app.screen == Screen::About).unwrap_or(false) {
                    unsafe {
                        let _ = DestroyWindow(window);
                    }
                }
                LRESULT(0)
            }
            WM_TIMER => {
                // The layout may have changed from the top bar meanwhile.
                if with_app(|app| app.settings.reload_if_changed()).unwrap_or(false) {
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
