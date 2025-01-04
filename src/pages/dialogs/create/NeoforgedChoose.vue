<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="main">
    <search-bar style="margin-bottom: 10px"> </search-bar>
    <div
      style="
        border-radius: 10px;
        overflow: hidden;
        border: 1px solid rgba(0, 0, 0, 0.16);
        box-shadow: rgba(0, 0, 0, 0.16) 0 0 10px;
      ">
      <list-item
        v-for="(version, index) in versions"
        :key="index"
        :title="version"
        logo="1"
        :click-able="true"
        @click="$emit('select', version)"
        :buttons="['circle-info', 'floppy-disk']"
        :description="`适用于 Minecraft ${props.minecraft}`">
        <template #icon>
          <img style="width: 100%; height: 100%" src="@/assets/images/neoforged.png" alt="" />
        </template>
        <template #subtitle> </template>
      </list-item>
    </div>
  </div>
</template>

<script setup lang="ts">
import SearchBar from "@/components/SearchBar.vue";
import ListItem from "@/components/ListItem.vue";

const props = defineProps<{
  show?: boolean;
  minecraft: string;
  versions: any[];
}>();
const emit = defineEmits(["no-version", "loaded", "select"]);
</script>

<style lang="less" scoped>
.main {
  height: 100%;
  width: 100%;
  margin: 12px 14px;
  width: calc(100% - 28px);
}
</style>
