<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="account-view">
    <div class="row1">
      <div>
        <list-item
          v-for="(account, index) in accounts"
          :key="index"
          :title="account.profile.profile_name"
          :logo="account.profile.avatar"
          :click-able="true"
          :buttons="['refresh', 'trash']"
          @click-refresh="refreshLogin(account.profile.uuid)"
          @click-trash="deleteAccount(account.profile.uuid)"
          @click="chooseAccount(account)">
          <template #subtitle>
            <tag
              v-if="
                currentTime.now >
                  (account.token_deadline ? account.token_deadline : currentTime.now + 100000) &&
                account.account_type === 'Microsoft'
              "
              text="需要刷新"
              :color="['249', '226', '175']"
              text-color="#f9e2af"
              :background="false"
              :border="true"
              font-size="10"
              :round="true"></tag>
          </template>
          <AppIcon
            name="badge-check"
            stroke="#74c7ec"
            fill="none"
            size="18"
            style="margin-right: 4px"></AppIcon>
          微软（验证服务）
        </list-item>
      </div>
      <div style="margin-top: 8px">
        <list-item
          class="list-item-user-plus"
          title="添加帐号"
          logo="user-add"
          @click="$emit('add')"
          :click-able="true"></list-item>
      </div>
    </div>
    <div class="row2">
      <p>在左侧选择帐号以查看皮肤</p>
    </div>
  </div>
</template>

<script lang="ts" setup>
import ListItem from "@/components/ListItem.vue";
import Tag from "@/components/Tag.vue";
import { listen } from "@tauri-apps/api/event";
import { ref } from "vue";
import { useConfigStore } from "@/store/config";
import { getAvatar } from "@/avatar";
import { useTimeStore } from "@/store/time";
import {
  Account,
  deleteAccount,
  listAccounts,
  refreshMicrosoftAccountByUuid,
} from "@conic/account";
import AppIcon from "@/components/AppIcon.vue";

const config = useConfigStore();

defineEmits(["add"]);

const accounts = ref<Account[]>([]);

async function getAccounts() {
  const res: Account[] = await listAccounts();
  for (let i = 0; i <= res.length - 1; i++) {
    res[i].profile.avatar = await getAvatar(res[i].profile.skins[0].url, 32);
  }
  accounts.value = res;
}

const currentTime = useTimeStore();

getAccounts().then(() => {});

listen("refresh_accounts_list", () => {
  getAccounts();
});

function refreshLogin(uuid: string) {
  refreshMicrosoftAccountByUuid(uuid);
}

function chooseAccount(account: Account) {
  config.current_account = account.profile.uuid;
}
</script>

<style lang="less" scoped>
.account-view {
  width: 100%;
  height: 100%;
  display: flex;
}

.row1 {
  width: 50%;
  height: 100%;
  padding: 0 12px;
  overflow: auto;

  > div {
    border-radius: 8px;
    overflow: hidden;
  }

  .list-item {
    width: 100%;
  }
}

.row2 {
  width: 50%;
  height: 100%;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;

  p {
    font-style: italic;
    opacity: 0.6;
  }
}
</style>
