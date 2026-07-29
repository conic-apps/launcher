<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="game-view" data-tauri-drag-region>
    <InstanceSummary></InstanceSummary>
    <InstancesList></InstancesList>
    <GameFooterBar></GameFooterBar>
  </div>
</template>

<script setup lang="ts">
import InstanceSummary from "./game/InstanceSummary.vue";
import InstancesList from "./game/InstancesList.vue";
import GameFooterBar from "./game/GameFooterBar.vue";
</script>

<style lang="less" scoped>
.game-view {
  width: 100%;
  height: 100%;
  position: relative;
}
</style>
