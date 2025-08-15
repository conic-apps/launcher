// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { Config, loadConfigFile } from "@conic/config"
import { defineStore } from "pinia"

const initialConfig = await loadConfigFile()

export const useConfigStore = defineStore("global_config", {
    state: (): Config => {
        return initialConfig
    },
})
