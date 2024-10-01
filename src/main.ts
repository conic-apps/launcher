import { createApp } from "vue"
import { createPinia } from "pinia"
import App from "./Main.vue"
import { event } from "@tauri-apps/api"
import { createI18n } from "vue-i18n"
import en_us from "./i18n/en_us"
import zh_cn from "./i18n/zh_cn"

const pinia = createPinia()
const i18n = createI18n({
    legacy: false,
    locale: "zh_cn",
    fallbackLocale: "en_us",
    messages: {
        en_us,
        zh_cn,
    },
})
const app = createApp(App)

app.use(pinia)
app.use(i18n)

app.mount("#window")
import $ from "jquery"
import { invoke } from "@tauri-apps/api/core"

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
    console.log(`
 █████╗ ███╗   ███╗███████╗████████╗██╗  ██╗██╗   ██╗███████╗████████╗    
██╔══██╗████╗ ████║██╔════╝╚══██╔══╝██║  ██║╚██╗ ██╔╝██╔════╝╚══██╔══╝    
███████║██╔████╔██║█████╗     ██║   ███████║ ╚████╔╝ ███████╗   ██║       
██╔══██║██║╚██╔╝██║██╔══╝     ██║   ██╔══██║  ╚██╔╝  ╚════██║   ██║       
██║  ██║██║ ╚═╝ ██║███████╗   ██║   ██║  ██║   ██║   ███████║   ██║       
╚═╝  ╚═╝╚═╝     ╚═╝╚══════╝   ╚═╝   ╚═╝  ╚═╝   ╚═╝   ╚══════╝   ╚═╝       
                                                                          
██╗      █████╗ ██╗   ██╗███╗   ██╗ ██████╗██╗  ██╗███████╗██████╗        
██║     ██╔══██╗██║   ██║████╗  ██║██╔════╝██║  ██║██╔════╝██╔══██╗       
██║     ███████║██║   ██║██╔██╗ ██║██║     ███████║█████╗  ██████╔╝       
██║     ██╔══██║██║   ██║██║╚██╗██║██║     ██╔══██║██╔══╝  ██╔══██╗       
███████╗██║  ██║╚██████╔╝██║ ╚████║╚██████╗██║  ██║███████╗██║  ██║       
╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝       
                                                                          
`)
    invoke("on_frontend_loaded")
    $("body").attr(
        "style",
        "transform: scale(1); opacity: 1;transition: all 250ms cubic-bezier(0, 0.74, 0.65, 1); ",
    )
    setTimeout(() => {
        $("body").attr("style", "")
    }, 500)
}
