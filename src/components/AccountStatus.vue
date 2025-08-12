<template>
  <div class="account" @click="dialogStore.accountManager = true">
    <div class="avatar">
      <img :src="currentAccountProfile.avatar" alt="player avatar" />
    </div>
    <span>{{ currentAccountProfile.name }}</span>
    <!-- TODO: Remove this, auto refresh when expired -->
    <tag
      style="margin-left: 8px"
      v-if="
        currentTime.now > currentAccountProfile.tokenDeadline &&
        currentAccountProfile.type === 'Microsoft'
      "
      text="需要刷新"
      :color="['249', '226', '175']"
      text-color="#f9e2af"
      :background="false"
      :border="true"
      font-size="10"
      :round="true"></tag>
  </div>
</template>

<script setup lang="ts">
import { useDialogStore } from "@/store/dialog";
import Tag from "./Tag.vue";
import { getAvatar, getMicrosoftAccount, refreshAllMicrosoftAccounts } from "@conic/account";
import { ref } from "vue";
import { useConfigStore } from "@/store/config";
import { listen } from "@tauri-apps/api/event";
import { useTimeStore } from "@/store/time";

const dialogStore = useDialogStore();
const configStore = useConfigStore();

const currentAccountProfile = ref<{
  name: string;
  avatar: string;
  tokenDeadline: number;
  type: "Microsoft" | "Offline";
}>({
  name: "Steve",
  avatar: "@/assets/images/steve_avatar.webp",
  tokenDeadline: 0,
  type: "Offline",
});
getMicrosoftAccount(configStore.current_account).then((res) => {
  if (!res[0]) {
    return;
  }
  const account = res[0];
  if (account != undefined) {
    getAvatar(account.profile.skins[0].url, 32).then((avatar) => {
      currentAccountProfile.value = {
        name: account.profile.profile_name,
        avatar,
        tokenDeadline: account.expires_on ? account.expires_on : -1,
        type: account.account_type,
      };
    });
  }
});
listen("refresh_accounts_list", async () => {
  const account = (await getMicrosoftAccount(configStore.current_account))[0];
  getAvatar(account.profile.skins[0].url, 32).then((avatar) => {
    currentAccountProfile.value = {
      name: account.profile.profile_name,
      avatar,
      tokenDeadline: account.expires_on ? account.expires_on : -1,
      type: account.account_type,
    };
  });
});
refreshAllMicrosoftAccounts();
listen("add-account", () => {
  dialogStore.accountManager = true;
});
const currentTime = useTimeStore();
</script>
