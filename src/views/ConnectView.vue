<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="connect-view" data-tauri-drag-region>
    <Transition name="fade" mode="default">
      <component
        :is="currentComponent"
        @switch-component-manage="$emit('switch-component-manage')"></component>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { markRaw, shallowRef } from "vue";
import ExtensionDownloadView from "./connect/ExtensionDownloadView.vue";
import GameFooterBar from "./game/GameFooterBar.vue";

const components = {
  extensionDownload: markRaw(ExtensionDownloadView),
};
const currentComponent = shallowRef(components.extensionDownload.component);
</script>

<style lang="less" scoped>
.connect-view {
  width: 100%;
  height: 100%;
  border: 1px solid #f00;
}
</style>
