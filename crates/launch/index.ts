// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { Config } from "@conic/config"
import { Instance } from "@conic/instance"
import { invoke } from "@tauri-apps/api/core"

export async function launch(config: Config, instance: Instance) {
    return await invoke("plugin:launch|cmd_launch", { config, instance })
}
