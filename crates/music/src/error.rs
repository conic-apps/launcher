// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Errors of the music crate (mirror of `crates/music/src/error.rs`).
//!
//! The original tags its variants for the Tauri IPC boundary, which a
//! Tauri-free mirror does not have, so the `Serialize`/`serde_with` derives are
//! gone. The variants themselves are the original's `Io` plus what the playback
//! engine — the part the webview used to do with an `<audio>` element and the
//! Web Audio API — can fail at.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The file is not an audio file the decoders understand, or its container
    /// is one they do not read. The webview reported this through
    /// `audio.onerror`; there is nothing left of that here, so it is a value.
    #[error("could not decode '{path}': {message}")]
    Decode { path: String, message: String },
    /// The platform has no output device the music could be played on.
    #[error("no audio output device is available")]
    NoOutputDevice,
    /// The output device refused the stream, or dropped it while playing.
    #[error("audio output failed: {0}")]
    Output(String),
}
