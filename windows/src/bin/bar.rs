//! NuktaBangla.exe: the top bar and the tray icon. Started with Windows (the installer adds it to the
//! Run key), by the installer, and by F12 after it was exited.
//!
//! The bar follows the locked design in `windows/design/top-bar.md` (reference:
//! `windows/design/top-bar-preview.html`):
//!
//! ```text
//!  ╭──────────────────────────────────────────────────────────╮
//!  │ [নু.]  [ English            ⌄ ]       ⚙   ⓘ   ⏏   ✕     │   378 × 48, padding 10, radius 10
//!  ╰──────────────────────────────────────────────────────────╯
//! ```
//!
//! - The logo: drag to move the bar; click to switch English ⇄ বাংলা.
//! - The switch opens the layout menu. ⚙ settings; ⓘ help; ⏏ to the tray; ✕ asks: tray, or exit.
//! - The tray icon (`nukta_bangla::logo`): EN on black when off, নু on the brand colour when on.
//!   Click toggles; double-click brings the bar back to its default spot; right-click opens the menu.
//! - F12 is caught by the keyboard itself (`nukta_bangla.dll`), so it works with this program closed,
//!   and starts it again. Exit closes the bar and the tray icon and turns Bangla off.
//! - Nothing here types. The keyboard reads the mode from the shared settings file; this program
//!   writes `english` and `layout` there and follows changes made elsewhere by re-reading it.
//! - The bar and its menus are drawn with Direct2D (smooth 1px Lucide icons, the logo from its SVG,
//!   DirectWrite text with Bangla shaping) into layered windows, so their corners are truly round.
//!   The bar never takes the keyboard focus, so typing goes on in the app underneath.
//!
//! ```text
//! NuktaBangla.exe            start (or, if running, show the bar)
//! NuktaBangla.exe --startup  start with the splash screen (Windows starting, from the Run key)
//! NuktaBangla.exe --quit     close the running copy (the uninstaller does this)
//! ```

#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(not(windows))]
fn main() {
    eprintln!("NuktaBangla is the Windows top bar and tray icon; it does nothing elsewhere.");
}

#[cfg(windows)]
fn main() {
    std::process::exit(app::main());
}

#[cfg(windows)]
mod app {
    use std::cell::RefCell;

    use nukta_bangla::gfx::{wide, Align, Color, Canvas, Gfx, Icon, Rect, Style};
    use nukta_bangla::settings::{Layout, Settings};
    use nukta_bangla::{sound, win};
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        ClientToScreen, GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_READ,
        KEY_WRITE, REG_DWORD, RRF_RT_REG_SZ,
    };
    use windows::Win32::UI::HiDpi::{GetDpiForSystem, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_SYSTEM_AWARE};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, GetCapture, GetDoubleClickTime, ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT, VK_DOWN, VK_LBUTTON,
        VK_ESCAPE, VK_LEFT, VK_RETURN, VK_RIGHT, VK_UP,
    };
    use windows::Win32::UI::Shell::{
        ShellExecuteW, Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
        NOTIFYICONDATAW, NOTIFY_ICON_MESSAGE,
    };
    use windows::Win32::UI::WindowsAndMessaging::*;

    const BAR_CLASS: PCWSTR = w!("NuktaBanglaBar");
    const SPLASH_CLASS: PCWSTR = w!("NuktaBanglaSplash");
    /// The splash's frames (about 60 a second while it fades and its line fills).
    const SPLASH_TIMER: usize = 4;
    const MENU_CLASS: PCWSTR = w!("NuktaBanglaMenu");
    const GUIDE_URL: PCWSTR = w!("https://www.nuktasolutions.com/nukta-bangla-keyboard/guide");
    const WEBSITE_URL: PCWSTR = w!("https://www.nuktasolutions.com/nukta-bangla-keyboard");
    /// The tray icon's messages to the bar window.
    const WM_TRAY: u32 = WM_APP + 1;
    /// A second start asks the running copy to show its bar.
    const WM_SHOW_BAR: u32 = WM_APP + 2;
    const TRAY_ID: u32 = 1;
    /// The pointer left the bar (asked for with `TrackMouseEvent`).
    const WM_MOUSELEAVE: u32 = 0x02A3;
    /// Re-reads the settings file: F12 and the settings window change it from elsewhere.
    const WATCH_TIMER: usize = 1;
    /// A few seconds after start, the tray icon is put on the taskbar itself (see `promote_tray_icon`).
    const PROMOTE_TIMER: usize = 2;
    /// A single click on the tray icon toggles once Windows' double-click time has passed without
    /// a second click; a double-click shows the top bar instead.
    const CLICK_TIMER: usize = 3;

    // MARK: The locked design (windows/design/top-bar.md), in DIPs.

    const BAR_WIDTH: f32 = 378.0;
    const BAR_HEIGHT: f32 = 48.0;
    const BAR_RADIUS: f32 = 10.0;
    const PADDING: f32 = 10.0;
    const LOGO_WIDTH: f32 = 37.0;
    const LOGO_HEIGHT: f32 = 28.0;
    const SWITCH_WIDTH: f32 = 161.0;
    const ICON: f32 = 20.0;
    const ICON_GAP: f32 = 16.0;
    /// The hover square behind each icon.
    const ICON_HIT: f32 = 28.0;

    const BAR_COLOR: Color = Color(0x0D0807, 1.0);
    const BOX: Color = Color(0x241C1A, 1.0);
    const BOX_HOVER: Color = Color(0x2B2220, 1.0);
    const BOX_BORDER: Color = Color(0x2D2827, 1.0);
    const INK: Color = Color(0xFFFFFF, 0.9);
    const WHITE: Color = Color(0xFFFFFF, 1.0);
    const DIM: Color = Color(0xFFFFFF, 0.55);
    const CLOSE_HOVER: Color = Color(0xC42B1C, 1.0);
    const DANGER_HOVER: Color = Color(0x3A1410, 1.0);

    // MARK: Icons: Lucide (24-unit view), and the designed "to the tray" sign (20-unit view).

    const CHEVRON_DOWN: Icon = Icon { view: 24.0, paths: &["m6 9 6 6 6-6"] };
    const CHEVRON_RIGHT: Icon = Icon { view: 24.0, paths: &["m9 18 6-6-6-6"] };
    const CHECK: Icon = Icon { view: 24.0, paths: &["M20 6 9 17l-5-5"] };
    const X: Icon = Icon { view: 24.0, paths: &["M18 6 6 18", "m6 6 12 12"] };
    const SETTINGS: Icon = Icon {
        view: 24.0,
        paths: &[
            "M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915",
            "M9 12a3 3 0 1 0 6 0a3 3 0 1 0 -6 0",
        ],
    };
    const INFO: Icon = Icon {
        view: 24.0,
        paths: &["M2 12a10 10 0 1 0 20 0a10 10 0 1 0 -20 0", "M12 16v-4", "M12 8h.01"],
    };
    const TO_TRAY: Icon = Icon {
        view: 20.0,
        paths: &[
            "M3.33 2.5h13.34a.83.83 0 0 1 .83.83v1.67a.83.83 0 0 1-.83.83H3.33a.83.83 0 0 1-.83-.83V3.33a.83.83 0 0 1 .83-.83z",
            "M2.5 9.17h15L10 17.5Z",
        ],
    };
    const POWER: Icon = Icon { view: 24.0, paths: &["M12 2v10", "M18.4 6.6a9 9 0 1 1-12.77.04"] };
    const PANEL_TOP: Icon = Icon {
        view: 24.0,
        paths: &["M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z", "M3 9h18"],
    };
    const KEYBOARD: Icon = Icon {
        view: 24.0,
        paths: &[
            "M10 8h.01", "M12 12h.01", "M14 8h.01", "M16 12h.01", "M18 8h.01", "M6 8h.01", "M7 16h10", "M8 12h.01",
            "M4 4h16a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2z",
        ],
    };
    const BOOK: Icon = Icon {
        view: 24.0,
        paths: &[
            "M12 7v14",
            "M3 18a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h5a4 4 0 0 1 4 4 4 4 0 0 1 4-4h5a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1h-6a3 3 0 0 0-3 3 3 3 0 0 0-3-3z",
        ],
    };
    const GLOBE: Icon = Icon {
        view: 24.0,
        paths: &["M2 12a10 10 0 1 0 20 0a10 10 0 1 0 -20 0", "M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20", "M2 12h20"],
    };
    const LOG_OUT: Icon = Icon {
        view: 24.0,
        paths: &["m16 17 5-5-5-5", "M21 12H9", "M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"],
    };

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Part {
        Logo,
        Switch,
        Settings,
        Help,
        ToTray,
        Close,
    }

    /// Each part's area on the bar, in DIPs.
    fn parts() -> [(Part, Rect); 6] {
        let icons_left = BAR_WIDTH - PADDING - (4.0 * ICON + 3.0 * ICON_GAP);
        let icon = |index: f32| {
            let center_x = icons_left + ICON / 2.0 + index * (ICON + ICON_GAP);
            Rect::new(center_x - ICON_HIT / 2.0, (BAR_HEIGHT - ICON_HIT) / 2.0, ICON_HIT, ICON_HIT)
        };
        [
            (Part::Logo, Rect::new(PADDING, PADDING, LOGO_WIDTH, LOGO_HEIGHT)),
            (Part::Switch, Rect::new(PADDING + LOGO_WIDTH + 12.0, PADDING, SWITCH_WIDTH, 28.0)),
            (Part::Settings, icon(0.0)),
            (Part::Help, icon(1.0)),
            (Part::ToTray, icon(2.0)),
            (Part::Close, icon(3.0)),
        ]
    }

    fn part_rect(part: Part) -> Rect {
        parts().into_iter().find(|(p, _)| *p == part).map(|(_, r)| r).unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0))
    }

    struct Drag {
        cursor: POINT,
        window: POINT,
        moved: bool,
    }

    struct App {
        window: HWND,
        gfx: Gfx,
        settings: Settings,
        /// System DPI: the bar is laid out in DIPs and drawn at this.
        dpi: f32,
        hover: Option<Part>,
        drag: Option<Drag>,
        tray_icon: HICON,
        /// Windows' message when Explorer restarts: the tray icon must be added again.
        taskbar_created: u32,
        /// Attempts at `promote_tray_icon` so far.
        promote_tries: u32,
        /// The button-up that ends a double-click on the tray icon is not a click of its own.
        ignore_tray_up: bool,
        /// A click on a bar button whose menu was open closed that menu: the button-up that follows
        /// must not open it again.
        swallow_up: Option<Part>,
    }

    thread_local! {
        static APP: RefCell<Option<App>> = const { RefCell::new(None) };
        static MENU: RefCell<Option<MenuSession>> = const { RefCell::new(None) };
    }

    fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
        APP.with(|app| app.try_borrow_mut().ok().and_then(|mut app| app.as_mut().map(f)))
    }

    fn px(dips: f32, dpi: f32) -> i32 {
        (dips * dpi / 96.0).round() as i32
    }

    pub fn main() -> i32 {
        let argument = std::env::args().nth(1).unwrap_or_default();
        unsafe {
            if let Ok(running) = FindWindowW(BAR_CLASS, None) {
                let message = if argument == "--quit" { WM_CLOSE } else { WM_SHOW_BAR };
                let _ = PostMessageW(Some(running), message, WPARAM(0), LPARAM(0));
                return 0;
            }
            if argument == "--quit" {
                return 0;
            }
            run(argument == "--startup")
        }
    }

    unsafe fn run(startup: bool) -> i32 {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE);
        let Ok(gfx) = Gfx::new(win::bangla_font_for_program().as_deref()) else { return 1 };
        let instance = GetModuleHandleW(None).unwrap_or_default();
        for (name, procedure) in [
            (BAR_CLASS, bar_procedure as unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT),
            (MENU_CLASS, menu_procedure),
            (SPLASH_CLASS, splash_procedure),
        ] {
            let class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(procedure),
                hInstance: instance.into(),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                lpszClassName: name,
                ..Default::default()
            };
            RegisterClassExW(&class);
        }

        let dpi = GetDpiForSystem().max(96) as f32;
        // Always on top, out of the taskbar and Alt+Tab, never activated by a click (the app being
        // typed in keeps the keyboard), and layered: drawn with per-pixel alpha.
        let Ok(window) = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            BAR_CLASS,
            w!("নুকতা বাংলা"),
            WS_POPUP,
            0,
            0,
            px(BAR_WIDTH, dpi),
            px(BAR_HEIGHT, dpi),
            None,
            None,
            Some(instance.into()),
            None,
        ) else {
            return 1;
        };

        let settings = Settings::new(&win::prepare_data_directory());
        let app = App {
            window,
            gfx,
            settings,
            dpi,
            hover: None,
            drag: None,
            tray_icon: HICON::default(),
            taskbar_created: RegisterWindowMessageW(w!("TaskbarCreated")),
            promote_tries: 0,
            ignore_tray_up: false,
            swallow_up: None,
        };
        APP.with(|slot| *slot.borrow_mut() = Some(app));

        place_bar();
        draw_bar();
        update_tray_icon(NIM_ADD);
        SetTimer(Some(window), WATCH_TIMER, 250, None);
        if !with_app(|app| app.settings.tray_promoted()).unwrap_or(true) {
            SetTimer(Some(window), PROMOTE_TIMER, 3000, None);
        }
        // নুকতা বাংলা is the keyboard F12 and English mode work through: make sure it is the active
        // one, in case another was picked.
        let _ = win::activate_keyboard();
        if startup {
            // The bar appears when the splash has gone (see `splash_finished`).
            show_splash();
        } else if with_app(|app| app.settings.bar_visible()).unwrap_or(true) {
            let _ = ShowWindow(window, SW_SHOWNOACTIVATE);
        }

        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }

        APP.with(|slot| {
            if let Some(app) = slot.borrow_mut().take() {
                if !app.tray_icon.is_invalid() {
                    let _ = DestroyIcon(app.tray_icon);
                }
            }
        });
        0
    }

    // MARK: The mode

    /// None: English (off). Otherwise the Bangla layout.
    fn mode() -> Option<Layout> {
        with_app(|app| (!app.settings.english()).then(|| app.settings.layout())).flatten()
    }

    fn mode_label(mode: Option<Layout>) -> &'static str {
        match mode {
            None => "English",
            Some(Layout::Bijoy) => "বাংলা · বিজয়",
            Some(Layout::Phonetic) => "বাংলা · ফোনেটিক",
        }
    }

    /// Switches the mode for every app, through the settings file, with the beep when it turns
    /// Bangla on or off.
    fn set_mode(mode: Option<Layout>) {
        let was_on = self::mode().is_some();
        with_app(|app| {
            if let Some(layout) = mode {
                app.settings.set_layout(layout);
            }
            app.settings.set_english(mode.is_none());
        });
        let _ = win::activate_keyboard();
        if was_on != mode.is_some() {
            sound::play_toggle(mode.is_some());
        }
        mode_changed();
    }

    /// English ⇄ Bangla, as F12 does.
    fn toggle() {
        // The latest from the file: the layout may have been changed elsewhere a moment ago.
        let layout = with_app(|app| {
            app.settings.reload();
            app.settings.layout()
        })
        .unwrap_or_default();
        set_mode(if mode().is_none() { Some(layout) } else { None });
    }

    /// Redraws the bar and the tray icon after the mode changed (here or elsewhere).
    fn mode_changed() {
        draw_bar();
        update_tray_icon(NIM_MODIFY);
    }

    fn show_bar(visible: bool) {
        let Some(window) = with_app(|app| {
            app.settings.set_bar_visible(visible);
            app.window
        }) else {
            return;
        };
        unsafe {
            let _ = ShowWindow(window, if visible { SW_SHOWNOACTIVATE } else { SW_HIDE });
        }
    }

    /// Exit: the bar and the tray icon close and Bangla turns off. F12 (caught by the keyboard)
    /// turns it on again and starts this program.
    fn exit() {
        let was_on = mode().is_some();
        with_app(|app| app.settings.set_english(true));
        if was_on {
            sound::play_toggle(false);
        }
        if let Some(window) = with_app(|app| app.window) {
            unsafe {
                let _ = DestroyWindow(window);
            }
        }
    }

    fn open(target: PCWSTR, arguments: Option<PCWSTR>) {
        unsafe {
            ShellExecuteW(None, w!("open"), target, arguments.unwrap_or(PCWSTR::null()), PCWSTR::null(), SW_SHOWNORMAL);
        }
    }

    fn open_settings(arguments: Option<PCWSTR>) {
        let Ok(this) = std::env::current_exe() else { return };
        let program = wide(&this.with_file_name("NuktaBanglaSettings.exe").to_string_lossy());
        open(PCWSTR(program.as_ptr()), arguments);
    }

    // MARK: Where the bar sits

    /// A double-click on the tray icon: the top bar, shown again at its default spot.
    fn restore_bar() {
        let Some((window, dpi)) = with_app(|app| (app.window, app.dpi)) else { return };
        let (x, y) = default_position(dpi);
        unsafe {
            let _ = SetWindowPos(window, Some(HWND_TOPMOST), x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
        }
        with_app(|app| app.settings.set_bar_position(x, y));
        show_bar(true);
    }

    /// The top right of the main screen's work area, clear of the window buttons there.
    fn default_position(dpi: f32) -> (i32, i32) {
        let mut work = RECT::default();
        unsafe {
            let _ = SystemParametersInfoW(
                SPI_GETWORKAREA,
                0,
                Some(&mut work as *mut RECT as *mut _),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            );
        }
        (work.right - px(BAR_WIDTH, dpi) - px(220.0, dpi), work.top + px(8.0, dpi))
    }

    /// Where it was left, if that is still on a screen; else the default spot.
    fn place_bar() {
        let Some((window, saved, dpi)) = with_app(|app| (app.window, app.settings.bar_position(), app.dpi)) else {
            return;
        };
        unsafe {
            let on_screen = saved.filter(|&(x, y)| {
                let left = GetSystemMetrics(SM_XVIRTUALSCREEN);
                let top = GetSystemMetrics(SM_YVIRTUALSCREEN);
                let right = left + GetSystemMetrics(SM_CXVIRTUALSCREEN);
                let bottom = top + GetSystemMetrics(SM_CYVIRTUALSCREEN);
                x >= left && y >= top && x + px(BAR_WIDTH, dpi) <= right && y + px(BAR_HEIGHT, dpi) <= bottom
            });
            let (x, y) = on_screen.unwrap_or_else(|| default_position(dpi));
            let _ = SetWindowPos(window, Some(HWND_TOPMOST), x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
        }
    }

    fn save_position() {
        with_app(|app| unsafe {
            let mut rect = RECT::default();
            if GetWindowRect(app.window, &mut rect).is_ok() {
                app.settings.set_bar_position(rect.left, rect.top);
            }
        });
    }

    // MARK: Drawing the bar

    fn draw_bar() {
        let mode = mode();
        with_app(|app| {
            let hover = app.hover;
            app.gfx.present(app.window, BAR_WIDTH, BAR_HEIGHT, app.dpi, 255, |canvas| {
                canvas.fill_round(Rect::new(0.0, 0.0, BAR_WIDTH, BAR_HEIGHT), BAR_RADIUS, BAR_COLOR);
                for (part, rect) in parts() {
                    let hovered = hover == Some(part);
                    match part {
                        Part::Logo => canvas.logo(rect.left, rect.top, LOGO_WIDTH),
                        Part::Switch => {
                            canvas.fill_round(rect, 6.0, if hovered { BOX_HOVER } else { BOX });
                            canvas.stroke_round(rect, 6.0, BOX_BORDER);
                            let size = if mode.is_some() { 14.0 } else { 15.0 };
                            let label = Rect { left: rect.left + 8.0, right: rect.right - 28.0, ..rect };
                            canvas.text(mode_label(mode), label, Style::new(size), WHITE);
                            canvas.icon(&CHEVRON_DOWN, rect.right - 4.0 - ICON, rect.top + 4.0, ICON, INK);
                        }
                        _ => {
                            if hovered {
                                canvas.fill_round(rect, 6.0, if part == Part::Close { CLOSE_HOVER } else { BOX });
                            }
                            let icon = match part {
                                Part::Settings => &SETTINGS,
                                Part::Help => &INFO,
                                Part::ToTray => &TO_TRAY,
                                _ => &X,
                            };
                            let x = rect.left + (ICON_HIT - ICON) / 2.0;
                            let y = rect.top + (ICON_HIT - ICON) / 2.0;
                            canvas.icon(icon, x, y, ICON, if hovered && part == Part::Close { WHITE } else { INK });
                        }
                    }
                }
            });
        });
    }

    fn part_at(lparam: LPARAM) -> Option<Part> {
        let point = point_of(lparam);
        let dpi = with_app(|app| app.dpi).unwrap_or(96.0);
        let (x, y) = (point.x as f32 * 96.0 / dpi, point.y as f32 * 96.0 / dpi);
        parts().into_iter().find(|(_, rect)| rect.contains(x, y)).map(|(part, _)| part)
    }

    /// A part's area in screen pixels, to open its menu under it.
    fn part_on_screen(part: Part) -> Option<RECT> {
        let (window, dpi) = with_app(|app| (app.window, app.dpi))?;
        let rect = part_rect(part);
        let mut origin = RECT::default();
        unsafe {
            GetWindowRect(window, &mut origin).ok()?;
        }
        Some(RECT {
            left: origin.left + px(rect.left, dpi),
            top: origin.top + px(rect.top, dpi),
            right: origin.left + px(rect.right, dpi),
            bottom: origin.top + px(rect.bottom, dpi),
        })
    }

    fn set_hover(part: Option<Part>) {
        let changed = with_app(|app| std::mem::replace(&mut app.hover, part) != part).unwrap_or(false);
        if changed {
            draw_bar();
        }
    }

    // MARK: Menus: drawn in the bar's style (top-bar.md), run modally like Windows' own.

    #[derive(Clone)]
    enum Entry {
        Item { id: usize, label: String, icon: Option<&'static Icon>, checked: bool, shortcut: Option<&'static str>, danger: bool },
        Separator,
        Submenu { label: &'static str, icon: &'static Icon, entries: Vec<Entry> },
    }

    fn item(id: usize, label: impl Into<String>, icon: Option<&'static Icon>) -> Entry {
        Entry::Item { id, label: label.into(), icon, checked: false, shortcut: None, danger: false }
    }

    fn checked(id: usize, label: &str, on: bool) -> Entry {
        Entry::Item { id, label: label.into(), icon: None, checked: on, shortcut: None, danger: false }
    }

    // Menu commands.
    const CMD_TOGGLE: usize = 1;
    const CMD_ENGLISH: usize = 2;
    const CMD_BIJOY: usize = 3;
    const CMD_PHONETIC: usize = 4;
    const CMD_SETTINGS: usize = 5;
    const CMD_BAR: usize = 6;
    const CMD_ABOUT: usize = 7;
    const CMD_EXIT: usize = 8;
    const CMD_HELP: usize = 9;
    const CMD_TO_TRAY: usize = 10;
    const CMD_WEBSITE: usize = 11;

    fn layout_entries() -> Vec<Entry> {
        let mode = mode();
        vec![
            checked(CMD_ENGLISH, "English", mode.is_none()),
            checked(CMD_BIJOY, "বাংলা · বিজয়", mode == Some(Layout::Bijoy)),
            checked(CMD_PHONETIC, "বাংলা · ফোনেটিক", mode == Some(Layout::Phonetic)),
        ]
    }

    /// The switch's menu: just the three modes (on and off are F12, the logo and the tray icon).
    fn layout_menu() -> Vec<Entry> {
        layout_entries()
    }

    fn help_menu() -> Vec<Entry> {
        vec![
            item(CMD_HELP, "ব্যবহারের নির্দেশিকা", Some(&BOOK)),
            item(CMD_WEBSITE, "ওয়েবসাইটে যান", Some(&GLOBE)),
            Entry::Separator,
            item(CMD_ABOUT, "নুকতা বাংলা সম্পর্কে", Some(&INFO)),
        ]
    }

    fn close_menu() -> Vec<Entry> {
        vec![
            item(CMD_TO_TRAY, "সিস্টেম ট্রেতে পাঠান", Some(&TO_TRAY)),
            Entry::Separator,
            Entry::Item { id: CMD_EXIT, label: "বন্ধ করুন".into(), icon: Some(&LOG_OUT), checked: false, shortcut: None, danger: true },
        ]
    }

    /// The tray icon's menu (and a right click on the bar): everything.
    fn tray_menu() -> Vec<Entry> {
        let on = mode().is_some();
        let bar_visible = with_app(|app| app.settings.bar_visible()).unwrap_or(true);
        vec![
            Entry::Item {
                id: CMD_TOGGLE,
                label: if on { "English-এ যান" } else { "বাংলা চালু করুন" }.into(),
                icon: Some(&POWER),
                checked: false,
                shortcut: Some("F12"),
                danger: false,
            },
            item(CMD_BAR, if bar_visible { "টপ বার লুকান" } else { "টপ বার দেখান" }, Some(&PANEL_TOP)),
            Entry::Separator,
            Entry::Submenu { label: "কীবোর্ড লেআউট", icon: &KEYBOARD, entries: layout_entries() },
            item(CMD_SETTINGS, "সেটিংস…", Some(&SETTINGS)),
            Entry::Submenu { label: "সাহায্য", icon: &INFO, entries: help_menu() },
            Entry::Separator,
            Entry::Item { id: CMD_EXIT, label: "বন্ধ করুন".into(), icon: Some(&LOG_OUT), checked: false, shortcut: None, danger: true },
        ]
    }

    fn run_command(command: usize) {
        match command {
            CMD_TOGGLE => toggle(),
            CMD_ENGLISH => set_mode(None),
            CMD_BIJOY => set_mode(Some(Layout::Bijoy)),
            CMD_PHONETIC => set_mode(Some(Layout::Phonetic)),
            CMD_SETTINGS => open_settings(None),
            CMD_BAR => {
                let visible = with_app(|app| app.settings.bar_visible()).unwrap_or(true);
                show_bar(!visible);
            }
            CMD_TO_TRAY => show_bar(false),
            CMD_HELP => open(GUIDE_URL, None),
            CMD_WEBSITE => open(WEBSITE_URL, None),
            CMD_ABOUT => open_settings(Some(w!("--about"))),
            CMD_EXIT => exit(),
            _ => {}
        }
    }

    // Menu metrics, in DIPs.
    const MENU_PADDING: f32 = 6.0;
    const MENU_RADIUS: f32 = 8.0;
    const ITEM_HEIGHT: f32 = 32.0;
    const SEPARATOR_HEIGHT: f32 = 13.0;
    const MENU_ICON: f32 = 16.0;
    const MENU_TEXT: f32 = 14.0;
    const SHORTCUT_TEXT: f32 = 12.0;

    /// One open menu: the root, or a submenu of the one before it.
    #[derive(Clone)]
    struct Level {
        window: HWND,
        entries: Vec<Entry>,
        /// Where it is on screen, in pixels.
        frame: RECT,
        width: f32,
        height: f32,
        hover: Option<usize>,
    }

    struct MenuSession {
        levels: Vec<Level>,
        done: bool,
        chosen: usize,
    }

    fn with_menu<R>(f: impl FnOnce(&mut MenuSession) -> R) -> Option<R> {
        MENU.with(|menu| menu.try_borrow_mut().ok().and_then(|mut menu| menu.as_mut().map(f)))
    }

    /// Each entry's area in a menu, in DIPs from its top left.
    fn entry_rects(entries: &[Entry], width: f32) -> Vec<Rect> {
        let mut y = MENU_PADDING;
        entries
            .iter()
            .map(|entry| {
                let height = if matches!(entry, Entry::Separator) { SEPARATOR_HEIGHT } else { ITEM_HEIGHT };
                let rect = Rect::new(MENU_PADDING, y, width - 2.0 * MENU_PADDING, height);
                y += height;
                rect
            })
            .collect()
    }

    /// The size a menu needs, in DIPs.
    fn menu_size(entries: &[Entry]) -> (f32, f32) {
        let mut width: f32 = 200.0;
        let mut height = 2.0 * MENU_PADDING;
        with_app(|app| {
            for entry in entries {
                match entry {
                    Entry::Separator => height += SEPARATOR_HEIGHT,
                    Entry::Item { label, shortcut, .. } => {
                        height += ITEM_HEIGHT;
                        let mut w = 10.0 + MENU_ICON + 10.0 + app.gfx.text_width(label, Style::new(MENU_TEXT)) + 10.0;
                        if let Some(shortcut) = shortcut {
                            w += 24.0 + app.gfx.text_width(shortcut, Style::new(SHORTCUT_TEXT));
                        }
                        width = width.max(w + 2.0 * MENU_PADDING);
                    }
                    Entry::Submenu { label, .. } => {
                        height += ITEM_HEIGHT;
                        let w = 10.0 + MENU_ICON + 10.0 + app.gfx.text_width(label, Style::new(MENU_TEXT)) + 8.0 + MENU_ICON + 10.0;
                        width = width.max(w + 2.0 * MENU_PADDING);
                    }
                }
            }
        });
        (width.ceil(), height)
    }

    /// The work area of the screen `point` is on.
    fn work_area(point: POINT) -> RECT {
        unsafe {
            let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
            let _ = GetMonitorInfoW(monitor, &mut info);
            info.rcWork
        }
    }

    /// Shows `entries` as a menu and returns the command chosen, or 0. `anchor`: a point (screen
    /// pixels) the menu opens from: below and right of it, or left of it with `align_right`; above
    /// `flip_top` when there is no room below.
    fn run_menu(entries: Vec<Entry>, anchor: POINT, align_right: bool, flip_top: i32) -> usize {
        if MENU.with(|menu| menu.borrow().is_some()) {
            return 0;
        }
        let dpi = with_app(|app| app.dpi).unwrap_or(96.0);
        let (width, height) = menu_size(&entries);
        let (w, h) = (px(width, dpi), px(height, dpi));
        let work = work_area(anchor);
        let mut x = if align_right { anchor.x - w } else { anchor.x };
        let mut y = anchor.y;
        if y + h > work.bottom {
            y = flip_top - h;
        }
        x = x.clamp(work.left, (work.right - w).max(work.left));
        y = y.clamp(work.top, (work.bottom - h).max(work.top));

        let Some(root) = open_level(entries, RECT { left: x, top: y, right: x + w, bottom: y + h }, width, height) else {
            return 0;
        };
        let root_window = root.window;
        MENU.with(|menu| *menu.borrow_mut() = Some(MenuSession { levels: vec![root], done: false, chosen: 0 }));
        unsafe {
            // In front, so keys reach it; capturing the mouse, so a click anywhere else closes it.
            let previous = GetForegroundWindow();
            let _ = SetForegroundWindow(root_window);
            SetCapture(root_window);

            let mut message = MSG::default();
            loop {
                if with_menu(|m| m.done).unwrap_or(true) {
                    break;
                }
                if !GetMessageW(&mut message, None, 0, 0).as_bool() {
                    PostQuitMessage(0);
                    break;
                }
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }

            let session = MENU.with(|menu| menu.borrow_mut().take());
            let _ = ReleaseCapture();
            let chosen = session.as_ref().map_or(0, |s| s.chosen);
            if let Some(session) = session {
                for level in session.levels.iter().rev() {
                    let _ = DestroyWindow(level.window);
                }
            }
            let bar = with_app(|app| app.window).unwrap_or_default();
            if !previous.is_invalid() && previous != bar {
                let _ = SetForegroundWindow(previous);
            }
            chosen
        }
    }

    /// Creates and draws one menu window.
    fn open_level(entries: Vec<Entry>, frame: RECT, width: f32, height: f32) -> Option<Level> {
        unsafe {
            let instance = GetModuleHandleW(None).ok()?;
            let window = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                MENU_CLASS,
                w!(""),
                WS_POPUP,
                frame.left,
                frame.top,
                frame.right - frame.left,
                frame.bottom - frame.top,
                None,
                None,
                Some(instance.into()),
                None,
            )
            .ok()?;
            let level = Level { window, entries, frame, width, height, hover: None };
            draw_level(&level);
            let _ = ShowWindow(window, SW_SHOWNOACTIVATE);
            Some(level)
        }
    }

    fn draw_level(level: &Level) {
        let rects = entry_rects(&level.entries, level.width);
        with_app(|app| {
            app.gfx.present(level.window, level.width, level.height, app.dpi, 255, |canvas| {
                let frame = Rect::new(0.0, 0.0, level.width, level.height);
                canvas.fill_round(frame, MENU_RADIUS, BAR_COLOR);
                canvas.stroke_round(frame, MENU_RADIUS, BOX_BORDER);
                for (index, (entry, rect)) in level.entries.iter().zip(&rects).enumerate() {
                    let hovered = level.hover == Some(index);
                    match entry {
                        Entry::Separator => {
                            let y = rect.top + (SEPARATOR_HEIGHT / 2.0).floor();
                            canvas.fill_round(Rect::new(rect.left + 4.0, y, rect.right - rect.left - 8.0, 1.0), 0.0, BOX_BORDER);
                        }
                        Entry::Item { label, icon, checked, shortcut, danger, .. } => {
                            if hovered {
                                canvas.fill_round(*rect, 6.0, if *danger { DANGER_HOVER } else { BOX });
                            }
                            let icon_y = rect.top + (ITEM_HEIGHT - MENU_ICON) / 2.0;
                            if let Some(icon) = icon {
                                canvas.icon(icon, rect.left + 10.0, icon_y, MENU_ICON, INK);
                            } else if *checked {
                                canvas.icon(&CHECK, rect.left + 10.0, icon_y, MENU_ICON, INK);
                            }
                            let text = Rect { left: rect.left + 36.0, right: rect.right - 10.0, ..*rect };
                            canvas.text(label, text, Style::new(MENU_TEXT), INK);
                            if let Some(shortcut) = shortcut {
                                canvas.text(shortcut, text, Style::new(SHORTCUT_TEXT).align(Align::Right), DIM);
                            }
                        }
                        Entry::Submenu { label, icon, .. } => {
                            if hovered {
                                canvas.fill_round(*rect, 6.0, BOX);
                            }
                            let icon_y = rect.top + (ITEM_HEIGHT - MENU_ICON) / 2.0;
                            canvas.icon(icon, rect.left + 10.0, icon_y, MENU_ICON, INK);
                            let text = Rect { left: rect.left + 36.0, right: rect.right - 34.0, ..*rect };
                            canvas.text(label, text, Style::new(MENU_TEXT), INK);
                            canvas.icon(&CHEVRON_RIGHT, rect.right - 10.0 - MENU_ICON, icon_y, MENU_ICON, INK);
                        }
                    }
                }
            });
        });
    }

    fn redraw_level(depth: usize) {
        if let Some(level) = with_menu(|m| m.levels.get(depth).cloned()).flatten() {
            draw_level(&level);
        }
    }

    /// Which menu level and entry the screen point is over, deepest first.
    fn menu_hit(point: POINT) -> Option<(usize, Option<usize>)> {
        let dpi = with_app(|app| app.dpi).unwrap_or(96.0);
        with_menu(|m| {
            for (depth, level) in m.levels.iter().enumerate().rev() {
                let f = level.frame;
                if point.x >= f.left && point.x < f.right && point.y >= f.top && point.y < f.bottom {
                    let x = (point.x - f.left) as f32 * 96.0 / dpi;
                    let y = (point.y - f.top) as f32 * 96.0 / dpi;
                    let index = entry_rects(&level.entries, level.width).iter().position(|r| r.contains(x, y));
                    let index = index.filter(|&i| !matches!(level.entries[i], Entry::Separator));
                    return Some((depth, index));
                }
            }
            None
        })
        .flatten()
    }

    /// Hovers entry `index` of level `depth`: closes deeper menus, and opens a submenu.
    fn hover_entry(depth: usize, index: Option<usize>) {
        let dpi = with_app(|app| app.dpi).unwrap_or(96.0);
        // Already hovered, with its submenu (if any) open: nothing to do.
        let unchanged = with_menu(|m| {
            let level = &m.levels[depth];
            let opens_submenu = index.is_some_and(|i| matches!(level.entries[i], Entry::Submenu { .. }));
            level.hover == index && m.levels.len() == depth + 1 + usize::from(opens_submenu)
        })
        .unwrap_or(true);
        if unchanged {
            return;
        }
        let closed: Vec<HWND> = with_menu(|m| m.levels.drain(depth + 1..).map(|l| l.window).collect()).unwrap_or_default();
        for window in closed {
            unsafe {
                let _ = DestroyWindow(window);
            }
        }
        let submenu = with_menu(|m| {
            let level = &mut m.levels[depth];
            level.hover = index;
            let index = index?;
            let Entry::Submenu { entries, .. } = &level.entries[index] else { return None };
            let rect = entry_rects(&level.entries, level.width)[index];
            Some((entries.clone(), level.frame, rect))
        })
        .flatten();
        redraw_level(depth);
        let Some((entries, parent, rect)) = submenu else { return };
        let (width, height) = menu_size(&entries);
        let (w, h) = (px(width, dpi), px(height, dpi));
        let work = work_area(POINT { x: parent.right, y: parent.top });
        let mut x = parent.right + px(2.0, dpi);
        if x + w > work.right {
            x = parent.left - w - px(2.0, dpi);
        }
        let mut y = parent.top + px(rect.top - MENU_PADDING, dpi);
        y = y.clamp(work.top, (work.bottom - h).max(work.top));
        if let Some(level) = open_level(entries, RECT { left: x, top: y, right: x + w, bottom: y + h }, width, height) {
            with_menu(|m| m.levels.push(level));
        }
    }

    fn finish_menu(chosen: usize) {
        with_menu(|m| {
            m.done = true;
            m.chosen = chosen;
        });
    }

    /// Activates entry `index` of level `depth`: a command ends the menu; a submenu opens.
    fn activate_entry(depth: usize, index: usize) {
        let entry = with_menu(|m| m.levels[depth].entries[index].clone());
        match entry {
            Some(Entry::Item { id, .. }) => finish_menu(id),
            Some(Entry::Submenu { .. }) => {
                hover_entry(depth, Some(index));
                // The first entry of the new submenu, ready for the arrow keys.
                let depth = depth + 1;
                if with_menu(|m| m.levels.len() > depth).unwrap_or(false) {
                    with_menu(|m| m.levels[depth].hover = Some(0));
                    redraw_level(depth);
                }
            }
            _ => {}
        }
    }

    /// Up and down: the next entry that isn't a separator, wrapping.
    fn step_hover(forward: bool) {
        let Some((depth, next)) = with_menu(|m| {
            let depth = m.levels.len() - 1;
            let level = &m.levels[depth];
            let count = level.entries.len();
            let mut index = level.hover.unwrap_or(if forward { count - 1 } else { 0 });
            for _ in 0..count {
                index = if forward { (index + 1) % count } else { (index + count - 1) % count };
                if !matches!(level.entries[index], Entry::Separator) {
                    break;
                }
            }
            (depth, index)
        }) else {
            return;
        };
        with_menu(|m| m.levels[depth].hover = Some(next));
        redraw_level(depth);
    }

    unsafe extern "system" fn menu_procedure(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let active = MENU.with(|menu| menu.try_borrow().map(|m| m.is_some()).unwrap_or(false));
        if !active {
            return unsafe { DefWindowProcW(window, message, wparam, lparam) };
        }
        let screen_point = || {
            let mut point = point_of(lparam);
            unsafe {
                let _ = ClientToScreen(window, &mut point);
            }
            point
        };
        match message {
            WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
            WM_MOUSEMOVE => {
                if let Some((depth, index)) = menu_hit(screen_point()) {
                    if index.is_some() || with_menu(|m| m.levels.len() == depth + 1).unwrap_or(false) {
                        hover_entry(depth, index);
                    }
                }
                LRESULT(0)
            }
            WM_LBUTTONUP | WM_RBUTTONUP => {
                match menu_hit(screen_point()) {
                    Some((depth, Some(index))) => activate_entry(depth, index),
                    Some((_, None)) => {}
                    None => finish_menu(0),
                }
                LRESULT(0)
            }
            WM_LBUTTONDOWN | WM_RBUTTONDOWN => {
                if menu_hit(screen_point()).is_none() {
                    finish_menu(0);
                }
                LRESULT(0)
            }
            WM_KEYDOWN => {
                let key = wparam.0 as u16;
                if key == VK_ESCAPE.0 {
                    finish_menu(0);
                } else if key == VK_DOWN.0 {
                    step_hover(true);
                } else if key == VK_UP.0 {
                    step_hover(false);
                } else if key == VK_LEFT.0 {
                    let closed = with_menu(|m| if m.levels.len() > 1 { m.levels.pop().map(|l| l.window) } else { None }).flatten();
                    if let Some(closed) = closed {
                        unsafe {
                            let _ = DestroyWindow(closed);
                        }
                    }
                } else if key == VK_RIGHT.0 || key == VK_RETURN.0 {
                    let target = with_menu(|m| {
                        let depth = m.levels.len() - 1;
                        m.levels[depth].hover.map(|i| (depth, i))
                    })
                    .flatten();
                    if let Some((depth, index)) = target {
                        let is_submenu =
                            with_menu(|m| matches!(m.levels[depth].entries[index], Entry::Submenu { .. })).unwrap_or(false);
                        if key == VK_RETURN.0 || is_submenu {
                            activate_entry(depth, index);
                        }
                    }
                }
                LRESULT(0)
            }
            // Switched away (another app came to the front): the menu closes.
            WM_ACTIVATE if (wparam.0 & 0xFFFF) as u32 == WA_INACTIVE => {
                let other = HWND(lparam.0 as *mut _);
                let ours = with_menu(|m| m.levels.iter().any(|l| l.window == other)).unwrap_or(false);
                if !ours {
                    finish_menu(0);
                }
                LRESULT(0)
            }
            // The mouse was taken away (another app, a system dialog): the menu closes.
            WM_CAPTURECHANGED => {
                let root = with_menu(|m| m.levels[0].window).unwrap_or_default();
                if window == root && unsafe { GetCapture() } != root {
                    finish_menu(0);
                }
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
        }
    }

    /// Opens `entries` under the bar's `part`: left-aligned, or right-aligned with `align_right`.
    fn menu_under(part: Part, entries: Vec<Entry>, align_right: bool) {
        let Some(rect) = part_on_screen(part) else { return };
        let dpi = with_app(|app| app.dpi).unwrap_or(96.0);
        let gap = px(6.0, dpi);
        let anchor = POINT { x: if align_right { rect.right } else { rect.left }, y: rect.bottom + gap };
        let command = run_menu(entries, anchor, align_right, rect.top - gap);
        // Closed by pressing this same button again: it stays closed when the button is let go.
        let pressed = unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) } < 0;
        if command == 0 && pressed && part_under_cursor() == Some(part) {
            with_app(|app| app.swallow_up = Some(part));
        }
        run_command(command);
    }

    /// The bar part under the mouse pointer, if any.
    fn part_under_cursor() -> Option<Part> {
        let (window, dpi) = with_app(|app| (app.window, app.dpi))?;
        let mut point = POINT::default();
        let mut frame = RECT::default();
        unsafe {
            GetCursorPos(&mut point).ok()?;
            GetWindowRect(window, &mut frame).ok()?;
        }
        let (x, y) = ((point.x - frame.left) as f32 * 96.0 / dpi, (point.y - frame.top) as f32 * 96.0 / dpi);
        parts().into_iter().find(|(_, rect)| rect.contains(x, y)).map(|(part, _)| part)
    }

    /// The tray menu at the pointer: opening up and to the left from the taskbar, as tray menus do.
    fn tray_menu_at_cursor() {
        let mut point = POINT::default();
        unsafe {
            let _ = GetCursorPos(&mut point);
        }
        let command = run_menu(tray_menu(), point, true, point.y);
        run_command(command);
    }

    // MARK: The tray icon

    fn update_tray_icon(action: NOTIFY_ICON_MESSAGE) {
        let mode = mode();
        with_app(|app| unsafe {
            let size = GetSystemMetrics(SM_CXSMICON);
            let icon = nukta_bangla::logo::tray_icon(mode.is_some(), size);
            // Filled apart and copied in whole: the structure is packed in 32-bit Windows, so its
            // fields can't be borrowed.
            let mut tip = [0u16; 128];
            let text = format!("নুকতা বাংলা: {}\nক্লিক বা F12: চালু/বন্ধ", mode_label(mode));
            for (slot, unit) in tip.iter_mut().zip(text.encode_utf16().take(127)) {
                *slot = unit;
            }
            let data = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: app.window,
                uID: TRAY_ID,
                uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
                uCallbackMessage: WM_TRAY,
                hIcon: icon,
                szTip: tip,
                ..Default::default()
            };
            let _ = Shell_NotifyIconW(action, &data);
            if !app.tray_icon.is_invalid() {
                let _ = DestroyIcon(app.tray_icon);
            }
            app.tray_icon = icon;
        });
    }

    fn remove_tray_icon() {
        with_app(|app| unsafe {
            let data = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: app.window,
                uID: TRAY_ID,
                ..Default::default()
            };
            let _ = Shell_NotifyIconW(NIM_DELETE, &data);
        });
    }

    /// Puts the tray icon on the taskbar, next to the clock, rather than behind its ^ (where
    /// Windows 11 puts new icons). The same as the user switching it on in Settings → Personalization
    /// → Taskbar → Other system tray icons: Windows keeps one entry per program under
    /// `HKCU\Control Panel\NotifyIconSettings`, and `IsPromoted` = 1 shows it. Done once (the
    /// settings remember it); true once our entry was found and set.
    fn promote_tray_icon() -> bool {
        let Ok(this) = std::env::current_exe() else { return false };
        let this = this.to_string_lossy().to_lowercase();
        unsafe {
            let mut root = HKEY::default();
            if RegOpenKeyExW(HKEY_CURRENT_USER, w!("Control Panel\\NotifyIconSettings"), None, KEY_READ, &mut root)
                .is_err()
            {
                return false;
            }
            let mut found = false;
            let mut index = 0;
            loop {
                let mut name = [0u16; 64];
                let mut length = name.len() as u32;
                if RegEnumKeyExW(root, index, Some(windows::core::PWSTR(name.as_mut_ptr())), &mut length, None, None, None, None)
                    .is_err()
                {
                    break;
                }
                index += 1;
                let mut path = [0u16; 1024];
                let mut size = (path.len() * 2) as u32;
                let read = RegGetValueW(
                    root,
                    PCWSTR(name.as_ptr()),
                    w!("ExecutablePath"),
                    RRF_RT_REG_SZ,
                    None,
                    Some(path.as_mut_ptr() as *mut _),
                    Some(&mut size),
                );
                if read.is_err() {
                    continue;
                }
                let end = path.iter().position(|&c| c == 0).unwrap_or(path.len());
                if String::from_utf16_lossy(&path[..end]).to_lowercase() != this {
                    continue;
                }
                let mut entry = HKEY::default();
                if RegOpenKeyExW(root, PCWSTR(name.as_ptr()), None, KEY_WRITE, &mut entry).is_ok() {
                    let one = 1u32.to_le_bytes();
                    found = RegSetValueExW(entry, w!("IsPromoted"), None, REG_DWORD, Some(&one)).is_ok();
                    let _ = RegCloseKey(entry);
                }
                break;
            }
            let _ = RegCloseKey(root);
            found
        }
    }


    // MARK: The splash (windows/design/splash-preview.html), shown as Windows starts.

    const SPLASH_WIDTH: f32 = 560.0;
    const SPLASH_HEIGHT: f32 = 320.0;
    /// Fade in, fill the line, hold, fade out: milliseconds from the start.
    const SPLASH_IN: f32 = 450.0;
    const SPLASH_LINE_START: f32 = 300.0;
    const SPLASH_LINE: f32 = 2200.0;
    const SPLASH_OUT_AT: f32 = 2700.0;
    const SPLASH_OUT: f32 = 450.0;
    const ALPHABET: &str = "অ আ ই ঈ উ ঊ ঋ এ ঐ ও ঔ ক খ গ ঘ ঙ চ ছ জ ঝ ঞ ট ঠ ড ঢ ণ ত থ দ ধ ন প ফ ব ভ ম য র ল শ ষ স হ ড় ঢ় য় ৎ ং ঃ অ আ ই ঈ উ ঊ ঋ এ ঐ ও ঔ ক খ গ ঘ ঙ চ ছ জ ঝ ঞ ট ঠ ড ঢ ণ ত থ দ ধ ন প ফ ব ভ ম য র ল শ ষ স হ";

    struct Splash {
        window: HWND,
        started: std::time::Instant,
        /// When a click asked it to go early: the fade-out starts from here.
        closing_from: Option<f32>,
    }

    thread_local! {
        static SPLASH: RefCell<Option<Splash>> = const { RefCell::new(None) };
    }

    fn show_splash() {
        let Some(dpi) = with_app(|app| app.dpi) else { return };
        let (w, h) = (px(SPLASH_WIDTH, dpi), px(SPLASH_HEIGHT, dpi));
        // Centred on the main screen's work area.
        let work = work_area(POINT { x: 0, y: 0 });
        let x = work.left + (work.right - work.left - w) / 2;
        let y = work.top + (work.bottom - work.top - h) / 2;
        unsafe {
            let Ok(instance) = GetModuleHandleW(None) else { return splash_finished() };
            let Ok(window) = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                SPLASH_CLASS,
                w!("নুকতা বাংলা"),
                WS_POPUP,
                x,
                y,
                w,
                h,
                None,
                None,
                Some(instance.into()),
                None,
            ) else {
                return splash_finished();
            };
            SPLASH.with(|splash| {
                *splash.borrow_mut() = Some(Splash { window, started: std::time::Instant::now(), closing_from: None })
            });
            draw_splash();
            let _ = ShowWindow(window, SW_SHOWNOACTIVATE);
            SetTimer(Some(window), SPLASH_TIMER, 16, None);
        }
    }

    /// The splash has gone: the bar takes its place (unless it was hidden to the tray).
    fn splash_finished() {
        SPLASH.with(|splash| splash.borrow_mut().take());
        let Some((window, visible)) = with_app(|app| (app.window, app.settings.bar_visible())) else { return };
        if visible {
            unsafe {
                let _ = ShowWindow(window, SW_SHOWNOACTIVATE);
            }
        }
    }

    /// Draws the splash for this moment. False once it has fully faded out.
    fn draw_splash() -> bool {
        let Some((window, elapsed, closing_from)) = SPLASH.with(|splash| {
            splash.borrow().as_ref().map(|s| (s.window, s.started.elapsed().as_secs_f32() * 1000.0, s.closing_from))
        }) else {
            return false;
        };
        let out_at = closing_from.unwrap_or(SPLASH_OUT_AT);
        let ease = |t: f32| 1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3);
        let opacity = if elapsed >= out_at {
            1.0 - ((elapsed - out_at) / SPLASH_OUT).clamp(0.0, 1.0)
        } else {
            ease(elapsed / SPLASH_IN)
        };
        if elapsed >= out_at + SPLASH_OUT {
            return false;
        }
        let progress = ease((elapsed - SPLASH_LINE_START) / SPLASH_LINE);
        with_app(|app| {
            app.gfx.present(window, SPLASH_WIDTH, SPLASH_HEIGHT, app.dpi, (opacity * 255.0) as u8, |canvas| {
                paint_splash(canvas, progress);
            });
        });
        true
    }

    fn paint_splash(canvas: &Canvas, progress: f32) {
        let (w, h) = (SPLASH_WIDTH, SPLASH_HEIGHT);
        let panel = 210.0;
        let card = Rect::new(0.0, 0.0, w, h);
        canvas.fill_round(card, 16.0, BAR_COLOR);

        // Left: the logo glowing on a field of the alphabet.
        canvas.fill_round(Rect::new(0.0, 0.0, panel + 16.0, h), 16.0, Color(0x150C09, 1.0));
        canvas.fill_rect(Rect::new(panel, 0.0, 16.0, h), BAR_COLOR);
        canvas.rotated(-12.0, panel / 2.0, h / 2.0, Rect::new(4.0, 4.0, panel - 4.0, h - 8.0), |c| {
            c.text(ALPHABET, Rect::new(-40.0, -30.0, panel + 80.0, h + 60.0), Style::new(30.0).wrap(), Color(0xE87A1E, 0.07));
        });
        canvas.fill_rect(Rect::new(panel, 0.0, 1.0, h), BOX_BORDER);
        canvas.glow(panel / 2.0, h / 2.0, 130.0, Color(0xE87A1E, 0.35));
        canvas.logo(panel / 2.0 - 60.0, h / 2.0 - 45.4, 120.0);

        // Right: name, words, the F12 hint, the line, the small print.
        let left = panel + 32.0;
        let width = w - left - 34.0;
        canvas.text("NUKTA BANGLA KEYBOARD", Rect::new(left, 30.0, width, 16.0), Style::new(11.0).weight(600), Color(0xE87A1E, 1.0));
        canvas.text("নুকতা বাংলা", Rect::new(left, 52.0, width, 54.0), Style::new(38.0).weight(700), Color(0xFFFFFF, 0.94));
        canvas.text("মনের কথা, নিজের ভাষায়।", Rect::new(left, 108.0, width, 28.0), Style::new(16.0), Color(0xFFFFFF, 0.78));
        let mut x = left;
        for chip in ["বিজয়", "ফোনেটিক", "ইউনিকোড"] {
            let chip_width = canvas.gfx().text_width(chip, Style::new(12.5)) + 22.0;
            let rect = Rect::new(x, 148.0, chip_width, 26.0);
            canvas.fill_round(rect, 13.0, BOX);
            canvas.stroke_round(rect, 13.0, BOX_BORDER);
            canvas.text(chip, rect, Style::new(12.5).align(Align::Center), Color(0xFFFFFF, 0.92));
            x += chip_width + 8.0;
        }

        let key = Rect::new(left, 228.0, 34.0, 22.0);
        canvas.fill_round(key, 5.0, BOX);
        canvas.stroke_round(key, 5.0, BOX_BORDER);
        canvas.text("F12", key, Style::new(11.0).weight(600).align(Align::Center), Color(0xFFFFFF, 0.92));
        canvas.text("বাংলা চালু বা বন্ধ", Rect::new(left + 44.0, 228.0, width - 44.0, 22.0), Style::new(12.5), Color(0xFFFFFF, 0.55));

        let track = Rect::new(left, 264.0, width, 3.0);
        canvas.fill_round(track, 1.5, BOX);
        if progress > 0.0 {
            canvas.fill_round_brand(Rect::new(left, 264.0, (width * progress).max(3.0), 3.0), 1.5);
        }
        let small = Style::new(11.0);
        let faint = Color(0xFFFFFF, 0.32);
        canvas.text(&format!("Version {}", env!("CARGO_PKG_VERSION")), Rect::new(left, 278.0, width, 16.0), small, faint);
        canvas.text("© 2026 Nukta Solutions · nuktasolutions.com", Rect::new(left, 278.0, width, 16.0), small.align(Align::Right), faint);
        canvas.stroke_round(card, 16.0, Color(0xFFFFFF, 0.06));
    }

    unsafe extern "system" fn splash_procedure(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match message {
            WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
            // A click sends it off straight away.
            WM_LBUTTONUP => {
                SPLASH.with(|splash| {
                    if let Some(s) = splash.borrow_mut().as_mut() {
                        let now = s.started.elapsed().as_secs_f32() * 1000.0;
                        if s.closing_from.is_none() && now < SPLASH_OUT_AT {
                            s.closing_from = Some(now);
                        }
                    }
                });
                LRESULT(0)
            }
            WM_TIMER if wparam.0 == SPLASH_TIMER => {
                if !draw_splash() {
                    unsafe {
                        let _ = KillTimer(Some(window), SPLASH_TIMER);
                        let _ = DestroyWindow(window);
                    }
                    splash_finished();
                }
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
        }
    }

    // MARK: Messages

    fn point_of(lparam: LPARAM) -> POINT {
        POINT { x: (lparam.0 & 0xFFFF) as i16 as i32, y: ((lparam.0 >> 16) & 0xFFFF) as i16 as i32 }
    }

    unsafe extern "system" fn bar_procedure(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let taskbar_created = with_app(|app| app.taskbar_created).unwrap_or(0);
        if message == taskbar_created && taskbar_created != 0 {
            update_tray_icon(NIM_ADD);
            return LRESULT(0);
        }
        match message {
            // A click must not take the keyboard from the app being typed in.
            WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
            WM_LBUTTONDOWN => {
                if part_at(lparam) == Some(Part::Logo) {
                    // The logo: drag to move the bar, or click to toggle.
                    let mut cursor = POINT::default();
                    let mut frame = RECT::default();
                    unsafe {
                        let _ = GetCursorPos(&mut cursor);
                        let _ = GetWindowRect(window, &mut frame);
                        SetCapture(window);
                    }
                    with_app(|app| {
                        app.drag = Some(Drag { cursor, window: POINT { x: frame.left, y: frame.top }, moved: false })
                    });
                }
                LRESULT(0)
            }
            WM_MOUSEMOVE => {
                let dragging = with_app(|app| app.drag.as_ref().map(|d| (d.cursor, d.window))).flatten();
                if let Some((start, origin)) = dragging {
                    let mut cursor = POINT::default();
                    unsafe {
                        let _ = GetCursorPos(&mut cursor);
                    }
                    let (dx, dy) = (cursor.x - start.x, cursor.y - start.y);
                    let moved = with_app(|app| {
                        let drag = app.drag.as_mut()?;
                        if dx.abs() > 3 || dy.abs() > 3 {
                            drag.moved = true;
                        }
                        Some(drag.moved)
                    })
                    .flatten()
                    .unwrap_or(false);
                    if moved {
                        unsafe {
                            let _ = SetWindowPos(
                                window,
                                None,
                                origin.x + dx,
                                origin.y + dy,
                                0,
                                0,
                                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                            );
                        }
                    }
                    return LRESULT(0);
                }
                set_hover(part_at(lparam).filter(|&p| p != Part::Logo));
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
            WM_LBUTTONUP => {
                if let Some(part) = with_app(|app| app.swallow_up.take()).flatten() {
                    if part_at(lparam) == Some(part) {
                        set_hover(None);
                        return LRESULT(0);
                    }
                }
                if let Some(drag) = with_app(|app| app.drag.take()).flatten() {
                    unsafe {
                        let _ = ReleaseCapture();
                    }
                    if drag.moved {
                        save_position();
                    } else {
                        toggle();
                    }
                    return LRESULT(0);
                }
                match part_at(lparam) {
                    Some(Part::Switch) => menu_under(Part::Switch, layout_menu(), false),
                    Some(Part::Settings) => open_settings(None),
                    Some(Part::Help) => menu_under(Part::Help, help_menu(), true),
                    Some(Part::ToTray) => show_bar(false),
                    Some(Part::Close) => menu_under(Part::Close, close_menu(), true),
                    _ => {}
                }
                set_hover(None);
                LRESULT(0)
            }
            WM_CAPTURECHANGED => {
                with_app(|app| app.drag = None);
                LRESULT(0)
            }
            WM_RBUTTONUP => {
                tray_menu_at_cursor();
                LRESULT(0)
            }
            WM_TRAY => {
                match lparam.0 as u32 {
                    WM_LBUTTONUP => {
                        let ignore = with_app(|app| std::mem::take(&mut app.ignore_tray_up)).unwrap_or(false);
                        if !ignore {
                            unsafe {
                                SetTimer(Some(window), CLICK_TIMER, GetDoubleClickTime(), None);
                            }
                        }
                    }
                    WM_LBUTTONDBLCLK => {
                        unsafe {
                            let _ = KillTimer(Some(window), CLICK_TIMER);
                        }
                        with_app(|app| app.ignore_tray_up = true);
                        restore_bar();
                    }
                    WM_RBUTTONUP | WM_CONTEXTMENU => tray_menu_at_cursor(),
                    _ => {}
                }
                LRESULT(0)
            }
            WM_SHOW_BAR => {
                show_bar(true);
                LRESULT(0)
            }
            WM_TIMER if wparam.0 == CLICK_TIMER => {
                unsafe {
                    let _ = KillTimer(Some(window), CLICK_TIMER);
                }
                toggle();
                LRESULT(0)
            }
            WM_TIMER if wparam.0 == PROMOTE_TIMER => {
                // Windows writes the icon's entry a moment after it is added: try a few times.
                let done = promote_tray_icon();
                let tries = with_app(|app| {
                    app.promote_tries += 1;
                    app.promote_tries
                })
                .unwrap_or(10);
                if done || tries >= 10 {
                    unsafe {
                        let _ = KillTimer(Some(window), PROMOTE_TIMER);
                    }
                    if done {
                        with_app(|app| app.settings.set_tray_promoted());
                    }
                }
                LRESULT(0)
            }
            WM_TIMER => {
                // The mode or the bar's visibility changed elsewhere (F12, the settings window).
                let changed = with_app(|app| app.settings.reload_if_changed()).unwrap_or(false);
                if changed {
                    mode_changed();
                    let visible = with_app(|app| app.settings.bar_visible()).unwrap_or(true);
                    unsafe {
                        if IsWindowVisible(window).as_bool() != visible {
                            let _ = ShowWindow(window, if visible { SW_SHOWNOACTIVATE } else { SW_HIDE });
                        }
                    }
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                remove_tray_icon();
                unsafe { PostQuitMessage(0) };
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
        }
    }
}
