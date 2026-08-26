<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <BaseDialog
    :visible="dialogStore.multiplayerExtension.visible"
    :width="dialogSize.width"
    animate-height>
    <div class="connect" data-tauri-drag-region>
      <Transition name="slide-left" mode="out-in">
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
    width: 580,
    component: markRaw(DownloadDescription),
  },
  downloadProgress: {
    width: 480,
    component: markRaw(DownloadProgress),
  },
  multiplayerManager: {
    width: 580,
    component: markRaw(MultiplayerManager),
  },
};

const multiplayerManagerWidth = 600;

const dialogSize = computed(() => {
  if (dialogStore.multiplayerExtension.currentComponent === "multiplayerManager") {
    return { width: multiplayerManagerWidth };
  }
  return components[dialogStore.multiplayerExtension.currentComponent];
});
</script>

<style lang="less" scoped>
.connect {
  width: 100%;
  overflow: hidden;
  padding: 8px;
}
</style>
