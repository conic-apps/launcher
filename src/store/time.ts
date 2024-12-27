// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { defineStore } from "pinia"

export const useTimeStore = defineStore("current_time", {
    state: () => {
        return {
            now: Math.round(new Date().getTime() / 1000),
        }
    },
})
