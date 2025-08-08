// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { useConfigStore } from "./store/config"

export function reloadTheme(config: ReturnType<typeof useConfigStore>) {
    document.querySelectorAll("*").forEach((el) => {
        el.classList.add("changing-theme")
    })
    loadTheme(config)
    setTimeout(() => {
        document.querySelectorAll("*").forEach((el) => {
            el.classList.remove("changing-theme")
        })
    }, 300)
}

export function loadTheme(config: ReturnType<typeof useConfigStore>) {
    document.body.classList.forEach((cls) => {
        if (cls.startsWith("theme")) {
            document.body.classList.remove(cls)
        }
    })
    if (config.accessibility.high_contrast_mode) {
        document.body.classList.add(`theme-${config.appearance.theme}-hc`)
    } else {
        document.body.classList.add(`theme-${config.appearance.theme}`)
    }
}
