// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Errors of the music crate.
//!
//! [`Error::Io`] covers the filesystem; the rest cover what the playback engine
//! — decoding, the output device — can fail at.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The file is not an audio file the decoders understand, or its container
    /// is one they do not read. Reported as a value rather than a panic, so the
    /// caller can show it.
    #[error("could not decode '{path}': {message}")]
    Decode { path: String, message: String },
    /// The platform has no output device the music could be played on.
    #[error("no audio output device is available")]
    NoOutputDevice,
    /// The output device refused the stream, or dropped it while playing.
    #[error("audio output failed: {0}")]
    Output(String),
}
