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

window.__PLATFORM__ = await getPlatformInfo()
window.__DATA_LOCATION__ = await getDataLocation()

const app = createApp(App)

app.use(
    createI18n({
        legacy: false,
        locale: "zh_cn",
        warnHtmlMessage: false,
        missingWarn: false,
        fallbackWarn: false,
    }),
)

app.use(createPinia())

app.component("AppIcon", AppIcon)

app.mount("#window")
