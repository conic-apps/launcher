// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Windows window controls over the app's custom title bar: real non-client area,
//! hit-tested as `HTMINBUTTON`/`HTMAXBUTTON`/`HTCLOSE`, so Windows supplies the
//! snap-layout flyout, the system menu and the accessibility entries.
//!
//! DWM will not paint caption buttons over a window whose caption was removed, so
//! the glyphs come from the Windows caption font and the hover fills are DWM's
//! colours. The fill is published separately so the app can animate it.
//!
//! Rust owns the geometry because the platform hit-tests against it: three
//! [`CONTROL_WIDTH`]-wide slots in the client area's top-right corner, as tall as
//! `App.title-bar-height`. This module also owns the resize border and the
//! maximized work-area geometry.

use std::cell::{Cell, RefCell};
use std::ptr::{addr_of, null_mut};
use std::slice;
use std::sync::atomic::{AtomicBool, Ordering};

use i_slint_backend_winit::WinitWindowAccessor;
use slint::{
    Color, ComponentHandle, Image, ModelRc, Rgba8Pixel, SharedPixelBuffer, VecModel, Weak,
};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DWM_WINDOW_CORNER_PREFERENCE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, ClientToScreen,
    CreateCompatibleDC, CreateDIBSection, CreateFontW, DEFAULT_CHARSET, DEFAULT_PITCH,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, FF_DONTCARE, FW_NORMAL, GetMonitorInfoW, HGDIOBJ,
    MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow, OUT_TT_PRECIS, SelectObject,
    SetBkMode, SetTextAlign, SetTextColor, TA_BOTTOM, TA_CENTER, TRANSPARENT, TextOutW,
};
use windows::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow};
use windows::Win32::UI::Input::KeyboardAndMouse::{TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent};
use windows::Win32::UI::Shell::{DefSubclassProc, SUBCLASSPROC, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetCursorPos, GetWindowRect, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCLIENT,
    HTCLOSE, HTLEFT, HTMAXBUTTON, HTMINBUTTON, HTRIGHT, HTTOPLEFT, HTTOPRIGHT, IsZoomed,
    MINMAXINFO, NCCALCSIZE_PARAMS, PostMessageW, SC_CLOSE, SC_MAXIMIZE, SC_MINIMIZE, SC_RESTORE,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_ACTIVATE, WM_DPICHANGED, WM_GETMINMAXINFO, WM_LBUTTONUP,
    WM_MOUSEMOVE, WM_NCCALCSIZE, WM_NCHITTEST, WM_NCLBUTTONDOWN, WM_NCLBUTTONUP, WM_NCMOUSELEAVE,
    WM_NCMOUSEMOVE, WM_SIZE, WM_SYSCOMMAND, WS_CAPTION, WS_OVERLAPPEDWINDOW,
};
use windows::core::w;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::slint_backend::{App, WindowControl};

/// `SetWindowSubclass` keys the chain on this; it must be unique per window and
/// procedure.
const SUBCLASS_ID: usize = 0x00C0_011C;

/// Logical width of one control, matching Windows' 46px caption button at 96 DPI.
/// The height comes from the app's title bar.
const CONTROL_WIDTH: f64 = 46.0;

const CONTROL_COUNT: usize = 3;

/// `HTMINBUTTON` / `HTMAXBUTTON` / `HTCLOSE`, by index.
const CONTROL_HIT_TESTS: [u32; CONTROL_COUNT] = [HTMINBUTTON, HTMAXBUTTON, HTCLOSE];

const CLOSE: usize = 2;

/// Private-use glyphs Windows uses for its caption buttons.
const GLYPH_MINIMIZE: u16 = 0xE921;
const GLYPH_MAXIMIZE: u16 = 0xE922;
const GLYPH_RESTORE: u16 = 0xE923;
const GLYPH_CLOSE: u16 = 0xE8BB;

/// Windows 11 renamed the caption font; older builds only have the old name.
const GLYPH_FONTS: [windows::core::PCWSTR; 2] = [w!("Segoe Fluent Icons"), w!("Segoe MDL2 Assets")];

/// Point size of the glyphs. Where they sit is measured from the ink instead.
const GLYPH_POINTS: i32 = 8;

/// Caption ink over the app's title bar on hover and press.
const SUBTLE_HOVER: u8 = 26; // 10% of 255
const SUBTLE_PRESSED: u8 = 51; // 20% of 255

/// DWM's own close-button reds.
const CLOSE_HOVER: (u8, u8, u8) = (0xC4, 0x2B, 0x1C);
const CLOSE_PRESSED: (u8, u8, u8) = (0xB0, 0x25, 0x18);

const CLOSE_INK: (u8, u8, u8) = (255, 255, 255);

fn fill(color: (u8, u8, u8), alpha: u8) -> Color {
    Color::from_argb_u8(alpha, color.0, color.1, color.2)
}

/// The alpha that reaches `pressed` when the layer is stacked on `hover`.
fn press_increment(hover: u8, pressed: u8) -> u8 {
    if hover == 255 {
        return pressed;
    }
    let (from, to) = (f32::from(hover) / 255.0, f32::from(pressed) / 255.0);
    (((to - from) / (1.0 - from)) * 255.0).round() as u8
}

pub fn controls_inset() -> f32 {
    (CONTROL_WIDTH * CONTROL_COUNT as f64) as f32
}

/// Control geometry in physical screen pixels, anchored to the client area, not
/// the window frame: a maximized window's client area is the work area.
#[derive(Clone, Copy)]
struct Controls {
    left: i32,
    top: i32,
    unit: i32,
    height: i32,
}

impl Controls {
    fn width(&self) -> i32 {
        self.unit * CONTROL_COUNT as i32
    }
}

/// State the window procedure keeps for the window it is attached to.
#[derive(Clone, Copy)]
struct Frame {
    hwnd: HWND,
    /// Title-bar height in logical units.
    height: f64,
    /// Scale the artwork was last rendered for.
    scale: f64,
    maximized: bool,
    dark: bool,
    hovered: i32,
    pressed: i32,
}

thread_local! {
    // The window procedure runs on the event-loop thread, so no locking.
    static APP: RefCell<Option<Weak<App>>> = const { RefCell::new(None) };
    static FRAME: RefCell<Option<Frame>> = const { RefCell::new(None) };
    /// Remembered for a window that does not exist yet: the theme settles while
    /// the event loop is starting up.
    static DARK: Cell<bool> = const { Cell::new(true) };
}

fn frame() -> Option<Frame> {
    FRAME.with(|slot| *slot.borrow())
}

fn frame_of(hwnd: HWND) -> Option<Frame> {
    frame().filter(|frame| frame.hwnd == hwnd)
}

fn update_frame(hwnd: HWND, change: impl FnOnce(&mut Frame)) {
    FRAME.with(|slot| {
        if let Some(frame) = slot.borrow_mut().as_mut()
            && frame.hwnd == hwnd
        {
            change(frame);
        }
    });
}

fn set_pointer(hwnd: HWND, hovered: i32, pressed: i32) {
    update_frame(hwnd, |frame| {
        frame.hovered = hovered;
        frame.pressed = pressed;
    });
    publish_state(hwnd);
}

fn publish_state(hwnd: HWND) {
    let Some(frame) = frame_of(hwnd) else {
        return;
    };
    if let Some(ui) = app() {
        ui.set_window_control_hovered(frame.hovered);
        ui.set_window_control_pressed(frame.pressed);
    }
}

/// Takes the window frame over once winit has created the window.
pub fn install(ui: &App) {
    APP.with(|slot| *slot.borrow_mut() = Some(ui.as_weak()));

    // There is no `HWND` until the first winit event; the shared filter keeps
    // this watcher alive alongside the others.
    let weak = ui.as_weak();
    let attached = AtomicBool::new(false);
    window::on_window_event(ui, move |_| {
        if !attached.swap(true, Ordering::Relaxed)
            && let Some(ui) = weak.upgrade()
        {
            attach(&ui);
        }
    });
}

/// Redraws the controls in the app's palette (`false` for Latte).
pub fn set_dark(window: &slint::Window, dark: bool) {
    DARK.with(|slot| slot.set(dark));
    if let Some(hwnd) = hwnd_of(window) {
        render_and_publish(hwnd);
    }
}

fn attach(ui: &App) {
    let Some(hwnd) = hwnd_of(ui.window()) else {
        // Without a `Frame`, the controls stop being hit-testable.
        log::warn!(
            target: "shell",
            "windows caption: no window handle yet, so the caption controls will not be \
             wired up"
        );
        return;
    };

    // SAFETY: `hwnd` is the app's own top-level window, on the thread that owns
    // it, and the call is a documented Win32 entry point.
    let procedure: SUBCLASSPROC = Some(subclass as _);
    // SAFETY: as above.
    unsafe {
        if !SetWindowSubclass(hwnd, procedure, SUBCLASS_ID, 0).as_bool() {
            log::error!(target: "shell", "windows caption: could not subclass the window");
            return;
        }
        round_corners(hwnd);
    }

    FRAME.with(|slot| {
        *slot.borrow_mut() = Some(Frame {
            hwnd,
            height: f64::from(ui.get_title_bar_height()),
            scale: 0.0,
            maximized: is_zoomed(hwnd),
            dark: DARK.with(|dark| dark.get()),
            hovered: -1,
            pressed: -1,
        });
    });

    render_and_publish(hwnd);
    log::debug!(target: "shell", "windows caption: window controls installed");
}

fn app() -> Option<App> {
    APP.with(|slot| slot.borrow().as_ref().and_then(Weak::upgrade))
}

fn controls_of(hwnd: HWND) -> Option<Controls> {
    let frame = frame_of(hwnd)?;
    // SAFETY: the window is live and this runs on the thread that owns it; both
    // rectangles are locals the APIs only write to.
    let scale = unsafe { f64::from(GetDpiForWindow(hwnd)) } / 96.0;
    let unit = (CONTROL_WIDTH * scale) as i32;
    let mut client = RECT::default();
    let mut origin = POINT { x: 0, y: 0 };
    unsafe {
        GetClientRect(hwnd, &mut client).ok()?;
        let _ = ClientToScreen(hwnd, &mut origin);
    }
    Some(Controls {
        left: origin.x + client.right - unit * CONTROL_COUNT as i32,
        top: origin.y,
        unit,
        height: (frame.height * scale) as i32,
    })
}

fn render_and_publish(hwnd: HWND) {
    let Some(mut frame) = frame_of(hwnd) else {
        return;
    };

    // SAFETY: the window is live and this runs on the thread that owns it.
    let scale = unsafe { f64::from(GetDpiForWindow(hwnd)) } / 96.0;
    let maximized = is_zoomed(hwnd);
    if (frame.scale - scale).abs() > f64::EPSILON
        || frame.maximized != maximized
        || frame.dark != DARK.with(|dark| dark.get())
    {
        frame.scale = scale;
        frame.maximized = maximized;
        frame.dark = DARK.with(|dark| dark.get());
        update_frame(hwnd, |stored| *stored = frame);
        render_controls(&frame);
    }
    publish_state(hwnd);
}

fn render_controls(frame: &Frame) {
    let Some(ui) = app() else {
        return;
    };

    let scale = frame.scale;
    let (unit, height) = (
        (CONTROL_WIDTH * scale) as i32,
        (frame.height * scale) as i32,
    );
    if unit <= 0 || height <= 0 {
        return;
    }
    let dpi = unsafe { GetDpiForWindow(frame.hwnd) };
    let ink = if frame.dark {
        (255u8, 255u8, 255u8)
    } else {
        (0u8, 0u8, 0u8)
    };
    // A maximized window draws the restore glyph in place of maximize.
    let glyphs = [
        GLYPH_MINIMIZE,
        if frame.maximized {
            GLYPH_RESTORE
        } else {
            GLYPH_MAXIMIZE
        },
        GLYPH_CLOSE,
    ];

    let mut controls = Vec::with_capacity(CONTROL_COUNT);
    for (index, glyph) in glyphs.iter().enumerate() {
        let plate = index == CLOSE;
        // SAFETY: the DIB and the fonts belong to this thread, and `render_glyph`
        // releases both before it returns.
        let rendered = unsafe {
            (
                render_glyph(*glyph, ink, dpi, (unit, height)),
                if plate {
                    render_glyph(*glyph, CLOSE_INK, dpi, (unit, height))
                } else {
                    None
                },
            )
        };
        let (Some(glyph_image), rendered_plate_glyph) = rendered else {
            log::warn!(
                target: "shell",
                "windows caption: the caption font would not draw the window controls"
            );
            return;
        };
        // A control with no plate never changes the glyph's ink.
        let plate_glyph = rendered_plate_glyph.unwrap_or_else(|| glyph_image.clone());

        controls.push(WindowControl {
            index: index as i32,
            glyph: glyph_image,
            plate_glyph,
            plate,
            hover_fill: if plate {
                fill(CLOSE_HOVER, 255)
            } else {
                fill(ink, SUBTLE_HOVER)
            },
            press_fill: if plate {
                fill(CLOSE_PRESSED, 255)
            } else {
                fill(ink, press_increment(SUBTLE_HOVER, SUBTLE_PRESSED))
            },
        });
    }

    let model: ModelRc<WindowControl> = ModelRc::new(VecModel::from(controls));
    ui.set_window_controls(model);
}

/// Renders one glyph in one ink on a transparent background. The fill is drawn
/// separately so the app can animate it.
///
/// SAFETY: the DIB and font belong to this thread; handles are released on the
/// way out and the DIB is only read between the draw and the release.
unsafe fn render_glyph(
    glyph: u16,
    ink: (u8, u8, u8),
    dpi: u32,
    (width, height): (i32, i32),
) -> Option<Image> {
    // SAFETY: as above.
    unsafe {
        // Top-down 32-bit DIB: pixels come out in draw order.
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut core::ffi::c_void = null_mut();
        let bitmap = CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
        let screen = CreateCompatibleDC(None);
        if screen.0.is_null() {
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
            return None;
        }
        let previous = SelectObject(screen, HGDIOBJ(bitmap.0));
        // `CreateDIBSection` does not zero; full coverage is read from the ink.
        let bytes = (width * height * 4) as usize;
        slice::from_raw_parts_mut(bits.cast::<u8>(), bytes).fill(0);

        // A symbol font: the glyph is white on black, so its greyscale is its
        // coverage.
        let character_height = -(GLYPH_POINTS * dpi as i32) / 72;
        let _ = SetBkMode(screen, TRANSPARENT);
        let _ = SetTextColor(screen, COLORREF(0x00FF_FFFF));
        let _ = SetTextAlign(screen, TA_CENTER | TA_BOTTOM);

        // The first face that draws the glyph wins.
        for (index, face) in GLYPH_FONTS.into_iter().enumerate() {
            let font = CreateFontW(
                character_height,
                0,
                0,
                0,
                FW_NORMAL.0 as i32,
                false.into(),
                false.into(),
                false.into(),
                DEFAULT_CHARSET,
                OUT_TT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                CLEARTYPE_QUALITY,
                u32::from(DEFAULT_PITCH.0) | u32::from(FF_DONTCARE.0),
                face,
            );
            if font.is_invalid() {
                log::debug!(
                    target: "shell",
                    "windows caption: caption font #{index} is unavailable"
                );
                continue;
            }
            let previous_font = SelectObject(screen, HGDIOBJ(font.0));
            let _ = TextOutW(screen, width / 2, height / 2, &[glyph]);

            // The alignment centred the font box, not the ink; measure and redraw.
            let offset = ink_offset(bits, (width, height)).unwrap_or_default();
            if offset != (0, 0) {
                slice::from_raw_parts_mut(bits.cast::<u8>(), bytes).fill(0);
                let _ = TextOutW(
                    screen,
                    width / 2 + offset.0,
                    height / 2 + offset.1,
                    &[glyph],
                );
            }
            SelectObject(screen, previous_font);
            let _ = DeleteObject(HGDIOBJ(font.0));

            if slice::from_raw_parts(bits.cast::<u8>(), bytes)
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel[0] > 0)
            {
                log::debug!(
                    target: "shell",
                    "windows caption: the glyph came from caption font #{index}"
                );
                break;
            }
            // This face had no such glyph: clear before trying the next.
            slice::from_raw_parts_mut(bits.cast::<u8>(), bytes).fill(0);
        }
        // Neither face drew it; the button will be blank.
        if slice::from_raw_parts(bits.cast::<u8>(), bytes)
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| pixel[0] == 0)
        {
            log::debug!(
                target: "shell",
                "windows caption: neither caption font has this glyph; the button will be \\
                 blank"
            );
        }

        // Slint pixels are premultiplied; zero coverage is transparent.
        let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(width as u32, height as u32);
        let raw = slice::from_raw_parts(bits.cast::<u8>(), bytes);
        let pixels = buffer.make_mut_slice();
        for (pixel, source) in pixels.iter_mut().zip(raw.as_chunks::<4>().0) {
            let coverage = f32::from(source[0]) / 255.0;
            *pixel = Rgba8Pixel {
                r: (f32::from(ink.0) * coverage) as u8,
                g: (f32::from(ink.1) * coverage) as u8,
                b: (f32::from(ink.2) * coverage) as u8,
                a: source[0],
            };
        }

        SelectObject(screen, previous);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(screen);
        Some(Image::from_rgba8(buffer))
    }
}

/// How far to move the glyph so its ink, not its font box, is centred; `None`
/// when there is no ink or it was clipped at the DIB edge.
///
/// # Safety
///
/// `bits` must be a top-down 32-bit DIB of `(width, height)` owned by this
/// thread, with nobody else drawing into it.
unsafe fn ink_offset(
    bits: *mut core::ffi::c_void,
    (width, height): (i32, i32),
) -> Option<(i32, i32)> {
    let bytes = (width * height * 4) as usize;
    // SAFETY: the caller guarantees the DIB's size and ownership.
    let raw = unsafe { slice::from_raw_parts(bits.cast::<u8>(), bytes) };

    let mut left = width;
    let mut top = height;
    let mut right = 0;
    let mut bottom = 0;
    for (index, pixel) in raw.as_chunks::<4>().0.iter().enumerate() {
        if pixel[0] == 0 {
            continue;
        }
        let (x, y) = (index as i32 % width, index as i32 / width);
        left = left.min(x);
        right = right.max(x);
        top = top.min(y);
        bottom = bottom.max(y);
    }
    if right < left || bottom < top {
        return None;
    }
    // Ink touching the edge of the DIB was cut off by it.
    if left == 0 || top == 0 || right == width - 1 || bottom == height - 1 {
        return None;
    }

    let slot = (width / 2, height / 2);
    let ink = ((left + right + 1) / 2, (top + bottom + 1) / 2);
    Some((slot.0 - ink.0, slot.1 - ink.1))
}

/// Rounds the window's corners on Windows 11, where the app paints its own
/// square frame. A no-op on older versions.
///
/// SAFETY: `hwnd` is a live top-level window on this thread.
unsafe fn round_corners(hwnd: HWND) {
    let preference = DWMWCP_ROUND;
    // SAFETY: `hwnd` is a live top-level window on this thread.
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            addr_of!(preference).cast(),
            size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        );
    }
}

fn control_at(hwnd: HWND, point: POINT) -> Option<usize> {
    let controls = controls_of(hwnd)?;
    if point.x < controls.left
        || point.x >= controls.left + controls.width()
        || point.y < controls.top
        || point.y >= controls.top + controls.height
    {
        return None;
    }
    // Row order is minimize, maximize, close.
    let index = ((point.x - controls.left) / controls.unit) as usize;
    (index < CONTROL_COUNT).then_some(index)
}

/// The control a hit-test code belongs to, as the app numbers them.
fn control_of_hit(code: u32) -> Option<i32> {
    CONTROL_HIT_TESTS
        .iter()
        .position(|hit| *hit == code)
        .map(|index| index as i32)
}

/// The control under the cursor, or `-1`. Client mouse messages carry no
/// hit-test code, so the cursor position is used instead.
fn control_under_cursor(hwnd: HWND) -> i32 {
    let mut cursor = POINT::default();
    // SAFETY: `cursor` is a local the API only writes to.
    if unsafe { GetCursorPos(&mut cursor) }.is_err() {
        return -1;
    }
    control_at(hwnd, cursor).map_or(-1, |index| index as i32)
}

/// The command a control's caption button sends.
fn system_command(index: i32, maximized: bool) -> u32 {
    match index {
        0 => SC_MINIMIZE,
        1 if maximized => SC_RESTORE,
        1 => SC_MAXIMIZE,
        _ => SC_CLOSE,
    }
}

fn is_zoomed(hwnd: HWND) -> bool {
    // SAFETY: the window is live and this runs on the thread that owns it.
    unsafe { IsZoomed(hwnd).as_bool() }
}

fn hwnd_of(window: &slint::Window) -> Option<HWND> {
    window.with_winit_window(|winit_window| {
        let handle = winit_window.window_handle().ok()?;
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return None;
        };
        Some(HWND(handle.hwnd.get() as *mut core::ffi::c_void))
    })?
}

/// The window procedure. Messages it does not answer are forwarded unchanged.
///
/// SAFETY: called by `SetWindowSubclass` on the thread that owns `hwnd`.
unsafe extern "system" fn subclass(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _subclass_id: usize,
    _ref_data: usize,
) -> LRESULT {
    // SAFETY: `hwnd` is the live window of the app and this runs on the thread
    // that owns it. `WM_NCCALCSIZE` and `WM_NCHITTEST` read a pointer out of
    // `lparam`, but only for the messages that carry one — see the arms below.
    unsafe {
        let point = POINT {
            x: lparam.0 as i16 as i32,
            y: (lparam.0 >> 16) as i16 as i32,
        };

        match message {
            // The client area is the whole window, so the app's title bar reaches
            // the top edge. A maximized window is the exception: the frame is
            // handed back so the app does not draw over the taskbar.
            WM_NCCALCSIZE if wparam.0 != 0 => {
                if is_zoomed(hwnd) {
                    let (border_x, border_y) = frame_thickness(hwnd);
                    let proposed = &mut (*(lparam.0 as *mut NCCALCSIZE_PARAMS)).rgrc[0];
                    proposed.left += border_x;
                    proposed.top += border_y;
                    proposed.right -= border_x;
                    proposed.bottom -= border_y;
                }
                LRESULT(0)
            }

            // The controls are the only non-client area; these hit-test codes
            // give them the system's behaviour.
            WM_NCHITTEST => match control_at(hwnd, point) {
                Some(index) => LRESULT(CONTROL_HIT_TESTS[index] as isize),
                None => LRESULT(hit_test(hwnd, point) as isize),
            },

            // `TrackMouseEvent` is what gets the leave.
            WM_NCMOUSEMOVE => {
                let frame = frame_of(hwnd);
                let hovered = control_of_hit(wparam.0 as u32)
                    .or_else(|| frame.map(|frame| frame.hovered))
                    .unwrap_or(-1);
                let pressed = frame.map(|frame| frame.pressed).unwrap_or(-1);
                set_pointer(hwnd, hovered, pressed);

                let mut track = TRACKMOUSEEVENT {
                    cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                // Without the leave, a caption button stays highlighted forever.
                if let Err(error) = TrackMouseEvent(&mut track) {
                    log::debug!(
                        target: "shell",
                        "windows caption: could not track the pointer ({error}); a control may \\
                         stay highlighted"
                    );
                }
                DefSubclassProc(hwnd, message, wparam, lparam)
            }
            WM_NCMOUSELEAVE => {
                set_pointer(hwnd, -1, -1);
                DefSubclassProc(hwnd, message, wparam, lparam)
            }

            // Act on release so dragging off a button cancels it. `DefWindowProc`
            // cannot do this for a window with no caption: its tracking loop has
            // no button to track, so the release never comes back.
            WM_NCLBUTTONDOWN => {
                if let Some(index) = control_of_hit(wparam.0 as u32) {
                    let hovered = frame_of(hwnd).map_or(-1, |frame| frame.hovered);
                    set_pointer(hwnd, hovered, index);
                    return LRESULT(0);
                }
                DefSubclassProc(hwnd, message, wparam, lparam)
            }
            WM_NCLBUTTONUP => {
                let pressed = frame_of(hwnd).is_some_and(|frame| frame.pressed >= 0);
                let Some(index) = control_of_hit(wparam.0 as u32) else {
                    // Released off the control: cancelled press.
                    if pressed {
                        set_pointer(hwnd, -1, -1);
                    }
                    return DefSubclassProc(hwnd, message, wparam, lparam);
                };
                let maximized = frame_of(hwnd).is_some_and(|frame| frame.maximized);
                set_pointer(hwnd, index, -1);
                let command = system_command(index, maximized);
                if let Err(error) = PostMessageW(
                    Some(hwnd),
                    WM_SYSCOMMAND,
                    WPARAM(command as usize),
                    LPARAM(0),
                ) {
                    log::warn!(
                        target: "shell",
                        "windows caption: the {command} command could not be posted ({error}); \
                         the button will do nothing"
                    );
                } else {
                    log::debug!(target: "shell", "windows caption: posted {command}");
                }
                LRESULT(0)
            }

            // A press released over the client area is cancelled.
            WM_LBUTTONUP if frame_of(hwnd).is_some_and(|frame| frame.pressed >= 0) => {
                set_pointer(hwnd, -1, -1);
                DefSubclassProc(hwnd, message, wparam, lparam)
            }
            WM_MOUSEMOVE if frame_of(hwnd).is_some_and(|frame| frame.pressed >= 0) => {
                let pressed = frame_of(hwnd).map_or(-1, |frame| frame.pressed);
                set_pointer(hwnd, control_under_cursor(hwnd), pressed);
                DefSubclassProc(hwnd, message, wparam, lparam)
            }

            // winit supplies the track sizes; only the maximized geometry is
            // replaced here.
            WM_GETMINMAXINFO => {
                let result = DefSubclassProc(hwnd, message, wparam, lparam);
                fit_work_area(hwnd, lparam);
                result
            }

            // Maximizing changes the glyph; activation clears the pointer.
            WM_SIZE | WM_ACTIVATE => {
                if message == WM_ACTIVATE {
                    set_pointer(hwnd, -1, -1);
                }
                let result = DefSubclassProc(hwnd, message, wparam, lparam);
                render_and_publish(hwnd);
                result
            }
            WM_DPICHANGED => {
                let result = DefSubclassProc(hwnd, message, wparam, lparam);
                render_and_publish(hwnd);
                result
            }

            _ => DefSubclassProc(hwnd, message, wparam, lparam),
        }
    }
}

/// Hit test for the resize border. The top edge answers `HTCLIENT` so the press
/// reaches the app's controls and drag area.
fn hit_test(hwnd: HWND, point: POINT) -> u32 {
    // SAFETY: the window is live and this runs on the thread that owns it.
    unsafe {
        // A maximized window is the work area; there is no border to grab.
        if IsZoomed(hwnd).as_bool() {
            return HTCLIENT;
        }

        let mut window = RECT::default();
        if GetWindowRect(hwnd, &mut window).is_err() {
            return HTCLIENT;
        }

        let (border_x, border_y) = frame_thickness(hwnd);
        match (
            point.y - window.top < border_y,
            window.bottom - point.y < border_y,
            point.x - window.left < border_x,
            window.right - point.x < border_x,
        ) {
            (true, _, true, _) => HTTOPLEFT,
            (true, _, _, true) => HTTOPRIGHT,
            (_, true, true, _) => HTBOTTOMLEFT,
            (_, true, _, true) => HTBOTTOMRIGHT,
            (_, true, _, _) => HTBOTTOM,
            (_, _, true, _) => HTLEFT,
            (_, _, _, true) => HTRIGHT,
            _ => HTCLIENT,
        }
    }
}

/// The resize border's thickness, measured as the frame without its caption at
/// the window's DPI.
fn frame_thickness(hwnd: HWND) -> (i32, i32) {
    // SAFETY: the window is live and this runs on the thread that owns it.
    unsafe {
        let mut empty = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let without_caption = WINDOW_STYLE(WS_OVERLAPPEDWINDOW.0 & !WS_CAPTION.0);
        let _ = AdjustWindowRectExForDpi(
            &mut empty,
            without_caption,
            false,
            WINDOW_EX_STYLE(0),
            GetDpiForWindow(hwnd),
        );
        (-empty.left, -empty.top)
    }
}

/// Makes a maximized window exactly the monitor's work area, rather than the
/// frame grown by the system.
fn fit_work_area(hwnd: HWND, lparam: LPARAM) {
    // SAFETY: the window is live and this runs on the thread that owns it;
    // `lparam` is the `MINMAXINFO` Windows filled in.
    unsafe {
        let mut monitor = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let nearest = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if !GetMonitorInfoW(nearest, &mut monitor).as_bool() {
            return;
        }

        let Some(limits) = (lparam.0 as *mut MINMAXINFO).as_mut() else {
            return;
        };
        // The maximized position and size are relative to the monitor, not the
        // screen: a taskbar on the left, top or on more than one edge is why.
        limits.ptMaxPosition.x = monitor.rcWork.left - monitor.rcMonitor.left;
        limits.ptMaxPosition.y = monitor.rcWork.top - monitor.rcMonitor.top;
        limits.ptMaxSize.x = monitor.rcWork.right - monitor.rcWork.left;
        limits.ptMaxSize.y = monitor.rcWork.bottom - monitor.rcWork.top;
    }
}
