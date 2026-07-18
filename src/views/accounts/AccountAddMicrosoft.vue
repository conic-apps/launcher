<template>
  <div class="add-microsoft-account-container">
    <Transition name="fade">
      <p v-if="useDeviceCodeFlow" class="description">
        使用任意设备打开网页
        <a @click.prevent="openUrl(verificationUri)" :href="verificationUri">{{
          verificationUri
        }}</a
        >, 输入
        <code @click="writeText(userCode)">{{ userCode }}</code> 并登录帐户。此代码已复制到剪贴板。
      </p>
      <p v-else class="description">
        点击「登录」将打开浏览器以登录，你也可以
        <a @click.prevent="writeText(AUTH_CODE_LOGIN_URL)" :href="AUTH_CODE_LOGIN_URL">复制链接</a>
        并在浏览器粘贴以登录。若要在其他设备上完成登录步骤，请通过设备代码登录。
      </p>
    </Transition>
    <p>{{ c }}</p>
  </div>
</template>

<script setup lang="ts">
import BaseButton from "@/components/base/BaseButton.vue";
import {
  requestDeviceCode,
  pollDeviceCode,
  microsoftAccessTokenAuthFlow,
  addMicrosoftAccount,
} from "@conic/account";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { onOpenUrl } from "@tauri-apps/plugin-deep-link";
import { openUrl } from "@tauri-apps/plugin-opener";
import { type UnlistenFn } from "@tauri-apps/api/event";

import { onMounted, onUnmounted, ref, watch } from "vue";

const AUTH_CODE_LOGIN_URL =
  "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize" +
  "?client_id=94a1414e-e9ad-4bda-94f0-3368d979b0cc" +
  "&response_type=code" +
  "&redirect_uri=conic-launcher%3A%2F%2Foauth2%2Fmicrosoft%2Fcallback" +
  "&response_mode=query" +
  "&prompt=select_account" +
  "&scope=XboxLive.signin%20offline_access";
const verificationUri = ref("");
const userCode = ref("");
const useDeviceCodeFlow = ref(false);
let pollTimer: number;

watch(useDeviceCodeFlow, async (value) => {
  if (!value) {
    return;
  }
  const deviceCodeResponse = await requestDeviceCode();
  verificationUri.value = deviceCodeResponse.verification_uri;
  userCode.value = deviceCodeResponse.user_code;
  writeText(deviceCodeResponse.user_code);
  pollTimer = setInterval(async () => {
    const pollResult = await pollDeviceCode(deviceCodeResponse.device_code);
    switch (pollResult.status) {
      case "authorization_pending":
      case "slow_down":
        break;
      case "success":
        clearInterval(pollTimer);
        verificationUri.value = "";
        userCode.value = "";
        console.log("Login microsoft");
        const account = await microsoftAccessTokenAuthFlow(
          pollResult.access_token!,
          pollResult.refresh_token!,
        );
        await addMicrosoftAccount(account);
        console.log("account added");
        break;
      case "authorization_declined":
      case "bad_verification_code":
      case "expired_token":
        clearInterval(pollTimer);
        verificationUri.value = "";
        userCode.value = "";
        break;
    }
  }, deviceCodeResponse.interval * 1000);
});

const c = ref("");

let unListenDeepLink: UnlistenFn = () => {};
onMounted(async () => {
  unListenDeepLink = await onOpenUrl(([url]) => {
    if (!url) return;

    const u = new URL(url);
    const code = u.searchParams.get("code");
    if (code) {
      c.value = code;
    }
  });
});

onUnmounted(() => {
  unListenDeepLink();
  clearInterval(pollTimer);
});
</script>

<style lang="less" scoped>
.add-microsoft-account-container {
  p.description {
    font-size: 14px;
  }
}
</style>
