// Amethyst Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { warn, debug, trace, info, error } from "@tauri-apps/plugin-log"
import $ from "jquery"
import { invoke } from "@tauri-apps/api/core"
import { createApp } from "vue"
import { createPinia } from "pinia"
import App from "./Main.vue"
import { createI18n } from "vue-i18n"
import en_us from "./i18n/en_us"
import zh_cn from "./i18n/zh_cn"

const pinia = createPinia()
const i18n = createI18n({
    legacy: false,
    locale: "zh_cn",
    fallbackLocale: "en_us",
    warnHtmlMessage: false,
    missingWarn: false,
    fallbackWarn: false,
    messages: {
        en_us,
        zh_cn,
    },
})
const app = createApp(App)

app.use(pinia)
app.use(i18n)

app.mount("#window")

// window.getCurrent().setAlwaysOnTop(true)
// window.getCurrent().setResizable(false)
// const webview = new WebviewWindow("theUniqueLabel", {
//     url: "https://",
//     resizable: false,
//     focus: true,
//     alwaysOnTop: true,
//     skipTaskbar: true,
//     decorations: false,
//     width: 400,
//     height: 300,
//     x: (await window.getCurrent().innerPosition()).x,
//     y: (await window.getCurrent().innerPosition()).y,
// })

window.onload = () => {
    invoke("on_frontend_loaded")
    $("body").attr(
        "style",
        "transform: scale(1); opacity: 1;transition: all 250ms cubic-bezier(0, 0.74, 0.65, 1); ",
    )
    setTimeout(() => {
        $("body").attr("style", "")
    }, 500)
}

function forwardConsole(
    fnName: "log" | "debug" | "info" | "warn" | "error",
    logger: (message: string) => Promise<void>,
) {
    const original = console[fnName]
    console[fnName] = (message) => {
        original(message)
        logger(message)
    }
}

forwardConsole("log", trace)
forwardConsole("debug", debug)
forwardConsole("info", info)
forwardConsole("warn", warn)
forwardConsole("error", error)
