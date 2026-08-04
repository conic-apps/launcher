// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! C ABI bindings for `libconic_terracotta` (see `conic-terracotta/include/terracotta.h`).
//!
//! This mirrors the header exactly. No conic-terracotta Rust types are used;
//! every interaction happens through the dynamic library's exported symbols.
//!
//! Lifecycle: the [`Terracotta`] wrapper owns the loaded [`Library`] and must
//! be kept alive for the whole launcher run. `terracotta_create()` spawns the
//! library's own runtime thread and extracts an embedded EasyTier binary, and
//! `terracotta_destroy()` intentionally leaks its handle record, so the library
//! may be loaded only once per process. Contexts are created/destroyed through
//! [`Terracotta::create_context`] / [`Terracotta::destroy_context`].

use std::ffi::{CStr, CString, c_char, c_void};
use std::os::raw::c_int;
use std::sync::Mutex;

use libloader::libloading::{Library, Symbol};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// The result codes returned by the terracotta C API (see terracotta.h).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TerraResult {
    Ok,
    InvalidHandle,
    InvalidArgument,
    BadState,
    InvalidRoomCode,
    AlreadyActive,
    Internal,
    OutOfMemory,
    NoEvent,
    ShuttingDown,
}

impl std::fmt::Display for TerraResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

pub const TERRA_OK: i32 = 0;
pub const TERRA_ERR_INVALID_HANDLE: i32 = -1;
pub const TERRA_ERR_INVALID_ARGUMENT: i32 = -2;
pub const TERRA_ERR_BAD_STATE: i32 = -3;
pub const TERRA_ERR_INVALID_ROOM_CODE: i32 = -4;
pub const TERRA_ERR_ALREADY_ACTIVE: i32 = -5;
pub const TERRA_ERR_INTERNAL: i32 = -6;
pub const TERRA_ERR_OUT_OF_MEMORY: i32 = -7;
pub const TERRA_ERR_NO_EVENT: i32 = -8;
pub const TERRA_ERR_SHUTTING_DOWN: i32 = -9;

pub fn terra_result_from_code(code: i32) -> TerraResult {
    match code {
        TERRA_OK => TerraResult::Ok,
        TERRA_ERR_INVALID_HANDLE => TerraResult::InvalidHandle,
        TERRA_ERR_INVALID_ARGUMENT => TerraResult::InvalidArgument,
        TERRA_ERR_BAD_STATE => TerraResult::BadState,
        TERRA_ERR_INVALID_ROOM_CODE => TerraResult::InvalidRoomCode,
        TERRA_ERR_ALREADY_ACTIVE => TerraResult::AlreadyActive,
        TERRA_ERR_INTERNAL => TerraResult::Internal,
        TERRA_ERR_OUT_OF_MEMORY => TerraResult::OutOfMemory,
        TERRA_ERR_NO_EVENT => TerraResult::NoEvent,
        TERRA_ERR_SHUTTING_DOWN => TerraResult::ShuttingDown,
        _ => TerraResult::Internal,
    }
}

/// An opaque handle to a terracotta context owned by the library.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct TerracottaHandle(*mut c_void);

impl TerracottaHandle {
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

unsafe impl Send for TerracottaHandle {}
unsafe impl Sync for TerracottaHandle {}

/// A string owned by the library (see terracotta.h).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TerracottaString {
    data: *const c_char,
    len: u32,
}

impl Default for TerracottaString {
    fn default() -> Self {
        TerracottaString {
            data: std::ptr::null(),
            len: 0,
        }
    }
}

/// An event popped from the library's event queue (see terracotta.h).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct TerracottaEventRaw {
    pub sequence: u64,
    pub r#type: i32,
    pub payload: TerracottaString,
}

/// A full state snapshot returned by the library (see terracotta.h).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct TerracottaStateRaw {
    pub version: u64,
    pub state: i32,
    pub room_code: TerracottaString,
    pub detail: TerracottaString,
}

/// Configuration passed to terracotta_configure (see terracotta.h).
#[repr(C)]
pub struct TerracottaConfigRaw {
    public_nodes: *const TerracottaString,
    public_nodes_count: u32,
    data_dir: TerracottaString,
    motd: TerracottaString,
}

/// Configuration for a terracotta context (serialized to/from the frontend).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerracottaConfig {
    pub public_nodes: Vec<String>,
    pub data_dir: Option<String>,
    pub motd: Option<String>,
}

/// A full state snapshot (serialized to the frontend).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerracottaState {
    pub version: u64,
    pub state: i32,
    pub room_code: String,
    pub detail: serde_json::Value,
}

/// An event popped from the event queue (serialized to the frontend).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerracottaEvent {
    pub sequence: u64,
    pub r#type: i32,
    pub payload: serde_json::Value,
}

/// A loaded `libconic_terracotta` library with all exported symbols resolved.
///
/// The [`Library`] field is declared last so that struct drop order tears down
/// the symbols before the library is unloaded.
pub struct Terracotta {
    create: Symbol<'static, unsafe extern "C" fn() -> TerracottaHandle>,
    destroy: Symbol<'static, unsafe extern "C" fn(TerracottaHandle)>,
    configure:
        Symbol<'static, unsafe extern "C" fn(TerracottaHandle, *const TerracottaConfigRaw) -> i32>,
    create_room: Symbol<
        'static,
        unsafe extern "C" fn(TerracottaHandle, *const c_char, *const c_char) -> i32,
    >,
    join_room: Symbol<
        'static,
        unsafe extern "C" fn(TerracottaHandle, *const c_char, *const c_char) -> i32,
    >,
    set_waiting: Symbol<'static, unsafe extern "C" fn(TerracottaHandle) -> i32>,
    get_state:
        Symbol<'static, unsafe extern "C" fn(TerracottaHandle, *mut TerracottaStateRaw) -> i32>,
    poll_event:
        Symbol<'static, unsafe extern "C" fn(TerracottaHandle, *mut TerracottaEventRaw) -> i32>,
    verify_room_code: Symbol<'static, unsafe extern "C" fn(*const c_char) -> c_int>,
    version: Symbol<'static, unsafe extern "C" fn() -> *const c_char>,
    /// Kept for the complete ABI binding (see terracotta.h); unused because
    /// `free_state` / `free_event` release every string we receive.
    #[allow(dead_code)]
    free_string: Symbol<'static, unsafe extern "C" fn(*mut TerracottaString)>,
    free_state: Symbol<'static, unsafe extern "C" fn(*mut TerracottaStateRaw)>,
    free_event: Symbol<'static, unsafe extern "C" fn(*mut TerracottaEventRaw)>,
    handle: Mutex<Option<TerracottaHandle>>,
    /// Keeps the dynamic library loaded; never accessed directly.
    _lib: Library,
}

unsafe fn symbol<'lib, T>(lib: &'lib Library, name: &[u8]) -> Result<Symbol<'static, T>> {
    let symbol = unsafe { lib.get::<T>(name)? };
    // The Library is stored in `Terracotta` and kept alive for the entire
    // application, so the 'static borrow is sound.
    Ok(unsafe { std::mem::transmute::<Symbol<'lib, T>, Symbol<'static, T>>(symbol) })
}

/// # Safety
///
/// The `Library` must be a valid handle to a conic-terracotta library. The
/// resulting [`Terracotta`] must be kept alive for the whole process; dropping
/// it unloads the library and invalidates any active context.
pub unsafe fn terracotta_from_library(lib: Library) -> Result<Terracotta> {
    unsafe {
        let create =
            symbol::<unsafe extern "C" fn() -> TerracottaHandle>(&lib, b"terracotta_create")?;
        let destroy =
            symbol::<unsafe extern "C" fn(TerracottaHandle)>(&lib, b"terracotta_destroy")?;
        let configure = symbol::<
            unsafe extern "C" fn(TerracottaHandle, *const TerracottaConfigRaw) -> i32,
        >(&lib, b"terracotta_configure")?;
        let create_room = symbol::<
            unsafe extern "C" fn(TerracottaHandle, *const c_char, *const c_char) -> i32,
        >(&lib, b"terracotta_create_room")?;
        let join_room = symbol::<
            unsafe extern "C" fn(TerracottaHandle, *const c_char, *const c_char) -> i32,
        >(&lib, b"terracotta_join_room")?;
        let set_waiting = symbol::<unsafe extern "C" fn(TerracottaHandle) -> i32>(
            &lib,
            b"terracotta_set_waiting",
        )?;
        let get_state = symbol::<
            unsafe extern "C" fn(TerracottaHandle, *mut TerracottaStateRaw) -> i32,
        >(&lib, b"terracotta_get_state")?;
        let poll_event = symbol::<
            unsafe extern "C" fn(TerracottaHandle, *mut TerracottaEventRaw) -> i32,
        >(&lib, b"terracotta_poll_event")?;
        let verify_room_code = symbol::<unsafe extern "C" fn(*const c_char) -> c_int>(
            &lib,
            b"terracotta_verify_room_code",
        )?;
        let version =
            symbol::<unsafe extern "C" fn() -> *const c_char>(&lib, b"terracotta_version")?;
        let free_string =
            symbol::<unsafe extern "C" fn(*mut TerracottaString)>(&lib, b"terracotta_free_string")?;
        let free_state = symbol::<unsafe extern "C" fn(*mut TerracottaStateRaw)>(
            &lib,
            b"terracotta_free_state",
        )?;
        let free_event = symbol::<unsafe extern "C" fn(*mut TerracottaEventRaw)>(
            &lib,
            b"terracotta_free_event",
        )?;

        Ok(Terracotta {
            create,
            destroy,
            configure,
            create_room,
            join_room,
            set_waiting,
            get_state,
            poll_event,
            verify_room_code,
            version,
            free_string,
            free_state,
            free_event,
            handle: Mutex::new(None),
            _lib: lib,
        })
    }
}

impl Terracotta {
    /// Returns the library version string (e.g. "0.1.0"). Never NULL.
    pub fn version(&self) -> String {
        let ptr = unsafe { (self.version)() };
        if ptr.is_null() {
            return String::new();
        }
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }

    /// Verifies a room code without needing an active context.
    pub fn verify_room_code(&self, room_code: &str) -> bool {
        let Ok(code) = CString::new(room_code) else {
            return false;
        };
        unsafe { (self.verify_room_code)(code.as_ptr()) == 3 }
    }

    fn handle(&self) -> Result<TerracottaHandle> {
        self.handle
            .lock()
            .expect("Internal error")
            .ok_or(Error::NoContext)
    }

    /// Creates a context and applies `config`. Fails if a context already exists.
    pub fn create_context(&self, config: &TerracottaConfig) -> Result<()> {
        if self.handle.lock().expect("Internal error").is_some() {
            return Err(Error::ContextAlreadyExists);
        }
        let handle = unsafe { (self.create)() };
        if handle.is_null() {
            return Err(Error::TerraResult(TerraResult::Internal));
        }
        let result = self.configure(handle, config)?;
        if result != TERRA_OK {
            unsafe { (self.destroy)(handle) };
            return Err(Error::TerraResult(terra_result_from_code(result)));
        }
        *self.handle.lock().expect("Internal error") = Some(handle);
        Ok(())
    }

    /// Destroys the active context (if any), killing EasyTier / the fake server
    /// and stopping the library's runtime thread.
    pub fn destroy_context(&self) {
        if let Some(handle) = self.handle.lock().expect("Internal error").take() {
            unsafe { (self.destroy)(handle) };
        }
    }

    /// Hosts a room. `room_code == None` generates a new room; otherwise it must
    /// be a valid "U/XXXX-XXXX-XXXX-XXXX" code.
    pub fn create_room(&self, player_name: Option<&str>, room_code: Option<&str>) -> Result<()> {
        let handle = self.handle()?;
        let name = cstr_or_null(player_name)?;
        let code = cstr_or_null(room_code)?;
        let name_ptr = name
            .as_deref()
            .map(CStr::as_ptr)
            .unwrap_or(std::ptr::null());
        let code_ptr = code
            .as_deref()
            .map(CStr::as_ptr)
            .unwrap_or(std::ptr::null());
        let result = unsafe { (self.create_room)(handle, name_ptr, code_ptr) };
        check_result(result)
    }

    /// Joins an existing room. `room_code` is required and validated.
    pub fn join_room(&self, room_code: &str, player_name: Option<&str>) -> Result<()> {
        let handle = self.handle()?;
        let code = CString::new(room_code).map_err(Error::Nul)?;
        let name = cstr_or_null(player_name)?;
        let name_ptr = name
            .as_deref()
            .map(CStr::as_ptr)
            .unwrap_or(std::ptr::null());
        let result = unsafe { (self.join_room)(handle, code.as_ptr(), name_ptr) };
        check_result(result)
    }

    /// Aborts any active session and returns to the `Waiting` state.
    pub fn set_waiting(&self) -> Result<()> {
        let handle = self.handle()?;
        let result = unsafe { (self.set_waiting)(handle) };
        check_result(result)
    }

    /// Returns a full state snapshot.
    pub fn get_state(&self) -> Result<TerracottaState> {
        let handle = self.handle()?;
        let mut raw = TerracottaStateRaw::default();
        let result = unsafe { (self.get_state)(handle, &mut raw) };
        if result != TERRA_OK {
            return Err(Error::TerraResult(terra_result_from_code(result)));
        }
        let state = TerracottaState {
            version: raw.version,
            state: raw.state,
            room_code: self.read_string(&raw.room_code),
            detail: parse_json(&self.read_string(&raw.detail)),
        };
        unsafe { (self.free_state)(&mut raw) };
        Ok(state)
    }

    /// Pops the next pending event, if any. Non-blocking.
    pub fn poll_event(&self) -> Result<Option<TerracottaEvent>> {
        let handle = self.handle()?;
        let mut raw = TerracottaEventRaw::default();
        let result = unsafe { (self.poll_event)(handle, &mut raw) };
        match result {
            TERRA_ERR_NO_EVENT => Ok(None),
            TERRA_OK => {
                let event = TerracottaEvent {
                    sequence: raw.sequence,
                    r#type: raw.r#type,
                    payload: parse_json(&self.read_string(&raw.payload)),
                };
                unsafe { (self.free_event)(&mut raw) };
                Ok(Some(event))
            }
            _ => Err(Error::TerraResult(terra_result_from_code(result))),
        }
    }

    /// Copies a library-owned string into an owned [`String`]. The caller must
    /// release the original with the matching `terracotta_free_*` function.
    fn read_string(&self, string: &TerracottaString) -> String {
        if string.data.is_null() || string.len == 0 {
            return String::new();
        }
        let bytes =
            unsafe { std::slice::from_raw_parts(string.data as *const u8, string.len as usize) };
        String::from_utf8_lossy(bytes).into_owned()
    }

    /// Applies `config` via `terracotta_configure`. The CStrings backing the
    /// raw struct stay alive for the duration of the call.
    fn configure(&self, handle: TerracottaHandle, config: &TerracottaConfig) -> Result<i32> {
        let public_nodes = config
            .public_nodes
            .iter()
            .map(|node| CString::new(node.as_str()).map_err(Error::Nul))
            .collect::<Result<Vec<_>>>()?;
        let nodes: Vec<TerracottaString> = public_nodes
            .iter()
            .map(|node| TerracottaString {
                data: node.as_ptr(),
                len: node.as_bytes().len() as u32,
            })
            .collect();
        let data_dir = config
            .data_dir
            .as_deref()
            .map(|value| CString::new(value).map_err(Error::Nul))
            .transpose()?;
        let motd = config
            .motd
            .as_deref()
            .map(|value| CString::new(value).map_err(Error::Nul))
            .transpose()?;
        let raw = TerracottaConfigRaw {
            public_nodes: nodes.as_ptr(),
            public_nodes_count: nodes.len() as u32,
            data_dir: string_or_default(data_dir.as_ref()),
            motd: string_or_default(motd.as_ref()),
        };
        Ok(unsafe { (self.configure)(handle, &raw) })
    }
}

fn check_result(code: i32) -> Result<()> {
    if code == TERRA_OK {
        Ok(())
    } else {
        Err(Error::TerraResult(terra_result_from_code(code)))
    }
}

fn cstr_or_null(value: Option<&str>) -> Result<Option<CString>> {
    value
        .map(|value| CString::new(value).map_err(Error::Nul))
        .transpose()
}

fn parse_json(value: &str) -> serde_json::Value {
    serde_json::from_str(value).unwrap_or_else(|_| serde_json::Value::String(value.to_string()))
}

fn string_or_default(value: Option<&CString>) -> TerracottaString {
    match value {
        Some(value) => TerracottaString {
            data: value.as_ptr(),
            len: value.as_bytes().len() as u32,
        },
        None => TerracottaString::default(),
    }
}
