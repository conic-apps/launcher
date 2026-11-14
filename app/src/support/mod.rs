// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Process and OS plumbing shared by the app's layers. `usecases/` and `ui/` may
//! both use it, and it uses neither — except `native/`, the window-shell driver,
//! which touches `slint_backend`.

pub(crate) mod formatting;
pub(crate) mod json;
pub(crate) mod logs;
pub(crate) mod native;
pub(crate) mod runtime;
