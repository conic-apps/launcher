import { useConfigStore } from "./config"
import $ from "jquery"

let changeAfterLoad = false
setTimeout(() => {
    changeAfterLoad = true
}, 1000)

export function reloadTheme(config: ReturnType<typeof useConfigStore>) {
    if (changeAfterLoad) {
        $("*").addClass("changing-theme")
    }
    if (config.accessibility.high_contrast_mode) {
        $("body").attr("class", `theme-${config.appearance.theme}-hc`)
    } else {
        $("body").attr("class", `theme-${config.appearance.theme}`)
    }
    if (changeAfterLoad) {
        setTimeout(() => {
            $("*").removeClass("changing-theme")
        }, 300)
    }
}
