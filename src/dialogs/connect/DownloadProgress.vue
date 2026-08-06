<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="connect-extension-download-progress">
    <div style="display: flex; align-items: center">
      <AppIcon name="download" :size="50" style="margin-right: 16px; flex-shrink: 0"></AppIcon>
      <div class="message" style="width: 100%">
        <p
          style="
            font-size: 16px;
            width: 100%;
            display: flex;
            justify-content: space-between;
            align-items: end;
          ">
          <span>{{ progressPhase }}</span
          ><span style="font-size: 14px" v-if="progressBar.value && progressBar.max"
            >{{ formattedProgressValue.value }} / {{ formattedProgressValue.max }}</span
          >
        </p>
        <div style="margin-top: 16px">
          <BaseProgress
            :value="progressBar.value"
            :max="progressBar.max"
            :loading="progressBar.loading"></BaseProgress>
        </div>
      </div>
    </div>
    <div class="buttons">
      <BaseButton class="stop" @click="cancelDownload"> 停止下载 </BaseButton>
    </div>
  </div>
</template>

<script setup lang="ts">
import { formatBytes } from "@conic/download";
import { computed, onMounted, ref } from "vue";
import { ConicNexusLibraryDownloadTask } from "@conic/multiplayer";
import { useDialogStore } from "@/store/dialog";
import BaseButton from "@/components/BaseButton.vue";
import BaseProgress from "@/components/BaseProgress.vue";

const dialogStore = useDialogStore();

const progressBar = ref({
  loading: true,
  value: 0,
  max: 0,
});

enum ProgressPhase {
  Prepare = "准备下载 Conic Nexus 扩展",
  Downloading = "正在下载 Conic Nexus 扩展",
  Finished = "完成！",
}

const progressPhase = ref<ProgressPhase>(ProgressPhase.Prepare);
const formattedProgressValue = computed(() => {
  return {
    value: formatBytes(progressBar.value.value),
    max: formatBytes(progressBar.value.max),
  };
});

let cancelDownloadHandle: () => Promise<void>;
onMounted(async () => {
  const downloadTask = new ConicNexusLibraryDownloadTask({
    onProgress: (progress) => {
      if (progress.phase === "VerifyExistingFiles") {
        progressBar.value.loading = true;
        progressBar.value.max = 10;
        progressBar.value.value = 0;
        progressPhase.value = ProgressPhase.Prepare;
      } else if (progress.phase === "DownloadFiles") {
        progressPhase.value =
          progress.totalBytes === 0 ? ProgressPhase.Prepare : ProgressPhase.Downloading;
        progressBar.value.loading = progress.totalBytes === 0;
        progressBar.value.max = progress.totalBytes;
        progressBar.value.value = progress.completedBytes;
      }
    },
  });
  cancelDownloadHandle = downloadTask.cancel;
  try {
    await downloadTask.start();
    progressPhase.value = ProgressPhase.Finished;
    progressBar.value.loading = false;
    progressBar.value.value = progressBar.value.max;
    setTimeout(() => {
      dialogStore.connectExtension.currentComponent = "connectManager";
    }, 500);
  } catch (error) {
    console.log(error);
  }
});

async function cancelDownload() {
  try {
    await cancelDownloadHandle();
  } catch (e) {
    console.error(e);
  } finally {
    dialogStore.connectExtension.currentComponent = "downloadDescription";
    dialogStore.connectExtension.visible = false;
  }
}
</script>

<style lang="less" scoped>
.connect-extension-download-progress {
  .title {
    font-size: 22px;
    text-align: center;
    margin-bottom: 16px;
  }
  .description {
    font-size: 14px;
    line-height: 1.5;
    margin-bottom: 8px;
  }
  .description-important {
    color: var(--ctp-red);
    padding: 8px 16px;
    border: 1px solid var(--ctp-red);
    border-radius: 8px;
    background: rgba(var(--ctp-red-rgb), 0.15);
  }
  div.buttons {
    margin-top: 16px;
    display: flex;
    gap: 12px;
    .start {
      background: var(--ctp-blue);
      color: var(--ctp-text-inverse);
    }
    .stop {
      color: var(--ctp-red);
      border: 1px solid rgba(var(--ctp-red-rgb), 0.6);

      &:hover {
        background: var(--ctp-red);
        color: var(--ctp-text-inverse);
      }
    }
  }
}
</style>
