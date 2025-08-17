<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <setting-group :title="$t('settings.accessibility.extraFeatures')">
      <setting-item
        :title="$t('settings.accessibility.releaseReminder')"
        :description="$t('settings.accessibility.releaseReminderDesc')"
        icon="bell"
        icon-fill="none">
        <toggle-switch v-model="config.accessibility.release_reminder"></toggle-switch>
      </setting-item>
      <setting-item
        :title="$t('settings.accessibility.snapshotReminder')"
        :description="$t('settings.accessibility.snapshotReminderDesc')"
        icon="bell"
        icon-fill="none">
        <toggle-switch v-model="config.accessibility.snapshot_reminder"></toggle-switch>
      </setting-item>
      <setting-item
        :title="$t('settings.accessibility.hideLatestRelease')"
        :description="$t('settings.accessibility.hideLatestReleaseDesc')"
        icon="eye-off">
        <toggle-switch v-model="config.accessibility.hide_latest_release"></toggle-switch>
      </setting-item>
      <setting-item
        :title="$t('settings.accessibility.hideLatestSnapshot')"
        :description="$t('settings.accessibility.hideLatestSnapshotDesc')"
        icon="eye-off">
        <toggle-switch v-model="config.accessibility.hide_latest_snapshot"></toggle-switch>
      </setting-item>
      <setting-item
        :title="$t('settings.accessibility.changeGameLanguage')"
        :description="$t('settings.accessibility.changeGameLanguageDesc')"
        icon="language">
        <toggle-switch v-model="config.accessibility.change_game_language"></toggle-switch>
      </setting-item>
    </setting-group>
    <setting-group :title="$t('settings.accessibility.accessibility')">
      <setting-item
        :title="$t('settings.accessibility.disableAllAnimations')"
        :description="$t('settings.accessibility.disableAllAnimationsDesc')"
        icon="pause">
        <toggle-switch v-model="config.accessibility.disable_animations"></toggle-switch>
      </setting-item>
      <setting-item
        :title="$t('settings.accessibility.highContrastMode')"
        :description="$t('settings.accessibility.highContrastModeDesc')"
        icon="contrast">
        <toggle-switch v-model="config.accessibility.high_contrast_mode"></toggle-switch>
      </setting-item>
    </setting-group>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import ToggleSwitch from "@/components/ToggleSwitch.vue";
import { useConfigStore } from "@/store/config";
import { watch } from "vue";
import { reloadPalette } from "@/theme";
const config = useConfigStore();

watch(
  () => config.accessibility.disable_animations,
  (disableAnimations) => {
    if (disableAnimations) {
      document.body.classList.add("no-animations");
    } else {
      document.body.classList.remove("no-animations");
    }
  },
  {},
);

watch(
  () => config.accessibility.high_contrast_mode,
  (highContrastMode) => {
    reloadPalette(
      {
        palette: config.appearance.palette,
        paletteFollowSystem: config.appearance.palette_follow_system,
      },
      highContrastMode,
    );
  },
);
</script>

<style lang="less" scoped></style>
