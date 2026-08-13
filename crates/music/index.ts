// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { invoke } from "@tauri-apps/api/core"

export type MusicFile = {
    name: string
    path: string
}

export async function listMusicFiles(): Promise<MusicFile[]> {
    return await invoke("plugin:music|cmd_list_music_files")
}
