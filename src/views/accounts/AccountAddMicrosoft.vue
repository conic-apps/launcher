<template>
  <div class="add-microsoft-account-container">
    <Transition :name="transitionName" mode="out-in">
      <!-- <div v-if="errorOccured" class="processing"> -->
      <!--   <div class="loading"></div> -->
      <!--   <p class="description"></p> -->
      <!-- </div> -->
      <div v-if="processing" class="processing">
        <div class="loading">正在加载，但是没有动画</div>
        <p class="description">这是描述</p>
      </div>
      <div v-else-if="useDeviceCodeFlow" class="device-code">
        <p class="description">
          使用任意设备打开网页
          <a @click.prevent="openUrl(verificationUri)" :href="verificationUri">{{
            verificationUri
          }}</a
          >, 输入下方设备代码并登录帐户。此代码将于 {{ formatCountdown }} 后失效。
        </p>
        <div class="link">
          <div class="link-box code-box" :class="{ 'code-copied': copiedCode }" @click="copyCode">
            <p class="link-text">{{ userCode }}</p>
            <div class="checkmark-wrapper">
              <div class="tooltip">已复制！</div>
              <AppIcon name="checkmark-outline" :size="16" class="checkmark-icon" />
            </div>
          </div>
        </div>
        <div class="buttons">
          <BaseButton style="margin-right: 120px" @click="useDeviceCodeFlow = false">{{
            "使用系统浏览器登录"
          }}</BaseButton>
          <BaseButton
            :disabled="expiresIn > totalExpiresIn - 60 || expiresIn < 60 || isRefreshingDeviceCode"
            @click="refreshDeviceCode"
            >{{ isRefreshingDeviceCode ? "正在请求新代码" : "刷新设备代码"
            }}{{ refreshCountdown ? `（${refreshCountdown}）` : "" }}
            <!-- 这里要有个小加载动画，请求代码时显示 --></BaseButton
          >
        </div>
      </div>
      <div v-else class="auth-code">
        <p class="description">
          点击「登录」将打开浏览器以登录，你也可以
          <span class="copy-link-wrapper">
            <a @click.prevent="copyLinkText" :href="AUTH_CODE_LOGIN_URL">复制链接</a>
            <Transition name="tooltip">
              <div v-if="copiedLink" class="link-tooltip">已复制！</div>
            </Transition>
          </span>
          并在浏览器粘贴以登录。若要在其他设备上完成登录步骤，请通过设备代码登录。
        </p>
        <div class="link">
          <p class="description">登录链接：</p>
          <div class="link-box" @click="copyLink">
            <p class="link-text">{{ AUTH_CODE_LOGIN_URL }}</p>
            <Transition name="check">
              <div v-if="copied" class="checkmark-wrapper">
                <div class="tooltip">已复制！</div>
                <AppIcon name="checkmark-outline" :size="16" class="checkmark-icon"></AppIcon>
              </div>
            </Transition>
          </div>
        </div>
        <div class="buttons">
          <BaseButton style="margin-right: 120px" @click="useDeviceCodeFlow = true">{{
            "使用设备代码登录"
          }}</BaseButton>
          <BaseButton
            style="background: var(--ctp-blue); color: var(--ctp-text-inverse)"
            @click="openUrl(AUTH_CODE_LOGIN_URL)"
            >登录</BaseButton
          >
        </div>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import BaseButton from "@/components/base/BaseButton.vue";
import {
  requestDeviceCode,
  pollDeviceCode,
  microsoftAccessTokenAuthFlow,
  addMicrosoftAccount,
  redeemAccessToken,
} from "@conic/account";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { onOpenUrl } from "@tauri-apps/plugin-deep-link";
import { openUrl } from "@tauri-apps/plugin-opener";
import { type UnlistenFn } from "@tauri-apps/api/event";

import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useAccountStore } from "@/store/account";
import { useConfigStore } from "@/store/config";

const accountStore = useAccountStore();
const configStore = useConfigStore();
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
const copied = ref(false);
const copiedLink = ref(false);
const copiedCode = ref(false);
const expiresIn = ref(0);
const totalExpiresIn = ref(0);
let countdownTimer: number;

const emit = defineEmits(["switch-component-manage"]);

const formatCountdown = computed(() => {
  const total = expiresIn.value;
  if (total <= 0) return "0 秒";
  const m = Math.floor(total / 60);
  const s = total % 60;
  return m > 0 ? `${m} 分 ${String(s).padStart(2, "0")} 秒` : `${s} 秒`;
});

const refreshCountdown = computed(() => {
  const remaining = expiresIn.value - (totalExpiresIn.value - 60);
  if (remaining <= 0) return "";
  return `${remaining}`;
});

async function copyLinkText() {
  await writeText(AUTH_CODE_LOGIN_URL);
  copiedLink.value = true;
  setTimeout(() => {
    copiedLink.value = false;
  }, 1500);
}

async function copyCode() {
  await writeText(userCode.value);
  copiedCode.value = true;
  setTimeout(() => {
    copiedCode.value = false;
  }, 1500);
}

async function copyLink() {
  await writeText(AUTH_CODE_LOGIN_URL);
  copied.value = true;
  setTimeout(() => {
    copied.value = false;
  }, 1500);
}
let pollTimer: number;

watch(useDeviceCodeFlow, async (value) => {
  if (!value || (verificationUri.value && userCode.value)) {
    return;
  }
  processing.value = true;
  const deviceCodeResponse = await requestDeviceCode();
  verificationUri.value = deviceCodeResponse.verification_uri;
  userCode.value = deviceCodeResponse.user_code;
  expiresIn.value = deviceCodeResponse.expires_in;
  totalExpiresIn.value = deviceCodeResponse.expires_in;
  writeText(deviceCodeResponse.user_code);
  processing.value = false;
  clearInterval(countdownTimer);
  countdownTimer = window.setInterval(() => {
    if (expiresIn.value > 0) {
      expiresIn.value--;
    } else {
      clearInterval(countdownTimer);
    }
  }, 1000);
  pollTimer = setInterval(async () => {
    const pollResult = await pollDeviceCode(deviceCodeResponse.device_code);
    switch (pollResult.status) {
      case "authorization_pending":
      case "slow_down":
        break;
      case "success":
        clearInterval(pollTimer);
        clearInterval(countdownTimer);
        verificationUri.value = "";
        userCode.value = "";
        processing.value = true;
        const account = await microsoftAccessTokenAuthFlow(
          pollResult.access_token!,
          pollResult.refresh_token!,
        );
        await addMicrosoftAccount(account);
        if (!configStore.current_account) {
          configStore.current_account = { type: "Microsoft", data: account };
        }
        emit("switch-component-manage");
        await accountStore.reloadFromFile();
        processing.value = false;
        break;
      case "authorization_declined":
      case "bad_verification_code":
        clearInterval(pollTimer);
        clearInterval(countdownTimer);
        verificationUri.value = "";
        userCode.value = "";
        break;
      case "expired_token":
        refreshDeviceCode();
        break;
    }
  }, deviceCodeResponse.interval * 1000);
});

const isRefreshingDeviceCode = ref(false);

async function refreshDeviceCode() {
  isRefreshingDeviceCode.value = true;
  clearInterval(pollTimer);
  clearInterval(countdownTimer);
  const deviceCodeResponse = await requestDeviceCode();
  verificationUri.value = deviceCodeResponse.verification_uri;
  userCode.value = deviceCodeResponse.user_code;
  expiresIn.value = deviceCodeResponse.expires_in;
  totalExpiresIn.value = deviceCodeResponse.expires_in;
  writeText(deviceCodeResponse.user_code);
  countdownTimer = window.setInterval(() => {
    if (expiresIn.value > 0) {
      expiresIn.value--;
    } else {
      clearInterval(countdownTimer);
    }
  }, 1000);
  pollTimer = setInterval(async () => {
    const pollResult = await pollDeviceCode(deviceCodeResponse.device_code);
    switch (pollResult.status) {
      case "authorization_pending":
      case "slow_down":
        break;
      case "success":
        clearInterval(pollTimer);
        clearInterval(countdownTimer);
        verificationUri.value = "";
        userCode.value = "";
        processing.value = true;
        const account = await microsoftAccessTokenAuthFlow(
          pollResult.access_token!,
          pollResult.refresh_token!,
        );
        await addMicrosoftAccount(account);
        if (!configStore.current_account) {
          configStore.current_account = { type: "Microsoft", data: account };
        }
        emit("switch-component-manage");
        await accountStore.reloadFromFile();
        processing.value = false;
        break;
      case "authorization_declined":
      case "bad_verification_code":
        clearInterval(pollTimer);
        clearInterval(countdownTimer);
        verificationUri.value = "";
        userCode.value = "";
        break;
      case "expired_token":
        refreshDeviceCode();
        break;
    }
  }, deviceCodeResponse.interval * 1000);
  isRefreshingDeviceCode.value = false;
}

let unListenDeepLink: UnlistenFn = () => {};
onMounted(async () => {
  unListenDeepLink = await onOpenUrl(([url]) => {
    if (!url) return;
    const u = new URL(url);
    const code = u.searchParams.get("code");
    if (code) {
      processing.value = true;
      authCodeFlow(code)
        .then(() => {
          emit("switch-component-manage");
          processing.value = false;
          accountStore.reloadFromFile();
        })
        .catch((e) => {
          console.error(e);
        });
    }
  });
});

async function authCodeFlow(code: string) {
  const { access_token: accessToken, refresh_token: refreshToken } = await redeemAccessToken(code);
  const account = await microsoftAccessTokenAuthFlow(accessToken, refreshToken);
  await addMicrosoftAccount(account);
}

onUnmounted(() => {
  unListenDeepLink();
  clearInterval(pollTimer);
  clearInterval(countdownTimer);
});

const processing = ref(false);
const transitionName = computed(() => {
  if (processing.value) {
    return "slide-left";
  }
  return useDeviceCodeFlow.value ? "slide-left" : "slide-right";
});
</script>

<style lang="less" scoped>
.add-microsoft-account-container {
  width: 100%;

  p.description {
    font-size: 14px;
    line-height: 1.3;

    .copy-link-wrapper {
      position: relative;

      .link-tooltip {
        position: absolute;
        bottom: calc(100% + 8px);
        left: 50%;
        transform: translateX(-50%);
        padding: 4px 10px;
        background: var(--card-background);
        color: rgb(var(--default-text-color));
        border: 1px solid rgba(var(--ctp-overlay2-rgb), 0.5);
        border-radius: var(--tag-border-radius);
        font-size: 12px;
        white-space: nowrap;
        pointer-events: none;
      }
    }
  }

  div.buttons {
    display: flex;
    margin-top: 16px;
  }

  .device-code div.link {
    justify-content: center;
  }

  div.link {
    width: 100%;
    margin-top: 8px;
    display: flex;
    align-items: center;

    p.description {
      flex-shrink: 0;
    }

    .link-box {
      position: relative;
      flex: 1;
      min-width: 0;
      display: flex;
      align-items: center;
      border-radius: var(--controllers-border-radius);
      background: var(--controllers-background);
      border: var(--controllers-border);
      height: 30px;
      padding: 0 8px;
      transition: background 0.1s ease;

      &.code-box {
        flex: none;
        width: 12ch;
        justify-content: center;

        p.link-text {
          flex: none;
          text-align: center;
          transition: transform 0.3s ease;
        }

        .checkmark-wrapper {
          position: absolute;
          right: 8px;
          opacity: 0;
          transform: scale(0);
          transition:
            opacity 0.3s ease,
            transform 0.3s ease;
        }

        &.code-copied p.link-text {
          transform: translateX(-8px);
        }

        &.code-copied .checkmark-wrapper {
          opacity: 1;
          transform: scale(1);
        }
      }

      &:hover {
        background: var(--controllers-background-hover);
      }

      p.link-text {
        flex: 1;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-size: 12.5px;
      }

      .checkmark-wrapper {
        flex-shrink: 0;
        margin-left: 6px;
        position: relative;

        .tooltip {
          position: absolute;
          bottom: calc(100% + 8px);
          left: 50%;
          transform: translateX(-50%);
          padding: 4px 10px;
          background: var(--card-background);
          color: rgb(var(--default-text-color));
          border: 1px solid rgba(var(--ctp-overlay2-rgb), 0.5);
          border-radius: var(--tag-border-radius);
          font-size: 12px;
          white-space: nowrap;
          pointer-events: none;
        }

        .checkmark-icon {
          color: var(--ctp-green);
        }
      }
    }
  }
  .processing {
    width: 100%;
    height: 120px;
  }
}

.check-enter-active {
  animation: checkScale 0.3s ease;
}
.check-leave-active {
  animation: checkScale 0.3s ease reverse;
}

.tooltip-enter-active {
  animation: tooltipScale 0.3s ease;
}
.tooltip-leave-active {
  animation: tooltipScale 0.3s ease reverse;
}

@keyframes checkScale {
  0% {
    transform: scale(0);
    opacity: 0;
  }
  60% {
    transform: scale(1.3);
  }
  100% {
    transform: scale(1);
    opacity: 1;
  }
}

@keyframes tooltipScale {
  0% {
    transform: translateX(-50%) scale(0);
    opacity: 0;
  }
  60% {
    transform: translateX(-50%) scale(1.3);
  }
  100% {
    transform: translateX(-50%) scale(1);
    opacity: 1;
  }
}
</style>
