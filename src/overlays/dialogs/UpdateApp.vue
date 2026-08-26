<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <BaseDialog :visible="dialogStore.updateApp.visible" :width="500">
    <div class="update-app">
      <div class="header">
        <AppIcon :name="iconName" :size="50"></AppIcon>
        <div class="message">
          <p class="main" :class="{ error: state === 'error' }">{{ mainText }}</p>
          <p v-if="subText" class="sub">{{ subText }}</p>
        </div>
      </div>

      <div v-if="state === 'update' && updateStore.updateInfo?.notes" class="notes">
        {{ updateStore.updateInfo.notes }}
      </div>

      <div v-if="state === 'updating' && !installing" class="progress">
        <BaseProgress
          v-if="progressValue !== null"
          :value="progressValue"
          :max="progressMax"></BaseProgress>
        <BaseProgress v-else :value="0" :max="100" loading></BaseProgress>
      </div>

      <div v-if="!installing" class="buttons">
        <button class="back" @click="backAction">{{ backText }}</button>
        <button v-if="primaryAction" class="primary" @click="primaryAction">
          {{ primaryText }}
        </button>
      </div>
    </div>
  </BaseDialog>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import BaseDialog from "@/components/BaseDialog.vue";
import BaseProgress from "@/components/BaseProgress.vue";
import { useConfigStore } from "@/store/config";
import { useDialogStore } from "@/store/dialog";
import { useUpdateStore } from "@/store/update";
import { computed, watch } from "vue";

type DialogState = "checking" | "update" | "upToDate" | "updating" | "error";

const dialogStore = useDialogStore();
const updateStore = useUpdateStore();
const config = useConfigStore();

const state = computed<DialogState>(() => {
  if (updateStore.error) return "error";
  if (updateStore.updating) return "updating";
  if (updateStore.checking) return "checking";
  if (updateStore.updateInfo) return "update";
  return "upToDate";
});

const installing = computed(() => updateStore.progress.phase === "installing");

const iconName = computed(() => {
  switch (state.value) {
    case "upToDate":
      return "check";
    case "error":
      return "warning";
    default:
      return "circle-up";
  }
});

const CHANNEL_LABELS: Record<string, string> = {
  stable: "正式版",
  beta: "测试版",
  nightly: "夜间构建",
};

const channelText = computed(() => CHANNEL_LABELS[config.update_channel] ?? config.update_channel);

const mainText = computed(() => {
  switch (state.value) {
    case "checking":
      return "正在检查更新...";
    case "update":
      return `发现新版本：${updateStore.updateInfo?.version ?? ""}`;
    case "upToDate":
      return "已是最新版本";
    case "updating":
      return progressLabel.value;
    case "error":
      return updateStore.error;
  }
});

const subText = computed(() => {
  if (state.value === "update" || state.value === "checking") {
    return channelText.value;
  }
  return "";
});

const backText = computed(() => {
  switch (state.value) {
    case "updating":
      return "取消";
    case "checking":
    case "update":
      return "稍后再说";
    default:
      return "关闭";
  }
});

const primaryText = computed(() => {
  switch (state.value) {
    case "update":
      return "立即更新";
    case "upToDate":
    case "error":
      return "重新检查";
    default:
      return "";
  }
});

const primaryAction = computed(() => {
  switch (state.value) {
    case "update":
      return install;
    case "upToDate":
    case "error":
      return recheck;
    default:
      return null;
  }
});

const progressValue = computed(() => {
  const progress = updateStore.progress;
  if (progress.phase !== "downloading" || progress.total === undefined || progress.total === 0) {
    return null;
  }
  return Math.min(progress.downloaded, progress.total);
});
const progressMax = computed(() => {
  const progress = updateStore.progress;
  return progress.phase === "downloading" && progress.total ? progress.total : 100;
});
const progressLabel = computed(() => {
  const progress = updateStore.progress;
  switch (progress.phase) {
    case "checking":
      return "正在检查更新...";
    case "downloading": {
      if (progress.downloaded === 0) {
        return "正在下载更新...";
      }
      return `${progress.downloaded} / ${progress.total ?? "?"}`;
    }
    case "installing":
      return "正在安装并重启启动器...";
    default:
      return "";
  }
});

function check() {
  void updateStore.check(config.update_channel);
}

function recheck() {
  check();
}

function install() {
  void updateStore.downloadAndInstall(config.update_channel);
}

function backAction() {
  if (state.value === "updating") {
    updateStore.cancel();
  }
  dialogStore.updateApp.visible = false;
}

watch(
  () => dialogStore.updateApp.visible,
  (visible) => {
    if (
      visible &&
      !updateStore.updateInfo &&
      !updateStore.error &&
      !updateStore.checking &&
      !updateStore.updating
    ) {
      check();
    }
  },
);
</script>

<style lang="less" scoped>
.update-app {
  padding: 8px;

  .header {
    display: flex;
    align-items: center;

    .message {
      display: flex;
      flex-direction: column;
      justify-content: center;
      margin-left: 16px;
      min-width: 0;

      .main {
        font-size: 17px;
        margin: 0;
        word-break: break-word;

        &.error {
          color: var(--ctp-red);
        }
      }

      .sub {
        font-size: 12px;
        margin-top: 8px;
        opacity: 0.6;
      }
    }
  }

  .notes {
    margin-top: 14px;
    padding: 8px 10px;
    background: var(--ctp-surface0);
    border-radius: 6px;
    font-size: 12px;
    opacity: 0.7;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 120px;
    overflow-y: auto;
  }

  .progress {
    margin-top: 14px;
  }

  .buttons {
    display: flex;
    width: 100%;
    margin-top: 16px;

    button {
      appearance: none;
      border: none;
      width: 100%;
      border-radius: 4px;
      padding: 8px 0;
      transition: transform 200ms ease;
    }

    button.back {
      margin-right: 8px;
      background: var(--ctp-surface0);
      color: var(--default-text-color);
    }

    button.primary {
      background: var(--ctp-blue);
      color: var(--ctp-text-inverse);
    }

    button:hover {
      transform: scale(1.02);
    }

    button:active {
      transform: scale(0.97);
    }
  }
}
</style>
