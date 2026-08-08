// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use tauri::{
    Manager, Runtime,
    plugin::{Builder, TauriPlugin},
};

pub mod error;
// pub mod mods;
pub mod resourcepack;
pub mod saves;
pub mod worldmap;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("content")
        .invoke_handler(tauri::generate_handler![
            saves::cmd_get_all_levels,
            saves::datapack::cmd_get_all_datapacks,
            saves::cmd_get_save_icon,
            resourcepack::cmd_get_all_resourcepacks,
            worldmap::cmd_render_world_map,
        ])
        .setup(|app, _| {
            app.manage(worldmap::MapCache::default());
            Ok(())
        })
        .build()
}
