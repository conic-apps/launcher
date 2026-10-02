// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The app's use cases: UI-neutral orchestration over the domain crates.
//!
//! Nothing here mentions Slint, the `ui/` modules or `slint_backend`. A frontend
//! (`ui/`) owns the state and the ports, and calls into these; a second frontend
//! could call the same functions.

pub(crate) mod account_login;
pub(crate) mod game;
pub(crate) mod generation;
pub(crate) mod launch;
