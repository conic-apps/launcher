// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    ffi::{CString, c_char, c_void},
    path::{Path, PathBuf},
    ptr,
};

use libloader::libloading::{Library, Symbol};
use serde::Serialize;

use crate::{
    error::{Error, Result},
    library::load_library_from_file,
};

pub const CONIC_NEXUS_OK: i32 = 0;
pub const CONIC_NEXUS_ERR_INVALID_HANDLE: i32 = -1;
pub const CONIC_NEXUS_ERR_INVALID_ARGUMENT: i32 = -2;
pub const CONIC_NEXUS_ERR_BAD_STATE: i32 = -3;
pub const CONIC_NEXUS_ERR_INVALID_ROOM_CODE: i32 = -4;
pub const CONIC_NEXUS_ERR_ALREADY_ACTIVE: i32 = -5;
pub const CONIC_NEXUS_ERR_INTERNAL: i32 = -6;
pub const CONIC_NEXUS_ERR_OUT_OF_MEMORY: i32 = -7;
pub const CONIC_NEXUS_ERR_NO_EVENT: i32 = -8;
pub const CONIC_NEXUS_ERR_SHUTTING_DOWN: i32 = -9;

pub type NexusHandle = *mut c_void;

#[repr(C)]
struct ConicNexusString {
    data: *const c_char,
    len: u32,
}

#[repr(C)]
struct ConicNexusEvent {
    sequence: u64,
    r#type: i32,
    payload: ConicNexusString,
}

#[repr(C)]
struct ConicNexusState {
    version: u64,
    state: i32,
    room_code: ConicNexusString,
    detail: ConicNexusString,
}

#[repr(C)]
struct ConicNexusConfig {
    public_nodes: *const ConicNexusString,
    public_nodes_count: u32,
    data_dir: ConicNexusString,
    motd: ConicNexusString,
}

#[repr(C)]
struct ConicNexusPeer {
    hostname: ConicNexusString,
    ipv4: ConicNexusString,
    is_local: i32,
    nat: i32,
}

type ConicNexusCreateFn = unsafe extern "C" fn() -> NexusHandle;
type ConicNexusDestroyFn = unsafe extern "C" fn(handle: NexusHandle);
type ConicNexusConfigureFn =
    unsafe extern "C" fn(handle: NexusHandle, config: *const ConicNexusConfig) -> i32;
type ConicNexusCreateRoomFn = unsafe extern "C" fn(
    handle: NexusHandle,
    player_name: *const c_char,
    room_code: *const c_char,
) -> i32;
type ConicNexusJoinRoomFn = unsafe extern "C" fn(
    handle: NexusHandle,
    room_code: *const c_char,
    player_name: *const c_char,
) -> i32;
type ConicNexusResetToWaitingFn = unsafe extern "C" fn(handle: NexusHandle) -> i32;
type ConicNexusGetStateFn =
    unsafe extern "C" fn(handle: NexusHandle, out: *mut ConicNexusState) -> i32;
type ConicNexusPollEventFn =
    unsafe extern "C" fn(handle: NexusHandle, out: *mut ConicNexusEvent) -> i32;
type ConicNexusRoomCodeIsValidFn = unsafe extern "C" fn(room_code: *const c_char) -> bool;
type ConicNexusVersionFn = unsafe extern "C" fn() -> *const c_char;
type ConicNexusQueryPeersFn = unsafe extern "C" fn(
    handle: NexusHandle,
    out: *mut *mut ConicNexusPeer,
    count: *mut u32,
) -> i32;
type ConicNexusFreePeersFn = unsafe extern "C" fn(peers: *mut ConicNexusPeer, count: u32);
type ConicNexusRecentLogsFn = unsafe extern "C" fn(limit: u32, out: *mut ConicNexusString) -> i32;
type ConicNexusFreeStringFn = unsafe extern "C" fn(value: *mut ConicNexusString);
type ConicNexusFreeStateFn = unsafe extern "C" fn(value: *mut ConicNexusState);
type ConicNexusFreeEventFn = unsafe extern "C" fn(value: *mut ConicNexusEvent);

/// Startup options applied through `conic_nexus_configure` while the session
/// is still waiting.
#[derive(Debug, Clone, Default)]
pub struct SessionConfig {
    pub public_nodes: Vec<String>,
    pub data_dir: Option<PathBuf>,
    pub motd: Option<String>,
}

/// Point-in-time view of the session, mirroring the FFI `state` contract.
#[derive(Debug, Clone, Serialize)]
pub struct SessionState {
    pub version: u64,
    pub state: String,
    pub room_code: String,
    pub detail: serde_json::Value,
}

/// One notice drained from the event queue.
#[derive(Debug, Clone, Serialize)]
pub struct SessionEvent {
    pub sequence: u64,
    #[serde(rename = "type")]
    pub r#type: i32,
    pub payload: serde_json::Value,
}

/// One mesh peer returned by `query_peers`. `nat` uses the EasyTier NAT codes.
#[derive(Debug, Clone, Serialize)]
pub struct PeerInfo {
    pub hostname: String,
    pub ipv4: String,
    pub is_local: bool,
    pub nat: i32,
}

/// A loaded `libconic_nexus` dynamic library plus a live session handle.
///
/// # Safety
///
/// The FFI handle points at a `HandleRecord` leaked by the library after
/// destruction, wrapping an `Arc<ConicContext>` whose internals are mutex
/// guarded, so calling any `conic_nexus_*` function from any thread is sound.
/// `libloading::Library` is `Send + Sync`. Destruction must happen only after
/// the polling thread has been joined and no command is in flight.
pub struct NexusSession {
    library: Library,
    handle: NexusHandle,
}

// Safety: see the `NexusSession` doc comment above.
unsafe impl Send for NexusSession {}
// Safety: see the `NexusSession` doc comment above.
unsafe impl Sync for NexusSession {}

impl NexusSession {
    /// Loads the library from disk, verifies its checksum, creates a session
    /// and returns it unconfigured.
    pub async fn load(path: &Path) -> Result<NexusSession> {
        let library = unsafe { load_library_from_file(path).await? };
        unsafe {
            let create: Symbol<ConicNexusCreateFn> = library.get(b"conic_nexus_create")?;
            let handle = create();
            if handle.is_null() {
                return Err(Error::ConicNexus {
                    code: CONIC_NEXUS_ERR_INTERNAL,
                    description: "conic_nexus_create returned a null handle".into(),
                });
            }
            Ok(NexusSession { library, handle })
        }
    }

    /// Applies a startup configuration. Only valid while the session is
    /// waiting.
    pub fn configure(&self, config: &SessionConfig) -> Result<()> {
        unsafe {
            let configure: Symbol<ConicNexusConfigureFn> = self.symbol(b"conic_nexus_configure")?;
            let public_nodes: Vec<CString> = config
                .public_nodes
                .iter()
                .filter_map(|node| CString::new(node.as_str()).ok())
                .collect();
            let public_node_refs: Vec<ConicNexusString> = public_nodes
                .iter()
                .map(|node| ConicNexusString {
                    data: node.as_ptr(),
                    len: node.as_bytes().len() as u32,
                })
                .collect();
            let data_dir = c_string_of(
                config
                    .data_dir
                    .as_ref()
                    .map(|path| path.to_string_lossy())
                    .as_deref(),
            );
            let motd = c_string_of(config.motd.as_deref());
            let c_config = ConicNexusConfig {
                public_nodes: public_node_refs.as_ptr(),
                public_nodes_count: public_node_refs.len() as u32,
                data_dir: ConicNexusString {
                    data: data_dir.as_ptr(),
                    len: data_dir.as_bytes().len() as u32,
                },
                motd: ConicNexusString {
                    data: motd.as_ptr(),
                    len: motd.as_bytes().len() as u32,
                },
            };
            check_code(configure(self.handle, &c_config))
        }
    }

    /// Starts hosting a room. `room_code` may be `None` to mint a new one.
    pub fn create_room(&self, player_name: Option<&str>, room_code: Option<&str>) -> Result<()> {
        unsafe {
            let create_room: Symbol<ConicNexusCreateRoomFn> =
                self.symbol(b"conic_nexus_create_room")?;
            let player_name = to_c_string(player_name)?;
            let room_code = to_c_string(room_code)?;
            check_code(create_room(
                self.handle,
                player_name.as_deref().map_or(ptr::null(), |c| c.as_ptr()),
                room_code.as_deref().map_or(ptr::null(), |c| c.as_ptr()),
            ))
        }
    }

    /// Joins an existing room.
    pub fn join_room(&self, room_code: &str, player_name: Option<&str>) -> Result<()> {
        unsafe {
            let join_room: Symbol<ConicNexusJoinRoomFn> = self.symbol(b"conic_nexus_join_room")?;
            let room_code = to_c_string(Some(room_code))?;
            let player_name = to_c_string(player_name)?;
            check_code(join_room(
                self.handle,
                room_code.as_deref().map_or(ptr::null(), |c| c.as_ptr()),
                player_name.as_deref().map_or(ptr::null(), |c| c.as_ptr()),
            ))
        }
    }

    /// Aborts the active session and returns to the waiting state.
    pub fn reset_to_waiting(&self) -> Result<()> {
        unsafe {
            let reset: Symbol<ConicNexusResetToWaitingFn> =
                self.symbol(b"conic_nexus_reset_to_waiting")?;
            check_code(reset(self.handle))
        }
    }

    /// Fills the current session snapshot.
    pub fn get_state(&self) -> Result<SessionState> {
        unsafe {
            let get_state: Symbol<ConicNexusGetStateFn> = self.symbol(b"conic_nexus_get_state")?;
            let free_state: Symbol<ConicNexusFreeStateFn> =
                self.symbol(b"conic_nexus_free_state")?;
            let mut out = std::mem::zeroed::<ConicNexusState>();
            check_code(get_state(self.handle, &mut out))?;
            let state = SessionState {
                version: out.version,
                state: state_name(out.state).to_string(),
                room_code: read_string(&out.room_code),
                detail: serde_json::from_str(&read_string(&out.detail))
                    .unwrap_or(serde_json::Value::Null),
            };
            free_state(&mut out);
            Ok(state)
        }
    }

    /// Pops the next event, or `None` when the queue is empty.
    pub fn poll_event(&self) -> Result<Option<SessionEvent>> {
        unsafe {
            let poll_event: Symbol<ConicNexusPollEventFn> =
                self.symbol(b"conic_nexus_poll_event")?;
            let free_event: Symbol<ConicNexusFreeEventFn> =
                self.symbol(b"conic_nexus_free_event")?;
            let mut out = std::mem::zeroed::<ConicNexusEvent>();
            let code = poll_event(self.handle, &mut out);
            if code == CONIC_NEXUS_ERR_NO_EVENT {
                return Ok(None);
            }
            check_code(code)?;
            let event = SessionEvent {
                sequence: out.sequence,
                r#type: out.r#type,
                payload: serde_json::from_str(&read_string(&out.payload))
                    .unwrap_or(serde_json::Value::Null),
            };
            free_event(&mut out);
            Ok(Some(event))
        }
    }

    /// Reports whether `room_code` is well-formed. A pure query; never fails.
    pub fn room_code_is_valid(&self, room_code: &str) -> bool {
        unsafe {
            let Ok(is_valid) =
                self.symbol::<ConicNexusRoomCodeIsValidFn>(b"conic_nexus_room_code_is_valid")
            else {
                return false;
            };
            let Ok(room_code) = CString::new(room_code) else {
                return false;
            };
            is_valid(room_code.as_ptr())
        }
    }

    /// Returns the static version string of the library.
    pub fn version(&self) -> String {
        unsafe {
            let Ok(version) = self.symbol::<ConicNexusVersionFn>(b"conic_nexus_version") else {
                return String::new();
            };
            let ptr = version();
            if ptr.is_null() {
                return String::new();
            }
            std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    }

    /// Queries the active mesh node for its peers and their NAT types.
    pub fn query_peers(&self) -> Result<Vec<PeerInfo>> {
        unsafe {
            let query_peers: Symbol<ConicNexusQueryPeersFn> =
                self.symbol(b"conic_nexus_query_peers")?;
            let free_peers: Symbol<ConicNexusFreePeersFn> =
                self.symbol(b"conic_nexus_free_peers")?;
            let mut out = ptr::null_mut();
            let mut count: u32 = 0;
            check_code(query_peers(self.handle, &mut out, &mut count))?;
            let mut peers = Vec::with_capacity(count as usize);
            for index in 0..count as usize {
                // SAFETY: `out` points at an array of `count` peers allocated
                // by the library, valid until `free_peers` is called.
                let peer = &*out.add(index);
                peers.push(PeerInfo {
                    hostname: read_string(&peer.hostname),
                    ipv4: read_string(&peer.ipv4),
                    is_local: peer.is_local != 0,
                    nat: peer.nat,
                });
            }
            free_peers(out, count);
            Ok(peers)
        }
    }

    /// Fills `out` with the `limit` most recent log lines as a JSON array.
    pub fn recent_logs(&self, limit: u32) -> Result<Vec<String>> {
        unsafe {
            let recent_logs: Symbol<ConicNexusRecentLogsFn> =
                self.symbol(b"conic_nexus_recent_logs")?;
            let free_string: Symbol<ConicNexusFreeStringFn> =
                self.symbol(b"conic_nexus_free_string")?;
            let mut out = std::mem::zeroed::<ConicNexusString>();
            check_code(recent_logs(limit, &mut out))?;
            let json = read_string(&out);
            free_string(&mut out);
            Ok(serde_json::from_str(&json).unwrap_or_default())
        }
    }

    /// Destroys the session. After this call the handle is permanently invalid.
    pub fn destroy(&self) {
        unsafe {
            let Ok(destroy) = self
                .library
                .get::<ConicNexusDestroyFn>(b"conic_nexus_destroy")
            else {
                return;
            };
            destroy(self.handle);
        }
    }

    unsafe fn symbol<'a, T>(&'a self, name: &[u8]) -> Result<Symbol<'a, T>> {
        // SAFETY: resolving a symbol from an already-loaded library is a plain
        // pointer lookup; the returned symbol is valid for `self`'s lifetime.
        Ok(unsafe { self.library.get(name) }?)
    }
}

fn check_code(code: i32) -> Result<()> {
    if code == CONIC_NEXUS_OK {
        Ok(())
    } else {
        Err(Error::ConicNexus {
            code,
            description: describe_result_code(code).into(),
        })
    }
}

fn describe_result_code(code: i32) -> &'static str {
    match code {
        CONIC_NEXUS_OK => "ok",
        CONIC_NEXUS_ERR_INVALID_HANDLE => "invalid handle",
        CONIC_NEXUS_ERR_INVALID_ARGUMENT => "invalid argument",
        CONIC_NEXUS_ERR_BAD_STATE => "bad state",
        CONIC_NEXUS_ERR_INVALID_ROOM_CODE => "invalid room code",
        CONIC_NEXUS_ERR_ALREADY_ACTIVE => "already active",
        CONIC_NEXUS_ERR_INTERNAL => "internal error",
        CONIC_NEXUS_ERR_OUT_OF_MEMORY => "out of memory",
        CONIC_NEXUS_ERR_NO_EVENT => "no event",
        CONIC_NEXUS_ERR_SHUTTING_DOWN => "shutting down",
        _ => "unknown error",
    }
}

fn state_name(state: i32) -> &'static str {
    match state {
        0 => "waiting",
        1 => "host-scanning",
        2 => "host-starting",
        3 => "host-ok",
        4 => "guest-connecting",
        5 => "guest-starting",
        6 => "guest-ok",
        7 => "exception",
        _ => "unknown",
    }
}

unsafe fn read_string(value: &ConicNexusString) -> String {
    if value.data.is_null() || value.len == 0 {
        return String::new();
    }
    // SAFETY: the C ABI guarantees `data`/`len` describe a valid UTF-8 span
    // owned by the library for the duration of the call.
    let bytes = unsafe { std::slice::from_raw_parts(value.data as *const u8, value.len as usize) };
    String::from_utf8_lossy(bytes).into_owned()
}

fn to_c_string(value: Option<&str>) -> Result<Option<CString>> {
    value
        .map(|value| {
            CString::new(value).map_err(|_| Error::ConicNexus {
                code: CONIC_NEXUS_ERR_INVALID_ARGUMENT,
                description: "invalid UTF-8 string".into(),
            })
        })
        .transpose()
}

fn c_string_of(value: Option<&str>) -> CString {
    CString::new(value.unwrap_or_default()).unwrap_or_default()
}
