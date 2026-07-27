<template>
  <div class="current-instance">
    <div class="row-1">
      <div class="current-instance-info">
        <!-- TODO: Click here to change Minecraft version -->
        <p class="title" v-if="currentInstance.id === LATEST_RELEASE_INSTANCE_ID">
          {{ $t("game.latestRelease") }}
        </p>
        <p class="title" v-else-if="currentInstance.id === LATEST_SNAPSHOT_INSTANCE_ID">
          {{ $t("game.latestSnapshot") }}
        </p>
        <p class="title" v-else>{{ currentInstance.config.name }}</p>
      </div>
    </div>
    <div class="row-2">
      <p>
        <span> Minecraft 版本 </span>
        <span>{{ currentInstance.config.runtime.minecraft }}</span>
      </p>
      <div
        class="line"
        v-if="
          currentInstance.config.runtime.mod_loader_type &&
          currentInstance.config.runtime.mod_loader_version
        "></div>
      <p
        v-if="
          currentInstance.config.runtime.mod_loader_type &&
          currentInstance.config.runtime.mod_loader_version
        ">
        <span> {{ currentInstance.config.runtime.mod_loader_type }} 版本 </span>
        <span>{{ currentInstance.config.runtime.mod_loader_version }}</span>
      </p>
      <div class="line"></div>
      <p>
        <span>最后运行日期</span>
        <span>昨天</span>
      </p>
      <div class="line"></div>
      <AppIcon name="time" :size="22" style="margin-right: 2px"></AppIcon>
      <p>
        <span>游戏时间</span>
        <span>1032.2 小时</span>
      </p>
    </div>
    <div class="row-3">
      <button class="launch-button" @click="clickLaunch">
        <AppIcon name="play" fill="#fff" style="margin-right: 4px"></AppIcon>
        开始游戏
      </button>
      <button class="launch-sub-button">
        <AppIcon
          name="chevron-down"
          stroke="#ffffff"
          fill="#ffffff"
          style="color: #fff"
          :size="16"></AppIcon>
      </button>
      <div class="progress-container" v-if="processing">
        <p class="progress-description">{{ progressDescription }}</p>
        <BaseProgress
          :loading="progressBarLoading"
          :value="progressBarValue"
          :max="progressBarMax"></BaseProgress>
      </div>
      <div class="actions" :class="{ launching: processing }">
        <button class="action-button">
          <AppIcon name="folder"></AppIcon>
        </button>
        <button class="action-button">
          <AppIcon name="share-social-outline"></AppIcon>
        </button>
        <button class="action-button">
          <AppIcon name="settings"></AppIcon>
        </button>
      </div>
    </div>
    <div class="row-4">
      <div>
        <AppIcon name="save"></AppIcon>
        <div><span class="type">存档</span><span class="count">1 个</span></div>
      </div>
      <div>
        <AppIcon name="extension-puzzle" />
        <div><span class="type">模组</span><span class="count">1 个</span></div>
      </div>
      <div>
        <AppIcon name="folder" />
        <div><span class="type">资源包</span><span class="count">4 个</span></div>
      </div>
      <div>
        <AppIcon name="aperture-outline" />
        <div><span class="type">光影包</span><span class="count">5 个</span></div>
      </div>
      <div>
        <AppIcon name="images-outline" />
        <div><span class="type">截图</span><span class="count">1 个</span></div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import { useInstanceStore } from "@/store/instance";
import { computed, ref } from "vue";
import { LATEST_RELEASE_INSTANCE_ID, LATEST_SNAPSHOT_INSTANCE_ID } from "@conic/instance";
import BaseProgress from "@/components/base/BaseProgress.vue";
import { InstallTask, Job } from "@conic/install";
import { useConfigStore } from "@/store/config";
import { LaunchTask } from "@conic/launch";

const instanceStore = useInstanceStore();
const currentInstance = computed(() => {
  return instanceStore.currentInstance;
});

const configStore = useConfigStore();

const progressDescription = ref("正在准备");
const processing = ref(false);
const progressBarLoading = ref(true);
const progressBarValue = ref(0);
const progressBarMax = ref(0);

async function clickLaunch() {
  if (!instanceStore.currentInstance.installed) {
    await installGame();
  }
  await launchGame();
}

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
  await installTask.start();
}

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
      } else if (task.job === "LogSettingUser") {
        progressDescription.value = "等待游戏进程启动";
        processing.value = true;
        progressBarLoading.value = true;
      } else if (task.job === "LogLwjglVersion") {
        progressDescription.value = "等待游戏进程启动";
        processing.value = true;
        progressBarLoading.value = true;
      } else if (task.job === "LogOpenALLoaded") {
        progressDescription.value = "等待游戏进程启动";
        processing.value = true;
        progressBarLoading.value = true;
      } else if (task.job === "LogTextureLoaded") {
        progressDescription.value = "游戏已启动";
        processing.value = true;
        progressBarLoading.value = true;
        setTimeout(() => {
          processing.value = false;
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
  launchTask.start();
}
</script>

<style lang="less" scoped>
.current-instance {
  position: absolute;
  top: 40%;
  transform: translateY(-50%);
  margin-left: 48px;

  .row-1 {
    display: flex;
    align-items: center;

    .current-instance-icon {
      width: 40px;
      height: 40px;
      border-radius: calc(var(--card-icon-border-radius) + 4px);
      background: var(--card-icon-background);
    }
    .current-instance-info {
      width: 100%;
      height: 100%;
      display: flex;

      .title {
        font-size: 38px;
      }
    }
  }
  .row-2 {
    display: flex;
    align-items: center;
  }

  .row-3 {
    display: flex;
    align-items: center;
    margin-top: 16px;
    position: relative;

    .progress-container {
      position: absolute;
      width: 140px;
      height: 100%;
      display: flex;
      flex-direction: column;
      align-items: start;
      justify-content: center;
      left: 170px;

      p.progress-description {
        font-size: 13px;
      }

      .progress {
        width: 100%;
        margin-top: 8px;
      }
    }

    .open-instance-setting-button {
      appearance: none;
      background: none;
      border: none;
      width: 32px;
      height: 32px;
      margin-right: 16px;
    }

    .open-instance-setting-button:active {
      opacity: 0.9;
      transform: scale(0.95);
    }

    div.actions {
      display: flex;
      margin-left: 16px;
      transition: transform 100ms ease;
    }

    div.actions.launching {
      transform: translateX(152px);
    }

    .action-button {
      appearance: none;
      border: none;
      color: var(--ctp-text);
      width: 32px;
      height: 32px;
      border-radius: 100px;
      display: flex;
      align-items: center;
      justify-content: center;
      margin-right: 8px;
      background: none;
      transition:
        background 100ms ease,
        transform 100ms ease;

      &:hover {
        background: var(--ctp-surface1);
      }

      &:active {
        transform: scale(0.9);
        background: var(--ctp-surface0);
      }

      &:last-child {
        margin-right: 0;
      }
    }

    .launch-button {
      appearance: none;
      border: none;
      color: #fff;
      width: 128px;
      height: 42px;
      font-size: 15px;
      display: flex;
      align-items: center;
      justify-content: center;
      border-radius: 8px 0 0 8px;
      background: rgb(114, 135, 253);
    }

    .launch-sub-button {
      appearance: none;
      border: none;
      background: #ffffff4f;
      height: 42px;
      width: 24px;
      border-radius: 0 8px 8px 0;
      background: rgb(114, 135, 253);
      margin-left: 2px;
    }

    .launch-button:active {
      opacity: 0.9;
    }
  }
  .row-2 {
    display: flex;
    align-items: center;
    margin-top: 16px;

    > p {
      font-size: 12px;
      display: flex;
      flex-direction: column;
      align-items: initial;
      width: fit-content;
      padding: 2px 4px;

      :first-child {
        opacity: 0.8;
        font-size: 12px;
      }

      :last-child {
        margin-top: 2px;
        font-size: 15px;
      }
    }

    div.line {
      width: 1px;
      height: 26px;
      background: var(--ctp-surface2);
      margin: 0px 8px;
    }
  }

  .row-4 {
    display: flex;
    align-items: center;
    margin-top: 16px;
    background: #4f4f4f72;
    background: var(--ctp-base);
    backdrop-filter: blur(10px);
    width: fit-content;
    padding: 4px 4px;
    border-radius: 12px;

    > div {
      display: flex;
      align-items: center;
      padding: 8px 16px;
      border-radius: 8px;
      transition: all 100ms ease;

      &:hover {
        background: var(--ctp-surface0);
      }

      &:active {
        background: var(--ctp-surface1);
        transform: scale(0.95);
      }
    }
    > div > div {
      font-size: 12px;
      display: flex;
      flex-direction: column;
      align-items: initial;
      width: fit-content;
      padding: 2px 4px;
      margin-left: 8px;

      :first-child {
        opacity: 0.8;
        font-size: 12px;
      }

      :last-child {
        margin-top: 2px;
        font-size: 15px;
      }
    }
  }
}
</style>
