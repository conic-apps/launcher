<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <SettingGroup :title="'背景音乐'">
      <SettingItem :title="'启用背景音乐'" :description="'在启动器中播放背景音乐。'">
        <BaseSwitch v-model="config.music.enabled"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="'启动时继续播放'"
        :description="'应用启动时继续播放上次的曲目及其进度。'">
        <BaseSwitch v-model="config.music.resume_on_startup"></BaseSwitch>
      </SettingItem>
      <SettingItem :title="'音频可视化'" :description="'在游戏页面底部显示音频可视化效果。'">
        <BaseSwitch v-model="config.music.show_visualizer"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="'打开音乐文件夹'"
        :description="'将你想播放的音乐放入此文件夹。'"
        :navigable="true"
        @click="openMusicFolder"></SettingItem>
    </SettingGroup>
    <SettingGroup title="音量">
      <SettingItem :title="'主音量'" :description="`${config.music.main_volumn}%`">
        <BaseSliderBar
          :max="100"
          :min="0"
          :step="1"
          v-model="config.music.main_volumn"></BaseSliderBar>
      </SettingItem>
      <SettingItem :title="'主音量（窗口位于后台时）'">
        <BaseSwitch v-model="config.music.main_volumn_background"></BaseSwitch>
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

const config = useConfigStore();

async function openMusicFolder() {
  const dataLocation = await getDataLocation();
  invoke("open_path", { path: dataLocation.music });
}
</script>

<style lang="less" scoped></style>
