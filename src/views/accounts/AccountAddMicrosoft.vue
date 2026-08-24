<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="add-microsoft-account-container">
    <Transition :name="transitionName" mode="out-in">
      <div v-if="errorOccured" class="error">登录失败：{{ errorText }}</div>
      <div v-else-if="processing" class="processing">
        <div class="loading">
          <BaseLoading :size="40" :strokeWidth="5" :gap="12"></BaseLoading>
        </div>
        <p class="description">{{ progressDescription }}</p>
      </div>
      <div v-else-if="useDeviceCodeFlow" class="device-code">
        <p class="description">
          使用任意设备打开网页
          <a @click.prevent="openUrl(verificationUri)" :href="verificationUri">{{
            verificationUri
          }}</a
          >, 输入下方设备代码并登录帐户。此代码将于 {{ formatCountdown }} 后失效。
          <a @click.prevent="useDeviceCodeFlow = false">点击此处</a>以返回并使用系统浏览器登录。
        </p>
        <div class="link">
          <div class="link-box code-box" :class="{ 'code-copied': copiedCode }" @click="copyCode">
            <p class="link-text">{{ userCode }}</p>
            <div class="checkmark-wrapper">
              <div class="tooltip">已复制！</div>
              <AppIcon name="checkmark-outline" :size="20" class="checkmark-icon" />
            </div>
          </div>
        </div>
        <div class="buttons">
          <BaseButton @click="closeDialog">{{ "取消" }}</BaseButton>
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
          并在浏览器粘贴以登录。若要在其他设备上完成登录步骤，请
          <a @click.prevent="useDeviceCodeFlow = true">通过设备代码</a> 登录。
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
          <BaseButton @click="closeDialog">{{ "取消" }}</BaseButton>
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
import BaseButton from "@/components/BaseButton.vue";
import BaseLoading from "@/components/BaseLoading.vue";
import { MicrosoftLoginTask } from "@conic/account";
import type { LoginProgress } from "@conic/account";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { onOpenUrl } from "@tauri-apps/plugin-deep-link";
import { openUrl } from "@tauri-apps/plugin-opener";
import { type UnlistenFn } from "@tauri-apps/api/event";

import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useAccountStore } from "@/store/account";
import { useConfigStore } from "@/store/config";
import { useDialogStore } from "@/store/dialog";

const accountStore = useAccountStore();
const configStore = useConfigStore();
const dialogStore = useDialogStore();
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
let countdownTimer: number;

const emit = defineEmits(["switch-component-manage"]);

const formatCountdown = computed(() => {
  const total = expiresIn.value;
  if (total <= 0) return "0 秒";
  const m = Math.floor(total / 60);
  const s = total % 60;
  return m > 0 ? `${m} 分 ${String(s).padStart(2, "0")} 秒` : `${s} 秒`;
});

const transitionName = computed(() => {
  if (processing.value || errorOccured.value) {
    return "slide-left";
  }
  return useDeviceCodeFlow.value ? "slide-left" : "slide-right";
});

const processing = ref(false);
const progressDescription = ref("正在准备登录");
const errorOccured = ref(false);
const errorText = ref("");

interface RunningLogin {
  task: MicrosoftLoginTask;
  deviceFlow: boolean;
}
let runningLogin: RunningLogin | null = null;

function describeProgress(progress: LoginProgress): string | undefined {
  switch (progress.job) {
    case "Prepare":
      return "正在准备登录";
    case "RequestDeviceCode":
      return "正在请求设备代码";
    case "RedeemAccessToken":
      return "正在获取访问令牌";
    case "XboxAuthenticate":
      return "正在登录 Xbox";
    case "XstsAuthenticate":
      return "正在验证 XSTS";
    case "MinecraftAuthenticate":
      return "正在获取 Minecraft 访问令牌";
    case "GetProfile":
      return "正在获取游戏档案";
    case "SaveAccount":
      return "正在保存帐户信息";
    default:
      return undefined;
  }
}

function startCountdown() {
  clearInterval(countdownTimer);
  countdownTimer = window.setInterval(() => {
    if (expiresIn.value > 0) {
      expiresIn.value--;
    } else {
      clearInterval(countdownTimer);
    }
  }, 1000);
}

function stopCountdown() {
  clearInterval(countdownTimer);
}

function handleProgress(progress: LoginProgress) {
  if (progress.job === "WaitingForAuthorization") {
    const detail = progress.progress;
    if (detail.user_code !== userCode.value || detail.verification_uri !== verificationUri.value) {
      verificationUri.value = detail.verification_uri;
      userCode.value = detail.user_code;
      expiresIn.value = detail.expires_in;
      startCountdown();
      writeText(detail.user_code);
    }
    useDeviceCodeFlow.value = true;
    processing.value = false;
    return;
  }
  const description = describeProgress(progress);
  if (description) {
    processing.value = true;
    progressDescription.value = description;
  }
}

function handleError(error: unknown, deviceFlow: boolean) {
  const kind = (error as { kind?: string } | null)?.kind;
  if (kind === "Aborted") {
    // 用户主动取消了登录，静默返回之前的界面。
  } else if (kind === "DeviceCodeExpired" && deviceFlow && useDeviceCodeFlow.value) {
    // 设备代码过期后自动重新申请新的设备代码。
    startLogin(true);
    return;
  } else {
    errorText.value =
      typeof (error as { message?: unknown })?.message === "string"
        ? (error as { message: string }).message
        : String(error);
    errorOccured.value = true;
  }
  stopCountdown();
  processing.value = false;
}

async function startLogin(deviceFlow: boolean, code?: string) {
  if (runningLogin) return;
  errorOccured.value = false;
  processing.value = true;
  progressDescription.value = "正在准备登录";
  const task = new MicrosoftLoginTask(code, { onProgress: handleProgress });
  runningLogin = { task, deviceFlow };
  try {
    const account = await task.start();
    if (!configStore.current_account) {
      configStore.current_account = { type: "Microsoft", data: account };
    }
    emit("switch-component-manage");
    await accountStore.reloadFromFile();
    processing.value = false;
  } catch (error) {
    console.error(error);
    if (runningLogin?.task === task) runningLogin = null;
    handleError(error, deviceFlow);
  } finally {
    if (runningLogin?.task === task) runningLogin = null;
  }
}

async function cancelLogin() {
  const current = runningLogin;
  runningLogin = null;
  if (current) {
    try {
      await current.task.cancel();
    } catch {}
  }
  stopCountdown();
  processing.value = false;
}

function closeDialog() {
  cancelLogin();
  dialogStore.accountAdd.visible = false;
}

watch(useDeviceCodeFlow, (value) => {
  if (value) {
    if (!verificationUri.value && !userCode.value && !runningLogin) {
      startLogin(true);
    }
  } else if (runningLogin?.deviceFlow) {
    cancelLogin();
  }
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

let unListenDeepLink: UnlistenFn = () => {};
onMounted(async () => {
  unListenDeepLink = await onOpenUrl(([url]) => {
    if (!url || runningLogin) return;
    const u = new URL(url);
    const code = u.searchParams.get("code");
    if (code) {
      startLogin(false, code);
    }
  });
});

onUnmounted(() => {
  unListenDeepLink();
  cancelLogin();
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
    gap: 8px;
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
        width: 20ch;
        height: 48px;
        justify-content: center;
        background: none;

        &:hover {
          background: var(--ctp-surface0);
        }
        &:active {
          background: var(--ctp-surface1);
        }

        p.link-text {
          flex: none;
          text-align: center;
          transition: transform 0.3s ease;
          font-size: 20px;
          letter-spacing: 4px;
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
    display: flex;
    align-items: center;
    justify-content: center;
    flex-direction: column;
    gap: 8px;
    .loading {
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 8px;
    }
    p.description {
      font-size: 14px;
      margin-bottom: 8px;
    }
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
