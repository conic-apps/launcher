// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { invoke } from "@tauri-apps/api/core"

export type InstanceRuntime = {
    minecraft: string
    mod_loader_type?: "Quilt" | "Fabric" | "Neoforge" | "Forge"
    mod_loader_version?: string
}
export type InstanceConfig = {
    name: string
    icon?: Base64URLString
    runtime: InstanceRuntime
    group?: string[]
    launch_config: {
        enable_instance_specific_settings: boolean
        auto_memory?: boolean
        max_memory?: number
        server?: {
            ip: string
            port?: number
        }
        width?: number
        height?: number
        fullscreen?: boolean
        extra_jvm_args?: string
        extra_mc_args?: string
        is_demo?: boolean
        ignore_invalid_minecraft_certificates?: boolean
        ignore_patch_discrepancies?: boolean
        extra_class_paths?: string
        gc?: "Serial" | "Parallel" | "G1" | "Z"
        launcher_name?: string
        wrap_command?: string
        execute_before_launch?: string
        execute_after_launch?: string
        skip_check_files?: boolean
        quit_app_after_launch?: boolean
    }
    use_as_launcher_background?: boolean
}

export type Instance = {
    config: InstanceConfig
    installed: boolean
    id: string
    last_played: number
    has_background: boolean
}

export async function createInstance(instanceConfig: InstanceConfig, id?: string): Promise<string> {
    return await invoke("plugin:instance|cmd_create_instance", { config: instanceConfig, id })
}

export type InstanceSort = "Name" | "Version" | "Playtime" | "LastPlayed"

export async function listInstances(sortBy: InstanceSort): Promise<Instance[]> {
    return await invoke("plugin:instance|cmd_list_instances", { sortBy })
}

export async function getInstanceById(id: string): Promise<Instance | null> {
    return await invoke("plugin:instance|cmd_get_instance_by_id", { id })
}

export async function updateInstance(config: InstanceConfig, id: string): Promise<void> {
    return await invoke("plugin:instance|cmd_update_instance", { config, id })
}

export async function deleteInstance(id: string): Promise<void> {
    return await invoke("plugin:instance|cmd_delete_instance", { id })
}

export async function removeInstallLock(id: string): Promise<void> {
    return await invoke("plugin:instance|cmd_remove_install_lock", { id })
}

export async function addBackgroundImage(path: string, id: string): Promise<void> {
    return await invoke("plugin:instance|cmd_add_background_file", { path, id })
}

export async function getBackgroundPath(id: string): Promise<string> {
    return await invoke("plugin:instance|cmd_get_background_path", { id })
}

export async function removeBackground(id: string): Promise<void> {
    return await invoke("plugin:instance|cmd_remove_background", { id })
}

export async function calculatePlaytime(id: string): Promise<number> {
    return await invoke("plugin:instance|cmd_calculate_playtime", { id })
}

export type TimeFormatter = {
    justNow: string
    hoursAgo: (hours: number) => string
    yesterday: string
    monthDay: (month: number, day: number) => string
    yearMonthDay: (year: number, month: number, day: number) => string
}

export type PlayTimeFormatter = {
    seconds: (count: number) => string
    minutes: (count: number) => string
    hours: (count: number) => string
}

export function formatPlayTime(seconds: number, formatter: PlayTimeFormatter): string {
    if (seconds < 60) {
        return formatter.seconds(Math.floor(seconds))
    }
    const format = (value: number) => Number(value.toFixed(1)).toString()
    const minutes = seconds / 60
    if (minutes < 60) {
        return formatter.minutes(Number(format(minutes)))
    }
    return formatter.hours(Number(format(minutes / 60)))
}

export function formatLastPlayed(timestamp: number, formatter: TimeFormatter): string {
    const date = new Date(timestamp)
    const now = new Date()

    const sameDay =
        date.getFullYear() === now.getFullYear() &&
        date.getMonth() === now.getMonth() &&
        date.getDate() === now.getDate()

    if (sameDay) {
        const diff = now.getTime() - timestamp
        const hours = Math.floor(diff / (1000 * 60 * 60))

        if (hours < 1) {
            return formatter.justNow
        }

        return formatter.hoursAgo(hours)
    }

    const yesterday = new Date(now)
    yesterday.setDate(now.getDate() - 1)

    const isYesterday =
        date.getFullYear() === yesterday.getFullYear() &&
        date.getMonth() === yesterday.getMonth() &&
        date.getDate() === yesterday.getDate()

    if (isYesterday) {
        return formatter.yesterday
    }

    if (date.getFullYear() === now.getFullYear()) {
        return formatter.monthDay(date.getMonth() + 1, date.getDate())
    }

    return formatter.yearMonthDay(date.getFullYear(), date.getMonth() + 1, date.getDate())
}
