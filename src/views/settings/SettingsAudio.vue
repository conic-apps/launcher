<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <SettingGroup :title="t('settings.audio.bgm')">
      <SettingItem
        :title="t('settings.audio.enableBgm')"
        :description="t('settings.audio.enableBgmDesc')">
        <BaseSwitch v-model="config.music.enabled"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="t('settings.audio.resumeOnStart')"
        :description="t('settings.audio.resumeOnStartDesc')">
        <BaseSwitch v-model="config.music.resume_on_startup"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="t('settings.audio.visualizer')"
        :description="t('settings.audio.visualizerDesc')">
        <BaseSwitch v-model="config.music.show_visualizer"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="t('settings.audio.pauseOnLaunch')"
        :description="t('settings.audio.pauseOnLaunchDesc')">
        <BaseSwitch v-model="config.music.pause_on_launch"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="t('settings.audio.openFolder')"
        :description="t('settings.audio.openFolderDesc')"
        :navigable="true"
        @click="openMusicFolder"></SettingItem>
    </SettingGroup>
    <SettingGroup :title="t('settings.audio.volume')">
      <SettingItem
        :title="t('settings.audio.masterVolume')"
        :description="`${config.music.main_volumn}%`">
        <BaseSliderBar
          :max="100"
          :min="0"
          :step="1"
          v-model="config.music.main_volumn"></BaseSliderBar>
      </SettingItem>
      <SettingItem
        :title="t('settings.audio.backgroundVolume')"
        :description="`${config.music.main_volumn_background}%`">
        <BaseSliderBar
          :max="100"
          :min="0"
          :step="1"
          v-model="config.music.main_volumn_background"></BaseSliderBar>
      </SettingItem>
    </SettingGroup>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import BaseSwitch from "@/components/BaseSwitch.vue";
import { useConfigStore } from "@/store/config";
import { getDataLocation } from "@conic/folder";
import { invoke } from "@tauri-apps/api/core";
import BaseSliderBar from "@/components/BaseSliderBar.vue";
import { useI18n } from "vue-i18n";

const { t } = useI18n();
const config = useConfigStore();

async function openMusicFolder() {
  const dataLocation = await getDataLocation();
  invoke("open_path", { path: dataLocation.music });
}
</script>

<style lang="less" scoped></style>
