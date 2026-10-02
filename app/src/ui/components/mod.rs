// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Rust half of the reusable Slint components.
//!
//! Each module backs the matching component under `app/ui/components/`: they
//! are drivers and pixel producers, not use cases, and they are the only place
//! the app builds a Slint `Image` or a display list by hand.

pub(crate) mod account_avatar;
pub(crate) mod background;
// The Markdown/HTML renderer keeps the standalone crate's habit of
// `unwrap`/`expect` in internal invariants, so the crate-level deny is opted
// out of here.
#[allow(clippy::unwrap_used)]
pub(crate) mod markdown;
