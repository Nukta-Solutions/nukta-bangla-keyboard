//! The input indicator in the taskbar: the counterpart of the macOS নু menu and the Linux layout
//! menu. Its icon shows the layout (বি or ফ); a click switches layout; a right-click opens the menu
//! (বিজয় লেআউট, ফোনেটিক, সেটিংস…, about).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::core::{implement, w, IUnknownImpl, Interface, BSTR, GUID, IUnknown, Ref, Result};
use windows::Win32::Foundation::{E_INVALIDARG, POINT, RECT};
use windows::Win32::System::Ole::{CONNECT_E_ADVISELIMIT, CONNECT_E_CANNOTCONNECT};
use windows::Win32::Graphics::Gdi::{
    CreateBitmap, CreateCompatibleDC, CreateDIBSection, CreateFontW, DeleteDC, DeleteObject, DrawTextW, GdiFlush,
    SelectObject, SetBkMode, SetTextColor, ANTIALIASED_QUALITY, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DEFAULT_CHARSET, DIB_RGB_COLORS, DT_CENTER, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, FF_DONTCARE,
    FONT_CLIP_PRECISION, FONT_OUTPUT_PRECISION, FW_BOLD, TRANSPARENT,
};
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows::Win32::UI::TextServices::{
    ITfLangBarItemButton, ITfLangBarItemButton_Impl, ITfLangBarItemSink, ITfLangBarItem_Impl, ITfMenu, ITfSource,
    ITfSource_Impl, TfLBIClick, GUID_LBI_INPUTMODE, TF_LANGBARITEMINFO, TF_LBI_CLK_LEFT, TF_LBI_CLK_RIGHT,
    TF_LBI_ICON, TF_LBI_STYLE_BTN_BUTTON, TF_LBI_STYLE_SHOWNINTRAY, TF_LBI_TEXT, TF_LBI_TOOLTIP,
};
use windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, GetSystemMetrics, HICON, ICONINFO, SM_CXSMICON};

use super::{DllRef, CLSID_TEXT_SERVICE};
use crate::settings::Layout;

/// What the indicator asks the text service to do.
#[derive(Clone, Copy, Debug)]
pub(super) enum Command {
    /// A click: switch to the other layout.
    Toggle,
    /// A right-click at this screen point: show the menu.
    Menu(POINT),
}

#[implement(ITfLangBarItemButton, ITfSource)]
pub(super) struct LangBarButton {
    layout: Cell<Layout>,
    sink: RefCell<Option<ITfLangBarItemSink>>,
    /// Set while the text service is active; cleared on deactivation, which breaks the reference
    /// cycle between the two.
    on_command: RefCell<Option<Callback>>,
    _dll: DllRef,
}

type Callback = Rc<dyn Fn(Command)>;

const SINK_COOKIE: u32 = 0x4E554B;

impl LangBarButton {
    pub(super) fn new(layout: Layout, on_command: impl Fn(Command) + 'static) -> Self {
        LangBarButton {
            layout: Cell::new(layout),
            sink: RefCell::new(None),
            on_command: RefCell::new(Some(Rc::new(on_command))),
            _dll: DllRef::new(),
        }
    }

    pub(super) fn disconnect(&self) {
        self.on_command.borrow_mut().take();
        self.sink.borrow_mut().take();
    }

    /// Shows `layout`, telling the taskbar to redraw if it changed.
    pub(super) fn set_layout(&self, layout: Layout) {
        if self.layout.replace(layout) == layout {
            return;
        }
        let sink = self.sink.borrow().clone();
        if let Some(sink) = sink {
            unsafe {
                let _ = sink.OnUpdate(TF_LBI_ICON | TF_LBI_TEXT | TF_LBI_TOOLTIP);
            }
        }
    }

    fn send(&self, command: Command) {
        // Called without the borrow held: the command may disconnect us.
        let callback = self.on_command.borrow().clone();
        if let Some(callback) = callback {
            callback(command);
        }
    }
}

fn layout_name(layout: Layout) -> &'static str {
    match layout {
        Layout::Bijoy => "বিজয় লেআউট",
        Layout::Phonetic => "ফোনেটিক",
    }
}

impl ITfLangBarItem_Impl for LangBarButton_Impl {
    fn GetInfo(&self, info: *mut TF_LANGBARITEMINFO) -> Result<()> {
        if info.is_null() {
            return Err(E_INVALIDARG.into());
        }
        let mut description = [0u16; 32];
        for (slot, unit) in description.iter_mut().zip("নুকতা বাংলা লেআউট".encode_utf16().take(31)) {
            *slot = unit;
        }
        unsafe {
            *info = TF_LANGBARITEMINFO {
                clsidService: CLSID_TEXT_SERVICE,
                guidItem: GUID_LBI_INPUTMODE,
                dwStyle: TF_LBI_STYLE_BTN_BUTTON | TF_LBI_STYLE_SHOWNINTRAY,
                ulSort: 0,
                szDescription: description,
            };
        }
        Ok(())
    }

    fn GetStatus(&self) -> Result<u32> {
        Ok(0)
    }

    fn Show(&self, _show: windows::core::BOOL) -> Result<()> {
        Ok(())
    }

    fn GetTooltipString(&self) -> Result<BSTR> {
        Ok(BSTR::from(format!("নুকতা বাংলা: {} (বদলাতে ক্লিক করুন)", layout_name(self.layout.get()))))
    }
}

impl ITfLangBarItemButton_Impl for LangBarButton_Impl {
    fn OnClick(&self, click: TfLBIClick, point: &POINT, _area: *const RECT) -> Result<()> {
        if click == TF_LBI_CLK_LEFT {
            self.send(Command::Toggle);
        } else if click == TF_LBI_CLK_RIGHT {
            self.send(Command::Menu(*point));
        }
        Ok(())
    }

    /// Only for menu-style buttons; this one shows its own menu on a right-click.
    fn InitMenu(&self, _menu: Ref<ITfMenu>) -> Result<()> {
        Ok(())
    }

    fn OnMenuSelect(&self, _id: u32) -> Result<()> {
        Ok(())
    }

    /// The caller owns (and destroys) the icon.
    fn GetIcon(&self) -> Result<HICON> {
        Ok(layout_icon(self.layout.get()))
    }

    fn GetText(&self) -> Result<BSTR> {
        Ok(BSTR::from(layout_name(self.layout.get())))
    }
}

impl ITfSource_Impl for LangBarButton_Impl {
    fn AdviseSink(&self, iid: *const GUID, sink: Ref<IUnknown>) -> Result<u32> {
        if iid.is_null() || unsafe { *iid } != ITfLangBarItemSink::IID {
            return Err(CONNECT_E_CANNOTCONNECT.into());
        }
        if self.sink.borrow().is_some() {
            return Err(CONNECT_E_ADVISELIMIT.into());
        }
        let sink: ITfLangBarItemSink = sink.ok()?.cast()?;
        *self.sink.borrow_mut() = Some(sink);
        Ok(SINK_COOKIE)
    }

    fn UnadviseSink(&self, cookie: u32) -> Result<()> {
        if cookie != SINK_COOKIE {
            return Err(CONNECT_E_CANNOTCONNECT.into());
        }
        self.sink.borrow_mut().take();
        Ok(())
    }
}

pub(super) fn as_button(button: &LangBarButton_Impl) -> ITfLangBarItemButton {
    button.to_interface()
}

// MARK: The icon

/// The taskbar is light (dark text wanted) when Windows says so; dark otherwise, the default.
fn taskbar_is_light() -> bool {
    let mut value = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut value as *mut u32 as *mut _),
            Some(&mut size),
        )
        .is_ok()
            && value == 1
    }
}

/// বি or ফ, drawn at the small-icon size in the taskbar's text colour, with smooth edges.
fn layout_icon(layout: Layout) -> HICON {
    let letter = match layout {
        Layout::Bijoy => "বি",
        Layout::Phonetic => "ফ",
    };
    let size = unsafe { GetSystemMetrics(SM_CXSMICON) }.max(16);
    let (red, green, blue) = if taskbar_is_light() { (0x1F, 0x1F, 0x1F) } else { (0xFF, 0xFF, 0xFF) };

    unsafe {
        let info = BITMAPINFO {
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
        };
        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let Ok(color) = CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0) else {
            return HICON::default();
        };
        let dc = CreateCompatibleDC(None);
        let old_bitmap = SelectObject(dc, color.into());
        // White text on black: the brightness of each pixel becomes its opacity.
        let font = CreateFontW(
            -(size * 15 / 16), 0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0, DEFAULT_CHARSET, FONT_OUTPUT_PRECISION(0),
            FONT_CLIP_PRECISION(0), ANTIALIASED_QUALITY, FF_DONTCARE.0 as u32, w!("Nirmala UI"),
        );
        let old_font = SelectObject(dc, font.into());
        SetBkMode(dc, TRANSPARENT);
        SetTextColor(dc, windows::Win32::Foundation::COLORREF(0x00FF_FFFF));
        let mut text: Vec<u16> = letter.encode_utf16().collect();
        let mut rect = RECT { left: 0, top: 0, right: size, bottom: size };
        DrawTextW(dc, &mut text, &mut rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        let _ = GdiFlush();

        let pixels = std::slice::from_raw_parts_mut(bits as *mut u32, (size * size) as usize);
        for pixel in pixels.iter_mut() {
            let alpha = (*pixel & 0xFF).max((*pixel >> 8) & 0xFF).max((*pixel >> 16) & 0xFF);
            // Premultiplied BGRA.
            *pixel = (alpha << 24) | ((red * alpha / 255) << 16) | ((green * alpha / 255) << 8) | (blue * alpha / 255);
        }

        SelectObject(dc, old_font);
        SelectObject(dc, old_bitmap);
        let _ = DeleteObject(font.into());
        let _ = DeleteDC(dc);

        let mask = CreateBitmap(size, size, 1, 1, None);
        let icon_info = ICONINFO { fIcon: true.into(), xHotspot: 0, yHotspot: 0, hbmMask: mask, hbmColor: color };
        let icon = CreateIconIndirect(&icon_info).unwrap_or_default();
        let _ = DeleteObject(mask.into());
        let _ = DeleteObject(color.into());
        icon
    }
}
