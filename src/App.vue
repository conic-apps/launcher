<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="window" data-tauri-drag-region>
    <div class="title-bar" data-tauri-drag-region>
      <div class="title-bar-container">
        <search-bar style="width: 100%" :placeholder="$t('globalSearch.placeholder')"></search-bar>
      </div>
      <div class="window-buttons-container window-buttons-container-macos" v-if="isMacOS()">
        <WindowButton button-type="close" @close="closeWindow()"></WindowButton>
        <WindowButton
          button-type="minimize"
          @minimize="appWindow.getCurrentWindow().minimize()"></WindowButton>
        <WindowButton
          button-type="maximize"
          @maximize="
            appWindow.getCurrentWindow().setFullscreen(!appWindow.getCurrentWindow().isFullscreen())
          "></WindowButton>
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
      <ul class="sidebar-items" data-tauri-drag-region>
        <sidebar-item
          title="Accounts"
          icon="person-circle-outline"
          @click="changePage($event, 'accounts')"
          id="sidebar-home"></sidebar-item>
        <sidebar-item
          :title="$t('sidebar.home')"
          icon="house"
          @click="changePage($event, 'home')"
          id="sidebar-home"></sidebar-item>
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
import WindowButton from "./components/WindowButton.vue";
import SearchBar from "./components/SearchBar.vue";
import SidebarItem from "./components/SidebarItem.vue";
import HomeView from "./views/HomeView.vue";
import MarketView from "./views/MarketView.vue";
import SettingsView from "./views/SettingsView.vue";
import AccountsView from "./views/AccountsView.vue";
import DialogRoot from "./DialogRoot.vue";
import { markRaw, onMounted, reactive, ref, shallowRef, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useConfigStore } from "./store/config";
import { loadPalette } from "./theme";
import { window as appWindow } from "@tauri-apps/api";
import { listAccounts, getAvatar } from "@conic/account";
import steveAvatar from "@/assets/images/steve_avatar.webp";

const config = useConfigStore();
loadPalette(
  {
    palette: config.appearance.palette,
    paletteFollowSystem: config.appearance.palette_follow_system,
  },
  config.accessibility.high_contrast_mode,
);

const currentAvatar = ref<string | null>(null);

async function updateAvatar() {
  if (!config.current_account) {
    currentAvatar.value = null;
    return;
  }
  try {
    const accounts = await listAccounts();
    const account = accounts.microsoft.find((a) => a.profile.uuid === config.current_account[0]);
    if (account && account.profile.skins.length > 0) {
      currentAvatar.value = await getAvatar(account.profile.skins[0].url, 32);
    } else {
      currentAvatar.value = null;
    }
  } catch {
    currentAvatar.value = null;
  }
}

const pages = reactive({
  settings: markRaw(SettingsView),
  home: markRaw(HomeView),
  market: markRaw(MarketView),
  accounts: markRaw(AccountsView),
});
const transitionName = ref("slide-up");
const currentComponent = shallowRef(pages.home);

const i18n = useI18n();
i18n.locale.value = config.language;
watch(config, () => {
  i18n.locale.value = config.language;
});

type ComponentName = "home" | "settings" | "market" | "accounts";

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
  updateAvatar();
  requestAnimationFrame(() => {
    document.body.style.transform = "scale(1)";
    document.body.style.opacity = "1";
    setTimeout(() => {
      document.body.style.transform = "";
      document.body.style.transition = "";
    }, 500);
  });
});

watch(
  () => config.current_account,
  () => updateAvatar(),
);

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

.sidebar {
  width: 64px;
  display: flex;
  flex-direction: column;
  align-items: center;

  .sidebar-avatar {
    width: 48px;
    height: 46px;
    margin-top: 8px;
    border-radius: 10px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    align-content: center;
    justify-content: center;

    img.avatar {
      width: 100%;
      height: 100%;
      object-fit: cover;
    }

    .avatar-default {
      filter: grayscale(1);
      image-rendering: pixelated;
    }

    .text {
      font-size: 10px;
      margin-top: 6px;
    }

    &:active {
      opacity: 0.6;
      transform: scale(0.97);
    }
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
  padding-top: 36px;
}

main.main {
  position: fixed;
  right: 1px;
  bottom: 1px;
  height: calc(100vh - 44px);
  width: calc(100vw - 64px);
  border: var(--main-border);
  border-bottom: unset;
  border-right: unset;
  border-radius: 16px 0 16px 0;
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
