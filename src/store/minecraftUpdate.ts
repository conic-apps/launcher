// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { getMinecrafVersionManifest } from "@conic/install"
import { useConfigStore } from "@/store/config"
import { useDialogStore } from "@/store/dialog"

const RELEASE_REMINDER_KEY = "minecraft_reminder_release"
const SNAPSHOT_REMINDER_KEY = "minecraft_reminder_snapshot"

export async function checkMinecraftUpdateReminder() {
    const config = useConfigStore()
    const releaseEnabled = config.accessibility.release_reminder
    const snapshotEnabled = config.accessibility.snapshot_reminder

    if (!releaseEnabled && !snapshotEnabled) {
        localStorage.removeItem(RELEASE_REMINDER_KEY)
        localStorage.removeItem(SNAPSHOT_REMINDER_KEY)
        return
    }

    let manifest
    try {
        manifest = await getMinecrafVersionManifest()
    } catch (e) {
        console.error("获取 Minecraft 版本列表失败", e)
        return
    }

    let releaseToRemind: string | null = null
    let snapshotToRemind: string | null = null

    if (releaseEnabled) {
        const latestRelease = manifest.latest.release
        const stored = localStorage.getItem(RELEASE_REMINDER_KEY)
        if (stored !== null && stored !== latestRelease) {
            releaseToRemind = latestRelease
        }
        localStorage.setItem(RELEASE_REMINDER_KEY, latestRelease)
    } else {
        localStorage.removeItem(RELEASE_REMINDER_KEY)
    }

    if (snapshotEnabled) {
        const latestSnapshot = manifest.latest.snapshot
        const stored = localStorage.getItem(SNAPSHOT_REMINDER_KEY)
        if (stored !== null && stored !== latestSnapshot) {
            snapshotToRemind = latestSnapshot
        }
        localStorage.setItem(SNAPSHOT_REMINDER_KEY, latestSnapshot)
    } else {
        localStorage.removeItem(SNAPSHOT_REMINDER_KEY)
    }

    if (
        releaseToRemind !== null &&
        snapshotToRemind !== null &&
        releaseToRemind === snapshotToRemind
    ) {
        snapshotToRemind = null
    }

    if (releaseToRemind !== null) {
        showReminder(releaseToRemind)
    } else if (snapshotToRemind !== null) {
        showReminder(snapshotToRemind)
    }
}

function showReminder(version: string) {
    const dialogStore = useDialogStore()
    dialogStore.updateReminder.version = version
    dialogStore.updateReminder.visible = true
}
