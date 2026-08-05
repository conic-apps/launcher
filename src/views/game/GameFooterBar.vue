<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="game-view-footer" data-tauri-drag-region>
    <AccountAvatar
      v-if="configStore.current_account"
      :skin="accountSkin"
      :uuid="accountUuid"
      :size="56"
      :class="{
        'ms-account': configStore.current_account?.type === 'Microsoft',
        'ygg-account': configStore.current_account?.type === 'Yggdrasil',
        'offline-account': configStore.current_account?.type === 'Offline',
      }"
      @click="navigationStore.navigate('accounts')"></AccountAvatar>
    <AccountAvatar
      v-else
      :skin="SteveSkin"
      :uuid="accountUuid"
      :size="56"
      class="unlogin"
      @click="navigationStore.navigate('accounts')"></AccountAvatar>
    <p
      class="profile-name"
      v-if="configStore.current_account"
      @click="navigationStore.navigate('accounts')">
      <span>{{ profileName }}</span>
      <button class="account-switch" @click.stop=""><AppIcon name="chevron-up"></AppIcon></button>
    </p>
    <p
      class="profile-name"
      v-else
      style="border-radius: 8px; padding-right: 16px"
      @click="navigationStore.navigate('accounts')">
      <span>未登录</span>
    </p>
    <button class="connect" @click="openConnect">
      <AppIcon name="globe" :size="22"></AppIcon>
    </button>
    <button class="new-instance" @click="dialogStore.createInstance.visible = true">
      <AppIcon name="add" :size="22" style="margin-right: 8px"></AppIcon>
      创建新游戏
    </button>
    <button class="install-pack" @click="dialogStore.createInstance.visible = true">
      <AppIcon name="package" :size="22" fill="none"></AppIcon>
    </button>
    <button class="install-server" @click="dialogStore.createInstance.visible = true">
      <AppIcon name="server" :size="22" fill="none"></AppIcon>
    </button>
  </div>
</template>

<script setup lang="ts">
import AccountAvatar from "@/components/AccountAvatar.vue";
import AppIcon from "@/components/AppIcon.vue";
import { useConfigStore } from "@/store/config";
import { useDialogStore } from "@/store/dialog";
import { useNavigationStore } from "@/store/navigation";
import { yggdrasilGetSkinUrl } from "@conic/account";
import { computed } from "vue";
import SteveSkin from "@/assets/images/skins/wide/steve.webp?url";
import { isLibraryValid } from "@conic/multiplayer";

const configStore = useConfigStore();
const dialogStore = useDialogStore();
const navigationStore = useNavigationStore();

const accountSkin = computed(() => {
  if (configStore.current_account?.type === "Microsoft") {
    return configStore.current_account.data.profile.skins.length > 0
      ? configStore.current_account.data.profile.skins[0].url
      : undefined;
  } else if (configStore.current_account?.type === "Yggdrasil") {
    return yggdrasilGetSkinUrl(configStore.current_account.data.profile);
  } else if (configStore.current_account?.type === "Offline") {
    return configStore.current_account.data.skin;
  } else {
    return undefined;
  }
});

const accountUuid = computed(() => {
  if (configStore.current_account?.type === "Microsoft") {
    return configStore.current_account.data.profile.uuid;
  } else if (configStore.current_account?.type === "Yggdrasil") {
    return configStore.current_account.data.profile.id;
  } else {
    return configStore.current_account ? configStore.current_account.data.uuid : "";
  }
});

const profileName = computed(() => {
  if (configStore.current_account?.type === "Microsoft") {
    return configStore.current_account.data.profile.profile_name;
  } else if (configStore.current_account?.type === "Yggdrasil") {
    return configStore.current_account.data.profile.name;
  } else {
    return configStore.current_account ? configStore.current_account.data.name : "";
  }
});

async function openConnect() {
  if (await isLibraryValid()) {
    dialogStore.connectExtension.currentComponent = "connectManager";
  } else {
    dialogStore.connectExtension.currentComponent = "downloadDescription";
  }
  dialogStore.connectExtension.visible = true;
}
</script>

<style lang="less" scoped>
.game-view-footer {
  width: 100vw;
  height: 56px;
  position: absolute;
  bottom: 0;
  background: rgba(var(--ctp-surface0-rgb), 0.6);
  display: flex;
  align-items: center;
  backdrop-filter: blur(4px);

  > img {
    border-radius: 1000px;
    padding: 2px;
    background: var(--ctp-surface0);
    margin-bottom: 36px;
    margin-left: 28px;
    z-index: 100;
  }

  > img.ms-account {
    border: 2px solid var(--ctp-green);
  }

  > img.ygg-account {
    border: 2px solid var(--ctp-yellow);
  }

  > img.offline-account {
    border: 2px solid var(--ctp-red);
  }

  > img.unlogin {
    filter: grayscale(1);
    border: 2px solid var(--ctp-overlay0);
  }

  .profile-name {
    margin-left: -16px;
    margin-bottom: 32px;
    background: var(--ctp-surface0);
    height: 36px;
    border-radius: 8px;
    font-size: 14px;
    display: flex;
    align-items: center;
    padding: 0 0 0 32px;
    box-shadow: 0 0 10px 0px rgba(0, 0, 0, 0.2);

    button.account-switch {
      appearance: none;
      background: var(--ctp-surface1);
      margin-left: 16px;
      height: 100%;
      width: 36px;
      border: none;
      border-radius: 0 8px 8px 0;

      &:hover {
        background: var(--ctp-surface2);
      }

      &:active {
        background: var(--ctp-overlay0);
      }
    }
  }
  .connect {
    margin-left: 8px;
  }
  .connect,
  .new-instance,
  .install-pack,
  .install-server {
    appearance: none;
    background: var(--ctp-surface1);
    border-radius: 8px;
    border: none;
    box-shadow: 0 0 10px 0px rgba(0, 0, 0, 0.2);
    display: flex;
    align-items: center;
    height: 36px;
    font-size: 14px;
    padding: 0 12px;
    margin-bottom: 32px;
    &:hover {
      background: var(--ctp-surface2);
    }
    &:active {
      background: var(--ctp-overlay0);
    }
  }
  .new-instance {
    margin-left: auto;
  }
  .install-pack,
  .install-server {
    margin-left: 8px;
  }
  .install-server {
    margin-right: 36px;
  }
}
</style>
