<template>
  <div class="window" data-tauri-drag-region>
    <div class="title-bar" data-tauri-drag-region>
      <div></div>
      <div></div>
      <div style="display: flex; width: fit-content; align-items: center">
        <search-bar
          @click="openSearchPanel"
          id="global-search"
          :placeholder="$t('globalSearch.placeholder')"></search-bar>
      </div>
      <div class="account" @click="showAccountManager = true">
        <div class="avatar">
          <img src="@/assets/images/steve_avatar.webp" alt="player avatar" />
        </div>
        <span>Broken_Deer</span>
      </div>
      <div class="win-btn">
        <div class="min" @click="minimize"><i></i></div>
        <div class="max" @click="maximize"><i></i></div>
        <div class="close" @click="close"><i></i></div>
      </div>
    </div>
    <div class="sidebar" data-tauri-drag-region="">
      <ul class="sidebar-btns" data-tauri-drag-region>
        <sidebar-item
          title="家"
          icon="church"
          @click="changePage($event, 'home')"
          id="sidebar-home"></sidebar-item>
        <sidebar-item
          :title="$t('sidebar.game')"
          icon="gamepad"
          @click="changePage($event, 'game')"
          id="sidebar-game"></sidebar-item>
        <!-- <sidebar-item -->
        <!--   title="扩展" -->
        <!--   icon="puzzle-piece" -->
        <!--   @click="changePage($event, 'community')"></sidebar-item> -->
        <sidebar-item
          title="市场"
          icon="shop"
          @click="changePage($event, 'market')"
          id="sidebar-market"></sidebar-item>
        <sidebar-item
          :title="$t('sidebar.settings')"
          icon="nav-5"
          @click="changePage($event, 'settings')"
          id="sidebar-settings"
          style="margin-top: auto"></sidebar-item>
      </ul>
    </div>
    <main class="main" style="transition: none">
      <Transition :name="transitionName" mode="out-in">
        <KeepAlive>
          <component :is="currentComponent" @back-to-home="back" @jump="jumpTo"></component>
        </KeepAlive>
      </Transition>
    </main>
    <update-reminder></update-reminder>
    <account-manager
      :show="showAccountManager"
      @close="showAccountManager = false"></account-manager>
  </div>
</template>

<script setup lang="ts">
import { markRaw, reactive, ref, shallowRef } from "vue";
import SearchBar from "./components/SearchBar.vue";
import SidebarItem from "./components/SidebarItem.vue";
import AccountManager from "./pages/dialogs/AccountManager.vue";
import { window } from "@tauri-apps/api";
import $ from "jquery";
import Settings from "./pages/Settings.vue";
import Game from "./pages/Game.vue";
import UpdateReminder from "./pages/dialogs/UpdateReminder.vue";
import { invoke } from "@tauri-apps/api/core";
import { useConfigStore } from "./config";
import { watch } from "vue";
import { useI18n } from "vue-i18n";
import gsap from "gsap";
import { reloadTheme } from "./theme";
import Home from "./pages/Home.vue";

function minimize() {
  window.getCurrentWindow().minimize();
}
function maximize() {
  window.getCurrentWindow().maximize();
}
function close() {
  invoke("save_config").then(() => {
    window.getCurrentWindow().close();
  });
}

const pages: any = reactive({
  settings: markRaw(Settings),
  home: markRaw(Home),
  game: markRaw(Game),
});

const transitionName = ref("slide-up");
const currentComponent = shallowRef(pages.game);
let last: any;
const config = useConfigStore();
config.syncFromFile().then(() => {
  reloadTheme(config);
});
const i18n = useI18n();
i18n.locale.value = config.language;
watch(config, () => {
  i18n.locale.value = config.language;
});

function changePage(event: any, component: any) {
  if (component == "settings") {
    gsap.fromTo(event.currentTarget.querySelector("i"), { rotate: "0deg" }, { rotate: `120deg` });
  }
  if (typeof component == "string") {
    currentComponent.value = pages[component];
  } else {
    currentComponent.value = component;
  }
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

const showAccountManager = ref(false);
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

  .account {
    background: var(--controllers-background);
    border: var(--controllers-border);
    border-radius: var(--controllers-border-radius);
    display: flex;
    align-items: center;
    padding: 4px 8px;

    .avatar {
      width: 22px;
      height: 22px;
      top: 13px;
      background: rgba(255, 255, 255, 0.3);
      border-radius: 160px;
      flex-shrink: 0;
      transition: all 0.3s ease;
      display: flex;
      align-items: center;
      justify-content: center;
      overflow: hidden;
      transition: all 0.3s ease;
      margin-right: 8px;

      img {
        width: 100%;
        height: 100%;
      }
    }

    .avatar:active {
      transform: scale(0.92);
    }

    span {
      opacity: 0.9;
      font-size: 14px;
    }
  }
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
  color: var(--window-btn-icon-color);
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
  background: var(--min-btn-background);
}

.win-btn > div.max {
  background: var(--max-btn-background);
}

.win-btn > div.close {
  background: var(--close-btn-background);
}

.win-btn > div.min > i::before {
  content: "\f068";
  font-size: 12px;
  margin-top: 1px;
}

.win-btn > div.max > i::before {
  content: "\f065";
  font-size: 12px;
  margin-top: 1.6px;
  margin-left: 0.8px;
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

.sidebar .sidebar-btns {
  width: 100%;
  height: 100%;
  margin-top: 32px;
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

main.main {
  position: absolute;
  right: 0;
  bottom: 0;
  height: calc(100vh - 56px);
  width: calc(100vw - 80px);
  border: var(--main-border);
  border-radius: 16px;
  border-bottom: unset;
  border-right: unset;
  border-bottom-left-radius: unset;
  border-top-right-radius: unset;
  background: var(--main-background);
}

main.main-large {
  width: 100vw;
  border-radius: 0px;
  border-left: none;
}
</style>
