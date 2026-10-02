// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::io;

pub type Result<T> = std::result::Result<T, Error>;

/// The only way starting the listener can fail: the loopback socket could not
/// be bound. Everything after that is reported to the browser as a page and to
/// the caller as an [`Outcome`](crate::authcode::Outcome).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to bind the loopback callback listener: {0}")]
    Bind(#[source] io::Error),
}
