// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { invoke } from "@tauri-apps/api/core"

export async function loadConfigFile(): Promise<Config> {
    return await invoke("plugin:config|cmd_load_config_file")
}

export async function saveConfigToFile(config: Config): Promise<void> {
    return await invoke("plugin:config|cmd_save_config", { config })
}

export enum UpdateChannel {
    Weekly = "Weekly",
    Snapshot = "Snapshot",
    Release = "Release",
}
export enum Palette {
    Mocha = "Mocha",
    Latte = "Latte",
    Frappe = "Frappe",
    Macchiato = "Macchiato",
}

export type Config = {
    language: string
    update_channel: UpdateChannel
    auto_update: boolean
    current_account: string
    appearance: {
        palette_follow_system: boolean
        palette: Palette
    }
    accessibility: {
        release_reminder: boolean
        snapshot_reminder: boolean
        hide_latest_release: boolean
        hide_latest_snapshot: boolean
        change_game_language: boolean
        disable_animations: boolean
        high_contrast_mode: boolean
    }
    download: {
        max_connections: number
        max_download_speed: number
    }
    launch: {
        min_memory: number
        max_memory: number
        server?: {
            ip: string
            port: number
        }
        width: number
        height: number
        fullscreen: boolean
        extra_jvm_args: string
        extra_mc_args: string
        is_demo: boolean
        ignore_invalid_minecraft_certificates: boolean
        ignore_patch_discrepancies: boolean
        extra_class_paths: string
        gc: "Serial" | "Parallel" | "ParallelOld" | "G1" | "Z"
        launcher_name: string
        wrap_command: string
        execute_before_launch: string
        execute_after_launch: string
        skip_refresh_account: boolean
        skip_check_files: boolean
    }
}
