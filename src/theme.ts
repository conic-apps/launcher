// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { Palette } from "@conic/config"

export function reloadPalette(
    appearanceConfig: { palette: Palette; paletteFollowSystem: boolean },
    highContrastMode: boolean,
) {
    document.querySelectorAll("*").forEach((el) => {
        el.classList.add("changing-theme")
    })
    loadPalette(appearanceConfig, highContrastMode)
    setTimeout(() => {
        document.querySelectorAll("*").forEach((el) => {
            el.classList.remove("changing-theme")
        })
    }, 300)
}

export function loadPalette(
    appearanceConfig: { palette: Palette; paletteFollowSystem: boolean },
    highContrastMode: boolean,
) {
    document.body.classList.forEach((cls) => {
        if (cls.startsWith("theme")) {
            document.body.classList.remove(cls)
        }
    })
    let className = "theme-"
    if (appearanceConfig.paletteFollowSystem) {
        const isDarkMode = window.matchMedia("(prefers-color-scheme: dark)").matches
        if (isDarkMode) {
            className += Palette.Mocha
        } else {
            className += Palette.Macchiato
        }
    } else {
        className += appearanceConfig.palette
    }
    if (highContrastMode) {
        className += "-hc"
    }
    document.body.classList.add(className)
}
