<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="launch-view">
    <div class="container" v-if="currentInstance">
      <AccountAvatar
        class="avatar"
        :skin="accountSkin"
        :uuid="accountUuid"
        :class="{
          'ms-account': configStore.current_account?.type === 'Microsoft',
          'ygg-account': configStore.current_account?.type === 'Yggdrasil',
          'offline-account': configStore.current_account?.type === 'Offline',
        }"
        :size="48"></AccountAvatar>
      <p class="instance-name">{{ currentInstance.config.name }}</p>
      <div class="instance-info">
        <p>Minecraft {{ currentInstance.config.runtime.minecraft }}</p>
        <p
          v-if="
            currentInstance.config.runtime.mod_loader_type &&
            currentInstance.config.runtime.mod_loader_version
          ">
          {{ currentInstance.config.runtime.mod_loader_type }}
          {{ currentInstance.config.runtime.mod_loader_version }}
        </p>
      </div>
      <div class="progress-container">
        <p>{{ progressDescription }}</p>
        <BaseProgress
          :loading="progressBarLoading"
          :value="progressBarValue"
          :max="progressBarMax"></BaseProgress>
      </div>
      <div class="other-info">
        <span class="label">登录认证服务</span> <span>{{ configStore.current_account?.type }}</span>
        <span class="label">档案名称</span>
        <span v-if="configStore.current_account?.type === 'Microsoft'">{{
          configStore.current_account?.data.profile.profile_name
        }}</span>
        <span v-else-if="configStore.current_account?.type === 'Yggdrasil'">{{
          configStore.current_account?.data.profile.name
        }}</span>
        <span v-else>{{ configStore.current_account?.data.name }}</span>
      </div>
      <div class="back-button">
        <BaseButton :class="{ disabled: backButtonDisabled }" @click="back()">取消启动</BaseButton>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import AccountAvatar from "@/components/AccountAvatar.vue";
import BaseButton from "@/components/BaseButton.vue";
import BaseProgress from "@/components/BaseProgress.vue";
import { useConfigStore } from "@/store/config";
import { useDialogStore } from "@/store/dialog";
import { useInstanceStore } from "@/store/instance";
import { useMusicStore } from "@/store/music";
import { useNavigationStore } from "@/store/navigation";
import { yggdrasilGetSkinUrl } from "@conic/account";
import { formatBytes } from "@conic/download";
import { InstallTask, Job } from "@conic/install";
import { LaunchTask } from "@conic/launch";
import { computed, onMounted, onUnmounted, ref } from "vue";
import { window as appWindow } from "@tauri-apps/api";
import { useAccountStore } from "@/store/account";

const instanceStore = useInstanceStore();
const currentInstance = computed(() => {
  return instanceStore.currentInstance;
});
const navigationStore = useNavigationStore();
const accountStore = useAccountStore();
const configStore = useConfigStore();
const dialogStore = useDialogStore();
const musicStore = useMusicStore();

const progressDescription = ref("正在准备");
const progressBarLoading = ref(true);
const progressBarValue = ref(0);
const progressBarMax = ref(0);

const accountSkin = computed(() => {
  if (configStore.current_account?.type === "Microsoft") {
    return configStore.current_account.data.profile.skins.length > 0
      ? configStore.current_account.data.profile.skins[0].url
      : undefined;
  } else if (configStore.current_account?.type === "Yggdrasil") {
    return yggdrasilGetSkinUrl(configStore.current_account.data.profile);
  } else if (configStore.current_account?.type === "Offline") {
    return configStore.current_account.data.skin;
  } else {
    return undefined;
  }
});

const accountUuid = computed(() => {
  if (configStore.current_account?.type === "Microsoft") {
    return configStore.current_account.data.profile.uuid;
  } else if (configStore.current_account?.type === "Yggdrasil") {
    return configStore.current_account.data.profile.id;
  } else {
    return configStore.current_account ? configStore.current_account.data.uuid : "";
  }
});

async function launch() {
  if (configStore.language !== "zh_cn" && accountStore.microsoft.length === 0) {
    dialogStore.noMicrosoftAccountError.visible = true;
    return;
  }
  if (!configStore.current_account) {
    dialogStore.noAccountError.visible = true;
    return;
  }
  try {
    if (!instanceStore.currentInstance) throw "currentInstance is null";
    if (!instanceStore.currentInstance.installed) {
      await installGame();
    }
    await launchGame();
  } catch (error) {
    if (isNoSuitableJavaError(error)) {
      dialogStore.noSuitableJavaError.visible = true;
      return;
    }
    console.error(error);
  }
}

function isNoSuitableJavaError(error: unknown): boolean {
  if (typeof error === "object" && error !== null && "kind" in error) {
    return (error as { kind?: unknown }).kind === "NoSuitableJavaRuntime";
  }
  const raw = typeof error === "string" ? error : error instanceof Error ? error.message : "";
  try {
    const parsed = JSON.parse(raw);
    return parsed?.kind === "NoSuitableJavaRuntime";
  } catch {
    return false;
  }
}

let cancelInstallHandle: () => Promise<void>;

async function installGame() {
  if (!instanceStore.currentInstance) throw "currentInstance is null";
  const installTask = new InstallTask(configStore.$state, instanceStore.currentInstance, {
    onProgress: (task) => {
      if (!instanceStore.currentInstance) throw "currentInstance is null";
      if (task.job === Job.Prepare) {
        progressDescription.value = "准备下载";
        progressBarLoading.value = true;
      }
      if (task.job === Job.InstallGame) {
        if (
          task.progress?.phase === "VerifyExistingFiles" ||
          (task.progress && task.progress.totalBytes === 0)
        ) {
          progressDescription.value = "校验游戏文件";
          progressBarLoading.value = true;
        } else if (task.progress?.phase === "DownloadFiles") {
          progressDescription.value = `下载游戏文件 ${formatBytes(task.progress.completedBytes)} / ${formatBytes(task.progress.totalBytes)}`;
          progressBarLoading.value = false;
          progressBarValue.value = task.progress.completedBytes;
          progressBarMax.value = task.progress.totalBytes;
        }
      }
      if (task.job === Job.InstallJava) {
        if (
          task.progress?.phase === "VerifyExistingFiles" ||
          (task.progress && task.progress.totalBytes === 0)
        ) {
          progressDescription.value = "检查 Java 运行环境";
          progressBarLoading.value = true;
        } else if (task.progress?.phase === "DownloadFiles") {
          progressDescription.value = `下载 Java ${formatBytes(task.progress.completedBytes)}/${formatBytes(task.progress.totalBytes)}`;
          progressBarLoading.value = false;
          progressBarValue.value = task.progress.completedBytes;
          progressBarMax.value = task.progress.totalBytes;
        }
      }
      if (task.job === Job.InstallModLoader) {
        const modLoaderName = instanceStore.currentInstance.config.runtime.mod_loader_type;
        const modLoaderProgress = task.progress;
        if (!modLoaderProgress || modLoaderProgress.phase === "prepare") {
          progressDescription.value = `安装 ${modLoaderName}`;
          progressBarLoading.value = true;
        } else if (
          modLoaderProgress.phase === "downloadInstaller" ||
          modLoaderProgress.phase === "prefetchDependencies"
        ) {
          const detail = modLoaderProgress.detail;
          const stageText =
            modLoaderProgress.phase === "downloadInstaller"
              ? `下载 ${modLoaderName} 安装器`
              : "下载依赖库";
          if (!detail || detail.phase === "VerifyExistingFiles" || detail.totalBytes === 0) {
            progressDescription.value = stageText;
            progressBarLoading.value = true;
          } else {
            progressDescription.value = `${stageText} ${formatBytes(detail.completedBytes)} / ${formatBytes(detail.totalBytes)}`;
            progressBarLoading.value = false;
            progressBarValue.value = detail.completedBytes;
            progressBarMax.value = detail.totalBytes;
          }
        } else if (modLoaderProgress.phase === "runInstaller") {
          const message = modLoaderProgress.detail?.message ?? "";
          progressDescription.value = `运行 ${modLoaderName} installer：${message}`;
          progressBarLoading.value = true;
        }
      }
    },
  });
  cancelInstallHandle = installTask.cancel;
  await installTask.start();
}

let cancelLaunchHandle: () => Promise<void>;

async function launchGame() {
  if (!instanceStore.currentInstance) throw "currentInstance is null";
  const launchTask = new LaunchTask(configStore.$state, instanceStore.currentInstance, {
    onProgress: (task) => {
      if (task.job === "Prepare") {
        progressDescription.value = "准备启动";
        progressBarLoading.value = true;
      } else if (task.job === "CompleteFiles") {
        if (task.downloadState?.phase === "VerifyExistingFiles") {
          progressDescription.value = "校验游戏文件";
          progressBarLoading.value = true;
        } else if (task.downloadState?.phase === "DownloadFiles") {
          progressDescription.value = `下载游戏文件 ${task.downloadState.completedBytes} / ${task.downloadState.totalBytes}`;
          progressBarLoading.value = false;
          progressBarValue.value = task.downloadState.completedBytes;
          progressBarMax.value = task.downloadState.totalBytes;
        }
      } else if (task.job === "GenerateScriptlet") {
        progressDescription.value = "生成启动脚本";
        progressBarLoading.value = true;
      } else if (task.job === "WaitForLaunch") {
        progressDescription.value = "等待游戏进程启动";
        progressBarLoading.value = true;
        backButtonDisabled.value = true;
      } else if (task.job === "LogSettingUser") {
        progressDescription.value = "等待游戏进程启动";
        progressBarLoading.value = true;
        backButtonDisabled.value = true;
      } else if (task.job === "LogLwjglVersion") {
        progressDescription.value = "等待游戏进程启动";
        progressBarLoading.value = true;
        backButtonDisabled.value = true;
      } else if (task.job === "LogOpenALLoaded") {
        progressDescription.value = "等待游戏进程启动";
        progressBarLoading.value = true;
        backButtonDisabled.value = true;
      } else if (task.job === "LogTextureLoaded") {
        progressDescription.value = "游戏已启动";
        progressBarLoading.value = true;
        backButtonDisabled.value = true;
      }
    },
  });
  cancelLaunchHandle = launchTask.cancel;
  await launchTask.start();
  if (configStore.music.pause_on_launch) {
    musicStore.pause();
  }
  if (
    currentInstance.value?.config.launch_config.quit_app_after_launch ??
    configStore.launch.quit_app_after_launch
  ) {
    appWindow.getCurrentWindow().close();
  }
  navigationStore.back();
}

const backButtonDisabled = ref(false);

onMounted(() => launch());
onUnmounted(async () => {
  try {
    await cancelInstallHandle();
  } catch {}
  try {
    await cancelLaunchHandle();
  } catch {}
  await instanceStore.loadInstances();
});

function back() {
  navigationStore.back();
}
</script>

<style lang="less" scoped>
.launch-view {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;

  .container {
    > .avatar :deep(.avatar-image) {
      background: var(--ctp-surface0);
      border-radius: 1000px;
      padding: 2px;
      margin-bottom: 16px;
    }
    > .avatar.ms-account :deep(.avatar-image) {
      border: 2px solid var(--ctp-green);
    }
    > .avatar.ygg-account :deep(.avatar-image) {
      border: 2px solid var(--ctp-yellow);
    }
    > .avatar.offline-account :deep(.avatar-image) {
      border: 2px solid var(--ctp-red);
    }
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-top: -32px;

    .instance-name {
      font-size: 32px;
    }
    .instance-info {
      display: flex;
      margin: 16px 0;
      p:last-child {
        margin-left: 16px;
      }
    }
    .progress-container {
      width: 340px;
      background: rgba(var(--ctp-overlay2-rgb), 0.16);
      backdrop-filter: blur(2px);
      display: flex;
      align-items: center;
      flex-direction: column;
      border-radius: 8px;
      padding: 16px 32px;
      margin-bottom: 40px;

      p {
        font-size: 14px;
        max-width: 100%;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
      }

      .progress {
        width: 100%;
        margin-top: 10px;
      }
    }
    .other-info {
      display: grid;
      grid-template-columns: max-content auto;
      column-gap: 12px;
      row-gap: 6px;
      font-size: 13px;
    }
    .label {
      text-align: right;
      opacity: 0.8;
    }
  }
  .back-button {
    width: 240px;
    margin-top: 36px;
    button {
      border: 1px solid var(--ctp-red);
      color: var(--ctp-red);
      background: var(--ctp-surface0);
      transition: all 0.2s ease;
    }
    button:hover {
      background: var(--ctp-red);
      color: var(--ctp-text-inverse);
    }
    button:active {
      transform: scale(0.98);
    }
    button.disabled {
      pointer-events: none;
      opacity: 0.5;
    }
  }
}
</style>
