// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Process and OS plumbing shared by the app's layers.
//!
//! Nothing here touches the UI: `usecases/` and `ui/` may both use it, and it
//! uses neither.

pub(crate) mod formatting;
pub(crate) mod json;
pub(crate) mod logs;
pub(crate) mod platform;
pub(crate) mod runtime;
