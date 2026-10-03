// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Process and OS plumbing shared by the app's layers.
//!
//! `usecases/` and `ui/` may both use it, and it uses neither — with one
//! exception: `native/` is the window-shell driver (the winit backend hook and
//! the platform's window chrome), so it is the one part here that touches
//! `slint_backend`.

pub(crate) mod formatting;
pub(crate) mod json;
pub(crate) mod logs;
pub(crate) mod native;
pub(crate) mod runtime;
