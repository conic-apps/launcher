<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <SettingGroup :title="t('settings.accessibility.title')">
      <SettingItem
        :title="t('settings.accessibility.releaseReminder')"
        :description="t('settings.accessibility.releaseReminderDesc')"
        icon="bell"
        icon-fill="none">
        <BaseSwitch v-model="config.accessibility.release_reminder"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="t('settings.accessibility.snapshotReminder')"
        :description="t('settings.accessibility.snapshotReminderDesc')"
        icon="bell"
        icon-fill="none">
        <BaseSwitch v-model="config.accessibility.snapshot_reminder"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="t('settings.accessibility.autoGameLang')"
        :description="t('settings.accessibility.autoGameLangDesc')"
        icon="language">
        <BaseSwitch v-model="config.accessibility.change_game_language"></BaseSwitch>
      </SettingItem>
    </SettingGroup>
    <SettingGroup :title="'无障碍'">
      <!-- <SettingItem -->
      <!--   :title="'禁用所有动画'" -->
      <!--   :description="'眨眼和闪烁的动画对于有认知问题的人来说是有问题的，比如注意力缺陷多动障碍 (ADHD)。此外，某些动画效果可以触发前庭神经紊乱、癫痫、偏头痛和暗点敏感性。'" -->
      <!--   icon="pause"> -->
      <!--   <BaseSwitch v-model="config.accessibility.disable_animations"></BaseSwitch> -->
      <!-- </SettingItem> -->
      <SettingItem
        :title="t('settings.accessibility.highContrast')"
        :description="t('settings.accessibility.highContrastDesc')"
        icon="contrast">
        <BaseSwitch v-model="config.accessibility.high_contrast_mode"></BaseSwitch>
      </SettingItem>
    </SettingGroup>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import BaseSwitch from "@/components/BaseSwitch.vue";
import { useConfigStore } from "@/store/config";
import { watch } from "vue";
import { reloadPalette } from "@/theme";
import { useI18n } from "vue-i18n";

const { t } = useI18n();
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
