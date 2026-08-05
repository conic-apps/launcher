<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <BaseDialog
    :visible="dialogStore.connectExtension.visible"
    :width="dialogSize.width"
    :height="dialogSize.height">
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
import { computed, markRaw } from "vue";
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

const connectManagerDialogSize = {
  waiting: {
    width: 600,
    height: 274,
  },
  hostScan: {
    width: 600,
    height: 280,
  },
  hostReady: {
    width: 600,
    height: 340,
  },
  guestCodeInput: {
    width: 600,
    height: 244,
  },
  guestJoining: {
    width: 600,
    height: 210,
  },
  guestReady: {
    width: 600,
    height: 340,
  },
  exception: {
    width: 600,
    height: 190,
  },
};

const dialogSize = computed(() => {
  if (dialogStore.connectExtension.currentComponent === "connectManager") {
    return connectManagerDialogSize[dialogStore.connectExtension.connectManagerComponent];
  }
  return components[dialogStore.connectExtension.currentComponent];
});
</script>

<style lang="less" scoped>
.connect {
  width: 100%;
  height: 100%;
  overflow: hidden;
  padding: 8px;
}
</style>
