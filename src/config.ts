import { invoke } from "@tauri-apps/api/core"
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
        open_log_viewer: boolean
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
        process_priority: "High" | "AboveNormal" | "Normal" | "BelowNormal" | "Low"
        ignore_invalid_minecraft_certificates: boolean
        ignore_patch_discrepancies: boolean
        extra_class_paths: string
        gc: "Serial" | "Parallel" | "ParallelOld" | "G1" | "Z"
        launcher_name: string
        wrap_command: string
        execute_before_launch: string
        execute_after_launch: string
    }
}

export const useConfigStore = defineStore("global_config", {
    state: (): Config => {
        return {
            language: "en",
            update_channel: "Release",
            auto_update: false,
            current_account: "",
            appearance: {
                theme: "Dark",
            },
            accessibility: {
                release_reminder: true,
                snapshot_reminder: true,
                hide_latest_release: true,
                hide_latest_snapshot: true,
                change_game_language: true,
                open_log_viewer: false,
                disable_animations: true,
                high_contrast_mode: true,
            },
            download: {
                max_connection: 0,
                max_download_speed: 0,
            },
            launch: {
                min_memory: 0,
                max_memory: 0,
                server: {
                    ip: "",
                    port: 0,
                },
                width: 0,
                height: 0,
                fullscreen: false,
                extra_jvm_args: "",
                extra_mc_args: "",
                is_demo: false,
                process_priority: "Normal",
                ignore_invalid_minecraft_certificates: false,
                ignore_patch_discrepancies: false,
                extra_class_paths: "",
                gc: "G1",
                launcher_name: "",
                wrap_command: "",
                execute_before_launch: "",
                execute_after_launch: "",
            },
        }
    },
    actions: {
        async syncFromFile() {
            await syncFromFile()
        },
    },
})

/** 将渲染进程和主进程存储的配置与文件同步 */
async function syncFromFile() {
    const Config = useConfigStore()
    const config: Config = await invoke("read_config_file")
    Config.$patch(config)
    await invoke("update_config", { config: config })
}
