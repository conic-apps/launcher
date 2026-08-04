// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { invoke } from "@tauri-apps/api/core"

export type StatisticsProfile = { Microsoft: string } | { Offline: string } | { Yggdrasil: string }

export type StatisticsEntry = {
    profile: StatisticsProfile
    instance_id: string
    launch_at_unix_secs: number
}

export async function getStatistics(): Promise<StatisticsEntry[]> {
    return await invoke("plugin:statistics|cmd_get_statistics")
}

export async function getStatisticsByProfile(
    profile: StatisticsProfile,
): Promise<StatisticsEntry[]> {
    return await invoke("plugin:statistics|cmd_get_statistics_by_profile", { profile })
}
