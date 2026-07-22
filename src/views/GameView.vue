<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="game-view" data-tauri-drag-region>
    <InstanceSummary></InstanceSummary>
    <InstanceDrawer></InstanceDrawer>
  </div>
</template>

<script setup lang="ts">
import InstanceSummary from "./game/InstanceSummary.vue";
import InstanceDrawer from "./game/InstanceDrawer.vue";
</script>

<style lang="less" scoped>
.game-view {
  width: 100%;
  height: 100%;
  position: relative;
  // background:
  //   radial-gradient(
  //     circle at 95% 50%,
  //     transparent 90px,
  //     rgba(0, 0, 0, 0.4) 420px,
  //     rgba(0, 0, 0, 0.7) 100%
  //   ),
  //   url("@/assets/images/截屏2026-07-22 19.15.29.png") center / cover no-repeat;
}
</style>
