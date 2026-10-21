// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Assets the Rust side hands to Slint as bytes.
//!
//! A `.slint` file's `@image-url` is embedded by the compiler and never reaches
//! Rust. The textures below are different: the app decodes or forwards them, so
//! it needs the bytes, and they live here to keep the interface's assets with
//! the interface instead of reaching into this crate's directory.

/// The bundled default skins, slim models first, then wide.
///
/// The order matches `DEFAULT_SKINS` in
/// `app/src/ui/components/account_avatar.rs`, whose `get_default_skin` indexes
/// into it, so it must not be sorted.
pub mod skins {
    pub const SLIM_ALEX: &[u8] = include_bytes!("../assets/skins/slim/alex.webp");
    pub const SLIM_ARI: &[u8] = include_bytes!("../assets/skins/slim/ari.webp");
    pub const SLIM_EFE: &[u8] = include_bytes!("../assets/skins/slim/efe.webp");
    pub const SLIM_KAI: &[u8] = include_bytes!("../assets/skins/slim/kai.webp");
    pub const SLIM_MAKENA: &[u8] = include_bytes!("../assets/skins/slim/makena.webp");
    pub const SLIM_NOOR: &[u8] = include_bytes!("../assets/skins/slim/noor.webp");
    pub const SLIM_STEVE: &[u8] = include_bytes!("../assets/skins/slim/steve.webp");
    pub const SLIM_SUNNY: &[u8] = include_bytes!("../assets/skins/slim/sunny.webp");
    pub const SLIM_ZURI: &[u8] = include_bytes!("../assets/skins/slim/zuri.webp");

    pub const WIDE_ALEX: &[u8] = include_bytes!("../assets/skins/wide/alex.webp");
    pub const WIDE_ARI: &[u8] = include_bytes!("../assets/skins/wide/ari.webp");
    pub const WIDE_EFE: &[u8] = include_bytes!("../assets/skins/wide/efe.webp");
    pub const WIDE_KAI: &[u8] = include_bytes!("../assets/skins/wide/kai.webp");
    pub const WIDE_MAKENA: &[u8] = include_bytes!("../assets/skins/wide/makena.webp");
    pub const WIDE_NOOR: &[u8] = include_bytes!("../assets/skins/wide/noor.webp");
    pub const WIDE_STEVE: &[u8] = include_bytes!("../assets/skins/wide/steve.webp");
    pub const WIDE_SUNNY: &[u8] = include_bytes!("../assets/skins/wide/sunny.webp");
    pub const WIDE_ZURI: &[u8] = include_bytes!("../assets/skins/wide/zuri.webp");
}

/// The 512px app icon, used where Slint's `Window.icon` cannot reach — the
/// macOS Dock's `setApplicationIconImage:`.
pub const APP_ICON_PNG: &[u8] = include_bytes!("../assets/images/app-icon.png");

/// The card icon a save, mod, resource pack or pack falls back to when it has
/// no icon of its own.
pub const UNKNOWN_SERVER_WEBP: &[u8] = include_bytes!("../assets/images/unknown-server.webp");
