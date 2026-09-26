// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Windows: the platform's own window controls, over the app's custom title bar.
//!
//! # What DWM will and will not do
//!
//! The obvious route — keep a stock `WS_OVERLAPPEDWINDOW` window, answer
//! `WM_NCCALCSIZE` with "the client area is the whole window", and ask DWM to
//! paint the caption buttons on top of the app's own drawing — does not work on
//! any Windows since 8. Verified on Windows 11: with the caption removed from
//! the non-client area the buttons are simply not drawn, whether or not
//! `DwmExtendFrameIntoClientArea` is called. (That call succeeds; it is the
//! Windows 7 Aero Glass hook and has been a no-op for opaque windows ever since.)
//! Restoring the default `WM_NCCALCSIZE` brings the caption, and the three
//! buttons, straight back — the buttons live in the non-client area, and that is
//! the only place they live. Microsoft's own answers agree: both
//! `Window.ExtendsContentIntoTitleBar` (WinUI 3) and
//! `AppWindowTitleBar.ExtendsContentIntoTitleBar` remove the frame the same way
//! and then *draw the caption buttons themselves*.
//!
//! # What this does instead
//!
//! The controls are real non-client area, so Windows treats them as its own:
//! `WM_NCHITTEST` answers `HTMINBUTTON` / `HTMAXBUTTON` / `HTCLOSE` for them, and
//! everything that hangs off those codes comes with them — the Windows 11
//! snap-layouts flyout, which is DWM's own and appears for a maximizable window
//! whose maximize button hit tests as `HTMAXBUTTON`; the system menu on a right
//! click; the buttons' own entries in the accessibility tree. This is what
//! Chromium, Electron, VS Code and Windows Terminal do, and the reason their
//! controls feel like the system's.
//!
//! What a press *does* is a `WM_SYSCOMMAND` carrying the one command a caption
//! button ever sends — `SC_MINIMIZE`, `SC_MAXIMIZE`/`SC_RESTORE`, `SC_CLOSE` — so
//! the window is still minimized, maximized and closed by Windows, with its own
//! animation and its own idea of where a maximized window goes. It is sent on the
//! release, and only if the pointer is still on the control, because that is what
//! makes dragging off a caption button cancel it. `DefWindowProc` is not asked to
//! do this: it acts on a close press at once, but the other two are acted on from
//! inside the tracking loop it starts for a button, and a window with no caption
//! has no button for that loop to track — the press goes in and the release never
//! comes back out.
//!
//! The *artwork* is the platform's too, as far as Windows will give it up: the
//! glyphs come from the font Windows draws its caption buttons with
//! ([`GLYPH_FONTS`]), not from paths of our own, and the fills are DWM's. It
//! cannot come from `DrawThemeBackground`, which is worth writing down because it
//! looks like the obvious answer and is not usable here:
//!
//! * The `Explorer` class — the one the shell draws its own caption with, and the
//!   one Windows Terminal asks for — does not open at all on Windows 11: there are
//!   no `.msstyles` files left to load it from, and `OpenThemeData` answers with
//!   a null handle.
//! * The `Window` class does open, and does draw `WP_CLOSEBUTTON` and friends, but
//!   what it draws is the *classic* caption: an opaque button face, filled in even
//!   in its normal state, with the close button red throughout. It also renders
//!   light regardless of the system theme, because a process only gets the dark
//!   variants by opting in per window, and the opt-in is not reachable without
//!   the class that is missing. On a dark title bar that is a row of pale blocks.
//!
//! So the glyphs are rendered with GDI, and the two colours the theme would have
//! supplied — the subtle fill on hover and press, and the close button's red — are
//! the ones DWM uses ([`SUBTLE_HOVER`], [`SUBTLE_PRESSED`], [`CLOSE_HOVER`],
//! [`CLOSE_PRESSED`]). The ink and the subtle fill follow the *app's* palette
//! rather than the system's, because the title bar they sit on is the app's: a
//! white glyph is right on Catppuccin Mocha and invisible on Latte.
//!
//! # Why the fill is a colour and not part of the bitmap
//!
//! The glyph is the only artwork this module renders, and that is what lets the
//! app *animate* the controls rather than snapping between three finished states.
//! Hovering and pressing are the fill changing, so a fill that can be interpolated
//! is the transition; one baked into a bitmap can only be cross-faded against
//! another bitmap, and three of those cannot be stacked — they composite
//! additively, which would make a press read as hover *plus* pressed. Publishing
//! the fill as a pair of colours lets the app drive the whole ramp off one
//! animated number, and lets the glyph stay perfectly still behind it, which is
//! what the platform's own buttons do.
//!
//! # Where the controls are
//!
//! Rust owns the geometry, because it is what the platform hit tests against:
//! three [`CONTROL_WIDTH`]-wide slots in the client area's top-right corner, as
//! tall as the app's title bar — read from `App.title-bar-height`, so this module
//! and the `.slint` side cannot drift apart — and the total is published as
//! `App.window-controls-inset` for the title bar to lay its own controls out
//! around. This module also owns the rest of the frameless window: the caption
//! (gone, see `WM_NCCALCSIZE`), the invisible resize border, and a maximized
//! window that is exactly the monitor's work area rather than one border too big
//! for a client area that covers the whole window.

use std::cell::{Cell, RefCell};
use std::ptr::{addr_of, null_mut};
use std::slice;
use std::sync::atomic::{AtomicBool, Ordering};

use i_slint_backend_winit::{EventResult, WinitWindowAccessor};
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

/// Identifies our subclass. Arbitrary, but it has to be unique per window and
/// procedure pair — `SetWindowSubclass` keys the chain on it.
const SUBCLASS_ID: usize = 0x00C0_011C;

/// One control is 46 logical pixels wide, which is what Windows 10 and 11 give a
/// caption button at 96 DPI, and as tall as the app's title bar — that is the
/// area the fill on hover and press covers.
///
/// It is a logical width because that is the unit the title bar is laid out in,
/// so it follows the DPI the way every other length in the design does; the
/// artwork is rendered at the matching physical size (see [`render_controls`]).
const CONTROL_WIDTH: f64 = 46.0;

/// How many controls there are, and the order they are laid out in — the same
/// order `App.window-control-hovered` / `-pressed` use, so the app can pick the
/// right artwork without having to know which one Windows thinks it is on.
const CONTROL_COUNT: usize = 3;

/// `HTMINBUTTON` / `HTMAXBUTTON` / `HTCLOSE`, by index.
const CONTROL_HIT_TESTS: [u32; CONTROL_COUNT] = [HTMINBUTTON, HTMAXBUTTON, HTCLOSE];

/// Which control is the close button, and so the one whose fill is an opaque
/// plate rather than a translucent wash.
const CLOSE: usize = 2;

/// The caption-button glyphs, in the private-use area the font publishes them
/// at. Windows draws its caption buttons with these.
const GLYPH_MINIMIZE: u16 = 0xE921;
const GLYPH_MAXIMIZE: u16 = 0xE922;
const GLYPH_RESTORE: u16 = 0xE923;
const GLYPH_CLOSE: u16 = 0xE8BB;

/// The faces that caption font is published under: Windows 11 renamed it, and
/// Windows 10 and the early Windows 11 builds only carry the old name. The
/// first one that actually draws the glyph wins, so a build that has neither
/// falls through to the next rather than drawing boxes.
const GLYPH_FONTS: [windows::core::PCWSTR; 2] = [w!("Segoe Fluent Icons"), w!("Segoe MDL2 Assets")];

/// The point size the caption glyphs are drawn at, which is the caption's own:
/// 10pt is what Windows 11 draws its caption buttons at, and the unit the caption
/// itself uses, so the controls come out the size the platform's would be.
///
/// **This is the knob for the glyphs' size.** It is a point size, converted to the
/// font's character height at the window's DPI, so it scales with the display
/// rather than being pinned to a pixel count — which is why 10 stays 10 on a 4K
/// monitor. Try 9 or 11 and re-run to compare; anything from about 8 to 12 reads
/// well inside the 46px slot. Where the glyphs *sit* is not set here: that is
/// measured from the rendered ink, see [`ink_offset`].
const GLYPH_POINTS: i32 = 8;

/// What goes behind a caption button on hover and while it is pressed: the
/// caption ink at ten and twenty percent. Alpha, because it is a fill over
/// whatever the title bar is, and the title bar is the app's.
///
/// These are the app's numbers, not a platform constant. They read like what
/// Windows 11 puts there, and ten and twenty are what makes a press legible
/// against a dark title bar, which six and twelve were not. They are published as
/// *colours* rather than drawn into the glyph so the app can interpolate between
/// them — see `WindowControl` in `components/title-bar.slint`.
const SUBTLE_HOVER: u8 = 26; // 10% of 255
const SUBTLE_PRESSED: u8 = 51; // 20% of 255

/// The red Windows 11 fills the close button with on hover, and the shade it
/// goes to while pressed. These are DWM's own, and what Explorer, Edge and
/// Windows Terminal all paint.
const CLOSE_HOVER: (u8, u8, u8) = (0xC4, 0x2B, 0x1C);
const CLOSE_PRESSED: (u8, u8, u8) = (0xB0, 0x25, 0x18);

/// The ink the close button's glyph takes on once it is on that red plate, which
/// is white whichever theme the app is in — see [`render_glyph`].
const CLOSE_INK: (u8, u8, u8) = (255, 255, 255);

/// A fill as Slint wants it: `color` at `alpha` over whatever is behind it.
fn fill(color: (u8, u8, u8), alpha: u8) -> Color {
    Color::from_argb_u8(alpha, color.0, color.1, color.2)
}

/// The alpha a press layer needs so that, stacked on the hover layer, it lands on
/// `pressed` rather than short of it.
///
/// Two washes of `a` and `b` composite to `a + b - a·b`, so a layer of `b` on top
/// of the hover wash only reaches `pressed` when `b` is
/// `(pressed - hover) / (1 - hover)`. The two close-button reds are both opaque,
/// where the hover layer already hides whatever is under it and the increment is
/// simply the colour to fade in.
fn press_increment(hover: u8, pressed: u8) -> u8 {
    if hover == 255 {
        return pressed;
    }
    let (from, to) = (f32::from(hover) / 255.0, f32::from(pressed) / 255.0);
    (((to - from) / (1.0 - from)) * 255.0).round() as u8
}

/// The width the controls take on the right of the title bar, in logical units.
pub fn controls_inset() -> f32 {
    (CONTROL_WIDTH * CONTROL_COUNT as f64) as f32
}

/// Where the controls are, in physical screen pixels: the top-left corner of the
/// row (`left`, `top`), how wide one control is (`unit`) and how tall they are.
///
/// They sit in the *client* area's top-right corner, which is where the app's
/// title bar draws them, and that is deliberately not the same as the window's
/// corner: a maximized window's client area is the monitor's work area while its
/// window rectangle is that plus the frame.
#[derive(Clone, Copy)]
struct Controls {
    left: i32,
    top: i32,
    unit: i32,
    height: i32,
}

impl Controls {
    /// How wide the whole row is.
    fn width(&self) -> i32 {
        self.unit * CONTROL_COUNT as i32
    }
}

/// What the window procedure knows about the window it is attached to: enough to
/// tell whether the artwork has to be rendered again, and to report the pointer.
#[derive(Clone, Copy)]
struct Frame {
    hwnd: HWND,
    /// The title bar's height in logical units, i.e. how tall the controls are.
    height: f64,
    /// The scale the artwork was last rendered for, so a DPI change re-renders it.
    scale: f64,
    /// The maximize control becomes the restore control while maximized.
    maximized: bool,
    /// Whether the app's theme is dark, which is what the glyphs and the subtle
    /// fill take their colour from.
    dark: bool,
    hovered: i32,
    pressed: i32,
}

thread_local! {
    /// The app, so the window procedure can report what it sees.
    ///
    /// The window procedure runs on the thread that owns the window — the one
    /// running the Slint event loop — so this needs no locking.
    static APP: RefCell<Option<Weak<App>>> = const { RefCell::new(None) };
    /// The window this module is attached to, once there is one. `Frame` is
    /// `Copy`, so it is read and written by value rather than through a borrow
    /// that would have to outlive the `RefCell` guard.
    static FRAME: RefCell<Option<Frame>> = const { RefCell::new(None) };
    /// The palette asked for, remembered for the window that does not exist yet:
    /// the theme settles while the event loop is still starting up.
    static DARK: Cell<bool> = const { Cell::new(true) };
}

/// A copy of the attached window's state, if this module is attached at all.
fn frame() -> Option<Frame> {
    FRAME.with(|slot| *slot.borrow())
}

/// The attached window's state, or `None` if `hwnd` is not the attached window.
fn frame_of(hwnd: HWND) -> Option<Frame> {
    frame().filter(|frame| frame.hwnd == hwnd)
}

/// Changes the attached window's state, if `hwnd` is the attached window.
fn update_frame(hwnd: HWND, change: impl FnOnce(&mut Frame)) {
    FRAME.with(|slot| {
        if let Some(frame) = slot.borrow_mut().as_mut()
            && frame.hwnd == hwnd
        {
            change(frame);
        }
    });
}

/// Records the pointer's state, which is the only state the platform reports
/// outward — nothing redraws the buttons for a window, so nothing else has to be
/// told about them.
fn set_pointer(hwnd: HWND, hovered: i32, pressed: i32) {
    update_frame(hwnd, |frame| {
        frame.hovered = hovered;
        frame.pressed = pressed;
    });
    publish_state(hwnd);
}

/// Publishes which control the pointer is on, and whether it is pressed.
fn publish_state(hwnd: HWND) {
    let Some(frame) = frame_of(hwnd) else {
        return;
    };
    if let Some(ui) = app() {
        ui.set_window_control_hovered(frame.hovered);
        ui.set_window_control_pressed(frame.pressed);
    }
}

/// Takes the window frame over, once winit has created the window.
///
/// Must be called with the app's root component, like `traffic_lights::install`.
pub fn install(ui: &App) {
    APP.with(|slot| *slot.borrow_mut() = Some(ui.as_weak()));

    // There is no `HWND` until the event loop has created the window, so this
    // takes the first winit event the window gets rather than polling for one.
    let weak = ui.as_weak();
    let attached = AtomicBool::new(false);
    ui.window().on_winit_window_event(move |_, _| {
        if !attached.swap(true, Ordering::Relaxed)
            && let Some(ui) = weak.upgrade()
        {
            attach(&ui);
        }
        EventResult::Propagate
    });
}

/// Asks for the window controls to be drawn in the palette the app resolved to.
///
/// The glyphs and the subtle fill take their colour from this, so they stay
/// legible on the app's title bar whatever the system theme is doing. `false` for
/// Latte, `true` for the three dark flavors — the same rule as
/// `ThemeProvider.dark`.
pub fn set_dark(window: &slint::Window, dark: bool) {
    DARK.with(|slot| slot.set(dark));
    if let Some(hwnd) = hwnd_of(window) {
        render_and_publish(hwnd);
    }
}

/// Installs the subclass, and takes the window's frame and controls over, the
/// first time there is a window to do it on.
fn attach(ui: &App) {
    let Some(hwnd) = hwnd_of(ui.window()) else {
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

/// The app, if the window this module belongs to is still alive.
fn app() -> Option<App> {
    APP.with(|slot| slot.borrow().as_ref().and_then(Weak::upgrade))
}

/// The controls' geometry for the window, if this module is attached to it.
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

/// Re-renders the controls if anything they are drawn from moved, and reports
/// the pointer's state either way.
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

/// Renders the three controls and hands them to the app.
///
/// The glyph is the only artwork: the fill behind it is published as a pair of
/// colours, because for two of the three controls the fill is the only thing that
/// changes between resting, hovered and pressed, and handing it over as a colour
/// is what lets the app interpolate it. (A fill baked into the bitmap would force
/// a cross-fade between whole rasters, and three of those cannot be stacked: they
/// composite additively, so a press would read as hover *plus* pressed.)
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
    // The caption's ink, which the glyph and the wash are both drawn from. On a
    // dark title bar that is a light glyph over a light wash; on Latte, the
    // reverse. Close is the one control that stops being caption-coloured, because
    // its fill is an opaque plate rather than a wash.
    let ink = if frame.dark {
        (255u8, 255u8, 255u8)
    } else {
        (0u8, 0u8, 0u8)
    };
    // The maximize control becomes the restore control while the window is
    // maximized, and the platform draws that as a different glyph.
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
        // A control with no plate keeps one image for both inks: its glyph never
        // changes colour, only the wash behind it does.
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

/// Renders one control's glyph, in one ink, on a transparent background: the
/// platform's own glyph, from the font it draws its caption buttons with.
///
/// The fill is *not* in here — the app draws that as a rectangle so it can animate
/// it (see [`render_controls`]).
///
/// SAFETY: the DIB and the font below belong to this thread; every handle is
/// released on the way out, and the DIB is only read between the draw and the
/// release.
unsafe fn render_glyph(
    glyph: u16,
    ink: (u8, u8, u8),
    dpi: u32,
    (width, height): (i32, i32),
) -> Option<Image> {
    // SAFETY: as above.
    unsafe {
        // A top-down 32-bit DIB, so the pixels come out in the order they are
        // drawn in and every channel is meaningful.
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
        // `CreateDIBSection` does not promise zeroed memory, and the glyph's
        // coverage is read out of the greyscale it is drawn as, so what was there
        // has to go.
        let bytes = (width * height * 4) as usize;
        slice::from_raw_parts_mut(bits.cast::<u8>(), bytes).fill(0);

        // The caption font, at the caption's own size. The face is a symbol font,
        // so the glyph comes out white on black and its greyscale *is* its
        // coverage — which is what the compositing below reads, and what the
        // centring pass measures.
        let character_height = -(GLYPH_POINTS * dpi as i32) / 72;
        let _ = SetBkMode(screen, TRANSPARENT);
        let _ = SetTextColor(screen, COLORREF(0x00FF_FFFF));
        // `TA_TOP` is zero, so centring horizontally and vertically is `TA_CENTER`
        // with the bottom half of the vertical pair. Both are relative to the
        // point that is passed, so the correction below is a plain translation.
        let _ = SetTextAlign(screen, TA_CENTER | TA_BOTTOM);

        // The first face that actually draws the glyph wins, so a Windows 10
        // build with only the old font name still gets its caption buttons rather
        // than a row of empty rectangles.
        for face in GLYPH_FONTS {
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
                continue;
            }
            let previous_font = SelectObject(screen, HGDIOBJ(font.0));
            let _ = TextOutW(screen, width / 2, height / 2, &[glyph]);

            // What the text alignment above centred is the *font's* box, from
            // ascent to descent, which for a private-use glyph is only near where
            // the ink is — and near is a different amount for a thin dash than for
            // a square. So the glyph is measured where it landed and drawn again a
            // pixel or two away, which centres the ink itself.
            let offset = ink_offset(bits, (width, height)).unwrap_or_default();
            if offset != (0, 0) {
                // `bits` is the DIB this DC has selected, and it is only read and
                // written between the two draws.
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
                .chunks_exact(4)
                .any(|pixel| pixel[0] > 0)
            {
                break;
            }
            // This face had no such glyph: clear it before trying the next.
            slice::from_raw_parts_mut(bits.cast::<u8>(), bytes).fill(0);
        }

        // The ink at the coverage the glyph drew, in premultiplied form, which is
        // what Slint's pixels are: the colour channels already carry the alpha, so
        // a pixel with no coverage comes out fully transparent. That is also the
        // right answer for a build carrying neither caption font — an empty glyph
        // rather than a broken one.
        let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(width as u32, height as u32);
        let raw = slice::from_raw_parts(bits.cast::<u8>(), bytes);
        let pixels = buffer.make_mut_slice();
        for (pixel, source) in pixels.iter_mut().zip(raw.chunks_exact(4)) {
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

/// How far the glyph has to move from where GDI's text alignment put it for its
/// *ink* to be centred in the control, or `None` if the DIB holds no ink at all.
///
/// The DIB's alpha channel is the glyph's coverage, white-on-black, so the ink is
/// simply the non-zero pixels. `TA_CENTER`/`TA_BOTTOM` centre the font's own box
/// (ascent to descent) rather than that, and a private-use glyph does not fill its
/// em evenly: a thin horizontal dash sits high in the box, the cross and the square
/// sit differently again. Measuring is what keeps all three looking centred instead
/// of nudging all three by one guessed amount.
///
/// A glyph whose ink reaches the edge of the DIB has been clipped, so what was
/// measured is not the whole glyph and the correction would be guesswork; that
/// returns no offset at all, leaving GDI's centring to do as good a job as it can.
///
/// # Safety
///
/// `bits` must be a top-down 32-bit DIB of `(width, height)` that this thread
/// owns and nobody else is drawing into.
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
    for (index, pixel) in raw.chunks_exact(4).enumerate() {
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

    // Half-open, so the two centres are the middle of each box in the same units:
    // the slot's middle is between pixels `width / 2` and `width / 2 + 1`.
    let slot = (width / 2, height / 2);
    let ink = ((left + right + 1) / 2, (top + bottom + 1) / 2);
    Some((slot.0 - ink.0, slot.1 - ink.1))
}

/// Keeps the window rounded, which is what the desktop window manager does for a
/// top-level window on Windows 11. Asked for explicitly because the frame is the
/// app's own now, and the app draws square corners inside it.
///
/// SAFETY: `hwnd` is a live top-level window on this thread.
unsafe fn round_corners(hwnd: HWND) {
    let preference = DWMWCP_ROUND;
    // SAFETY: `hwnd` is a live top-level window on this thread.
    unsafe {
        // A no-op before Windows 11, which has no such attribute.
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            addr_of!(preference).cast(),
            size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        );
    }
}

/// The index of the control a screen point is on, or `None`.
fn control_at(hwnd: HWND, point: POINT) -> Option<usize> {
    let controls = controls_of(hwnd)?;
    if point.x < controls.left
        || point.x >= controls.left + controls.width()
        || point.y < controls.top
        || point.y >= controls.top + controls.height
    {
        return None;
    }
    // The row runs left to right in the app's own order — minimize, maximize,
    // close — so the offset from its left edge is the control's index.
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

/// The control the pointer is on right now, in the app's numbering, or `-1`.
///
/// A press that wanders off a control is reported as client mouse messages, which
/// carry no hit-test code, so the cursor itself is what says where it is — the
/// same answer `WM_NCHITTEST` would have given.
fn control_under_cursor(hwnd: HWND) -> i32 {
    let mut cursor = POINT::default();
    // SAFETY: `cursor` is a local the API only writes to.
    if unsafe { GetCursorPos(&mut cursor) }.is_err() {
        return -1;
    }
    control_at(hwnd, cursor).map_or(-1, |index| index as i32)
}

/// The system command a control stands for — the one its caption button sends,
/// and the only thing a caption button does. The maximize control restores a
/// maximized window, which is why the command depends on the window's state.
fn system_command(index: i32, maximized: bool) -> u32 {
    match index {
        0 => SC_MINIMIZE,
        1 if maximized => SC_RESTORE,
        1 => SC_MAXIMIZE,
        _ => SC_CLOSE,
    }
}

/// Whether the window is maximized, which is a state change in its own right: the
/// maximize control becomes the restore control.
fn is_zoomed(hwnd: HWND) -> bool {
    // SAFETY: the window is live and this runs on the thread that owns it.
    unsafe { IsZoomed(hwnd).as_bool() }
}

/// The `HWND` behind a Slint window, once winit has created it.
fn hwnd_of(window: &slint::Window) -> Option<HWND> {
    window.with_winit_window(|winit_window| {
        let handle = winit_window.window_handle().ok()?;
        // `RawWindowHandle::Win32` is the only variant a Win32 window can have,
        // and its `hwnd` is the pointer this module's `HWND` wraps.
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return None;
        };
        Some(HWND(handle.hwnd.get() as *mut core::ffi::c_void))
    })?
}

/// The window procedure.
///
/// SAFETY: called by `SetWindowSubclass` on the thread that owns `hwnd`, with
/// the arguments Windows passed; every message it does not answer itself is
/// forwarded to winit's own procedure unchanged.
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
            // The client area is the whole window: no caption is reserved, so the
            // app's title bar reaches the top edge. `wParam` is only non-zero
            // when `lParam` is an `NCCALCSIZE_PARAMS`, and the proposed client
            // area already is the whole window — returning `0` takes it.
            //
            // A maximized window is the exception. Its proposed rectangle is the
            // work area (`WM_GETMINMAXINFO` below) grown by the frame, because a
            // window with a frame is expected to paint *inside* it; taking the
            // whole proposal would run the app's own drawing over the taskbar, so
            // the frame is handed back and the client area is the work area.
            WM_NCCALCSIZE if wparam.0 != 0 => {
                if is_zoomed(hwnd) {
                    let (border_x, border_y) = frame_thickness(hwnd);
                    // `wParam` is non-zero, so `lParam` is the
                    // `NCCALCSIZE_PARAMS` of the message being answered, and the
                    // rectangle is the one Windows proposed.
                    let proposed = &mut (*(lparam.0 as *mut NCCALCSIZE_PARAMS)).rgrc[0];
                    proposed.left += border_x;
                    proposed.top += border_y;
                    proposed.right -= border_x;
                    proposed.bottom -= border_y;
                }
                LRESULT(0)
            }

            // The controls first: they are the only part of this window that is
            // non-client area, and answering with their hit-test codes is what
            // hands the user the system's own behaviour for them.
            WM_NCHITTEST => match control_at(hwnd, point) {
                Some(index) => LRESULT(CONTROL_HIT_TESTS[index] as isize),
                None => LRESULT(hit_test(hwnd, point) as isize),
            },

            // The pointer is only ever reported through the non-client messages,
            // and `TrackMouseEvent` is what gets the leave.
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
                let _ = TrackMouseEvent(&mut track);
                DefSubclassProc(hwnd, message, wparam, lparam)
            }
            WM_NCMOUSELEAVE => {
                set_pointer(hwnd, -1, -1);
                DefSubclassProc(hwnd, message, wparam, lparam)
            }

            // A press on a control acts on the *release*: dragging off a caption
            // button and letting go cancels it. The command that goes out on the
            // release is the one the platform's own button sends — the app never
            // maximizes, minimizes or closes a window itself.
            //
            // `DefWindowProc` is deliberately not asked to do this. It acts on a
            // close press at once (`HTCLOSE` becomes `SC_CLOSE`), but the other two
            // are acted on from inside the tracking loop it starts for a button,
            // and a window with no caption has no button for that loop to track:
            // the press goes in and the release never comes back out.
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
                    // Released on the rest of the frame: a cancelled press.
                    if pressed {
                        set_pointer(hwnd, -1, -1);
                    }
                    return DefSubclassProc(hwnd, message, wparam, lparam);
                };
                let maximized = frame_of(hwnd).is_some_and(|frame| frame.maximized);
                set_pointer(hwnd, index, -1);
                let _ = PostMessageW(
                    Some(hwnd),
                    WM_SYSCOMMAND,
                    WPARAM(system_command(index, maximized) as usize),
                    LPARAM(0),
                );
                LRESULT(0)
            }

            // A press that wanders off the control and is let go over the app's
            // own drawing is cancelled too, and stops being highlighted while it
            // is off: the pointer is over the client area now, not a control.
            WM_LBUTTONUP if frame_of(hwnd).is_some_and(|frame| frame.pressed >= 0) => {
                set_pointer(hwnd, -1, -1);
                DefSubclassProc(hwnd, message, wparam, lparam)
            }
            WM_MOUSEMOVE if frame_of(hwnd).is_some_and(|frame| frame.pressed >= 0) => {
                let pressed = frame_of(hwnd).map_or(-1, |frame| frame.pressed);
                set_pointer(hwnd, control_under_cursor(hwnd), pressed);
                DefSubclassProc(hwnd, message, wparam, lparam)
            }

            // winit answers this with the min/max *track* sizes Slint asked for
            // and leaves the maximized geometry at the system's defaults, so the
            // two parts do not conflict: only the latter is replaced here.
            WM_GETMINMAXINFO => {
                let result = DefSubclassProc(hwnd, message, wparam, lparam);
                fit_work_area(hwnd, lparam);
                result
            }

            // Maximizing swaps the maximize control for the restore one, and a
            // DPI change moves and re-scales all of them.
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

/// The hit test for a window that has no non-client area, once the window
/// controls have had their say: the invisible resize border, and nothing else.
///
/// The top edge between the corners answers `HTCLIENT` rather than `HTTOP` or
/// `HTCAPTION`. The client area *is* the title bar here, so either of the other
/// two would take the press away from the app's own controls — and away from
/// `WindowMoveArea`, which is what starts the window drag.
fn hit_test(hwnd: HWND, point: POINT) -> u32 {
    // SAFETY: the window is live and this runs on the thread that owns it; the
    // rectangle is a local the API only writes to.
    unsafe {
        // A maximized window is the work area, edge to edge: there is no border
        // left to grab, and one would only resize into the taskbar.
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

/// The thickness of the invisible border the system keeps around a resizable
/// window: the resize area, not the hairline DWM draws.
///
/// It is the window frame without its caption, which is exactly the part
/// `WM_NCCALCSIZE` hands back to the non-client area. Asking
/// `AdjustWindowRectExForDpi` is what makes this follow the DPI.
fn frame_thickness(hwnd: HWND) -> (i32, i32) {
    // SAFETY: the window is live and this runs on the thread that owns it; the
    // rectangle is a local the API only writes to.
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

/// Makes a maximized window exactly the monitor's work area.
///
/// Left to itself the system grows the window by the frame thickness on every
/// side, so that the *client* area lands on the work area. That is right for a
/// windowed frame and one border too big for this one, whose client area is the
/// whole window: the extra strip would be app content drawn over the taskbar.
fn fit_work_area(hwnd: HWND, lparam: LPARAM) {
    // SAFETY: the window is live and this runs on the thread that owns it;
    // `lparam` is the `MINMAXINFO` of the `WM_GETMINMAXINFO` being answered,
    // which Windows filled in on the way in.
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
