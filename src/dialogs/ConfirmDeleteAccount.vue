<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <BaseDialog :visible="dialogStore.confirmDeleteAccount.visible" :width="400" :height="138">
    <div class="confirm-quit-app" ref="main">
      <div style="display: flex; align-items: center">
        <AccountAvatar :skin="accountSkin" :uuid="accountUuid" :size="40"></AccountAvatar>
        <div class="message">
          <p style="font-size: 16px">是否确认删除此帐户？</p>
          <p style="font-size: 12px; margin-top: 8px">最后的反悔机会</p>
        </div>
      </div>
      <div class="buttons">
        <BaseButton class="back" @click="dialogStore.confirmDeleteAccount.visible = false">
          取消
        </BaseButton>
        <BaseButton class="quit" @click="confirmDelete">确认删除</BaseButton>
      </div>
    </div>
  </BaseDialog>
</template>

<script setup lang="ts">
import BaseDialog from "@/components/BaseDialog.vue";
import BaseButton from "@/components/BaseButton.vue";
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

const dialogStore = useDialogStore();
const accountStore = useAccountStore();
const config = useConfigStore();

const account = computed<Account | null>(() => {
  return dialogStore.confirmDeleteAccount.account;
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
.confirm-quit-app {
  padding: 8px;
  .message {
    display: flex;
    flex-direction: column;
    justify-content: center;
    margin-left: 16px;
  }
  .buttons {
    display: flex;
    width: 100%;
    margin-top: 16px;
    button {
      appearance: none;
      border: none;
      width: 100%;
      border-radius: 4px;
      transition: transform 200ms ease;
    }
    button.back {
      margin-right: 8px;
      background: var(--ctp-blue);
      color: var(--ctp-text-inverse);
      padding: 8px 0;
    }
    button.quit {
      background: var(--ctp-red);
      color: var(--ctp-text-inverse);
    }
    button:hover {
      transform: scale(1.02);
    }
    button:active {
      transform: scale(0.97);
    }
  }
}
</style>
