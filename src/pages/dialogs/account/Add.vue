<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="account-add">
    <p>需要 Tauri 的多 Webview 支持</p>
    <p>打开下面的页面，登录完成后粘贴最后的 URL</p>
    <input v-model="redirect" style="width: 100%" />
  </div>
</template>

<script lang="ts" setup>
import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";

// https://login.live.com/oauth20_authorize.srf?client_id=00000000402b5328&response_type=code&prompt=select_account&scope=service%3A%3Auser.auth.xboxlive.com%3A%3AMBI_SSL&redirect_uri=https%3A%2F%2Flogin.live.com%2Foauth20_desktop.srf
const WEBVIEW_URL =
  "https://login.live.com/oauth20_authorize.srf" +
  "?client_id=00000000402b5328" +
  "&response_type=code" +
  "&prompt=select_account" +
  "&scope=service%3A%3Auser.auth.xboxlive.com%3A%3AMBI_SSL" +
  "&redirect_uri=https%3A%2F%2Flogin.live.com%2Foauth20_desktop.srf";

// 以后与webview重定向事件绑定
const redirect = ref(WEBVIEW_URL);

const PREF = "https://login.live.com/oauth20_desktop.srf?";
watch(redirect, (value) => {
  if (!value.startsWith(PREF + "code=")) {
    return;
  }
  // 这个 code 就是用来登录的，丢给rust吧
  const code = value.substring(PREF.length).split("&")[0].split("=")[1];

  invoke("add_microsoft_account", {
    code,
  });
});
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
