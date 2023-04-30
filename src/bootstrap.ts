import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './Main.vue'

const app = createApp(App)

app.use(createPinia())

app.mount('#window')
import { invoke, window, } from '@tauri-apps/api'

// 调用命令
// 在应用窗口中右键，打开开发者工具
// 你会看到控制台上输出了 "Hello, World!"！
invoke('greet', { name: 'World' })
  // `invoke` 返回的是一个 Promise
  .then((response) => console.log(response))

window.getCurrent().setAlwaysOnTop(true)
window.getCurrent().setResizable(false)
const webview = new window.WebviewWindow('theUniqueLabel', {
  url: 'https://login.live.com/oauth20_authorize.srf?client_id=00000000402b5328&response_type=code&prompt=select_account&scope=service%3A%3Auser.auth.xboxlive.com%3A%3AMBI_SSL&redirect_uri=https%3A%2F%2Flogin.live.com%2Foauth20_desktop.srf',
  resizable: false,
  focus: true,
  alwaysOnTop: true,
  skipTaskbar: true,
  decorations: false,
  width: 400,
  height: 300,
  x: (await window.getCurrent().innerPosition()).x,
  y: (await window.getCurrent().innerPosition()).y
});