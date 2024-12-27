<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="tabs">
    <div
      :class="active == index ? 'tab-active' : ''"
      class="tab"
      v-for="(item, index) in props.tabs"
      :key="index"
      @click="$emit('choose-tab', index)">
      <i :class="props.icons[index]"></i>
      <span>{{ item }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
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

    i {
      font-family: "fa-pro";
      font-size: 0.8em;
      font-style: normal;
      margin-right: 4px;
      margin-top: 2px;
    }
  }

  .tab-active {
    background: rgba(255, 255, 255, 0.08);
    outline: 1px solid rgba(255, 255, 255, 0.08);
    opacity: 1;
  }
}
</style>
