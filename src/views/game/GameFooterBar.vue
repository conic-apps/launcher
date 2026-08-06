<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="game-view-footer" ref="game-view-footer" data-tauri-drag-region>
    <AccountAvatar
      v-if="configStore.current_account"
      ref="avatar"
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
      ref="avatar"
      :skin="SteveSkin"
      :uuid="accountUuid"
      :size="56"
      class="unlogin"
      @click="navigationStore.navigate('accounts')"></AccountAvatar>
    <p class="profile-name" v-if="configStore.current_account" ref="accountMenuRef">
      <span>{{ profileName }}</span>
      <button class="account-switch" tabindex="-1" @click.stop="accountMenuOpen = !accountMenuOpen">
        <AppIcon name="chevron-up"></AppIcon>
      </button>
      <Transition name="game-footer-dropdown-fade">
        <ul class="dropdown" v-if="accountMenuOpen">
          <li
            class="dropdown-option"
            v-for="player in allPlayers"
            :key="player.key"
            :class="{ selected: currentAccountKey === player.key }"
            @click.stop="selectPlayer(player)">
            <AccountAvatar :skin="player.skinUrl" :uuid="player.uuid" :size="18"></AccountAvatar>
            <span class="player-name">{{ player.name }}</span>
          </li>
        </ul>
      </Transition>
    </p>
    <p
      class="profile-name"
      v-else
      ref="accountMenuRef"
      style="border-radius: 8px; padding-right: 16px"
      @click="navigationStore.navigate('accounts')">
      <span>未登录</span>
    </p>
    <button class="connect" @click="openConnect" ref="connect">
      <AppIcon name="globe" :size="22"></AppIcon>
    </button>
    <button
      class="new-instance"
      @click="dialogStore.createInstance.visible = true"
      ref="new-instance">
      <AppIcon name="add" :size="22" style="margin-right: 8px"></AppIcon>
      创建新游戏
    </button>
    <button
      class="install-pack"
      @click="dialogStore.createInstance.visible = true"
      ref="install-pack">
      <AppIcon name="package" :size="22" fill="none"></AppIcon>
    </button>
    <button
      class="install-server"
      @click="dialogStore.createInstance.visible = true"
      ref="install-server">
      <AppIcon name="server" :size="22" fill="none"></AppIcon>
    </button>
  </div>
</template>

<script setup lang="ts">
import AccountAvatar from "@/components/AccountAvatar.vue";
import AppIcon from "@/components/AppIcon.vue";
import { useAccountStore } from "@/store/account";
import { useConfigStore } from "@/store/config";
import { useDialogStore } from "@/store/dialog";
import { useNavigationStore } from "@/store/navigation";
import { yggdrasilGetSkinUrl, type Account } from "@conic/account";
import { computed, nextTick, onMounted, onUnmounted, ref, useTemplateRef } from "vue";
import SteveSkin from "@/assets/images/skins/wide/steve.webp?url";
import { isLibraryValid } from "@conic/multiplayer";
import gsap from "gsap";

const configStore = useConfigStore();
const dialogStore = useDialogStore();
const navigationStore = useNavigationStore();
const accountStore = useAccountStore();

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

const accountMenuOpen = ref(false);

const accountMenuRef = useTemplateRef("accountMenuRef");

function onPointerDownOutside(event: PointerEvent) {
  const target = event.target as HTMLElement;
  if (accountMenuRef.value && !accountMenuRef.value.contains(target)) {
    accountMenuOpen.value = false;
  }
}

onMounted(() => {
  document.addEventListener("pointerdown", onPointerDownOutside);
});

onUnmounted(() => {
  document.removeEventListener("pointerdown", onPointerDownOutside);
});

type PlayerItem = {
  key: string;
  name: string;
  skinUrl?: string;
  uuid: string;
  raw: Account;
};

const allPlayers = computed<PlayerItem[]>(() => {
  const result: PlayerItem[] = [];

  for (const account of accountStore.microsoft) {
    result.push({
      key: `microsoft-${account.profile.uuid}`,
      name: account.profile.profile_name,
      skinUrl: account.profile.skins.length > 0 ? account.profile.skins[0].url : undefined,
      uuid: account.profile.uuid,
      raw: { type: "Microsoft", data: account },
    });
  }

  for (const account of accountStore.yggdrasil) {
    result.push({
      key: `yggdrasil-${account.profile.id}`,
      name: account.profile.name,
      skinUrl: yggdrasilGetSkinUrl(account.profile),
      uuid: account.profile.id,
      raw: { type: "Yggdrasil", data: account },
    });
  }

  for (const account of accountStore.offline) {
    result.push({
      key: `offline-${account.uuid}`,
      name: account.name,
      skinUrl: account.skin,
      uuid: account.uuid,
      raw: { type: "Offline", data: account },
    });
  }

  return result;
});

const currentAccountKey = computed(() => {
  const currentAccount = configStore.current_account;
  if (!currentAccount) {
    return null;
  }
  switch (currentAccount.type) {
    case "Microsoft":
      return `microsoft-${currentAccount.data.profile.uuid}`;
    case "Yggdrasil":
      return `yggdrasil-${currentAccount.data.profile.id}`;
    case "Offline":
      return `offline-${currentAccount.data.uuid}`;
  }
});

function selectPlayer(player: PlayerItem) {
  configStore.current_account = player.raw;
  accountMenuOpen.value = false;
}

async function openConnect() {
  if (await isLibraryValid()) {
    dialogStore.connectExtension.currentComponent = "connectManager";
  } else {
    dialogStore.connectExtension.currentComponent = "downloadDescription";
  }
  dialogStore.connectExtension.visible = true;
}

const elements = {
  root: useTemplateRef("game-view-footer"),
  avatar: useTemplateRef("avatar"),
  profileName: accountMenuRef,
  connect: useTemplateRef("connect"),
  newInstance: useTemplateRef("new-instance"),
  installPack: useTemplateRef("install-pack"),
  installServer: useTemplateRef("install-server"),
};

let resolveReady: () => void;

const ready = new Promise<void>((resolve) => {
  resolveReady = resolve;
});

onMounted(() => {
  elements.avatar.value?.ready.then(() => resolveReady());
});

const playIntro = () => {
  return gsap
    .timeline()
    .from(elements.root.value, {
      opacity: 1,
      y: 56,
      duration: 0.6,
      ease: "power3.out",
    })
    .from(
      [elements.avatar.value?.$el, elements.profileName.value, elements.connect.value],
      {
        opacity: 0,
        y: 10,
        duration: 0.2,
        stagger: 0.05,
        ease: "power3.out",
      },
      "<0.1",
    )
    .from(
      [elements.installServer.value, elements.installPack.value, elements.newInstance.value],
      {
        opacity: 0,
        y: 10,
        duration: 0.2,
        stagger: 0.05,
        ease: "power3.out",
      },
      "<",
    );
};

defineExpose({
  playIntro,
  ready,
});
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

  > .avatar {
    border-radius: 1000px;
    padding: 2px;
    background: var(--ctp-surface0);
    margin-bottom: 36px;
    margin-left: 28px;
    z-index: 100;
  }

  > .avatar.ms-account {
    border: 2px solid var(--ctp-green);
  }

  > .avatar.ygg-account {
    border: 2px solid var(--ctp-yellow);
  }

  > .avatar.offline-account {
    border: 2px solid var(--ctp-red);
  }

  > .avatar.unlogin {
    filter: grayscale(1);
    border: 2px solid var(--ctp-overlay0);
  }

  .profile-name {
    position: relative;
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

    .dropdown {
      position: absolute;
      right: 0;
      bottom: calc(100% + 4px);
      min-width: 100%;
      padding: 8px 10px;
      border-radius: var(--dialog-border-radius);
      border: var(--controllers-border);
      background: var(--ctp-base);
      box-shadow: 0px 0px 10px #4500611d;
      z-index: 100000;
      list-style: none;

      .dropdown-option {
        height: 26px;
        padding: 0 8px;
        display: flex;
        align-items: center;
        gap: 8px;
        margin: 4px 0;
        border-radius: var(--controllers-border-radius);
        font-size: 12px;
        list-style: none;
        white-space: nowrap;
        transition: all 30ms ease;

        &:hover {
          background: #ffffff1f;
        }

        &:active {
          background: #ffffff15;
        }

        &.selected {
          background: #ffffff17;
        }

        .player-name {
          overflow: hidden;
          text-overflow: ellipsis;
        }
      }
    }

    .game-footer-dropdown-fade-leave-active,
    .game-footer-dropdown-fade-enter-active {
      transition: all 120ms ease;
    }

    .game-footer-dropdown-fade-leave-from,
    .game-footer-dropdown-fade-enter-to {
      opacity: 1;
    }

    .game-footer-dropdown-fade-leave-to,
    .game-footer-dropdown-fade-enter-from {
      opacity: 0;
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
