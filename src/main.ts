// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { warn, debug, trace, info, error } from "@tauri-apps/plugin-log"
import { createApp } from "vue"
import { createPinia } from "pinia"
import App from "./Main.vue"
import { createI18n } from "vue-i18n"
import en_us from "./i18n/en_us"
import zh_cn from "./i18n/zh_cn"

const pinia = createPinia()

type LooseString<T> = {
    [K in keyof T]: T[K] extends string ? string : T[K] extends object ? LooseString<T[K]> : T[K]
}
type MessageSchema = LooseString<typeof en_us>

const i18n = createI18n<[MessageSchema], "en_us" | "zh_cn">({
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
    console.log("Frontend loaded")
    document.body.style.cssText =
        "transform: scale(1); opacity: 1;transition: all 250ms cubic-bezier(0, 0.74, 0.65, 1); "
    setTimeout(() => {
        document.body.style.cssText = ""
    }, 500)
}

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

forwardConsole("log", trace)
forwardConsole("debug", debug)
forwardConsole("info", info)
forwardConsole("warn", warn)
forwardConsole("error", error)
