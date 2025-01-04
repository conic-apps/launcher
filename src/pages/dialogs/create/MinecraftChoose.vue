<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="main">
    <search-bar style="margin-bottom: 10px; width: 100%"> </search-bar>
    <div
      style="
        border-radius: var(--list-border-radius);
        overflow: hidden;
        border: 1px solid rgba(0, 0, 0, 0.16);
        box-shadow: rgba(0, 0, 0, 0.16) 0 0 10px;
        content-visibility: auto;
      ">
      <TransitionGroup>
        <list-item
          v-for="(version, index) in versions"
          :key="index"
          :title="`Minecraft ${version.id}`"
          logo="1"
          :click-able="true"
          @click="$emit('select', version.id)"
          :buttons="['circle-info', 'floppy-disk']"
          :description="parseTime(version.releaseTime)">
          <template #icon>
            <img
              v-if="version.type == `release`"
              style="width: 100%; height: 100%; content-visibility: auto"
              src="@/assets/images/Grass_Block.webp"
              alt="" />
            <img
              v-else-if="version.type == `snapshot`"
              style="width: 100%; height: 100%; content-visibility: auto"
              src="@/assets/images/Command_Block.webp"
              alt="" />
            <img
              v-else
              style="width: 100%; height: 100%; content-visibility: auto"
              src="@/assets/images/Ancient_Debris.webp"
              alt="" />
          </template>
          <template #subtitle v-if="version.type == 'release'">
            <tag
              text="正式版"
              :color="['74', '194', '107']"
              :background="true"
              :border="true"
              :round="true"></tag>
          </template>
          <template #subtitle v-else-if="version.type == 'snapshot'">
            <tag
              text="测试版"
              :color="['200', '200', '0']"
              :background="true"
              :border="true"
              :round="true"></tag>
          </template>
          <template #subtitle v-else>
            <tag
              text="远古版"
              :color="['255', '129', '120']"
              :background="true"
              :border="true"
              :round="true"></tag>
          </template>
        </list-item>
      </TransitionGroup>
    </div>
  </div>
</template>

<script setup lang="ts">
import SearchBar from "@/components/SearchBar.vue";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import ListItem from "@/components/ListItem.vue";
import Tag from "@/components/Tag.vue";

let versions = ref<Array<any>>([]);
invoke("get_minecraft_version_list")
  .then((res: any) => {
    if (res != null) {
      // versions.value = res.versions;
      versions.value = res.versions.filter((i: any) => i.type == "release");
    } else {
      throw "get_version_list failed!";
    }
  })
  .catch((err) => {
    console.log(err);
  });

function parseTime(time: string) {
  let date = new Date(time);
  return `发布于 ${date.getFullYear()}年${date.getMonth() + 1}月${date.getDate()}日`;
}
</script>

<style lang="less" scoped>
.main {
  height: 100%;
  width: 100%;
  margin: 12px 14px;
  width: calc(100% - 28px);
}
</style>
