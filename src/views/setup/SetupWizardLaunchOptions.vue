<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="game-settings">
    <p class="wizard-title">{{ t("setup.launchOptions.title") }}</p>
    <p class="wizard-message">
      {{ t("setup.launchOptions.desc") }}
    </p>
    <SettingsGame style="margin-top: 16px"></SettingsGame>
  </div>
</template>

<script setup lang="ts">
import SettingsGame from "../settings/SettingsGame.vue";
import { useI18n } from "vue-i18n";

const { t } = useI18n();
</script>

<style lang="less" scoped>
.game-settings {
  :deep(.setting-group),
  :deep(.setting-collapse) {
    width: 100%;
  }
}
</style>
