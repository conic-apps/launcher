<!-- Conic Launcher -->
<!-- Copyright 2022-2026 OakChaser and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <BaseDialog
    :visible="dialogStore.connectExtension.visible"
    :width="components[dialogStore.connectExtension.currentComponent].width"
    :height="components[dialogStore.connectExtension.currentComponent].height">
    <div class="connect" data-tauri-drag-region>
      <Transition name="slide-left">
        <component
          :is="components[dialogStore.connectExtension.currentComponent].component"></component>
      </Transition>
    </div>
  </BaseDialog>
</template>

<script setup lang="ts">
import BaseDialog from "@/components/base/BaseDialog.vue";
import { useDialogStore } from "@/store/dialog";
import { markRaw } from "vue";
import DownloadDescription from "./connect/DownloadDescription.vue";
import DownloadProgress from "./connect/DownloadProgress.vue";
import ConnectManager from "./connect/ConnectManager.vue";

const dialogStore = useDialogStore();

const components = {
  downloadDescription: {
    height: 295,
    width: 580,
    component: markRaw(DownloadDescription),
  },
  downloadProgress: {
    height: 148,
    width: 480,
    component: markRaw(DownloadProgress),
  },
  connectManager: {
    height: 280,
    width: 580,
    component: markRaw(ConnectManager),
  },
};
</script>

<style lang="less" scoped>
.connect {
  width: 100%;
  height: 100%;
  overflow: hidden;
  padding: 8px;
}
</style>
