<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <SettingGroup :title="$t('settings.audio.sidebar')">
      <SettingItem
        :title="$t('settings.audio.enableMusic')"
        :description="$t('settings.audio.enableMusicDesc')"
        icon="musical-notes">
        <BaseSwitch v-model="config.music.enabled"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="$t('settings.audio.resumeOnStartup')"
        :description="$t('settings.audio.resumeOnStartupDesc')"
        icon="play-circle-outline">
        <BaseSwitch v-model="config.music.resume_on_startup"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="$t('settings.audio.showVisualizer')"
        :description="$t('settings.audio.showVisualizerDesc')"
        icon="musical-notes">
        <BaseSwitch v-model="config.music.show_visualizer"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="$t('settings.audio.openFolder')"
        :description="$t('settings.audio.openFolderDesc')"
        icon="music-folder"
        :navigable="true"
        @click="openMusicFolder"></SettingItem>
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

const config = useConfigStore();

async function openMusicFolder() {
  const dataLocation = await getDataLocation();
  invoke("open_path", { path: dataLocation.music });
}
</script>

<style lang="less" scoped></style>
