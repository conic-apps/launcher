<template>
  <div class="title-bar" data-tauri-drag-region>
    <div class="title" data-tauri-drag-region>Magical Launcher<tag text="Beta" :color="['255', '255', '255']"
        text-color="#fff" :round="true" :border="true" style="transform: scale(0.8) translate(-1px, -6px);"></tag>
    </div>
    <div class="button">
      <div class="window-btn" id="min" @click="minimize"></div>
      <div class="window-btn" id="close" @click="close"></div>
    </div>
  </div>
  <div :class="sidebarClassName" :style="sidebarInlineStyle">
    <div>
      <div class="sidebar-button" @click="sidebar"><i></i></div>
      <ul class="sidebar-links">
        <sidebar-item title="游戏" icon="gamepad" @click="switchPage($event, 'wareHouse')"></sidebar-item>
        <sidebar-item title="新闻" icon="newspaper" @click="switchPage($event, 'newspaper')"></sidebar-item>
        <sidebar-item title="社区" icon="church" @click="switchPage($event, 'community')"></sidebar-item>
        <sidebar-item title="设置" icon="nav-5" @click="switchPage($event, 'settings')"
          style="margin-top: auto;"></sidebar-item>
        <sidebar-item title="更多" icon="cube" @click="switchPage($event, '#more');"></sidebar-item>
      </ul>
    </div>
  </div>
  <main class="page" @click=" sidebarClose ">
    <div class="page" id="main">
      <Transition :name=" transitionName " mode="out-in">
        <KeepAlive>
          <component :is=" activeComponent " @back-to-home=" back " @jump=" jumpTo "></component>
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
import Newspaper from './pages/Newspaper.vue';
import Community from './pages/Community.vue';
import NewInstance from './pages/NewInstance.vue';
import Tag from './components/Tag.vue';
import { invoke, window, } from '@tauri-apps/api'

function minimize() {
  window.getCurrent().minimize()
}
function close() {
  window.getCurrent().close()
}
let sidebarClassName = ref('main-sidebar sidebar-close')
let sidebarClosed: boolean = true
function sidebar() {
  if (sidebarClosed) {
    sidebarClassName.value = 'main-sidebar'
  } else {
    sidebarClassName.value = 'main-sidebar sidebar-close'
  }
  sidebarClosed = !sidebarClosed
}

const pages: any = reactive({
  settings: markRaw(Settings),
  wareHouse: markRaw(WareHouse),
  newspaper: markRaw(Newspaper),
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
    if (component === 'settings') sidebarInlineStyle.value = 'width: 0px !important'
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
  sidebarClosed = true
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
  background-image: linear-gradient(135deg, #00688b 0%, #06759e 50%, #00688b 100%);
  animation: 3s background-position cubic-bezier(1, 1, 0, 0) infinite;
  align-items: center;
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
  font-family: 'minecraft_ten';
  font-size: 1.3rem;
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
  align-content: center;
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
  background-color: #ffffffcf;
  width: 230px;
  height: calc(100% - 50px);
  overflow: hidden;
  transition: all 300ms cubic-bezier(0.48, 0.01, 0.12, 1);
  display: inline-block;
  margin-right: -500px;
  position: sticky;
  z-index: 114514;
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


div.main-sidebar li:hover {
  background-color: #aeaeae45;
}

div.main-sidebar .sidebar-links i {
  font-size: 1.1rem;
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

div.main-sidebar span {
  display: inline-block;
  white-space: nowrap;
  font-size: 14px;
  transition: all 0.2s ease;
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

.sidebar-button:hover {
  background-color: #aeaeae45;
}

.sidebar-button:active {
  background-color: #90909045;
}

.sidebar-button i {
  font-size: 1rem;
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

.sidebar-close {
  width: 60px !important;
  margin-right: -60px !important;
}

.sidebar-close span {
  opacity: 0;
}

.main-sidebar-active {
  color: rgb(var(--theme-color));
  background: rgba(var(--theme-color), 0.1);
}
</style>