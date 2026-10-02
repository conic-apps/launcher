// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Slint frontend: interface adapters, presenters and drivers.
//!
//! This subtree mirrors `app/ui/`. `views/`, `overlays/` and `components/`
//! correspond to the same buckets there; `services/` holds the cross-surface UI
//! concerns (delivery onto the event loop, the config adapter, scrolling).

pub(crate) mod components;
pub(crate) mod overlays;
pub(crate) mod services;
pub(crate) mod views;
