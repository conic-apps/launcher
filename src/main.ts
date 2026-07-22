// Conic Launcher
// Copyright 2022-2026 OakChaser and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { init } from "./log"
init()
import { createApp } from "vue"
import { createPinia } from "pinia"
import { createI18n } from "vue-i18n"
import en_us from "./i18n/en_us"
import zh_cn from "./i18n/zh_cn"
import zh_tw from "./i18n/zh_tw"
import ja_jp from "./i18n/ja_jp"
import ko_kr from "./i18n/ko_kr"
import de_de from "./i18n/de_de"
import fr_fr from "./i18n/fr_fr"
import es_es from "./i18n/es_es"
import pt_br from "./i18n/pt_br"
import ru_ru from "./i18n/ru_ru"
import tr_tr from "./i18n/tr_tr"
import pl_pl from "./i18n/pl_pl"
import App from "./App.vue"
import AppIcon from "./components/AppIcon.vue"
import { getPlatformInfo } from "@conic/platform"

window.__PLATFORM__ = await getPlatformInfo()

type LooseString<T> = {
    [K in keyof T]: T[K] extends string ? string : T[K] extends object ? LooseString<T[K]> : T[K]
}
type MessageSchema = LooseString<typeof en_us>

const app = createApp(App)

app.use(
    createI18n<
        [MessageSchema],
        | "en_us"
        | "zh_cn"
        | "zh_tw"
        | "ja_jp"
        | "ko_kr"
        | "de_de"
        | "fr_fr"
        | "es_es"
        | "pt_br"
        | "ru_ru"
        | "tr_tr"
        | "pl_pl"
    >({
        legacy: false,
        locale: "zh_cn",
        fallbackLocale: "en_us",
        warnHtmlMessage: false,
        missingWarn: false,
        fallbackWarn: false,
        messages: {
            en_us,
            zh_cn,
            zh_tw,
            ja_jp,
            ko_kr,
            de_de,
            fr_fr,
            es_es,
            pt_br,
            ru_ru,
            tr_tr,
            pl_pl,
        },
    }),
)
app.use(createPinia())

app.component("AppIcon", AppIcon)

app.mount("#window")
