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
    </div>
    <Transition name="custom-slide-bottom">
      <div class="back-button" :class="{ disabled: backButtonDisabled }" v-if="showBackButton">
        <button @click="back()">
          <div>
            <AppIcon name="arrow-back-outline"></AppIcon>
          </div>
          <span>取消启动</span>
        </button>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import AccountAvatar from "@/components/AccountAvatar.vue";
import BaseProgress from "@/components/base/BaseProgress.vue";
import { useAccountStore } from "@/store/account";
import { useConfigStore } from "@/store/config";
import { useInstanceStore } from "@/store/instance";
import { useNavigationStore } from "@/store/navigation";
import { yggdrasilGetSkinUrl } from "@conic/account";
import { InstallTask, Job } from "@conic/install";
import { LaunchTask } from "@conic/launch";
import { computed, onMounted, onUnmounted, ref } from "vue";

const instanceStore = useInstanceStore();
const currentInstance = computed(() => {
  return instanceStore.currentInstance;
});
const navigationStore = useNavigationStore();
const configStore = useConfigStore();

const progressDescription = ref("正在准备");
const processing = ref(false);
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
  if (!instanceStore.currentInstance.installed) {
    await installGame();
  }
  await launchGame();
}

let cancelInstallHandle: () => Promise<void>;

async function installGame() {
  const installTask = new InstallTask(configStore, instanceStore.currentInstance, {
    onStart: () => {
      progressDescription.value = "准备下载";
      processing.value = true;
      progressBarLoading.value = true;
    },
    onProgress: (task) => {
      if (task.job === Job.Prepare) {
        progressDescription.value = "准备下载";
        processing.value = true;
        progressBarLoading.value = true;
      }
      if (task.job === Job.InstallGame) {
        if (task.downloadState?.phase === "VerifyExistingFiles") {
          progressDescription.value = "校验游戏文件";
          processing.value = true;
          progressBarLoading.value = true;
        } else if (task.downloadState?.phase === "DownloadFiles") {
          progressDescription.value = "下载游戏文件";
          processing.value = true;
          progressBarLoading.value = false;
          progressBarValue.value = task.downloadState.completed;
          progressBarMax.value = task.downloadState.total;
        }
      }
      if (task.job === Job.InstallJava) {
        if (task.downloadState?.phase === "VerifyExistingFiles") {
          progressDescription.value = "安装 Java";
          processing.value = true;
          progressBarLoading.value = true;
        } else if (task.downloadState?.phase === "DownloadFiles") {
          progressDescription.value = "安装 Java";
          processing.value = true;
          progressBarLoading.value = false;
          progressBarValue.value = task.downloadState.completed;
          progressBarMax.value = task.downloadState.total;
        }
      }
      if (task.job === Job.InstallModLoader) {
        progressDescription.value = `安装 ${instanceStore.currentInstance.config.runtime.mod_loader_type}`;
        processing.value = true;
        progressBarLoading.value = true;
      }
    },
    onFailed: (error) => {
      processing.value = false;
      console.error(error);
    },
    onSucceed: () => {
      processing.value = false;
    },
    onCancelled: () => {
      processing.value = false;
    },
  });
  cancelInstallHandle = installTask.cancel;
  await installTask.start();
  try {
    await installTask.start();
  } catch (error) {
    console.error(error);
  }
}

let cancelLaunchHandle: () => Promise<void>;

async function launchGame() {
  const launchTask = new LaunchTask(configStore, instanceStore.currentInstance, {
    onStart: () => {
      progressDescription.value = "准备启动";
      processing.value = true;
      progressBarLoading.value = true;
    },
    onProgress: (task) => {
      if (task.job === "Prepare") {
        progressDescription.value = "准备启动";
        processing.value = true;
        progressBarLoading.value = true;
      } else if (task.job === "RefreshAccount") {
        progressDescription.value = "更新登录凭据";
        processing.value = true;
        progressBarLoading.value = true;
      } else if (task.job === "CompleteFiles") {
        if (task.downloadState?.phase === "VerifyExistingFiles") {
          progressDescription.value = "校验游戏文件";
          processing.value = true;
          progressBarLoading.value = true;
        } else if (task.downloadState?.phase === "DownloadFiles") {
          progressDescription.value = "下载游戏文件";
          processing.value = true;
          progressBarLoading.value = false;
          progressBarValue.value = task.downloadState.completed;
          progressBarMax.value = task.downloadState.total;
        }
      } else if (task.job === "GenerateScriptlet") {
        progressDescription.value = "生成启动脚本";
        processing.value = true;
        progressBarLoading.value = true;
      } else if (task.job === "WaitForLaunch") {
        progressDescription.value = "等待游戏进程启动";
        processing.value = true;
        progressBarLoading.value = true;
        backButtonDisabled.value = true;
      } else if (task.job === "LogSettingUser") {
        progressDescription.value = "等待游戏进程启动";
        processing.value = true;
        progressBarLoading.value = true;
        backButtonDisabled.value = true;
      } else if (task.job === "LogLwjglVersion") {
        progressDescription.value = "等待游戏进程启动";
        processing.value = true;
        progressBarLoading.value = true;
        backButtonDisabled.value = true;
      } else if (task.job === "LogOpenALLoaded") {
        progressDescription.value = "等待游戏进程启动";
        processing.value = true;
        progressBarLoading.value = true;
        backButtonDisabled.value = true;
      } else if (task.job === "LogTextureLoaded") {
        progressDescription.value = "游戏已启动";
        processing.value = true;
        progressBarLoading.value = true;
        backButtonDisabled.value = true;
        setTimeout(() => {
          processing.value = false;
          navigationStore.back();
        }, 1000);
      }
    },
    onFailed: (error) => {
      processing.value = false;
      console.error(error);
    },
    onSucceed: () => {
      processing.value = false;
    },
    onCancelled: () => {
      processing.value = false;
    },
  });
  cancelLaunchHandle = launchTask.cancel;
  try {
    await launchTask.start();
  } catch (error) {
    console.error(error);
  }
}

const showBackButton = ref(true);
const backButtonDisabled = ref(false);

onMounted(() => launch());
onUnmounted(async () => {
  try {
    await cancelInstallHandle();
  } catch {}
  try {
    await cancelLaunchHandle();
  } catch {}
});

function back() {
  showBackButton.value = false;
  setTimeout(() => {
    navigationStore.back();
  }, 300);
}
</script>

<style lang="less" scoped>
.launch-view {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;

  .back-button {
    position: fixed;
    bottom: 32px;
    left: 32px;
    button {
      appearance: none;
      background: var(--ctp-latte-lavender);
      border: none;
      height: 48px;
      padding-right: 26px;
      border-radius: 1000px;
      display: flex;
      align-items: center;
      transition: all 0.3s ease;
      transform: scale(1);
      font-size: 14px;
      div {
        background: #ffffff3f;
        border-radius: 100px;
        width: 40px;
        height: 40px;
        display: flex;
        align-items: center;
        justify-content: center;
        margin-left: 6px;
      }
      span {
        margin-left: 10px;
      }
      &:active {
        transition: all 0.3s cubic-bezier(0, 0.75, 0.2, 1);
        transform: scale(0.97);
      }
    }
  }
  .back-button.disabled {
    pointer-events: none;
    opacity: 0.6;
  }

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
      p:first-child {
        margin-right: 16px;
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
}

.custom-slide-bottom-leave-active {
  transition: all 0.3s cubic-bezier(0.75, 0, 1, 0.2);
}

.custom-slide-bottom-enter-active {
  transition: all 0.3s cubic-bezier(0, 0.75, 0.2, 1);
}

.custom-slide-bottom-leave-from {
  transform: translate(0, 0);
}

.custom-slide-bottom-leave-to {
  transform: translate(0, 70px);
}

.custom-slide-bottom-enter-from {
  transform: translate(0, 70px);
}

.custom-slide-bottom-enter-to {
  transform: translate(0, 0);
}
</style>
