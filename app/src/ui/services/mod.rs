// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Cross-surface UI services.
//!
//! The `ui/globals/*.slint` singletons that no single view owns live behind
//! these modules: delivering a background result onto the event loop, the
//! `AppConfig` adapter, and scrolling.

pub(crate) mod app_config;
pub(crate) mod report;
pub(crate) mod scroll;
