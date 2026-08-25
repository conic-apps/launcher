// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { init } from "./log"
init()
import { createApp } from "vue"
import { createPinia } from "pinia"
import { createI18n } from "vue-i18n"
import App from "./App.vue"
import AppIcon from "./components/AppIcon.vue"
import { getPlatformInfo } from "@conic/platform"
import { getDataLocation } from "@conic/folder"
import zhCn from "./locales/zh_cn"
import enUs from "./locales/en_us"
import zhTw from "./locales/zh_tw"
import jaJp from "./locales/ja_jp"
import koKr from "./locales/ko_kr"
import deDe from "./locales/de_de"
import frFr from "./locales/fr_fr"
import esEs from "./locales/es_es"
import ptBr from "./locales/pt_br"
import ruRu from "./locales/ru_ru"
import trTr from "./locales/tr_tr"
import plPl from "./locales/pl_pl"

window.__PLATFORM__ = await getPlatformInfo()
window.__DATA_LOCATION__ = await getDataLocation()

// Slavic plural rule (ru, pl): forms are "zero | one | few | many".
// e.g. ru: 1 час / 2 часа / 5 часов, pl: 1 godzina / 2 godziny / 5 godzin
function slavicPluralRules(choice: number, choicesLength: number): number {
    if (choice === 0) {
        return 0
    }
    const teen = choice > 10 && choice < 20
    const endsWithOne = choice % 10 === 1
    if (choicesLength < 4) {
        return !teen && endsWithOne ? 1 : 2
    }
    if (!teen && endsWithOne) {
        return 1
    }
    if (!teen && choice % 10 >= 2 && choice % 10 <= 4) {
        return 2
    }
    return 3
}

const app = createApp(App)

app.use(
    createI18n({
        legacy: false,
        locale: "zh_cn",
        fallbackLocale: "en_us",
        warnHtmlMessage: false,
        missingWarn: false,
        fallbackWarn: false,
        messages: {
            zh_cn: zhCn,
            en_us: enUs,
            zh_tw: zhTw,
            ja_jp: jaJp,
            ko_kr: koKr,
            de_de: deDe,
            fr_fr: frFr,
            es_es: esEs,
            pt_br: ptBr,
            ru_ru: ruRu,
            tr_tr: trTr,
            pl_pl: plPl,
        },
        pluralRules: {
            ru_ru: slavicPluralRules,
            pl_pl: slavicPluralRules,
        },
    }),
)

app.use(createPinia())

app.component("AppIcon", AppIcon)

app.mount("#window")
