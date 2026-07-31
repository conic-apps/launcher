<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <SettingGroup title="背景音乐">
      <SettingItem title="启用背景音乐" icon="musical-notes"><BaseSwitch></BaseSwitch></SettingItem>
      <SettingItem
        title="打开音乐文件夹"
        description="将你想要播放的背景音乐添加到此文件夹"
        icon="music-folder"
        :navigable="true"
        @click="openMusicFolder"></SettingItem>
    </SettingGroup>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import BaseInput from "@/components/base/BaseInput.vue";
import BaseSwitch from "@/components/base/BaseSwitch.vue";
import { useConfigStore } from "@/store/config";
import { getDataLocation, getInstanceRoot } from "@conic/folder";
import { openPath } from "@tauri-apps/plugin-opener";
import { invoke } from "@tauri-apps/api/core";
const config = useConfigStore();

async function openMusicFolder(instanceId: string) {
  invoke("open_path", { path: await getInstanceRoot(instanceId) });
}
</script>

<style lang="less" scoped></style>
