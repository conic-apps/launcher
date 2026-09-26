// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Single-instance guard on Windows, over a named mutex and a hidden window.
//!
//! A mutex named after [`crate::APP_ID`] is what tells the two launches apart:
//! taking it succeeds for the first process and reports `ERROR_ALREADY_EXISTS`
//! for every one after it, and the system drops the name when the last handle
//! to it closes, which is what makes a crashed primary recoverable. The primary
//! also creates a message window under a class name derived from the same id,
//! because a mutex cannot carry a payload: a later launch finds that window with
//! `FindWindowW` and hands it its command line with `WM_COPYDATA`. This is the
//! pair `tauri-plugin-single-instance` uses on Windows.
//!
//! The window is never shown and never activated (`WS_EX_NOACTIVATE`,
//! `WS_EX_TOOLWINDOW`, `WS_EX_LAYERED`, and no `WS_VISIBLE`), so it is
//! invisible, stays out of the taskbar and cannot take the focus; it exists to
//! be found and to receive the message. `WM_COPYDATA` does not need a visible
//! window, and it is the only message the loop ever sends it.

use std::{
    ffi::OsStr,
    os::windows::ffi::OsStrExt,
    sync::{Mutex, OnceLock, mpsc::Sender},
    time::{Duration, Instant},
};

use windows_sys::Win32::{
    Foundation::{
        CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, LRESULT,
        SetLastError, WPARAM,
    },
    System::{
        DataExchange::COPYDATASTRUCT,
        LibraryLoader::GetModuleHandleW,
        Threading::{CreateMutexW, GetCurrentThreadId, ReleaseMutex},
    },
    UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, FindWindowW, RegisterClassExW,
        SendMessageW, WM_COPYDATA, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
        WS_EX_TRANSPARENT, WS_POPUP,
    },
};

use crate::framing::{decode, encode};
use crate::{APP_ID, AlreadyRunning, Launch, current_launch, report};

/// The id the message's payload carries, so a `WM_COPYDATA` from some other
/// program that guessed the window's name is not read as a launch.
const WM_COPYDATA_SINGLE_INSTANCE_DATA: usize = 1542;

/// How long a later launch waits for the primary to publish its window.
///
/// The primary takes the mutex and creates the window as two separate steps, so
/// a launch that lands between them finds the mutex taken and no window yet.
/// The window follows within microseconds, so this only has to cover a slow
/// scheduler — and it is a last resort either way: past it the launch starts on
/// its own rather than keeping the user waiting.
const WINDOW_WAIT: Duration = Duration::from_secs(2);

/// Where a reported launch goes, for the message window to hand over.
///
/// There is one single-instance window per process, so a process-wide slot is
/// enough and saves the window any user data of its own.
static LAUNCHES: OnceLock<Mutex<Option<Sender<Launch>>>> = OnceLock::new();

fn launches() -> &'static Mutex<Option<Sender<Launch>>> {
    LAUNCHES.get_or_init(|| Mutex::new(None))
}

/// Claims the role by taking the mutex, and publishes the window a later launch
/// reports to.
///
/// # Errors
///
/// [`AlreadyRunning`] once the launch has been handed to the process that took
/// the mutex first.
pub(crate) fn claim(launches: Sender<Launch>) -> Result<Guard, AlreadyRunning> {
    let class_name = wide(&format!("{APP_ID}-sic"));
    let window_name = wide(&format!("{APP_ID}-siw"));
    let mutex_name = wide(&format!("{APP_ID}-sim"));

    // The mutex is asked for owned (`TRUE`), so a process that takes the name
    // holds it for as long as it lives.
    let mutex = unsafe { CreateMutexW(std::ptr::null(), true.into(), mutex_name.as_ptr()) };
    if mutex.is_null() {
        log::warn!(target: "shell", "single instance is unavailable: the mutex was not created");
        return Ok(Guard::default());
    }

    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        return match find_window(&class_name, &window_name) {
            Some(window) => {
                if let Err(error) = send_launch(window) {
                    log::warn!(target: "shell", "the running instance was not notified: {error}");
                }
                unsafe { CloseHandle(mutex) };
                Err(AlreadyRunning)
            }
            None => {
                // The name is taken, but nothing answers on it, and the only
                // thing that could is the process holding it. Stand down rather
                // than fight it for the name.
                unsafe { CloseHandle(mutex) };
                log::warn!(
                    target: "shell",
                    "another instance holds the single-instance name but never published its window; \
                     starting on its own"
                );
                Ok(Guard::default())
            }
        };
    }

    *lock_launches() = Some(launches);
    match create_window(&class_name, &window_name) {
        Ok(window) => Ok(Guard {
            mutex: Some(mutex),
            window: Some(window),
            owner: current_thread(),
        }),
        Err(error) => {
            // The mutex is still ours, so every later launch would wait out
            // `WINDOW_WAIT` for a window that is never coming; give the name
            // back and run unguarded, which is no worse and does not make every
            // later launch wait.
            log::warn!(target: "shell", "single instance is unavailable: {error}");
            drop(Guard {
                mutex: Some(mutex),
                window: None,
                owner: current_thread(),
            });
            Ok(Guard::default())
        }
    }
}

/// Looks for the primary's window, waiting briefly for one that is on its way.
fn find_window(class_name: &[u16], window_name: &[u16]) -> Option<HWND> {
    let deadline = Instant::now() + WINDOW_WAIT;
    loop {
        let window = unsafe { FindWindowW(class_name.as_ptr(), window_name.as_ptr()) };
        if !window.is_null() {
            return Some(window);
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Registers the window class and creates the hidden message window.
fn create_window(class_name: &[u16], window_name: &[u16]) -> Result<HWND, std::io::Error> {
    unsafe {
        let class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: GetModuleHandleW(std::ptr::null()),
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        if RegisterClassExW(&class) == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let window = CreateWindowExW(
            WS_EX_NOACTIVATE
                | WS_EX_TRANSPARENT
                | WS_EX_LAYERED
                // A tool window is kept out of the taskbar and the Alt+Tab list;
                // without it a zero-size popup can turn up in one of them.
                | WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            window_name.as_ptr(),
            WS_POPUP,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            GetModuleHandleW(std::ptr::null()),
            std::ptr::null(),
        );
        if window.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        Ok(window)
    }
}

/// Sends this launch to the primary's window.
///
/// `SendMessageW` rather than `PostMessageW`, so the launch is delivered before
/// this process exits — a posted message would be lost the moment it did, and
/// it has nothing left to do once it has sent one.
fn send_launch(window: HWND) -> Result<(), std::io::Error> {
    let launch = current_launch();
    // `WM_COPYDATA` carries a counted block of bytes, which for a string is a
    // NUL terminated one. The terminator is not part of the report, hence the
    // `cbData` past the end of it.
    let mut payload = encode(&launch).into_bytes();
    payload.push(0);
    unsafe {
        let data = COPYDATASTRUCT {
            dwData: WM_COPYDATA_SINGLE_INSTANCE_DATA,
            cbData: payload.len() as u32,
            lpData: payload.as_ptr().cast_mut().cast(),
        };
        // The window procedure answers a `WM_COPYDATA` with 0, so the return
        // value says nothing on its own. A failure is reported through the last
        // error, and only reliably if it was cleared first — a call that
        // succeeds leaves the previous value in place.
        SetLastError(0);
        let sent = SendMessageW(
            window,
            WM_COPYDATA,
            0,
            &data as *const COPYDATASTRUCT as LPARAM,
        );
        if sent == 0 && GetLastError() != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

/// The message window's window procedure.
///
/// The only message it is interested in is [`WM_COPYDATA`]; everything else —
/// including the `WM_CREATE` and `WM_DESTROY` of a window that is never visible
/// — goes to `DefWindowProcW`.
unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_COPYDATA {
        // The message carries a pointer to a `COPYDATASTRUCT` that owns its
        // payload, and it is the primary that filled it in, so the only thing
        // to check is that the payload is one of ours.
        let data = unsafe { &*(lparam as *const COPYDATASTRUCT) };
        if data.dwData == WM_COPYDATA_SINGLE_INSTANCE_DATA && !data.lpData.is_null() {
            let raw = unsafe {
                std::slice::from_raw_parts(data.lpData.cast::<u8>(), data.cbData as usize)
            };
            let raw = String::from_utf8_lossy(raw);
            if let Some(launches) = lock_launches().as_ref()
                && let Some(launch) = decode(raw.strip_suffix('\0').unwrap_or(&raw))
            {
                report(launches, launch);
            }
        }
        return 0;
    }
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

/// The claim: the mutex name, and the window later launches report to.
///
/// Both are given up when this is dropped, which is what tells the next launch
/// the role is free.
#[derive(Default)]
pub struct Guard {
    /// The handle to the named mutex, released and closed on drop.
    mutex: Option<HANDLE>,

    /// The hidden message window, destroyed on drop.
    window: Option<HWND>,

    /// The thread that created the window, which is the only one allowed to
    /// destroy it. The claim is handed to the app's watcher thread, so this is
    /// usually a different one; the process ending destroys the window anyway.
    owner: u32,
}

// The handles are `*mut c_void`, which is not `Send` because nothing about the
// type says so — they are opaque kernel and user32 handles, not pointers into
// this process' memory. Every operation on them is thread-safe except
// destroying the window, and `Drop` only does that on the thread that created
// it. Moving the claim to a watcher thread is what requires this; the app's
// `watch_launches` does exactly that.
unsafe impl Send for Guard {}

impl Drop for Guard {
    fn drop(&mut self) {
        if let Some(window) = self.window.take()
            && current_thread() == self.owner
        {
            unsafe { DestroyWindow(window) };
        }
        if let Some(mutex) = self.mutex.take() {
            unsafe {
                ReleaseMutex(mutex);
                CloseHandle(mutex);
            }
        }
    }
}

/// The calling thread's id, which the window and the mutex want to compare
/// against the thread that created them.
fn current_thread() -> u32 {
    unsafe { GetCurrentThreadId() }
}

/// The launch feed the message window reports to.
///
/// The window procedure runs on the thread that owns the window, and dropping
/// the lock there is a normal shutdown, so a poisoned lock is recovered rather
/// than propagated.
fn lock_launches() -> std::sync::MutexGuard<'static, Option<Sender<Launch>>> {
    launches()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Encodes a string as a Win32 name, which is a NUL terminated UTF-16 sequence.
fn wide(string: &str) -> Vec<u16> {
    OsStr::new(string)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
