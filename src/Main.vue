<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="window" data-tauri-drag-region>
    <div class="title-bar" data-tauri-drag-region>
      <div
        style="
          display: flex;
          width: calc(100vw - 560px);
          margin-left: 120px;
          flex-shrink: 0;
          align-items: center;
        ">
        <search-bar style="width: 100%" :placeholder="$t('globalSearch.placeholder')"></search-bar>
      </div>
      <AccountStatus></AccountStatus>
      <div style="display: flex; align-items: center; margin-right: 20px">
        <!-- TODO: Move to right and change ordering on macos -->
        <WindowButton
          button-type="minimize"
          @minimize="window.getCurrentWindow().minimize()"></WindowButton>
        <WindowButton
          button-type="maximize"
          @maximize="window.getCurrentWindow().maximize()"></WindowButton>
        <WindowButton button-type="close" @close="window.getCurrentWindow().close()"></WindowButton>
      </div>
    </div>
    <div class="sidebar" data-tauri-drag-region>
      <component :is="Logo" height="60" style="margin-top: 24px"></component>
      <ul class="sidebar-btns" data-tauri-drag-region>
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
import { markRaw, reactive, ref, shallowRef } from "vue";
import SearchBar from "./components/SearchBar.vue";
import SidebarItem from "./components/SidebarItem.vue";
import { window } from "@tauri-apps/api";
import Settings from "./pages/Settings.vue";
import Game from "./pages/Game.vue";
import { useConfigStore } from "./store/config";
import { watch } from "vue";
import { useI18n } from "vue-i18n";
import { loadTheme } from "./theme";
import Home from "./pages/Home.vue";
import { useTimeStore } from "./store/time";
import Market from "./pages/Market.vue";
import Logo from "@/assets/logo.svg";
import WindowButton from "./components/WindowButton.vue";
import AccountStatus from "./components/AccountStatus.vue";
import DialogRoot from "./DialogRoot.vue";

const config = useConfigStore();
loadTheme(config);

const pages = reactive({
  settings: markRaw(Settings),
  home: markRaw(Home),
  market: markRaw(Market),
  game: markRaw(Game),
});
const transitionName = ref("slide-up");
const currentComponent = shallowRef(pages.game);

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

const currentTime = useTimeStore();
setInterval(() => {
  currentTime.now = Math.round(new Date().getTime() / 1000);
}, 3000);
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

.sidebar {
  width: 80px;
  display: flex;
  flex-direction: column;
  align-items: center;

  .logo {
    width: 40px;
    height: 40px;
    margin-top: 20px;
    pointer-events: none;
  }
}

.sidebar .sidebar-btns {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  margin-top: 8px;
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
