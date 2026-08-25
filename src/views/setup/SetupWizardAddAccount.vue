<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="add-account">
    <div class="account-add-container" v-if="!configStore.current_account">
      <AccountAdd style="width: 100%; height: 100%"></AccountAdd>
    </div>
    <div class="account-view-container" v-else>
      <p class="wizard-title">帐户已添加！</p>
      <p class="wizard-message">
        以下档案已设为默认档案。当你没有给实例设置要使用的档案时，启动器会使用此档案登录游戏
      </p>
      <div class="profile-card">
        <AccountAvatar
          v-if="configStore.current_account"
          ref="avatar"
          :skin="accountSkin"
          :uuid="accountUuid"
          :size="56"
          class="avatar"></AccountAvatar>
        <p class="profile-name">
          {{ accountProfileName }}
        </p>
        <p class="profile-type microsoft" v-if="configStore.current_account.type === 'Microsoft'">
          微软（正版帐户）
        </p>
        <p
          class="profile-type yggdrasil"
          v-else-if="configStore.current_account.type === 'Yggdrasil'">
          Yggdrasil（外置登录）
        </p>
        <p class="profile-type offline" v-else-if="configStore.current_account.type === 'Offline'">
          无认证服务（离线帐户）
        </p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import AccountAvatar from "@/components/AccountAvatar.vue";
import { useConfigStore } from "@/store/config";
import { yggdrasilGetSkinUrl } from "@conic/account";
import { computed } from "vue";
import AccountAdd from "../accounts/AccountAdd.vue";

const configStore = useConfigStore();

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

const accountProfileName = computed(() => {
  if (configStore.current_account?.type === "Microsoft") {
    return configStore.current_account.data.profile.profile_name;
  } else if (configStore.current_account?.type === "Yggdrasil") {
    return configStore.current_account.data.profile.name;
  } else {
    return configStore.current_account ? configStore.current_account.data.name : "";
  }
});
</script>

<style lang="less" scoped>
.add-account {
  height: 100%;
  :deep(.account-add) {
    padding: 0;
  }
  :deep(.account-add-container),
  :deep(.auth-code) {
    height: 100%;
  }
  :deep(.cancel-login) {
    display: none;
  }
  :deep(.add-microsoft-account-container) {
    flex: 1;
  }
  :deep(.link) {
    margin: 36px 0;
  }
  :deep(.device-code .link) {
    margin-top: 36px;
  }
  .account-view-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    p.wizard-title {
      width: 100%;
    }
    .avatar {
      filter: drop-shadow(0 0 8px rgba(0, 0, 0, 0.5));
    }
    p.profile-name {
      margin-top: 16px;
      font-size: 16px;
    }
    p.profile-type {
      font-size: 13px;
      margin-top: 8px;
      &.microsoft {
        color: var(--ctp-green);
      }
      &.yggdrasil {
        color: var(--ctp-yellow);
      }
      &.offline {
        color: var(--ctp-red);
      }
    }
  }
}
.profile-card {
  background: var(--ctp-surface0);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  border-radius: 12px;
  padding: 24px;
  margin-top: 24px;
  width: 320px;
}
</style>
