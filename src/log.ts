// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { warn, debug, trace, info, error } from "@tauri-apps/plugin-log"

function forwardConsole(
    functionName: "log" | "debug" | "info" | "warn" | "error",
    logger: (message: string) => Promise<void>,
) {
    const original = console[functionName]
    console[functionName] = (message) => {
        original(message)
        logger(typeof message === "string" ? message : JSON.stringify(message))
    }
}

export function init() {
    forwardConsole("log", trace)
    forwardConsole("debug", debug)
    forwardConsole("info", info)
    forwardConsole("warn", warn)
    forwardConsole("error", error)
}
