<template>
  <div class="add-microsoft-account-container">
    <p class="description">
      You can scan the QR Code and complete the login process on a separate device, or you can open
      the link and login on this machine.
    </p>
    <div class="qrcode-container">
      <canvas ref="qrcode" @click="openUrl(verificationUri)"></canvas>
    </div>
    <BaseButton @click="openUrl(verificationUri)"></BaseButton>
  </div>
</template>

<script setup lang="ts">
import BaseButton from "@/components/base/BaseButton.vue";
import { requestDeviceCode, pollDeviceCode, addMicrosoftAccount } from "@conic/account";
import { openUrl } from "@tauri-apps/plugin-opener";
import qrcode from "qrcode";
import { onMounted, ref, useTemplateRef } from "vue";

const qrcodeCanvas = useTemplateRef("qrcode");
const verificationUri = ref("");
let pollTimer: NodeJS.Timeout;
// onMounted(async () => {
//   const deviceCodeResponse = await requestDeviceCode();
//   // https://www.microsoft.com/link?otc=4FSU9FVK
//   loginURL.value = `https://www.microsoft.com/link?otc=${deviceCodeResponse.user_code}`;
//   await qrcode.toCanvas(qrcodeCanvas.value, loginURL.value, { version: 3 });
//   timer = setInterval(async () => {
//     const pollResult = await pollDeviceCode(deviceCodeResponse.device_code);
//     console.log(pollResult);
//   }, deviceCodeResponse.interval * 1000);
// });

onMounted(async () => {
  const deviceCodeResponse = await requestDeviceCode();
  verificationUri.value = deviceCodeResponse.verification_uri;
  pollTimer = setInterval(async () => {
    const pollResult = await pollDeviceCode(deviceCodeResponse.device_code);
    switch (pollResult.status) {
      case "authorization_pending":
      case "slow_down":
        break;
      case "success":
        clearInterval(pollTimer);
        await addMicrosoftAccount();
        break;
      case "authorization_declined":
      case "bad_verification_code":
      case "expired_token":
        clearInterval(pollTimer);
        break;
    }
  }, deviceCodeResponse.interval * 1000);
});
</script>

<style lang="less" scoped>
.add-microsoft-account-container {
  p.description {
    font-size: 14px;
  }
}
</style>
