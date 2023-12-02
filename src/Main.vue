<template>
  <div class="title-bar" data-tauri-drag-region>
    
  </div>
</template>

<script setup lang="ts">

</script>

<style lang="less" scoped>

</style>
<!-- <template>
  <div class="title-bar" data-tauri-drag-region>
    <div class="title" data-tauri-drag-region>Amethyst Launcher<tag text="Beta" :color="['255', '255', '255']"
        text-color="#fff" :round="true" :border="true" style="transform: scale(0.8) translate(-1px, -6px);"></tag>
    </div>
    <div class="button">
      <div class="window-btn" id="min" @click="minimize"></div>
      <div class="window-btn" id="close" @click="close"></div>
    </div>
  </div>
  <div :class="sidebarClassName" :style="sidebarInlineStyle" data-tauri-drag-region>
    <div>
      <ul class="sidebar-links" data-tauri-drag-region>
        <sidebar-item title="游戏" icon="gamepad" @click="switchPage($event, 'wareHouse')"></sidebar-item>
        <sidebar-item title="社区" icon="church" @click="switchPage($event, 'community')"></sidebar-item>
        <sidebar-item title="设置" icon="nav-5" @click="switchPage($event, 'settings')"
          style="margin-top: auto;"></sidebar-item>
        <sidebar-item title="更多" icon="cube" @click="switchPage($event, '#more');"></sidebar-item>
      </ul>
    </div>
  </div>
  <main class="page" @click="sidebarClose">
    <div class="page" id="main">
      <Transition :name="transitionName" mode="out-in">
        <KeepAlive>
          <component :is=" activeComponent " @back-to-home="back" @jump="jumpTo"></component>
        </KeepAlive>
      </Transition>
    </div>
  </main>
</template>

<script setup lang="ts">
import { markRaw, reactive, ref, shallowRef, type Ref } from 'vue';
import SidebarItem from './components/SidebarItem.vue';
import WareHouse from './pages/Game.vue';
import Settings from './pages/Settings.vue';
import Community from './pages/Community.vue';
import NewInstance from './pages/NewInstance.vue';
import Tag from './components/Tag.vue';
import { window } from '@tauri-apps/api'
import $ from 'jquery'

// 修复webkitgtk的奇怪问题
setTimeout(() => {
  let fontSizeError = 0
  const fixWebkitgtk = document.getElementById('fix-webkitgtk') as HTMLElement
  fixWebkitgtk.innerHTML = /* css */ `    
  :root {
    --font-size-error: ${fontSizeError}px;
  }`

  if ($('#font-test').outerHeight() > 15.78) {
    console.log((15.78 / $('#font-test').outerHeight()) * 15)
    fontSizeError = 13.5 - (15.78 / $('#font-test').outerHeight()) * 15
    fixWebkitgtk.innerHTML = /* css */ `    
    :root {
      --font-size-error: ${fontSizeError}px;
    }`
  }
}, 48);

function minimize() {
  window.getCurrent().minimize()
}
function close() {
  window.getCurrent().close()
}
let sidebarClassName = ref('main-sidebar sidebar-close')
const pages: any = reactive({
  settings: markRaw(Settings),
  wareHouse: markRaw(WareHouse),
  community: markRaw(Community),
  newInstance: markRaw(NewInstance),
})
const activeComponent = shallowRef(pages.wareHouse)
let transitionName = ref('entrance')
let sidebarInlineStyle = ref('')
let last: any
function switchPage(event: any, component: any) {
  sidebarClose()
  if (component === 'settings' || component === 'newInstance') {
    transitionName.value = 'zoom-out'
    if (component === 'settings') sidebarInlineStyle.value = 'width: 0px !important;overflow: hidden;'
  } else {
    let isSettingPage = JSON.stringify(activeComponent.value) == JSON.stringify(pages.settings);
    let isNewInstancePage = JSON.stringify(activeComponent.value) == JSON.stringify(pages.newInstance)
    if (isSettingPage || isNewInstancePage) {
      transitionName.value = 'zoom-in'
    } else {
      transitionName.value = 'entrance'
    }
    sidebarInlineStyle.value = ''
  }
  last = activeComponent.value
  if (typeof component == 'string') {
    activeComponent.value = pages[component];
  } else {
    activeComponent.value = component
  }
}
function sidebarClose() {
  sidebarClassName.value = 'main-sidebar sidebar-close'
}

function back() {
  let isSettingPage = JSON.stringify(last) == JSON.stringify(pages.settings);
  if (isSettingPage) {
    switchPage(null, 'wareHouse')
    return
  }
  switchPage(null, last)
}
function jumpTo(name: string) {
  switchPage(null, name)
}
</script>

<style lang="less" scoped>
.title-bar {
  height: 50px;
  -webkit-app-region: drag;
  display: flex;
  justify-content: space-between;
  color: #fff;
  // background-image: linear-gradient(135deg, #3c8aa4 0%, #06759e 50%, #3c8aa4 100%);
  animation: 3s background-position cubic-bezier(1, 1, 0, 0) infinite;
  align-items: center;
  // border-bottom: 1px solid #ffffff39;
}

@keyframes background-position {
  from {
    background-position: 0vw;
  }

  to {
    background-position: 100vw;
  }
}

.title {
  display: flex;
  margin-left: 1rem;
  margin-top: -3px;
  align-items: center;
  padding: 0 20px;
  font-family: 'Harmony-re';
  font-weight: 500;
  font-size: calc(20.8px - var(--font-size-error));
}

.button {
  width: 100px;
  height: inherit;
  overflow: hidden;
  border-top-right-radius: 8px;
  display: flex;
  margin-right: 0.6rem;
}

.window-btn {
  width: 50px;
  display: flex;
  justify-content: center;
  font-family: "fa-pro" !important;
  align-items: center;
  -webkit-app-region: no-drag;
  border-radius: 50%;
  transition: all 0.15s ease;
  transform: scale(0.8);

}

.window-btn:hover {
  background-color: #ffffff14;
}

.window-btn:active {
  transform: scale(0.7);
}

main.page {
  padding-left: 60px;
  box-sizing: border-box;
  padding-bottom: 2rem;
  overflow-y: overlay;
  overflow-x: hidden;
  height: 100%;
  transition: all 0.3s ease;
  float: right;
  width: 100%;
}

div.page {
  margin: 0 auto;
  display: block;
  margin-top: 0;
  opacity: 1;
}

div.page>div {
  height: calc(100vh - 60px);
  display: flex;
}

.main-sidebar {
  // background-color: #ffffffcf;
  width: 60px;
  height: calc(100% - 50px);
  // overflow: hidden;
  transition: all 300ms cubic-bezier(0.48, 0.01, 0.12, 1);
  display: inline-block;
  margin-right: -60px;
  position: sticky;
  z-index: 114514;
  color: #fff;
}

.main-sidebar>div {
  margin: 16px 8px 7px 8px;
  height: calc(100% - 26px);
  display: flex;
  flex-direction: column;
}

.main-sidebar ul.sidebar-links {
  display: flex;
  justify-content: space-between;
  flex-grow: 1;
  flex-direction: column;
}

.main-sidebar ul.sidebar-links>div {
  width: 100%;
}

div.main-sidebar li:active {
  transform: scale(0.94);
}

div.main-sidebar li {
  transition: transform 0.17s ease;
  width: 100%;
  height: 36px;
  align-items: center;
  border-radius: 6px;
  white-space: nowrap;
  display: flex;
  padding-left: 7.5px;
}


div.main-sidebar .sidebar-links i {
  font-size: calc(17.6px - var(--font-size-error));
  font-weight: 400;
  font-family: "fa-pro";
  font-style: normal;
  width: 24px;
  display: inline-block;
  margin-right: 0.7rem;
  text-align: center;
  flex-shrink: 0;
  padding-left: 3px;
}

.sidebar-button {
  width: 40px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--border-radius-small);
  margin-left: 1.6px;
}


.sidebar-button i {
  font-size: calc(16px - var(--font-size-error));
  font-family: "fa-pro";
  font-style: normal;
  height: 1em;
  width: 1em;
  text-align: center;
  transition: all 0.2s cubic-bezier(0, 0.61, 0.16, 0.98);
}

.sidebar-button:active i {
  transform: scale(0.7, 1);
}

.main-sidebar-active {
  color: rgb(var(--theme-color));
  background: rgba(var(--theme-color), 0.1);
}

#main>div {
  background-image: linear-gradient(135deg, #e0dcfb, #efeefd);
  border-left: 1px solid #00000026;
  border-top: 1px solid #00000026;
  border-radius: var(--border-radius-large);
}
</style> -->