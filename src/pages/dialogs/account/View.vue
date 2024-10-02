<template>
  <div class="account-view">
    <div class="row1">
      <div>
        <list-item v-for="(account, index) in accounts" :key="index" :title="account.profile.profile_name"
          :logo="account.profile.avatar" :click-able="false" :buttons="['arrows-rotate', 'trash']"
          @click-arrows-rotate="refreshLogin(account.profile.uuid)" @click-trash="deleteAccount(account.profile.uuid)">
          <template #subtitle>
            <tag v-if="!tokenValid(account)" text="需要刷新" :color="['249', '226', '175']" text-color="#f9e2af"
              :background="false" :border="true" font-size="10" :round="true"></tag>
          </template>
          <i class="badge-check" style="color: #74c7ec; font-style: normal; font-family: fa-pro"></i>
          微软（验证服务）
        </list-item>
      </div>
      <div style="margin-top: 8px">
        <list-item class="list-item-user-plus" title="添加帐号" logo="user-plus" @click="$emit('add')"
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
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ref } from "vue";

const emit = defineEmits(["add"]);

const accounts = ref<Account[]>([]);
export type Account = {
  refresh_token?: string;
  access_token?: string;
  token_deadline: number;
  profile: {
    avatar: string;
    profile_name: string;
    uuid: string;
    skins: {
      id: string;
      state: string;
      textureKey: string;
      url: string;
      variant: string;
    }[];
    capes: {
      alias: string;
      id: string;
      state: string;
      url: string;
    }[];
  };
  account_type: "Microsoft" | "Offline";
};

async function getAccounts() {
  let res: Account[] = await invoke("get_accounts");
  for (let i = 0; i <= res.length - 1; i++) {
    res[i].profile.avatar = await getAvatar(res[i].profile.skins[0].url, 32);
  }
  console.log(res);
  accounts.value = res;
}

getAccounts().then(() => { });
function tokenValid(account: Account) {
  const now = Math.round(new Date().getTime() / 1000);
  return account.token_deadline > now;
}

async function getAvatar(src: string, size: number) {
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d");
  if (ctx == null) {
    return "";
  }
  const img = new Image();
  img.src = src;
  await new Promise<void>((reslove) => {
    img.onload = function () {
      const scale = img.width / 64;
      const faceOffset = Math.round(size / 18.0);
      ctx.imageSmoothingEnabled = false;
      /* Inspired by HMCL */
      ctx.drawImage(
        img,
        8 * scale,
        8 * scale,
        16 * scale - 8 * scale,
        16 * scale - 8 * scale,
        faceOffset,
        faceOffset,
        size - faceOffset - faceOffset,
        size - faceOffset - faceOffset,
      );
      ctx.drawImage(
        img,
        40 * scale,
        8 * scale,
        48 * scale - 40 * scale,
        16 * scale - 8 * scale,
        0,
        0,
        size,
        size,
      );
      reslove();
    };
  });
  return canvas.toDataURL("image/png");
}
listen("refresh_accounts_list", () => {
  getAccounts();
});

function deleteAccount(uuid: string) {
  invoke("delete_account", { uuid });
}

function refreshLogin(uuid: string) {
  invoke("refresh_microsoft_account_by_uuid", {
    uuid,
  });
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

  >div {
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
