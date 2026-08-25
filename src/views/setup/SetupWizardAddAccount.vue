<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="add-account">
    <div class="account-add-container" v-if="!configStore.current_account">
      <AccountAdd style="width: 100%; height: 100%"></AccountAdd>
    </div>
    <div class="account-view-container" v-else>
      <p class="wizard-title">{{ t("setup.addAccount.added") }}</p>
      <p class="wizard-message">
        {{ t("setup.addAccount.profileDesc") }}
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
          {{ t("setup.addAccount.microsoft") }}
        </p>
        <p
          class="profile-type yggdrasil"
          v-else-if="configStore.current_account.type === 'Yggdrasil'">
          {{ t("setup.addAccount.yggdrasil") }}
        </p>
        <p class="profile-type offline" v-else-if="configStore.current_account.type === 'Offline'">
          {{ t("setup.addAccount.offline") }}
        </p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import AccountAvatar from "@/components/AccountAvatar.vue";
import { useConfigStore } from "@/store/config";
import { useI18n } from "vue-i18n";

const { t } = useI18n();
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
