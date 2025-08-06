// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use tauri_build::InlinedPlugin;

fn main() {
    let attributes = tauri_build::Attributes::new()
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
                "cmd_delete_account",
                "cmd_refresh_all_microsoft_accounts",
                "cmd_refresh_microsoft_account_by_uuid",
            ]),
        )
        .plugin(
            "install",
            InlinedPlugin::new().commands(&[
                "cmd_get_minecraft_version_list",
                "cmd_get_fabric_version_list",
                "cmd_get_quilt_version_list",
                "cmd_get_forge_version_list",
                "cmd_get_neoforged_version_list",
                "cmd_install",
            ]),
        )
        .plugin(
            "instance",
            InlinedPlugin::new().commands(&[
                "cmd_create_instance",
                "cmd_list_instances",
                "cmd_update_instance",
                "cmd_delete_instance",
            ]),
        )
        .plugin("launch", InlinedPlugin::new().commands(&["cmd_launch"]))
        .plugin(
            "platform",
            InlinedPlugin::new().commands(&["cmd_get_platform_info"]),
        );
    tauri_build::try_build(attributes).unwrap();
}
