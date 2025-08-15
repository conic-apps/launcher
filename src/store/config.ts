// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { Config } from "@conic/config"
import { defineStore } from "pinia"

// const config = (await invoke("read_config_file")) as Config
export const useConfigStore = defineStore("global_config", {
    state: (): Config => {
        return window.__CONIC_CONFIG__
    },
})
