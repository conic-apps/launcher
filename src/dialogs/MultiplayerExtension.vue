<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <BaseDialog
    :visible="dialogStore.multiplayerExtension.visible"
    :width="dialogSize.width"
    :height="dialogSize.height">
    <div class="connect" data-tauri-drag-region>
      <Transition name="slide-left">
        <component
          :is="components[dialogStore.multiplayerExtension.currentComponent].component"></component>
      </Transition>
    </div>
  </BaseDialog>
</template>

<script setup lang="ts">
import BaseDialog from "@/components/BaseDialog.vue";
import { useDialogStore } from "@/store/dialog";
import { computed, markRaw } from "vue";
import MultiplayerManager from "./multiplayer/MultiplayerManager.vue";
import DownloadDescription from "./multiplayer/DownloadDescription.vue";
import DownloadProgress from "./multiplayer/DownloadProgress.vue";

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
  multiplayerManager: {
    height: 280,
    width: 580,
    component: markRaw(MultiplayerManager),
  },
};

const multiplayerManagerDialogSize = {
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
  if (dialogStore.multiplayerExtension.currentComponent === "multiplayerManager") {
    return multiplayerManagerDialogSize[
      dialogStore.multiplayerExtension.multiplayerManagerComponent
    ];
  }
  return components[dialogStore.multiplayerExtension.currentComponent];
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
