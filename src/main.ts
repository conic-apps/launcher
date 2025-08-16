// conic launcher
// copyright 2022-2026 broken-deer and contributors. all rights reserved.
// spdx-license-identifier: gpl-3.0-only

import { init } from "./log"
init()

import { createApp } from "vue"
import { createPinia } from "pinia"
import { createI18n } from "vue-i18n"
import en_us from "./i18n/en_us"
import zh_cn from "./i18n/zh_cn"
import App from "./Main.vue"
import AppIcon from "./components/AppIcon.vue"

type LooseString<T> = {
    [K in keyof T]: T[K] extends string ? string : T[K] extends object ? LooseString<T[K]> : T[K]
}
type MessageSchema = LooseString<typeof en_us>

const app = createApp(App)

app.use(
    createI18n<[MessageSchema], "en_us" | "zh_cn">({
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
    }),
)
app.use(createPinia())

app.component("AppIcon", AppIcon)
app.mount("#window")
