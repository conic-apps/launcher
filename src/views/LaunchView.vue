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
      <div class="progress-container" :class="{ error }" ref="progressContainerRef">
        <Transition name="fade" mode="out-in">
          <template v-if="error">
            <p class="error-message">{{ errorMessage }}</p>
          </template>
          <template v-else>
            <p>{{ progressDescription }}</p>
          </template>
        </Transition>
        <Transition name="fade" mode="out-in">
          <BaseProgress
            v-if="!error"
            :loading="progressBarLoading"
            :value="progressBarValue"
            :max="progressBarMax"></BaseProgress>
        </Transition>
      </div>
      <div class="other-info">
        <span class="label">{{ t("game.launch.authService") }}</span>
        <span>{{ configStore.current_account?.type }}</span>
        <span class="label">{{ t("game.launch.profileName") }}</span>
        <span v-if="configStore.current_account?.type === 'Microsoft'">{{
          configStore.current_account?.data.profile.profile_name
        }}</span>
        <span v-else-if="configStore.current_account?.type === 'Yggdrasil'">{{
          configStore.current_account?.data.profile.name
        }}</span>
        <span v-else>{{ configStore.current_account?.data.name }}</span>
      </div>
      <div class="back-button">
        <BaseButton :class="{ disabled: backButtonDisabled }" @click="back()">{{
          t("game.launch.cancelLaunch")
        }}</BaseButton>
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
import {
  refreshMicrosoftAccount,
  updateYggdrasilAccount,
  yggdrasilGetSkinUrl,
  yggdrasilRefreshAccount,
  yggdrasilValidateAccount,
} from "@conic/account";
import { formatBytes } from "@conic/download";
import { InstallTask, Job } from "@conic/install";
import { LaunchTask } from "@conic/launch";
import { computed, onMounted, onUnmounted, ref, watch, nextTick } from "vue";
import { window as appWindow } from "@tauri-apps/api";
import { useAccountStore } from "@/store/account";
import { useI18n } from "vue-i18n";
import gsap from "gsap";

const { t } = useI18n();
const instanceStore = useInstanceStore();
const currentInstance = computed(() => {
  return instanceStore.currentInstance;
});
const navigationStore = useNavigationStore();
const accountStore = useAccountStore();
const configStore = useConfigStore();
const dialogStore = useDialogStore();
const musicStore = useMusicStore();

const progressDescription = ref(t("game.launch.progress.preparing"));
const progressBarLoading = ref(true);
const progressBarValue = ref(0);
const progressBarMax = ref(0);
const error = ref(false);
const errorMessage = ref("");

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
    if (!configStore.launch.skip_refresh_account) {
      try {
        await refreshAccountCredentials();
      } catch (error) {
        console.error(error);
        dialogStore.accountRefreshFailed.visible = true;
        return;
      }
    }
    if (!instanceStore.currentInstance.installed) {
      await installGame();
    }
    await launchGame();
  } catch (error) {
    if (isNoSuitableJavaError(error)) {
      dialogStore.noSuitableJavaError.visible = true;
      return;
    }
    handleError(error);
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

async function refreshAccountCredentials() {
  const account = configStore.current_account;
  if (!account || account.type === "Offline") return;
  progressDescription.value = t("game.launch.progress.refreshAccount");
  progressBarLoading.value = true;
  if (account.type === "Microsoft") {
    const refreshed = await refreshMicrosoftAccount(account.data.profile.uuid, false);
    configStore.current_account = { type: "Microsoft", data: refreshed };
  } else if (account.type === "Yggdrasil") {
    if (!(await yggdrasilValidateAccount(account.data))) {
      const refreshed = await yggdrasilRefreshAccount(account.data);
      await updateYggdrasilAccount(refreshed.identifier, refreshed);
      configStore.current_account = { type: "Yggdrasil", data: refreshed };
    }
  }
  await accountStore.reloadFromFile();
}

let cancelInstallHandle: () => Promise<void>;

async function installGame() {
  if (!instanceStore.currentInstance) throw "currentInstance is null";
  const installTask = new InstallTask(configStore.$state, instanceStore.currentInstance, {
    onProgress: (task) => {
      if (!instanceStore.currentInstance) throw "currentInstance is null";
      if (task.job === Job.Prepare) {
        progressDescription.value = t("game.launch.progress.prepareDownload");
        progressBarLoading.value = true;
      }
      if (task.job === Job.InstallGame) {
        if (
          task.progress?.phase === "VerifyExistingFiles" ||
          (task.progress && task.progress.totalBytes === 0)
        ) {
          progressDescription.value = t("game.launch.progress.verifyFiles");
          progressBarLoading.value = true;
        } else if (task.progress?.phase === "DownloadFiles") {
          progressDescription.value = t("game.launch.progress.downloadFiles", {
            current: formatBytes(task.progress.completedBytes),
            total: formatBytes(task.progress.totalBytes),
          });
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
          progressDescription.value = t("game.launch.progress.checkJava");
          progressBarLoading.value = true;
        } else if (task.progress?.phase === "DownloadFiles") {
          progressDescription.value = t("game.launch.progress.downloadJava", {
            current: formatBytes(task.progress.completedBytes),
            total: formatBytes(task.progress.totalBytes),
          });
          progressBarLoading.value = false;
          progressBarValue.value = task.progress.completedBytes;
          progressBarMax.value = task.progress.totalBytes;
        }
      }
      if (task.job === Job.InstallModLoader) {
        const modLoaderName = instanceStore.currentInstance.config.runtime.mod_loader_type;
        const modLoaderProgress = task.progress;
        if (!modLoaderProgress || modLoaderProgress.phase === "prepare") {
          progressDescription.value = t("game.launch.progress.installModLoader", {
            name: modLoaderName,
          });
          progressBarLoading.value = true;
        } else if (
          modLoaderProgress.phase === "downloadInstaller" ||
          modLoaderProgress.phase === "prefetchDependencies"
        ) {
          const detail = modLoaderProgress.detail;
          const stageText =
            modLoaderProgress.phase === "downloadInstaller"
              ? t("game.launch.progress.downloadInstaller", { name: modLoaderName })
              : t("game.launch.progress.downloadDeps");
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
          progressDescription.value = t("game.launch.progress.runInstaller", {
            name: modLoaderName,
            message,
          });
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
        progressDescription.value = t("game.launch.progress.preparingLaunch");
        progressBarLoading.value = true;
      } else if (task.job === "CompleteFiles") {
        if (task.downloadState?.phase === "VerifyExistingFiles") {
          progressDescription.value = t("game.launch.progress.verifyFiles");
          progressBarLoading.value = true;
        } else if (task.downloadState?.phase === "DownloadFiles") {
          progressDescription.value = t("game.launch.progress.downloadFiles", {
            current: task.downloadState.completedBytes,
            total: task.downloadState.totalBytes,
          });
          progressBarLoading.value = false;
          progressBarValue.value = task.downloadState.completedBytes;
          progressBarMax.value = task.downloadState.totalBytes;
        }
      } else if (task.job === "GenerateScriptlet") {
        progressDescription.value = t("game.launch.progress.generateScript");
        progressBarLoading.value = true;
      } else if (task.job === "WaitForLaunch") {
        progressDescription.value = t("game.launch.progress.waitForLaunch");
        progressBarLoading.value = true;
        backButtonDisabled.value = true;
      } else if (task.job === "LogSettingUser") {
        progressDescription.value = t("game.launch.progress.waitForLaunch");
        progressBarLoading.value = true;
        backButtonDisabled.value = true;
      } else if (task.job === "LogLwjglVersion") {
        progressDescription.value = t("game.launch.progress.waitForLaunch");
        progressBarLoading.value = true;
        backButtonDisabled.value = true;
      } else if (task.job === "LogOpenALLoaded") {
        progressDescription.value = t("game.launch.progress.waitForLaunch");
        progressBarLoading.value = true;
        backButtonDisabled.value = true;
      } else if (task.job === "LogTextureLoaded") {
        progressDescription.value = t("game.launch.progress.gameStarted");
        progressBarLoading.value = true;
        backButtonDisabled.value = true;
      }
    },
  });
  cancelLaunchHandle = launchTask.start;
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

function handleError(err: unknown) {
  error.value = true;
  const errorMsg = typeof err === "string" ? err : err instanceof Error ? err.message : String(err);
  errorMessage.value = t("game.launch.progress.errorMessage", { error: errorMsg });
}

const progressContainerRef = ref<HTMLElement | null>(null);

function updateProgressContainerHeight() {
  const container = progressContainerRef.value;
  if (!container) return;
  gsap.to(container, {
    height: "auto",
    duration: 0.3,
    ease: "power2.out",
    overwrite: "auto",
    onComplete: () => {
      if (progressContainerRef.value === container && container.isConnected) {
        container.style.height = `${container.offsetHeight}px`;
      }
    },
  });
}

watch([error, progressDescription], () => {
  nextTick(() => {
    updateProgressContainerHeight();
  });
});

onMounted(async () => {
  launch();
  await new Promise((resolve) => setTimeout(resolve, 100));
  updateProgressContainerHeight();
});

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
      transition: all 0.3s ease;
      overflow: hidden;
      height: auto;

      &.error {
        border: 2px solid var(--ctp-red);
      }

      p {
        font-size: 14px;
        max-width: 100%;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        transition: color 0.3s ease;

        &.error-message {
          color: var(--ctp-red);
          text-align: center;
        }
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
