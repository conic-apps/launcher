<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="mods">
    <div class="mod-list">
      <list-item
        v-for="(resourcepack, index) in resourcepacks"
        :key="index"
        :title="resourcepack.name"
        :logo="resourcepack.icon"
        :click-able="false"
        :buttons="['circle-info', 'folders', 'trash-can']">
        <template #subtitle v-if="resourcepack.type == 'texture'">
          <tag
            text="纹理包"
            :color="['74', '194', '107']"
            style="transform: scale(0.9)"
            :border="true"
            text-color="#1f883d"></tag>
        </template>
        <template #subtitle v-else-if="resourcepack.type == 'data'">
          <tag
            text="数据包"
            :color="['200', '200', '0']"
            style="transform: scale(0.9)"
            :border="true"
            text-color="#9a6700">
          </tag>
        </template>
        <template #subtitle v-else>
          <tag
            text="未知"
            :color="['200', '200', '200']"
            :border="true"
            text-color="#ffffffd0"></tag>
        </template>
        {{ resourcepack.metadata.description }}
      </list-item>
    </div>
  </div>
</template>

<script setup lang="ts">
import ListItem from "@/components/ListItem.vue";
import Tag from "@/components/Tag.vue";
import { useInstanceStore } from "@/store/instance";
import { invoke } from "@tauri-apps/api/core";
import { ref, watch } from "vue";

type Resourcepack = {
  icon: string;
  metadata: {
    description: string;
    pack_format: number;
    other: any;
  };
  name: string;
  type: "unknown" | "texture" | "data";
};

const instanceStore = useInstanceStore();

const resourcepacks = ref<Resourcepack[]>([]);

watch(
  instanceStore.currentInstance,
  () => {
    invoke("scan_resourcepack_folder").then((res: any) => {
      resourcepacks.value = res as Resourcepack[];
    });
  },
  {
    immediate: true,
  },
);
</script>

<style lang="less" scoped></style>
