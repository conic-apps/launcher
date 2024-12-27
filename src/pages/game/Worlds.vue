<!-- Amethyst Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="worlds">
    <div class="world-list">
      <list-item
        v-for="(world, index) in saves"
        :key="index"
        :title="world.levelData.LevelName"
        :logo="world.icon"
        :click-able="false"
        :buttons="['circle-info', 'folders', 'arrow-up-right-from-square']">
        <template #subtitle>
          <tag
            :text="world.levelData.Version.Name"
            :color="['180', '180', '180']"
            text-color="#fffffff0"
            :border="true"
            :round="true"></tag>
          <tag
            v-if="world.levelData.allowCommands"
            :text="$t('game.saves.allowCheat')"
            :color="['180', '180', '180']"
            text-color="#fffffff0"
            :border="true"
            :round="true"></tag>
          <tag
            v-if="world.levelData.hardcore"
            :text="$t('game.saves.hardcore')"
            :color="['180', '180', '180']"
            text-color="#fffffff0"
            :border="true"
            :round="true"></tag>
        </template>
        {{ world.folderName }}
      </list-item>
    </div>
  </div>
</template>

<script setup lang="ts">
import ListItem from "@/components/ListItem.vue";
import Tag from "@/components/Tag.vue";
import { useInstanceStore } from "@/store/instance";
import { invoke } from "@tauri-apps/api/core";
import { onMounted, onUnmounted, ref, watch } from "vue";

type Save = {
  icon: string;
  levelData: any;
  folderName: string;
};

const instanceStore = useInstanceStore();

const saves = ref<Save[]>([]);

watch(
  instanceStore.currentInstance,
  () => {
    invoke("scan_saves_folder").then((res: any) => {
      saves.value = res as Save[];
    });
  },
  {
    immediate: true,
  },
);

// onMounted(() => {
//   invoke("scan_saves_folder").then((res: any) => {
//     saves.value = res as Save[];
//   });
// });
</script>

<style lang="less" scoped>
.worlds {
  .world-list {
    display: flex;
    width: 100%;
    flex-direction: column;
    border-radius: var(--list-border-radius);
    overflow: hidden;
  }
}
</style>
