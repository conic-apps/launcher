import { createApp, watch } from "vue"
import { createPinia } from "pinia"
import App from "./Main.vue"
import { type Config, useConfigStore } from "./config.ts"
import { invoke } from "@tauri-apps/api/core"
import { event } from "@tauri-apps/api"
// import $ from "jquery";

// $("#window").attr(
//     "style",
//     "transform: scale(1); opacity: 1; transition: all 250ms cubic-bezier(0.04, 0.47, 0.47, 0.98)"
// );
const pinia = createPinia()
const app = createApp(App)

app.use(pinia)

app.mount("#window")

// // window.getCurrent().setAlwaysOnTop(true)
// // window.getCurrent().setResizable(false)
// const webview = new window.WebviewWindow('theUniqueLabel', {
//   url: 'https://',
//   resizable: false,
//   focus: true,
//   alwaysOnTop: true,
//   skipTaskbar: true,
//   decorations: false,
//   width: 400,
//   height: 300,
//   x: (await window.getCurrent().innerPosition()).x,
//   y: (await window.getCurrent().innerPosition()).y
// },);

globalThis.onload = () => {
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
    event.emit("fontend-loaded")
}
