// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { defineStore } from "pinia"

export type Config = {
    language: string
    update_channel: "Weekly" | "Snapshot" | "Release"
    auto_update: boolean
    current_account: string
    appearance: {
        theme: string
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
        max_connection: number
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

// const config = (await invoke("read_config_file")) as Config
// console.log(await invoke("read_config_file"))
export const useConfigStore = defineStore("global_config", {
    state: (): Config => {
        return window.__APPLICATION_CONFIG__
    },
})
