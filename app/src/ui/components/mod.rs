// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Rust half of the reusable Slint components: drivers and pixel producers
//! for their matching components under `ui/components/`.

pub(crate) mod account_avatar;
pub(crate) mod background;
// The Markdown/HTML renderer keeps the standalone crate's habit of
// `unwrap`/`expect` in internal invariants, so the crate-level deny is opted
// out of here.
#[allow(clippy::unwrap_used)]
pub(crate) mod markdown;
