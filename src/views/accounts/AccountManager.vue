<!-- Conic Launcher -->
<!-- Copyright 2022-2026 OakChaser and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="account-manager">
    <div class="left-panel">
      <div class="current-account">
        <AccountAvatar
          v-if="currentAccount"
          :skin="currentAccount.skinUrl"
          :size="64"
          :uuid="currentAccount.uuid"
          class="avatar"></AccountAvatar>
        <p class="account-name">{{ currentName }}</p>
        <p class="account-type" :class="`type-${currentType}`">
          {{ currentTypeLabel }}
        </p>
      </div>
      <div class="switch-section">
        <p class="section-title">切换档案</p>
        <ul class="account-list">
          <li
            v-for="account in allAccounts"
            :key="account.key"
            class="account-item"
            :class="{ active: account.key === currentKey }"
            @click="switchAccount(account)">
            <AccountAvatar
              :skin="account.skinUrl"
              :size="32"
              class="item-avatar"
              :uuid="account.uuid"></AccountAvatar>
            <div class="item-info">
              <span class="item-name">{{ account.name }}</span>
              <span class="item-type" :class="`type-${account.type}`">
                {{ getTypeLabel(account) }}
              </span>
            </div>
            <p
              v-if="isDefault(account.raw)"
              style="margin-left: auto; font-size: 11px; opacity: 0.8; margin-right: 4px">
              默认
            </p>
          </li>
          <li class="account-item" @click="$emit('switch-component-add')">
            <AppIcon name="user-add"></AppIcon>
            <div class="item-info">
              <span class="item-name">新增一个档案</span>
            </div>
          </li>
        </ul>
      </div>
    </div>
    <div class="right-panel">
      <p class="section-title" style="margin-top: 24px; margin-left: 16px">
        最近一年内使用此档案启动了 <strong>{{ totalLaunches }}</strong> 次游戏
      </p>
      <div class="section">
        <ActivityCalendar :data="calendarData"></ActivityCalendar>
      </div>

      <SettingGroup title="皮肤与披风">
        <div class="skin-section">
          <SkinModel3D
            v-if="currentAccount"
            :skin="currentAccount.skinUrl ?? getDefaultSkinUrl(currentAccount.uuid)"
            class="skin-model"></SkinModel3D>
          <CapeView
            v-if="currentAccount?.capeUrl"
            :cape="currentAccount.capeUrl"
            class="cape-view"></CapeView>
          <div class="skin-actions">
            <BaseButton
              class="skin-btn"
              :disabled="!currentAccount?.skinUrl"
              @click="saveSkinButton"
              >保存我的 Minecraft 角色皮肤</BaseButton
            >
            <BaseButton class="skin-btn" :disabled="currentAccount?.type === 'Yggdrasil'">{{
              uploadSkinButtonText
            }}</BaseButton>
            <p class="description">更换皮肤后，请重启 Minecraft，以便在游戏中看到新皮肤</p>
          </div>
        </div>
      </SettingGroup>
      <SettingGroup title="默认游戏档案">
        <SettingItem
          title="设为默认档案"
          description="当实例设置没有指定要使用的档案时，默认使用此档案启动游戏"
          :navigable="true"
          :disabled="currentIsDefault"
          @click="setDefault">
        </SettingItem>
      </SettingGroup>
      <SettingGroup :danger="true" title="删除档案">
        <SettingItem
          title="从 Conic Launcher 中删除档案"
          description="此操作将移除本地存储的帐户凭证，但不会影响 Microsoft 帐户本身"
          :navigable="true"
          icon="trash"
          @click="deleteAccount">
        </SettingItem>
      </SettingGroup>
    </div>
  </div>
</template>

<script setup lang="ts">
import AccountAvatar from "@/components/AccountAvatar.vue";
import SkinModel3D from "@/components/SkinModel3D.vue";
import CapeView from "@/components/CapeView.vue";
import ActivityCalendar from "@/components/ActivityCalendar.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import SettingItem from "@/components/SettingItem.vue";
import BaseButton from "@/components/base/BaseButton.vue";
import { useConfigStore } from "@/store/config";
import { useAccountStore } from "@/store/account";
import { useDialogStore } from "@/store/dialog";
import { computed, ref } from "vue";
import {
  getDefaultSkin,
  saveSkin,
  yggdrasilGetCapeUrl,
  yggdrasilGetSkinUrl,
  type Account,
  type MicrosoftAccount,
  type OfflineAccount,
  type YggdrasilAccount,
} from "@conic/account";
import AppIcon from "@/components/AppIcon.vue";
import { useYggdrasilServersStore } from "@/store/yggdrasilServers";
import { save } from "@tauri-apps/plugin-dialog";

const config = useConfigStore();
const accountStore = useAccountStore();
const dialogStore = useDialogStore();

if (!config.current_account) {
  if (accountStore.microsoft.length > 0) {
    config.current_account = {
      data: accountStore.microsoft[0],
      type: "Microsoft",
    };
  } else if (accountStore.yggdrasil.length > 0) {
    config.current_account = {
      data: accountStore.yggdrasil[0],
      type: "Yggdrasil",
    };
  } else {
    config.current_account = {
      data: accountStore.offline[0],
      type: "Offline",
    };
  }
}

type FlatAccount = {
  key: string;
  name: string;
  type: "Microsoft" | "Yggdrasil" | "Offline";
  skinUrl?: string;
  capeUrl?: string;
  uuid: string;
  raw: Account;
  accountKey?: string;
};

const allAccounts = computed<FlatAccount[]>(() => {
  const result: FlatAccount[] = [];

  for (const account of accountStore.microsoft) {
    result.push({
      key: `microsoft-${account.profile.uuid}`,
      name: account.profile.profile_name,
      type: "Microsoft",
      skinUrl: account.profile.skins.length > 0 ? account.profile.skins[0].url : undefined,
      capeUrl: account.profile.capes.find((cape) => cape.state === "ACTIVE")?.url,
      uuid: account.profile.uuid,
      raw: { type: "Microsoft", data: account },
    });
  }

  for (const [key, account] of Object.entries(accountStore.yggdrasil)) {
    result.push({
      key: `yggdrasil-${account.profile.id}`,
      name: account.profile.name,
      type: "Yggdrasil",
      skinUrl: yggdrasilGetSkinUrl(account.profile),
      capeUrl: yggdrasilGetCapeUrl(account.profile),
      uuid: account.profile.id,
      accountKey: key,
      raw: { type: "Yggdrasil", data: account },
    });
  }

  for (const account of accountStore.offline) {
    result.push({
      key: `offline-${account.uuid}`,
      name: account.name,
      type: "Offline",
      skinUrl: account.skin,
      uuid: account.uuid,
      raw: { type: "Offline", data: account },
    });
  }

  return result;
});

const currentKey = ref(
  (() => {
    const ca = config.current_account;
    if (!ca) return null;
    return `${ca.type.toLowerCase()}-${getUuidFromAccount(ca)}`;
  })(),
);

function getUuidFromAccount(account: Account): string {
  switch (account.type) {
    case "Microsoft":
      return (account.data as MicrosoftAccount).profile.uuid;
    case "Offline":
      return (account.data as OfflineAccount).uuid;
    case "Yggdrasil":
      return (account.data as YggdrasilAccount).profile.id;
  }
}

const currentAccount = computed(() => {
  return allAccounts.value.find((a) => a.key === currentKey.value) ?? null;
});

const currentName = computed(() => currentAccount.value?.name ?? "未选择帐户");

const currentType = computed(() => currentAccount.value?.type ?? "Offline");

const yggdrasilServersStore = useYggdrasilServersStore();

const currentTypeLabel = computed(() => {
  switch (currentType.value) {
    case "Microsoft":
      return "微软（正版帐户）";
    case "Yggdrasil":
      if (currentAccount.value?.raw.type === "Yggdrasil") {
        return `${yggdrasilServersStore.serverList[currentAccount.value?.raw.data.api_root]?.meta?.serverName ?? "Yggdrasil"}（外置登录）`;
      } else {
        return "Yggdrasil（外置登录）";
      }
    case "Offline":
      return "无认证服务（离线帐户）";
  }
});

function getTypeLabel(account: FlatAccount): string {
  switch (account.type) {
    case "Microsoft":
      return "微软（正版帐户）";
    case "Yggdrasil":
      if (account.raw.type === "Yggdrasil") {
        return `${yggdrasilServersStore.serverList[account.raw.data.api_root]?.meta?.serverName ?? "Yggdrasil"}（外置登录）`;
      } else {
        return "Yggdrasil（外置登录）";
      }
    case "Offline":
      return "无认证服务（离线帐户）";
  }
}

function switchAccount(account: FlatAccount) {
  currentKey.value = account.key;
}

const totalLaunches = computed(() => {
  return barChartData.value.reduce((sum, d) => sum + d.value, 0);
});

function generateCalendarData(): number[] {
  const data: number[] = [];
  for (let i = 0; i < 365; i++) {
    if (Math.random() > 0.6) {
      data.push(Math.floor(Math.random() * 8));
    } else {
      data.push(0);
    }
  }
  return data;
}

const calendarData = ref(generateCalendarData());

const barChartData = ref([
  { label: "Latest Release", value: 245 },
  { label: "Latest Snapshot", value: 87 },
  { label: "Forge 1.20", value: 134 },
  { label: "Fabric 1.21", value: 56 },
  { label: "Quilt 1.19", value: 23 },
]);

const defaultSkins = import.meta.glob("@/assets/images/skins/**/*.webp", {
  eager: true,
  import: "default",
});

function getDefaultSkinUrl(uuid: string) {
  const defaultSkin = getDefaultSkin(uuid);
  const skinUrl = defaultSkins[
    `/src/assets/images/skins/${defaultSkin.modelType}/${defaultSkin.textureName}.webp`
  ] as string;
  return skinUrl;
}

function deleteAccount() {
  if (!currentAccount.value) {
    return;
  }
  dialogStore.confirmDeleteAccount.account = currentAccount.value.raw;
  dialogStore.confirmDeleteAccount.visible = true;
}

function setDefault() {
  if (!currentAccount.value) {
    return;
  }
  config.current_account = currentAccount.value.raw;
}

const currentIsDefault = computed(() => {
  if (!currentAccount.value) {
    return false;
  }
  return isDefault(currentAccount.value.raw);
});

function isDefault(account: Account) {
  switch (account.type) {
    case "Microsoft":
      if (account.type === config.current_account?.type) {
        return account.data.profile.uuid === config.current_account?.data.profile.uuid;
      } else {
        return false;
      }
    case "Yggdrasil":
      if (account.type === config.current_account?.type) {
        return account.data.profile.id === config.current_account?.data.profile.id;
      } else {
        return false;
      }
    case "Offline":
      if (account.type === config.current_account?.type) {
        return account.data.uuid === config.current_account?.data.uuid;
      } else {
        return false;
      }
  }
}

async function saveSkinButton() {
  if (!currentAccount.value || !currentAccount.value.skinUrl) {
    return;
  }
  const path = await save({
    filters: [
      {
        name: "My Skin",
        extensions: ["png"],
      },
    ],
  });
  if (!path) {
    return;
  }
  saveSkin(currentAccount.value?.skinUrl, path);
}

const uploadSkinButtonText = computed(() => {
  switch (currentAccount.value?.type) {
    case "Offline":
      return "设置离线皮肤";
    case "Microsoft":
      return "上传新皮肤";
    case "Yggdrasil":
      return "上传新材质";
  }
  return "上传新皮肤";
});
</script>

<style lang="less" scoped>
.account-manager {
  width: 100%;
  height: 100%;
  display: flex;
  gap: 16px;
  overflow-y: auto;
  padding: 16px 16px 16px 16px;

  .left-panel {
    width: 280px;
    height: 100%;
    flex-shrink: 0;
    background: rgba(var(--ctp-base-rgb), 0.8);
    border-radius: 12px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    align-self: flex-start;
    overflow-y: auto;
  }
  .current-account {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 180px;
    border-bottom: 1px solid rgba(var(--ctp-overlay0-rgb), 0.3);
    margin-bottom: 16px;

    .avatar {
      filter: drop-shadow(0 0 8px rgba(0, 0, 0, 0.5));
    }

    .account-name {
      font-size: 20px;
      font-weight: 500;
      margin-top: 12px;
      text-align: center;
    }

    .account-type {
      font-size: 13px;
      margin-top: 4px;
      text-align: center;
    }
  }
  .type-Microsoft {
    color: var(--ctp-green);
  }

  .type-Yggdrasil {
    color: var(--ctp-yellow);
  }

  .type-Offline {
    color: var(--ctp-red);
  }

  .switch-section {
    display: flex;
    flex-direction: column;
  }

  .section-title {
    font-size: 13.5px;
  }
  .account-list {
    list-style: none;
    margin-top: 8px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .account-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 8px;
    transition: background 0.15s ease;
    background: var(--ctp-surface0);

    &:hover {
      background: var(--ctp-surface1);
    }

    &:active {
      background: var(--ctp-surface2);
    }

    .item-avatar {
      border-radius: 6px;
      overflow: hidden;
      flex-shrink: 0;
    }

    .item-info {
      display: flex;
      flex-direction: column;
      overflow: hidden;
    }

    .item-name {
      font-size: 13px;
      font-weight: 500;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .item-type {
      font-size: 11px;
    }
  }

  .right-panel {
    flex: 1;
    height: 100%;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-width: 0;
    background: rgba(var(--ctp-base-rgb), 0.8);
    border-radius: 16px;
    padding: 0 8px;
  }

  .section {
    border-radius: 12px;
    padding: 0 16px;

    .section-title {
      font-size: 14px;
      margin-bottom: 12px;
      color: rgba(var(--default-text-color), 0.85);

      strong {
        color: var(--ctp-text);
        font-weight: 600;
      }
    }
  }

  .skin-section {
    display: flex;
    align-items: center;
    gap: 24px;
    padding: 8px 16px;
    background: var(--ctp-surface0);

    .skin-model {
      flex-shrink: 0;
      margin-left: auto;
    }

    .skin-actions {
      display: flex;
      flex-direction: column;
      gap: 8px;
      margin-left: 8px;
      margin-right: auto;

      .skin-btn {
        width: 200px;
      }
      .description {
        font-size: 12px;
        width: 200px;
        margin-left: auto;
      }
    }
  }
  .cape-view {
    height: 100%;
    margin-left: 8px;
    margin-right: 8px;
  }
}
body.theme-Latte,
body.theme-Latte-hc {
  .section,
  .left-panel {
    background: none;
  }
  .account-item,
  .skin-section {
    background: rgba(var(--ctp-surface0-rgb), 0.4);
  }
}
</style>
