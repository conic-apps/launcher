// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use tauri_build::InlinedPlugin;

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .plugin(
                "config",
                InlinedPlugin::new().commands(&["cmd_load_config_file", "cmd_save_config"]),
            )
            .plugin(
                "account",
                InlinedPlugin::new().commands(&[
                    "cmd_list_accounts",
                    "cmd_get_account_by_uuid",
                    "cmd_add_microsoft_account",
                    "cmd_delete_accout",
                ]),
            ),
    )
    .unwrap();
}
