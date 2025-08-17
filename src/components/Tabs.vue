<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="tabs">
    <div
      :class="{ 'tab-active': active == index }"
      class="tab"
      v-for="(item, index) in props.tabs"
      :key="index"
      @click="$emit('choose-tab', index)">
      <AppIcon :name="props.icons[index]" :size="16"></AppIcon>
      <span>{{ item }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "./AppIcon.vue";

const props = defineProps<{
  tabs: string[];
  icons: string[];
  active: number;
}>();

defineEmits(["choose-tab"]);
</script>

<style lang="less" scoped>
.tabs {
  display: flex;
  border-bottom: 1px solid rgba(255, 255, 255, 0.18);

  .tab {
    display: flex;
    align-items: center;
    border-radius: 6px 6px 0px 0px;
    width: fit-content;
    padding: 8px 16px;
    margin: 0px 4px;
    opacity: 0.6;
    font-size: 14px;
    span {
      margin-left: 4px;
    }
  }

  .tab-active {
    background: rgba(255, 255, 255, 0.08);
    outline: 1px solid rgba(255, 255, 255, 0.08);
    opacity: 1;
  }
}
</style>
