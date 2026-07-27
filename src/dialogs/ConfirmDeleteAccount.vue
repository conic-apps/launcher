<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <BaseDialog :visible="dialogStore.confirmDeleteAccount.visible" :width="380" :height="280">
    <div class="confirm-delete-account" ref="main">
      <p
        style="
          margin-top: -4px;
          margin-bottom: 16px;
          padding-bottom: 16px;
          border-bottom: var(--card-border);
        ">
        是否确认删除此帐户？
      </p>
      <div
        v-if="!deleting"
        class="dialog-button"
        @click="dialogStore.confirmDeleteAccount.visible = false">
        <i></i>
      </div>
      <AccountAvatar
        :skin="accountSkin"
        :size="64"
        :uuid="accountUuid"
        class="avatar"></AccountAvatar>
      <p class="account-name">{{ accountName }}</p>
      <p
        class="account-type"
        :style="{
          color: accountTypeLabelColor,
        }">
        {{ accountTypeLabel }}
      </p>
      <div class="buttons">
        <BaseButton
          style="width: 100%; margin-right: 8px"
          :disabled="deleting"
          @click="dialogStore.confirmDeleteAccount.visible = false"
          >Cancel</BaseButton
        >
        <BaseButton
          style="width: 100%; font-weight: bold"
          @click="confirmDelete"
          :disabled="deleting"
          color="rgb(210, 15, 57)"
          >{{ deleting ? "Deleting..." : "Delete this account" }}</BaseButton
        >
      </div>
    </div>
  </BaseDialog>
</template>

<script setup lang="ts">
import BaseDialog from "@/components/base/BaseDialog.vue";
import BaseButton from "@/components/base/BaseButton.vue";
import AccountAvatar from "@/components/AccountAvatar.vue";
import { computed, ref } from "vue";
import {
  deleteMicrosoftAccount,
  deleteOfflineAccount,
  deleteYggdrasilAccount,
  yggdrasilGetSkinUrl,
  type Account,
} from "@conic/account";
import { useDialogStore } from "@/store/dialog";
import { useAccountStore } from "@/store/account";
import { useConfigStore } from "@/store/config";
import { useYggdrasilServersStore } from "@/store/yggdrasilServers";

const dialogStore = useDialogStore();
const accountStore = useAccountStore();
const config = useConfigStore();
const yggdrasilServersStore = useYggdrasilServersStore();

const account = computed<Account | null>(() => {
  return dialogStore.confirmDeleteAccount.account;
});

const accountName = computed(() => {
  if (!account.value) return "";
  switch (account.value.type) {
    case "Microsoft":
      return account.value.data.profile.profile_name;
    case "Offline":
      return account.value.data.name;
    case "Yggdrasil":
      return account.value.data.profile.name;
  }
});

const accountUuid = computed(() => {
  if (!account.value) return "";
  switch (account.value.type) {
    case "Microsoft":
      return account.value.data.profile.uuid;
    case "Offline":
      return account.value.data.uuid;
    case "Yggdrasil":
      return account.value.data.profile.id;
  }
});

const accountSkin = computed(() => {
  if (!account.value) return undefined;
  switch (account.value.type) {
    case "Microsoft":
      return account.value.data.profile.skins.length > 0
        ? account.value.data.profile.skins[0].url
        : undefined;
    case "Offline":
      return account.value.data.skin;
    case "Yggdrasil":
      return yggdrasilGetSkinUrl(account.value.data.profile);
  }
});

const accountTypeLabel = computed(() => {
  if (!account.value) return "";
  switch (account.value.type) {
    case "Microsoft":
      return "微软（正版帐户）";
    case "Yggdrasil":
      if (account.value?.type === "Yggdrasil") {
        return `${yggdrasilServersStore.serverList[account.value?.data.api_root]?.meta?.serverName ?? "Yggdrasil"}（外置登录）`;
      } else {
        return "Yggdrasil（外置登录）";
      }
    case "Offline":
      return "无认证服务（离线帐户）";
  }
});

const accountTypeLabelColor = computed(() => {
  if (!account.value) return "";
  switch (account.value.type) {
    case "Microsoft":
      return "var(--ctp-green)";
    case "Yggdrasil":
      return "var(--ctp-yellow)";
    case "Offline":
      return "var(--ctp-red)";
  }
});

const deleting = ref(false);
async function confirmDelete() {
  if (!account.value) return;
  deleting.value = true;
  try {
    switch (account.value.type) {
      case "Microsoft":
        await deleteMicrosoftAccount(account.value.data.profile.uuid);
        break;
      case "Offline":
        await deleteOfflineAccount(account.value.data.uuid);
        break;
      case "Yggdrasil":
        await deleteYggdrasilAccount(account.value.data);
        break;
    }
    await accountStore.reloadFromFile();
    if (accountStore.microsoft.length > 0) {
      config.current_account = { type: "Microsoft", data: accountStore.microsoft[0] };
    } else if (accountStore.offline.length > 0) {
      config.current_account = { type: "Offline", data: accountStore.offline[0] };
    } else {
      const yggdrasilAccounts = Object.values(accountStore.yggdrasil);
      config.current_account =
        yggdrasilAccounts.length > 0 ? { type: "Yggdrasil", data: yggdrasilAccounts[0] } : null;
    }
  } finally {
    deleting.value = false;
    dialogStore.confirmDeleteAccount.visible = false;
  }
}
</script>

<style lang="less" scoped>
.confirm-delete-account {
  width: 100%;
  height: 100%;
  padding: 12px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  align-items: center;
  position: relative;

  .avatar {
    filter: drop-shadow(0 0 8px rgba(0, 0, 0, 0.5));
  }

  p.account-name {
    font-size: 22px;
    text-align: center;
    margin-top: 8px;
    font-weight: 500;
  }

  p.account-type {
    font-size: 13px;
    margin-top: 4px;
    text-align: center;
  }

  > p {
    font-size: 16px;
    margin: 16px 0 8px 0;
    width: 100%;
  }

  div.buttons {
    margin-top: 8px;
    width: 100%;
    display: flex;
    margin-top: auto;
  }
}

.dialog-button {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  position: absolute;
  top: 4px;
  right: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform 100ms;
  background: var(--close-button-background);

  i::before {
    content: "\f00d";
    font-size: 12px;
    margin-top: 1px;
    margin-left: 0.6px;
    font-style: normal;
    font-family: "fa-pro";
    opacity: 0;
    transition: all 70ms ease;
  }

  i {
    transition: all 100ms ease;
  }
}
</style>
