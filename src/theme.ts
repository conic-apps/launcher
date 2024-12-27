// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { useConfigStore } from "./store/config"
import $ from "jquery"

export function reloadTheme(config: ReturnType<typeof useConfigStore>) {
    $("*").addClass("changing-theme")
    loadTheme(config)
    setTimeout(() => {
        $("*").removeClass("changing-theme")
    }, 300)
}

export function loadTheme(config: ReturnType<typeof useConfigStore>) {
    if (config.accessibility.high_contrast_mode) {
        $("body").attr("class", `theme-${config.appearance.theme}-hc`)
    } else {
        $("body").attr("class", `theme-${config.appearance.theme}`)
    }
}
