<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="game-settings">
    <p class="wizard-title">游戏启动选项</p>
    <p class="wizard-message">
      这里列出了一些基本的启动选项。稍后你还可以在设置和实例独立设置中更改更多选项。
    </p>
    <SettingsGame style="margin-top: 16px"></SettingsGame>
  </div>
</template>

<script setup lang="ts">
import SettingsGame from "../settings/SettingsGame.vue";
</script>

<style lang="less" scoped>
.game-settings {
  :deep(.setting-group),
  :deep(.setting-collapse) {
    width: 100%;
  }
}
</style>
