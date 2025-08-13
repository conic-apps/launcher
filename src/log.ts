// conic launcher
// copyright 2022-2026 broken-deer and contributors. all rights reserved.
// spdx-license-identifier: gpl-3.0-only

import { warn, debug, trace, info, error } from "@tauri-apps/plugin-log"

function forwardConsole(
    fnName: "log" | "debug" | "info" | "warn" | "error",
    logger: (message: string) => Promise<void>,
) {
    const original = console[fnName]
    console[fnName] = (message) => {
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
