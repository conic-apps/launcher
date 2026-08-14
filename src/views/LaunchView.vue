<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="launch-view" data-tauri-drag-region>
    <div class="container">
      <AccountAvatar :skin="accountSkin" :uuid="accountUuid" :size="48"></AccountAvatar>
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
import { useNavigationStore } from "@/store/navigation";
import { yggdrasilGetSkinUrl } from "@conic/account";
import { formatBytes } from "@conic/download";
import { InstallTask, Job } from "@conic/install";
import { LaunchTask } from "@conic/launch";
import { computed, onMounted, onUnmounted, ref } from "vue";
import { window as appWindow } from "@tauri-apps/api";

const instanceStore = useInstanceStore();
const currentInstance = computed(() => {
  return instanceStore.currentInstance;
});
const navigationStore = useNavigationStore();
const configStore = useConfigStore();
const dialogStore = useDialogStore();

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
  if (!configStore.current_account) {
    dialogStore.noAccountError.visible = true;
    return;
  }
  try {
    if (!instanceStore.currentInstance.installed) {
      await installGame();
    }
    await launchGame();
  } catch (error) {
    console.error(error);
  }
}

let cancelInstallHandle: () => Promise<void>;

async function installGame() {
  const installTask = new InstallTask(configStore.$state, instanceStore.currentInstance, {
    onProgress: (task) => {
      if (task.job === Job.Prepare) {
        progressDescription.value = "准备下载";
        progressBarLoading.value = true;
      }
      if (task.job === Job.InstallGame) {
        if (
          task.downloadState?.phase === "VerifyExistingFiles" ||
          (task.downloadState && task.downloadState.totalBytes === 0)
        ) {
          progressDescription.value = "校验游戏文件";
          progressBarLoading.value = true;
        } else if (task.downloadState?.phase === "DownloadFiles") {
          progressDescription.value = `下载游戏文件 ${formatBytes(task.downloadState.completedBytes)} / ${formatBytes(task.downloadState.totalBytes)}`;
          progressBarLoading.value = false;
          progressBarValue.value = task.downloadState.completedBytes;
          progressBarMax.value = task.downloadState.totalBytes;
        }
      }
      if (task.job === Job.InstallJava) {
        if (
          task.downloadState?.phase === "VerifyExistingFiles" ||
          (task.downloadState && task.downloadState.totalBytes === 0)
        ) {
          progressDescription.value = "检查 Java 运行环境";
          progressBarLoading.value = true;
        } else if (task.downloadState?.phase === "DownloadFiles") {
          progressDescription.value = `下载 Java ${formatBytes(task.downloadState.completedBytes)}/${formatBytes(task.downloadState.totalBytes)}`;
          progressBarLoading.value = false;
          progressBarValue.value = task.downloadState.completedBytes;
          progressBarMax.value = task.downloadState.totalBytes;
        }
      }
      if (task.job === Job.InstallModLoader) {
        progressDescription.value = `安装 ${instanceStore.currentInstance.config.runtime.mod_loader_type}`;
        progressBarLoading.value = true;
      }
    },
  });
  cancelInstallHandle = installTask.cancel;
  await installTask.start();
}

let cancelLaunchHandle: () => Promise<void>;

async function launchGame() {
  const launchTask = new LaunchTask(configStore.$state, instanceStore.currentInstance, {
    onProgress: (task) => {
      if (task.job === "Prepare") {
        progressDescription.value = "准备启动";
        progressBarLoading.value = true;
      } else if (task.job === "RefreshAccount") {
        progressDescription.value = "更新登录凭据";
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
  if (configStore.launch.quit_app_after_launch) {
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
    > img {
      background: var(--ctp-surface0);
      border-radius: 1000px;
      padding: 2px;
      margin-bottom: 16px;
      border: 2px solid var(--ctp-green);
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
      width: 280px;
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
