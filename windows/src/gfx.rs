//! Direct2D drawing into layered windows, shared by the top bar, its menus, the settings window, the
//! About box and the splash screen: shapes, SVG paths (the logo, Lucide icons), gradients and text.
//!
//! Text is DirectWrite with the brand's type: Bangla in Noto Serif Bengali (bundled with the app,
//! `fonts/NotoSerifBengali.ttf`, loaded privately: nothing is installed in Windows), everything else
//! in Aptos (Segoe UI where Aptos isn't installed). A font fallback does the split per character, as
//! the design previews do with CSS `unicode-range`.

use std::path::Path;

use windows::core::{w, Interface, Result, BOOL, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, POINT, RECT, SIZE};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_BEZIER_SEGMENT, D2D1_COLOR_F, D2D1_FIGURE_BEGIN_FILLED, D2D1_FIGURE_BEGIN_HOLLOW,
    D2D1_FIGURE_END_CLOSED, D2D1_FIGURE_END_OPEN, D2D1_FILL_MODE_WINDING, D2D1_GRADIENT_STOP, D2D1_PIXEL_FORMAT,
    D2D_RECT_F, D2D_SIZE_F,
};
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject, AC_SRC_ALPHA,
    AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
};
use windows::Win32::UI::WindowsAndMessaging::{UpdateLayeredWindow, ULW_ALPHA};
use windows_numerics::{Matrix3x2, Vector2};

/// The Bangla font's family name, as the bundled file names it.
pub const BANGLA_FAMILY: &str = "Noto Serif Bengali";

pub struct Gfx {
    d2d: ID2D1Factory,
    write: IDWriteFactory,
    /// Aptos where installed (Windows 11 with recent Office), else Segoe UI: the Latin text.
    family: Vec<u16>,
    /// Bangla → Noto Serif Bengali (when its file was found), then Windows' own fallback.
    fallback: Option<IDWriteFontFallback>,
    stroke: ID2D1StrokeStyle,
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Rect {
    pub fn new(left: f32, top: f32, width: f32, height: f32) -> Rect {
        Rect { left, top, right: left + width, bottom: top + height }
    }

    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    pub fn height(&self) -> f32 {
        self.bottom - self.top
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }

    pub fn inset(&self, by: f32) -> Rect {
        Rect { left: self.left + by, top: self.top + by, right: self.right - by, bottom: self.bottom - by }
    }

    fn d2d(&self) -> D2D_RECT_F {
        D2D_RECT_F { left: self.left, top: self.top, right: self.right, bottom: self.bottom }
    }
}

/// A colour as 0xRRGGBB and an opacity.
#[derive(Clone, Copy, Debug)]
pub struct Color(pub u32, pub f32);

impl Color {
    fn d2d(self) -> D2D1_COLOR_F {
        D2D1_COLOR_F {
            r: ((self.0 >> 16) & 0xFF) as f32 / 255.0,
            g: ((self.0 >> 8) & 0xFF) as f32 / 255.0,
            b: (self.0 & 0xFF) as f32 / 255.0,
            a: self.1,
        }
    }
}

/// The brand gradient's ends (the logo: `#D13A0E` → `#E87A1E`).
pub const BRAND_FROM: Color = Color(0xD13A0E, 1.0);
pub const BRAND_TO: Color = Color(0xE87A1E, 1.0);

/// An icon: SVG path data in a square `view` (24 for Lucide), drawn as 1px lines.
pub struct Icon {
    pub view: f32,
    pub paths: &'static [&'static str],
}

/// How text sits in its rectangle.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// Text options beyond the size and colour.
#[derive(Clone, Copy)]
pub struct Style {
    pub size: f32,
    pub weight: u32,
    pub align: Align,
    /// Wrap onto more lines (top-aligned); otherwise one line, centred vertically.
    pub wrap: bool,
}

impl Style {
    pub const fn new(size: f32) -> Style {
        Style { size, weight: 400, align: Align::Left, wrap: false }
    }

    pub const fn weight(self, weight: u32) -> Style {
        Style { weight, ..self }
    }

    pub const fn align(self, align: Align) -> Style {
        Style { align, ..self }
    }

    pub const fn wrap(self) -> Style {
        Style { wrap: true, ..self }
    }
}

impl Gfx {
    /// `bangla_font`: the bundled Noto Serif Bengali, if found (an installed copy has it in `fonts\`
    /// beside the programs); without it, Bangla falls back to Windows' own (Nirmala UI).
    pub fn new(bangla_font: Option<&Path>) -> Result<Gfx> {
        unsafe {
            let d2d: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let write: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let mut family = wide("Segoe UI");
            let mut collection: Option<IDWriteFontCollection> = None;
            if write.GetSystemFontCollection(&mut collection, false).is_ok() {
                if let Some(collection) = collection {
                    let aptos = wide("Aptos");
                    let (mut index, mut exists) = (0u32, BOOL(0));
                    if collection.FindFamilyName(PCWSTR(aptos.as_ptr()), &mut index, &mut exists).is_ok() && exists.as_bool() {
                        family = aptos;
                    }
                }
            }
            let fallback = bangla_font.and_then(|path| bangla_fallback(&write, path).ok());
            let stroke = d2d.CreateStrokeStyle(
                &D2D1_STROKE_STYLE_PROPERTIES {
                    startCap: D2D1_CAP_STYLE_ROUND,
                    endCap: D2D1_CAP_STYLE_ROUND,
                    dashCap: D2D1_CAP_STYLE_ROUND,
                    lineJoin: D2D1_LINE_JOIN_ROUND,
                    miterLimit: 10.0,
                    dashStyle: D2D1_DASH_STYLE_SOLID,
                    dashOffset: 0.0,
                },
                None,
            )?;
            Ok(Gfx { d2d, write, family, fallback, stroke })
        }
    }

    /// Draws a layered window `width` × `height` DIPs at `dpi`, with per-pixel alpha, the whole of
    /// it at `opacity` (0…255; fades).
    pub fn present(&self, window: HWND, width: f32, height: f32, dpi: f32, opacity: u8, draw: impl FnOnce(&Canvas)) {
        let px = |v: f32| (v * dpi / 96.0).ceil() as i32;
        let (w, h) = (px(width), px(height));
        unsafe {
            let screen = GetDC(None);
            let memory = CreateCompatibleDC(Some(screen));
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
            if let Ok(bitmap) = CreateDIBSection(Some(memory), &info, DIB_RGB_COLORS, &mut bits, None, 0) {
                let old = SelectObject(memory, bitmap.into());
                let properties = D2D1_RENDER_TARGET_PROPERTIES {
                    r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                    pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED },
                    dpiX: dpi,
                    dpiY: dpi,
                    usage: D2D1_RENDER_TARGET_USAGE_NONE,
                    minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
                };
                if let Ok(target) = self.d2d.CreateDCRenderTarget(&properties) {
                    if target.BindDC(memory, &RECT { left: 0, top: 0, right: w, bottom: h }).is_ok() {
                        target.BeginDraw();
                        target.Clear(Some(&Color(0, 0.0).d2d()));
                        target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
                        if let Ok(target) = target.cast::<ID2D1RenderTarget>() {
                            draw(&Canvas { gfx: self, target });
                        }
                        let _ = target.EndDraw(None, None);
                    }
                }
                let blend = BLENDFUNCTION {
                    BlendOp: AC_SRC_OVER as u8,
                    BlendFlags: 0,
                    SourceConstantAlpha: opacity,
                    AlphaFormat: AC_SRC_ALPHA as u8,
                };
                let _ = UpdateLayeredWindow(
                    window,
                    Some(screen),
                    None,
                    Some(&SIZE { cx: w, cy: h }),
                    Some(memory),
                    Some(&POINT { x: 0, y: 0 }),
                    COLORREF(0),
                    Some(&blend),
                    ULW_ALPHA,
                );
                SelectObject(memory, old);
                let _ = DeleteObject(bitmap.into());
            }
            let _ = DeleteDC(memory);
            ReleaseDC(None, screen);
        }
    }

    /// Draws an ordinary (opaque) window's client area into `dc`, from `WM_PAINT`: `width` ×
    /// `height` pixels at `dpi`.
    pub fn paint(&self, dc: windows::Win32::Graphics::Gdi::HDC, width: i32, height: i32, dpi: f32, draw: impl FnOnce(&Canvas)) {
        unsafe {
            let properties = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: windows::Win32::Graphics::Direct2D::Common::D2D1_ALPHA_MODE_IGNORE,
                },
                dpiX: dpi,
                dpiY: dpi,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };
            let Ok(target) = self.d2d.CreateDCRenderTarget(&properties) else { return };
            if target.BindDC(dc, &RECT { left: 0, top: 0, right: width, bottom: height }).is_err() {
                return;
            }
            target.BeginDraw();
            target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_CLEARTYPE);
            if let Ok(target) = target.cast::<ID2D1RenderTarget>() {
                draw(&Canvas { gfx: self, target });
            }
            let _ = target.EndDraw(None, None);
        }
    }

    /// How wide `text` is in `style`, in DIPs (one line).
    pub fn text_width(&self, text: &str, style: Style) -> f32 {
        self.text_size(text, Style { wrap: false, ..style }, 4000.0).0
    }

    /// How tall `text` is in `style` when wrapped to `width`, in DIPs.
    pub fn text_height(&self, text: &str, style: Style, width: f32) -> f32 {
        self.text_size(text, Style { wrap: true, ..style }, width).1
    }

    fn text_size(&self, text: &str, style: Style, width: f32) -> (f32, f32) {
        unsafe {
            let Ok(format) = self.format(style) else { return (0.0, 0.0) };
            let text: Vec<u16> = text.encode_utf16().collect();
            let Ok(layout) = self.write.CreateTextLayout(&text, &format, width, 4000.0) else { return (0.0, 0.0) };
            let mut metrics = DWRITE_TEXT_METRICS::default();
            if layout.GetMetrics(&mut metrics).is_ok() {
                (metrics.widthIncludingTrailingWhitespace, metrics.height)
            } else {
                (0.0, 0.0)
            }
        }
    }

    fn format(&self, style: Style) -> Result<IDWriteTextFormat> {
        unsafe {
            let format = self.write.CreateTextFormat(
                PCWSTR(self.family.as_ptr()),
                None,
                DWRITE_FONT_WEIGHT(style.weight as i32),
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                style.size,
                w!("bn-bd"),
            )?;
            format.SetTextAlignment(match style.align {
                Align::Left => DWRITE_TEXT_ALIGNMENT_LEADING,
                Align::Center => DWRITE_TEXT_ALIGNMENT_CENTER,
                Align::Right => DWRITE_TEXT_ALIGNMENT_TRAILING,
            })?;
            if style.wrap {
                format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_NEAR)?;
                format.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP)?;
            } else {
                format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
                format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
            }
            if let (Some(fallback), Ok(format1)) = (&self.fallback, format.cast::<IDWriteTextFormat1>()) {
                format1.SetFontFallback(fallback)?;
            }
            Ok(format)
        }
    }

    /// SVG path data as a Direct2D geometry. Handles M L H V C S Q A Z, absolute and relative.
    fn geometry(&self, data: &str, fill: bool) -> Option<ID2D1PathGeometry> {
        unsafe {
            let geometry = self.d2d.CreatePathGeometry().ok()?;
            let sink = geometry.Open().ok()?;
            sink.SetFillMode(D2D1_FILL_MODE_WINDING);
            trace(&sink, data, fill);
            sink.Close().ok()?;
            Some(geometry)
        }
    }
}

/// A fallback that sends Bangla to the bundled Noto Serif Bengali and everything else on to
/// Windows' own fallback.
fn bangla_fallback(write: &IDWriteFactory, path: &Path) -> Result<IDWriteFontFallback> {
    unsafe {
        let factory5: IDWriteFactory5 = write.cast()?;
        let file_path = wide(&path.to_string_lossy());
        let file = factory5.CreateFontFileReference(PCWSTR(file_path.as_ptr()), None)?;
        let builder = factory5.CreateFontSetBuilder()?;
        builder.AddFontFile(&file)?;
        let set = builder.CreateFontSet()?;
        let collection = factory5.CreateFontCollectionFromFontSet(&set)?;

        let factory2: IDWriteFactory2 = write.cast()?;
        let builder = factory2.CreateFontFallbackBuilder()?;
        let ranges = [
            DWRITE_UNICODE_RANGE { first: 0x0980, last: 0x09FF }, // Bengali
            DWRITE_UNICODE_RANGE { first: 0x0964, last: 0x0965 }, // । ॥
            DWRITE_UNICODE_RANGE { first: 0x200C, last: 0x200D }, // ZWNJ, ZWJ
            DWRITE_UNICODE_RANGE { first: 0x25CC, last: 0x25CC }, // ◌
        ];
        let family = wide(BANGLA_FAMILY);
        builder.AddMapping(&ranges, &[family.as_ptr()], &collection, PCWSTR::null(), PCWSTR::null(), 1.0)?;
        builder.AddMappings(&factory2.GetSystemFontFallback()?)?;
        builder.CreateFontFallback()
    }
}

pub struct Canvas<'a> {
    gfx: &'a Gfx,
    target: ID2D1RenderTarget,
}

impl Canvas<'_> {
    pub fn gfx(&self) -> &Gfx {
        self.gfx
    }

    fn brush(&self, color: Color) -> Option<ID2D1SolidColorBrush> {
        unsafe { self.target.CreateSolidColorBrush(&color.d2d(), None).ok() }
    }

    /// The brand gradient across `rect` (bottom left → top right, as in the logo).
    fn brand_brush(&self, rect: Rect) -> Option<ID2D1LinearGradientBrush> {
        unsafe {
            let stops = [
                D2D1_GRADIENT_STOP { position: 0.0, color: BRAND_FROM.d2d() },
                D2D1_GRADIENT_STOP { position: 1.0, color: BRAND_TO.d2d() },
            ];
            let collection = self.target.CreateGradientStopCollection(&stops, D2D1_GAMMA_2_2, D2D1_EXTEND_MODE_CLAMP).ok()?;
            self.target
                .CreateLinearGradientBrush(
                    &D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES {
                        startPoint: Vector2 { X: rect.left, Y: rect.bottom },
                        endPoint: Vector2 { X: rect.right, Y: rect.top },
                    },
                    None,
                    &collection,
                )
                .ok()
        }
    }

    pub fn fill_rect(&self, rect: Rect, color: Color) {
        let Some(brush) = self.brush(color) else { return };
        unsafe { self.target.FillRectangle(&rect.d2d(), &brush) };
    }

    pub fn fill_round(&self, rect: Rect, radius: f32, color: Color) {
        let Some(brush) = self.brush(color) else { return };
        unsafe {
            self.target
                .FillRoundedRectangle(&D2D1_ROUNDED_RECT { rect: rect.d2d(), radiusX: radius, radiusY: radius }, &brush);
        }
    }

    /// A rounded rectangle in the brand gradient (selected segments, switches that are on).
    pub fn fill_round_brand(&self, rect: Rect, radius: f32) {
        let Some(brush) = self.brand_brush(rect) else { return };
        unsafe {
            self.target
                .FillRoundedRectangle(&D2D1_ROUNDED_RECT { rect: rect.d2d(), radiusX: radius, radiusY: radius }, &brush);
        }
    }

    /// A 1px outline just inside `rect`.
    pub fn stroke_round(&self, rect: Rect, radius: f32, color: Color) {
        let Some(brush) = self.brush(color) else { return };
        let rect = rect.inset(0.5);
        unsafe {
            self.target.DrawRoundedRectangle(
                &D2D1_ROUNDED_RECT { rect: rect.d2d(), radiusX: (radius - 0.5).max(0.0), radiusY: (radius - 0.5).max(0.0) },
                &brush,
                1.0,
                None,
            );
        }
    }

    /// A circle; `outline` > 0 draws only its edge, that wide.
    pub fn circle(&self, x: f32, y: f32, radius: f32, color: Color, outline: f32) {
        let Some(brush) = self.brush(color) else { return };
        let ellipse = D2D1_ELLIPSE { point: Vector2 { X: x, Y: y }, radiusX: radius, radiusY: radius };
        unsafe {
            if outline > 0.0 {
                self.target.DrawEllipse(&ellipse, &brush, outline, None);
            } else {
                self.target.FillEllipse(&ellipse, &brush);
            }
        }
    }

    /// A soft round glow of `color`, fading out at `radius`.
    pub fn glow(&self, x: f32, y: f32, radius: f32, color: Color) {
        unsafe {
            let stops = [
                D2D1_GRADIENT_STOP { position: 0.0, color: color.d2d() },
                D2D1_GRADIENT_STOP { position: 0.6, color: Color(color.0, color.1 * 0.25).d2d() },
                D2D1_GRADIENT_STOP { position: 1.0, color: Color(color.0, 0.0).d2d() },
            ];
            let Ok(collection) = self.target.CreateGradientStopCollection(&stops, D2D1_GAMMA_2_2, D2D1_EXTEND_MODE_CLAMP) else {
                return;
            };
            let Ok(brush) = self.target.CreateRadialGradientBrush(
                &D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES {
                    center: Vector2 { X: x, Y: y },
                    gradientOriginOffset: Vector2 { X: 0.0, Y: 0.0 },
                    radiusX: radius,
                    radiusY: radius,
                },
                None,
                &collection,
            ) else {
                return;
            };
            self.target.FillEllipse(
                &D2D1_ELLIPSE { point: Vector2 { X: x, Y: y }, radiusX: radius, radiusY: radius },
                &brush,
            );
        }
    }

    /// Draws `draw` rotated by `degrees` about (x, y), clipped to `clip`.
    pub fn rotated(&self, degrees: f32, x: f32, y: f32, clip: Rect, draw: impl FnOnce(&Self)) {
        let (sin, cos) = degrees.to_radians().sin_cos();
        unsafe {
            self.target.PushAxisAlignedClip(&clip.d2d(), D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
            self.target.SetTransform(&Matrix3x2 {
                M11: cos,
                M12: sin,
                M21: -sin,
                M22: cos,
                M31: x - x * cos + y * sin,
                M32: y - x * sin - y * cos,
            });
            draw(self);
            self.target.SetTransform(&Matrix3x2::identity());
            self.target.PopAxisAlignedClip();
        }
    }

    pub fn text(&self, text: &str, rect: Rect, style: Style, color: Color) {
        let (Ok(format), Some(brush)) = (self.gfx.format(style), self.brush(color)) else { return };
        let text: Vec<u16> = text.encode_utf16().collect();
        unsafe {
            self.target.DrawText(
                &text,
                &format,
                &rect.d2d(),
                &brush,
                if style.wrap { D2D1_DRAW_TEXT_OPTIONS_NONE } else { D2D1_DRAW_TEXT_OPTIONS_CLIP },
                DWRITE_MEASURING_MODE_NATURAL,
            );
        }
    }

    /// An icon `size` DIPs square at (x, y), in lines `width` DIPs wide (1 in the designs).
    pub fn icon(&self, icon: &Icon, x: f32, y: f32, size: f32, color: Color) {
        self.icon_stroke(icon, x, y, size, color, 1.0);
    }

    pub fn icon_stroke(&self, icon: &Icon, x: f32, y: f32, size: f32, color: Color, width: f32) {
        let Some(brush) = self.brush(color) else { return };
        let scale = size / icon.view;
        unsafe {
            self.target.SetTransform(&Matrix3x2 { M11: scale, M12: 0.0, M21: 0.0, M22: scale, M31: x, M32: y });
            for data in icon.paths {
                // Lucide draws a dot (ⓘ, the keyboard's keys) as a line 0.01 long with round ends;
                // Direct2D draws nothing that short, so it is a filled circle here.
                if let Some((cx, cy)) = dot(data) {
                    self.target.FillEllipse(
                        &D2D1_ELLIPSE { point: Vector2 { X: cx, Y: cy }, radiusX: width / scale, radiusY: width / scale },
                        &brush,
                    );
                } else if let Some(geometry) = self.gfx.geometry(data, false) {
                    self.target.DrawGeometry(&geometry, &brush, width / scale, &self.gfx.stroke);
                }
            }
            self.target.SetTransform(&Matrix3x2::identity());
        }
    }

    /// The নু logo (nu-icon.svg, 37 × 28) at (x, y), `width` DIPs wide.
    pub fn logo(&self, x: f32, y: f32, width: f32) {
        let scale = width / 37.0;
        unsafe {
            self.target.SetTransform(&Matrix3x2 { M11: scale, M12: 0.0, M21: 0.0, M22: scale, M31: x, M32: y });
            let stops = [
                D2D1_GRADIENT_STOP { position: 0.0, color: BRAND_FROM.d2d() },
                D2D1_GRADIENT_STOP { position: 1.0, color: BRAND_TO.d2d() },
            ];
            if let Ok(collection) = self.target.CreateGradientStopCollection(&stops, D2D1_GAMMA_2_2, D2D1_EXTEND_MODE_CLAMP) {
                if let Ok(gradient) = self.target.CreateLinearGradientBrush(
                    &D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES {
                        startPoint: Vector2 { X: 1.761, Y: 20.8 },
                        endPoint: Vector2 { X: 35.665, Y: 3.782 },
                    },
                    None,
                    &collection,
                ) {
                    self.target.FillRoundedRectangle(
                        &D2D1_ROUNDED_RECT {
                            rect: D2D_RECT_F { left: 0.0, top: 0.0, right: 37.0, bottom: 28.0 },
                            radiusX: 6.0,
                            radiusY: 6.0,
                        },
                        &gradient,
                    );
                }
            }
            if let (Some(glyph), Some(white)) = (self.gfx.geometry(LOGO_GLYPH, true), self.brush(Color(0xFFFFFF, 0.9))) {
                self.target.SetTransform(&Matrix3x2 {
                    M11: scale,
                    M12: 0.0,
                    M21: 0.0,
                    M22: scale,
                    M31: x + 10.0 * scale,
                    M32: y + 4.989 * scale,
                });
                self.target.FillGeometry(&glyph, &white, None);
                self.target.SetTransform(&Matrix3x2 { M11: scale, M12: 0.0, M21: 0.0, M22: scale, M31: x, M32: y });
                self.target.FillEllipse(
                    &D2D1_ELLIPSE { point: Vector2 { X: 25.5, Y: 17.489 }, radiusX: 1.5, radiusY: 1.5 },
                    &white,
                );
            }
            self.target.SetTransform(&Matrix3x2::identity());
        }
    }
}

/// The নু glyph from nu-icon.svg, drawn at (10, 4.989) in the logo.
const LOGO_GLYPH: &str = "M10.541 13.665L9.372 12.773C9.33 12.277 9.245 11.749 9.117 11.189C8.99 10.63 8.82 10.084 8.607 9.553C8.395 9.022 8.147 8.543 7.863 8.118C7.58 7.693 7.254 7.35 6.886 7.088C6.517 6.826 6.121 6.694 5.696 6.694C5.54 6.694 5.38 6.712 5.217 6.748C5.055 6.783 4.916 6.836 4.803 6.907L5.419 5.802C5.632 5.944 5.788 6.142 5.887 6.397C5.986 6.652 6.036 6.907 6.036 7.162C6.036 7.559 5.951 7.916 5.781 8.235C5.611 8.554 5.366 8.806 5.047 8.99C4.729 9.174 4.328 9.266 3.847 9.266C3.039 9.266 2.423 9.025 1.998 8.543C1.573 8.062 1.36 7.452 1.36 6.716C1.36 5.894 1.622 5.256 2.146 4.803C2.671 4.35 3.351 4.123 4.187 4.123C4.81 4.123 5.387 4.233 5.919 4.452C6.45 4.672 6.921 4.977 7.332 5.366C7.743 5.756 8.104 6.202 8.416 6.705C8.728 7.208 8.983 7.739 9.181 8.299C9.379 8.859 9.514 9.415 9.585 9.967L9.287 9.989C9.273 9.705 9.248 9.326 9.213 8.852C9.177 8.377 9.146 7.87 9.117 7.332C9.089 6.794 9.075 6.305 9.075 5.866L9.075 1.36L0.829 1.36L0 0L12.263 0L13.091 1.36L10.69 1.36L10.69 13.644L10.541 13.665ZM4.608 14.877C4.608 14.154 4.873 13.563 5.405 13.102C5.936 12.642 6.69 12.411 7.668 12.411C8.461 12.411 9.223 12.553 9.953 12.836C10.682 13.12 11.362 13.502 11.993 13.984C12.623 14.466 13.194 15.008 13.704 15.61C14.214 16.212 14.646 16.839 15 17.491L14.192 18.022C13.64 17.186 13.027 16.463 12.354 15.854C11.681 15.245 10.934 14.77 10.112 14.43C9.29 14.09 8.391 13.92 7.413 13.92C6.917 13.92 6.559 14.002 6.34 14.165C6.12 14.328 6.01 14.551 6.01 14.834C6.01 15.117 6.117 15.348 6.329 15.525C6.542 15.702 6.846 15.79 7.243 15.79C7.625 15.79 7.965 15.702 8.263 15.525C8.561 15.348 8.798 15.061 8.975 14.664C9.152 14.267 9.241 13.736 9.241 13.07L10.686 13.772C10.686 14.437 10.558 15.043 10.303 15.589C10.048 16.134 9.673 16.57 9.177 16.896C8.681 17.221 8.072 17.384 7.349 17.384C6.556 17.384 5.9 17.151 5.383 16.683C4.866 16.216 4.608 15.613 4.608 14.877Z";

pub fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

/// The centre of a Lucide dot, path data `M<x> <y>h.01`, or None for anything else.
fn dot(data: &str) -> Option<(f32, f32)> {
    let rest = data.strip_prefix('M')?.strip_suffix("h.01")?;
    let mut numbers = rest.split(' ').map(str::parse::<f32>);
    let (x, y) = (numbers.next()?.ok()?, numbers.next()?.ok()?);
    numbers.next().is_none().then_some((x + 0.005, y))
}

// MARK: SVG path data

/// Splits path data into commands and numbers ("a2 2 0 0 1-1 1.73", ".5.5" and all).
fn tokens(data: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let chars: Vec<char> = data.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_alphabetic() && c != 'e' && c != 'E' {
            out.push(Token::Command(c));
            i += 1;
        } else if c == '-' || c == '+' || c == '.' || c.is_ascii_digit() {
            let start = i;
            i += 1;
            let mut seen_dot = c == '.';
            while i < chars.len() {
                let d = chars[i];
                if d.is_ascii_digit() {
                    i += 1;
                } else if d == '.' && !seen_dot {
                    seen_dot = true;
                    i += 1;
                } else if (d == 'e' || d == 'E') && i + 1 < chars.len() {
                    i += 1;
                    if chars[i] == '-' || chars[i] == '+' {
                        i += 1;
                    }
                } else {
                    break;
                }
            }
            let text: String = chars[start..i].iter().collect();
            out.push(Token::Number(text.parse().unwrap_or(0.0)));
        } else {
            i += 1;
        }
    }
    out
}

enum Token {
    Command(char),
    Number(f32),
}

/// Follows SVG path data into `sink`. `fill`: figures are filled shapes (the logo), else lines.
fn trace(sink: &ID2D1GeometrySink, data: &str, fill: bool) {
    let tokens = tokens(data);
    let mut i = 0;
    let mut command = 'M';
    let (mut x, mut y) = (0f32, 0f32);
    let (mut start_x, mut start_y) = (0f32, 0f32);
    let mut last_control: Option<(f32, f32)> = None;
    let mut open = false;
    let begin = if fill { D2D1_FIGURE_BEGIN_FILLED } else { D2D1_FIGURE_BEGIN_HOLLOW };
    let point = |x: f32, y: f32| Vector2 { X: x, Y: y };

    let take = |i: &mut usize, n: usize| -> Option<Vec<f32>> {
        let mut values = Vec::with_capacity(n);
        for _ in 0..n {
            match tokens.get(*i) {
                Some(Token::Number(v)) => {
                    values.push(*v);
                    *i += 1;
                }
                _ => return None,
            }
        }
        Some(values)
    };

    unsafe {
        while i < tokens.len() {
            if let Token::Command(c) = tokens[i] {
                command = c;
                i += 1;
                if c == 'Z' || c == 'z' {
                    if open {
                        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                        open = false;
                    }
                    x = start_x;
                    y = start_y;
                    last_control = None;
                    continue;
                }
            }
            let relative = command.is_ascii_lowercase();
            let (ox, oy) = if relative { (x, y) } else { (0.0, 0.0) };
            match command.to_ascii_uppercase() {
                'M' => {
                    let Some(v) = take(&mut i, 2) else { break };
                    if open {
                        sink.EndFigure(D2D1_FIGURE_END_OPEN);
                    }
                    x = ox + v[0];
                    y = oy + v[1];
                    start_x = x;
                    start_y = y;
                    sink.BeginFigure(point(x, y), begin);
                    open = true;
                    // Further pairs after a move are lines.
                    command = if relative { 'l' } else { 'L' };
                    last_control = None;
                }
                'L' => {
                    let Some(v) = take(&mut i, 2) else { break };
                    x = ox + v[0];
                    y = oy + v[1];
                    sink.AddLine(point(x, y));
                    last_control = None;
                }
                'H' => {
                    let Some(v) = take(&mut i, 1) else { break };
                    x = if relative { x + v[0] } else { v[0] };
                    sink.AddLine(point(x, y));
                    last_control = None;
                }
                'V' => {
                    let Some(v) = take(&mut i, 1) else { break };
                    y = if relative { y + v[0] } else { v[0] };
                    sink.AddLine(point(x, y));
                    last_control = None;
                }
                'C' => {
                    let Some(v) = take(&mut i, 6) else { break };
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: point(ox + v[0], oy + v[1]),
                        point2: point(ox + v[2], oy + v[3]),
                        point3: point(ox + v[4], oy + v[5]),
                    });
                    last_control = Some((ox + v[2], oy + v[3]));
                    x = ox + v[4];
                    y = oy + v[5];
                }
                'S' => {
                    let Some(v) = take(&mut i, 4) else { break };
                    let (c1x, c1y) = match last_control {
                        Some((cx, cy)) => (2.0 * x - cx, 2.0 * y - cy),
                        None => (x, y),
                    };
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: point(c1x, c1y),
                        point2: point(ox + v[0], oy + v[1]),
                        point3: point(ox + v[2], oy + v[3]),
                    });
                    last_control = Some((ox + v[0], oy + v[1]));
                    x = ox + v[2];
                    y = oy + v[3];
                }
                'Q' => {
                    let Some(v) = take(&mut i, 4) else { break };
                    // A quadratic as a cubic.
                    let (qx, qy) = (ox + v[0], oy + v[1]);
                    let (ex, ey) = (ox + v[2], oy + v[3]);
                    sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                        point1: point(x + 2.0 / 3.0 * (qx - x), y + 2.0 / 3.0 * (qy - y)),
                        point2: point(ex + 2.0 / 3.0 * (qx - ex), ey + 2.0 / 3.0 * (qy - ey)),
                        point3: point(ex, ey),
                    });
                    last_control = None;
                    x = ex;
                    y = ey;
                }
                'A' => {
                    let Some(v) = take(&mut i, 7) else { break };
                    x = ox + v[5];
                    y = oy + v[6];
                    sink.AddArc(&D2D1_ARC_SEGMENT {
                        point: point(x, y),
                        size: D2D_SIZE_F { width: v[0], height: v[1] },
                        rotationAngle: v[2],
                        sweepDirection: if v[4] != 0.0 { D2D1_SWEEP_DIRECTION_CLOCKWISE } else { D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE },
                        arcSize: if v[3] != 0.0 { D2D1_ARC_SIZE_LARGE } else { D2D1_ARC_SIZE_SMALL },
                    });
                    last_control = None;
                }
                _ => {
                    i += 1;
                }
            }
        }
        if open {
            sink.EndFigure(D2D1_FIGURE_END_OPEN);
        }
    }
}
