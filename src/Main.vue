<template>
  <div class="window" data-tauri-drag-region>
    <div class="title-bar" data-tauri-drag-region>
      <div></div>
      <search-bar
        @click="openSearchPanel"
        id="global-search"
        placeholder="在 Amethyst 中搜索，或输入命令"
      ></search-bar>
      <div class="win-btn">
        <div class="min" @click="minimize"><i></i></div>
        <div class="max" @click="maximize"><i></i></div>
        <div class="close" @click="close"><i></i></div>
      </div>
    </div>
    <div class="sidebar" data-tauri-drag-region="">
      <div class="logo"></div>
      <ul class="sidebar-btns" data-tauri-drag-region>
        <sidebar-item
          title="游戏"
          icon="gamepad"
          @click="changePage($event, 'wareHouse')"
        ></sidebar-item>
        <sidebar-item
          title="扩展"
          icon="puzzle-piece"
          @click="changePage($event, 'community')"
        ></sidebar-item>
        <sidebar-item
          title="设置"
          icon="nav-5"
          @click="changePage($event, 'settings')"
          style="margin-top: auto"
        ></sidebar-item>
        <!-- <sidebar-item title="更多" icon="cube" @click="switchPage($event, '#more');"></sidebar-item> -->
      </ul>
    </div>
    <main class="main" style="transition: none">
      <Transition :name="transitionName" mode="out-in">
        <KeepAlive>
          <component
            :is="currentComponent"
            @back-to-home="back"
            @jump="jumpTo"
          ></component>
        </KeepAlive>
      </Transition>
    </main>
    <!-- <div class="line">
    </div> TODO: line -->
  </div>
</template>

<script setup lang="ts">
import { markRaw, reactive, ref, shallowRef } from "vue";
import SearchBar from "./components/SearchBar.vue";
import SidebarItem from "./components/SidebarItem.vue";
import { window } from "@tauri-apps/api";
import $ from "jquery";
import Settings from "./pages/Settings.vue";
import Game from "./pages/Game.vue";

function minimize() {
  window.getCurrentWindow().minimize();
}
function maximize() {
  window.getCurrentWindow().maximize();
}
function close() {
  window.getCurrentWindow().close();
}

const pages: any = reactive({
  settings: markRaw(Settings),
  game: markRaw(Game),
});

let transitionName = ref("entrance");
const currentComponent = shallowRef(pages.game);
let last: any;
function changePage(_event: any, component: any) {
  if (component === "settings") {
    transitionName.value = "zoom-out";
    hideSidebar();
  } else {
    showSidebar();
    let isSettingPage =
      JSON.stringify(currentComponent.value) == JSON.stringify(pages.settings);
    if (isSettingPage) {
      transitionName.value = "zoom-in";
    } else {
      transitionName.value = "entrance";
    }
  }
  last = currentComponent.value;
  if (typeof component == "string") {
    currentComponent.value = pages[component];
  } else {
    currentComponent.value = component;
  }
}

function hideSidebar() {
  $(".main").attr("style", "");
  $(".sidebar").addClass("sidebar-hidden");
  $(".main").addClass("main-large");
  setTimeout(() => {
    $(".main").attr("style", "transition: none");
  }, 300);
}

function showSidebar() {
  $(".main").attr("style", "");
  $(".sidebar").removeClass("sidebar-hidden");
  $(".main").removeClass("main-large");
  setTimeout(() => {
    $(".main").attr("style", "transition: none");
  }, 300);
}

function back() {
  let isSettingPage = JSON.stringify(last) == JSON.stringify(pages.settings);
  if (isSettingPage) {
    changePage(null, "wareHouse");
    return;
  }
  changePage(null, last);
}
function jumpTo(name: string) {
  changePage(null, name);
}

function moveLine(position: number[]) {
  // todo
}

function openSearchPanel() {
  $("#global-search")
    .attr(
      "style",
      /* css */ `
  top: 300px; 
  height: 400px; 
  width: 500px; 
  background: #000; 
  z-index: 10001;
  border-radius: 16px;
  `,
    )
    .children("*")
    .hide();
  $("#model-shadow").attr("style", "opacity: 1; z-index: 10000;");
  setTimeout(() => {
    closeSearchPanel();
  }, 1000);
}

function closeSearchPanel() {
  $("#global-search").attr("style", "").children("*").show();
  $("#model-shadow").attr("style", "");
}
</script>

<style lang="less" scoped>
.window {
  width: 100%;
  height: 100%;
  display: flex;
}

.title-bar {
  height: 56px;
  width: calc(100% - 80px);
  position: absolute;
  left: 80px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.win-btn {
  display: flex;
  align-items: center;
  margin-right: 20px;
}

.win-btn > div {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  margin-left: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform 100ms;
  background: #ffffff40;
}

.win-btn > div > i {
  font-style: normal;
  font-family: "fa-pro";
  font-weight: 100;
  display: flex;
  align-items: center;
  justify-content: center;
}

.win-btn > div > i::before {
  line-height: 1;
  color: #ffffffb7;
  opacity: 0;
}

.win-btn > div:hover > i::before {
  opacity: 1;
}

.win-btn > div:active {
  transform: scale(0.9);
}

.win-btn > div:active > i {
  opacity: 0.9;
}

.win-btn > div.min {
  // background: rgb(117, 121, 0);
}

.win-btn > div.min > i::before {
  content: "\f068";
  font-size: 12px;
  margin-top: 1px;
}

.win-btn > div.max {
  // background: rgb(2, 136, 0);
}

.win-btn > div.max > i::before {
  content: "\f065";
  font-size: 12px;
  margin-top: 1.6px;
  margin-left: 0.8px;
}

.win-btn > div.close {
  // background: rgba(158, 0, 0, 0.677);
}

.win-btn > div.close > i::before {
  content: "\f00d";
  font-size: 14px;
  margin-top: 1px;
  margin-left: 0.6px;
}

.sidebar {
  width: 80px;
  display: flex;
  flex-direction: column;
  align-items: center;
}

.sidebar .logo {
  width: 48px;
  height: 48px;
  background: rgba(255, 255, 255, 30%);
  border-radius: 50%;
  margin-top: 16px;
  flex-shrink: 0;
  position: absolute;
  top: 0;
  transition: all 0.3s ease;
}

.sidebar .sidebar-btns {
  width: 100%;
  height: 100%;
  margin-top: 72px;
  display: flex;
  flex-direction: column;
  align-items: center;
  margin-bottom: 22px;
}

.sidebar > * {
  transition: opacity 0.3s ease;
}

.sidebar-hidden > * {
  opacity: 0;
}

.sidebar-hidden .logo {
  opacity: 1;
  position: absolute;
  transform: scale(0.76);
  top: -13px;
}

main.main {
  position: absolute;
  right: 0;
  bottom: 0;
  height: calc(100vh - 56px);
  width: calc(100vw - 80px);
  // border: 1px solid #303241;
  border-radius: 16px;
  border-bottom: unset;
  border-right: unset;
  border-bottom-left-radius: unset;
  border-top-right-radius: unset;
  background-color: #ffffff0f;
  transition: all 0.3s ease;
}

main.main-large {
  width: 100vw;
  border-radius: 0px;
  border-left: none;
}

.line {
  width: 1px;
  height: calc(100vh - 140px);
  background-color: rgba(255, 255, 255, 0.12);
  position: fixed;
  right: 328px;
  top: 100px;
  transition: all 0.3s ease;
}
</style>
