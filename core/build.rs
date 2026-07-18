// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use tauri_build::InlinedPlugin;

fn main() {
    let attributes = tauri_build::Attributes::new()
        .plugin(
            "config",
            InlinedPlugin::new().commands(&[
                "cmd_load_config_file",
                "cmd_get_default_config",
                "cmd_save_config",
            ]),
        )
        .plugin(
            "account",
            InlinedPlugin::new().commands(&[
                "cmd_list_accounts",
                "cmd_get_microsoft_account",
                "cmd_delete_microsoft_account",
                "cmd_add_microsoft_account",
                "cmd_update_microsoft_account",
                "cmd_redeem_access_token",
                "cmd_microsoft_access_token_auth_flow",
                "cmd_refresh_microsoft_account",
                "cmd_request_device_code",
                "cmd_poll_device_code",
                "cmd_add_offline_account",
                "cmd_delete_offline_account",
                "cmd_update_offline_account",
                "cmd_get_offline_account",
                "cmd_add_yggdrasil_server",
                "cmd_delete_yggdrasil_server",
                "cmd_list_yggdrasil_server",
                "cmd_get_yggdrasil_server_info",
                "cmd_yggdrasil_authenticate_account",
                "cmd_yggdrasil_validate_account",
                "cmd_yggdrasil_refresh_account",
                "cmd_yggdrasil_invalidate_account",
                "cmd_yggdrasil_get_profile",
                "cmd_add_yggdrasil_account",
                "cmd_delete_yggdrasil_account",
                "cmd_get_yggdrasil_account",
                "cmd_list_yggdrasil_accounts",
                "cmd_update_yggdrasil_account",
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
                "cmd_spawn_install_task",
                "cmd_cancel_install_task",
            ]),
        )
        .plugin(
            "instance",
            InlinedPlugin::new().commands(&[
                "cmd_create_instance",
                "cmd_list_instances",
                "cmd_get_instance_by_id",
                "cmd_update_instance",
                "cmd_delete_instance",
            ]),
        )
        .plugin(
            "launch",
            InlinedPlugin::new().commands(&["cmd_spawn_launch_task", "cmd_cancel_launch_task"]),
        )
        .plugin(
            "modrinth",
            InlinedPlugin::new().commands(&[
                "cmd_search_projects",
                "cmd_get_project",
                "cmd_get_multiple_projects",
                "cmd_get_all_dependencies",
                "cmd_list_project_versions",
                "cmd_spawn_download_mod_task",
                "cmd_cancel_download_task",
            ]),
        )
        .plugin(
            "platform",
            InlinedPlugin::new().commands(&["cmd_get_platform_info"]),
        )
        .plugin(
            "folder",
            InlinedPlugin::new().commands(&["cmd_get_data_location"]),
        );
    tauri_build::try_build(attributes).unwrap();
}
