<!-- Conic Launcher -->
<!-- Copyright 2022-2026 OakChaser and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="window" data-tauri-drag-region>
    <AppBackground style="position: fixed"></AppBackground>
    <div class="title-bar" data-tauri-drag-region>
      <div class="title-bar-container">
        <search-bar style="width: 100%" :placeholder="$t('globalSearch.placeholder')"></search-bar>
      </div>
      <div
        class="window-buttons-container window-buttons-container-macos"
        @mouseenter="windowButtonHover = true"
        @mouseleave="windowButtonHover = false"
        v-if="isMacOS()">
        <WindowButton
          button-type="close"
          @close="closeWindow()"
          :hover="windowButtonHover"
          :lit="windowButtonLit"></WindowButton>
        <WindowButton
          button-type="minimize"
          :lit="windowButtonLit"
          :hover="windowButtonHover"
          @minimize="appWindow.getCurrentWindow().minimize()"></WindowButton>
        <WindowButton
          button-type="maximize"
          :lit="windowButtonLit"
          :hover="windowButtonHover"
          @maximize="
            appWindow.getCurrentWindow().setFullscreen(!appWindow.getCurrentWindow().isFullscreen())
          "></WindowButton>
      </div>
      <div
        class="window-buttons-container"
        @mouseenter="windowButtonHover = true"
        @mouseleave="windowButtonHover = false"
        v-else>
        <WindowButton
          button-type="minimize"
          :lit="windowButtonLit"
          :hover="windowButtonHover"
          @minimize="appWindow.getCurrentWindow().minimize()"></WindowButton>
        <WindowButton
          :lit="windowButtonLit"
          :hover="windowButtonHover"
          button-type="maximize"
          @maximize="appWindow.getCurrentWindow().maximize()"></WindowButton>
        <WindowButton
          button-type="close"
          @close="closeWindow()"
          :hover="windowButtonHover"
          :lit="windowButtonLit"></WindowButton>
      </div>
      <div class="title-bar-actions">
        <button class="title-bar-action-btn">
          <AppIcon name="server" :size="18" />
        </button>
        <button class="title-bar-action-btn" @click="navigation.navigate('market')">
          <AppIcon name="earth" :size="18" />
        </button>
        <button class="title-bar-action-btn" @click="navigation.navigate('settings')">
          <AppIcon name="settings" :size="18" />
        </button>
      </div>
    </div>
    <main class="main" style="transition: none">
      <Transition :name="transitionName" mode="out-in">
        <component :is="pages[navigation.currentPage]"></component>
      </Transition>
    </main>
    <DialogRoot></DialogRoot>
  </div>
</template>

<script setup lang="ts">
import WindowButton from "./components/WindowButton.vue";
import SearchBar from "./components/SearchBar.vue";
import GameView from "./views/GameView.vue";
import MarketView from "./views/MarketView.vue";
import SettingsView from "./views/SettingsView.vue";
import AccountsView from "./views/AccountsView.vue";
import DialogRoot from "./DialogRoot.vue";
import { computed, markRaw, onMounted, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useConfigStore } from "./store/config";
import { loadPalette } from "./theme";
import { window as appWindow } from "@tauri-apps/api";
import { Event } from "@tauri-apps/api/event";
import AppBackground from "./components/AppBackground.vue";
import { useNavigationStore } from "./store/navigation";
import { getSystemLanguage } from "@conic/config";
import LaunchView from "./views/LaunchView.vue";
import InstancesView from "./views/InstancesView.vue";

const config = useConfigStore();
const navigation = useNavigationStore();

loadPalette(
  {
    palette: config.appearance.palette,
    paletteFollowSystem: config.appearance.palette_follow_system,
  },
  config.accessibility.high_contrast_mode,
);

const appWindowFocused = ref(true);
const windowButtonHover = ref(false);

const windowButtonLit = computed(() => {
  return windowButtonHover.value ? true : appWindowFocused.value;
});

appWindow
  .getCurrentWindow()
  .isFocused()
  .then((focused) => {
    appWindowFocused.value = focused;
  });
appWindow.getCurrentWindow().onFocusChanged((event: Event<boolean>) => {
  appWindowFocused.value = event.payload;
});

const pages = reactive({
  settings: markRaw(SettingsView),
  game: markRaw(GameView),
  launch: markRaw(LaunchView),
  market: markRaw(MarketView),
  accounts: markRaw(AccountsView),
  instances: markRaw(InstancesView),
});
const transitionName = ref("slide-up");

const i18n = useI18n();
getSystemLanguage().then((systemLanguage) => {
  i18n.locale.value = config.language ?? systemLanguage;
});
watch(
  () => config.language,
  async (value) => {
    i18n.locale.value = value ?? (await getSystemLanguage());
  },
);

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
  width: 100%;
  position: fixed;
  display: flex;
  border-radius: 16px 16px 0 0;
  align-items: center;
  justify-content: space-between;
  background: rgba(var(--ctp-surface0-rgb), 0.8);

  .title-bar-container {
    display: flex;
    width: calc(100vw - 500px);
    margin-left: 160px;
    flex-shrink: 0;
    align-items: center;
  }

  .account {
    background: var(--controllers-background);
    border: var(--controllers-border);
    border-radius: var(--controllers-border-radius);
    display: flex;
    align-items: center;
    padding: 4px 8px;
  }
}

main.main {
  height: calc(100vh - 44px);
  margin-top: 44px;
  width: 100vw;
  border-radius: 0 0 16px 16px;
  transform: translateZ(0);
}

.window-buttons-container {
  position: fixed;
  right: 24px;
  top: 14px;
  height: fit-content;
  display: flex;
  align-items: center;
}

.window-buttons-container-macos {
  right: unset;
  left: 8px;
}

.title-bar-avatar {
  position: relative;
  margin-left: 8px;
  width: 22px;
  height: 22px;
  border-radius: 50%;
  overflow: hidden;
  flex-shrink: 0;

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 50%;
  }

  .tooltip {
    position: absolute;
    bottom: calc(100% + 8px);
    left: 50%;
    transform: translateX(-50%) translateY(4px);
    padding: 4px 10px;
    background: var(--card-background);
    color: rgb(var(--default-text-color));
    border: 1px solid rgba(var(--ctp-mocha-overlay2-rgb), 0.5);
    border-radius: var(--tag-border-radius);
    font-size: 12px;
    white-space: nowrap;
    pointer-events: none;
    opacity: 0;
    transition:
      opacity 0.1s ease,
      transform 0.1s ease;
  }

  &:hover .tooltip {
    opacity: 1;
    transform: translateX(-50%) translateY(0);
  }
}

.title-bar-actions {
  position: fixed;
  right: 10px;
  top: 8px;
  z-index: 10;
  display: flex;
  align-items: center;
  gap: 4px;
}

.title-bar-action-btn {
  appearance: none;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--window-btn-icon-color);
  transition: background 120ms ease;

  &:hover {
    background: rgba(255, 255, 255, 0.1);
  }
}
</style>
