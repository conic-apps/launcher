<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="window" data-tauri-drag-region>
    <div class="title-bar" data-tauri-drag-region>
      <div
        style="
          display: flex;
          width: calc(100vw - 500px);
          margin-left: 160px;
          flex-shrink: 0;
          align-items: center;
        ">
        <search-bar style="width: 100%" :placeholder="$t('globalSearch.placeholder')"></search-bar>
      </div>
      <div class="window-buttons-container window-buttons-container-macos" v-if="isMacOS()">
        <WindowButton
          button-type="minimize"
          @minimize="appWindow.getCurrentWindow().minimize()"></WindowButton>
        <WindowButton
          button-type="maximize"
          @maximize="appWindow.getCurrentWindow().maximize()"></WindowButton>
        <WindowButton button-type="close" @close="closeWindow()"></WindowButton>
      </div>
      <div class="window-buttons-container" v-else>
        <WindowButton
          button-type="minimize"
          @minimize="appWindow.getCurrentWindow().minimize()"></WindowButton>
        <WindowButton
          button-type="maximize"
          @maximize="appWindow.getCurrentWindow().maximize()"></WindowButton>
        <WindowButton button-type="close" @close="closeWindow()"></WindowButton>
      </div>
    </div>
    <div class="sidebar" :class="{ 'sidebar-macos': isMacOS() }" data-tauri-drag-region>
      <component :is="Logo" height="40" style="margin-top: 24px"></component>
      <ul class="sidebar-items" data-tauri-drag-region>
        <sidebar-item
          :title="$t('sidebar.home')"
          icon="house"
          @click="changePage($event, 'home')"
          id="sidebar-home"></sidebar-item>
        <sidebar-item
          :title="$t('sidebar.game')"
          icon="gamepad"
          @click="changePage($event, 'game')"
          id="sidebar-game"></sidebar-item>
        <sidebar-item
          :title="$t('sidebar.market')"
          icon="earth"
          @click="changePage($event, 'market')"
          id="sidebar-market"></sidebar-item>
        <sidebar-item
          :title="$t('sidebar.settings')"
          icon="settings"
          @click="changePage($event, 'settings')"
          id="sidebar-settings"
          style="margin-top: auto"></sidebar-item>
      </ul>
    </div>
    <main class="main" style="transition: none">
      <Transition :name="transitionName" mode="out-in">
        <component :is="currentComponent" @jump="jumpTo"></component>
      </Transition>
    </main>
    <DialogRoot></DialogRoot>
  </div>
</template>

<script setup lang="ts">
import Logo from "@/assets/logo.svg";
import WindowButton from "./components/WindowButton.vue";
import SearchBar from "./components/SearchBar.vue";
import SidebarItem from "./components/SidebarItem.vue";
import HomeView from "./views/HomeView.vue";
import GameView from "./views/GameView.vue";
import MarketView from "./views/MarketView.vue";
import SettingsView from "./views/SettingsView.vue";
import DialogRoot from "./DialogRoot.vue";
import { markRaw, onMounted, reactive, ref, shallowRef, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useConfigStore } from "./store/config";
import { loadPalette } from "./theme";
import { window as appWindow } from "@tauri-apps/api";

const config = useConfigStore();
loadPalette(
  {
    palette: config.appearance.palette,
    paletteFollowSystem: config.appearance.palette_follow_system,
  },
  config.accessibility.high_contrast_mode,
);

const pages = reactive({
  settings: markRaw(SettingsView),
  home: markRaw(HomeView),
  market: markRaw(MarketView),
  game: markRaw(GameView),
});
const transitionName = ref("slide-up");
const currentComponent = shallowRef(pages.home);

const i18n = useI18n();
i18n.locale.value = config.language;
watch(config, () => {
  i18n.locale.value = config.language;
});

type ComponentName = "home" | "settings" | "game" | "market";

function changePage(event: MouseEvent | null, component: ComponentName) {
  // TODO: Add class active for event element
  if (typeof component === "string") {
    currentComponent.value = pages[component];
  } else {
    currentComponent.value = component;
  }
}

function jumpTo(name: ComponentName) {
  changePage(null, name);
}

onMounted(() => {
  console.log("Frontend loaded");
  requestAnimationFrame(() => {
    document.body.style.transform = "scale(1)";
    document.body.style.opacity = "1";
    setTimeout(() => {
      document.body.style.transform = "";
      document.body.style.transition = "";
    }, 500);
  });
});

function closeWindow() {
  requestAnimationFrame(() => {
    document.body.style.transition = "all 250ms cubic-bezier(0, 0.74, 0.65, 1)";
    document.body.style.transform = "scale(0.93)";
    document.body.style.opacity = "0";
    setTimeout(() => {
      appWindow.getCurrentWindow().close();
    }, 500);
  });
}

function isMacOS() {
  return window.__PLATFORM__.os_family === "Macos";
}
</script>

<style lang="less" scoped>
.window {
  width: 100%;
  height: 100%;
  display: flex;
}

.title-bar {
  height: 44px;
  width: calc(100% - 80px);
  position: absolute;
  left: 64px;
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

.sidebar {
  width: 64px;
  display: flex;
  flex-direction: column;
  align-items: center;

  .logo {
    width: 40px;
    height: 40px;
    margin-top: 20px;
    pointer-events: none;
  }
  .sidebar-items {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-top: 8px;
    margin-bottom: 22px;
  }
}

.sidebar-macos {
  padding-top: 24px;
}

main.main {
  position: fixed;
  right: 1px;
  bottom: 1px;
  height: calc(100vh - 44px);
  width: calc(100vw - 64px);
  border: var(--main-border);
  border-radius: 16px;
  border-bottom: unset;
  border-right: unset;
  border-bottom-left-radius: unset;
  border-top-right-radius: unset;
  background: var(--main-background);
}

.window-buttons-container {
  position: fixed;
  right: 24px;
  top: 0px;
  height: 44px;
  display: flex;
  align-items: center;
}

.window-buttons-container-macos {
  right: unset;
  left: 8px;
}
</style>
