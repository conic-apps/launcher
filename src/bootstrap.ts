import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './Main.vue'
import $ from 'jquery'

$("#window").attr(
    "style",
    "transform: scale(1); opacity: 1; transition: all 250ms cubic-bezier(0.04, 0.47, 0.47, 0.98)"
);

const app = createApp(App)

app.use(createPinia())

app.mount('#window')
// import { invoke, window, } from '@tauri-apps/api'

// // 调用命令
// // 在应用窗口中右键，打开开发者工具
// // 你会看到控制台上输出了 "Hello, World!"！
// invoke('greet', { name: 'World' })
//   // `invoke` 返回的是一个 Promise
//   .then((response) => console.log(response))

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

