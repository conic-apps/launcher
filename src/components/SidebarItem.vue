<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <li class="sidebar-item">
    <AppIcon :name="icon" :size="20"></AppIcon>
    <div class="text" v-if="title">{{ title }}</div>
    <!-- todo: button active style -->
  </li>
</template>

<script setup lang="ts">
import AppIcon from "./AppIcon.vue";

defineProps<{
  title?: string;
  icon: string;
}>();
</script>

<style lang="less" scoped>
.sidebar-item {
  width: 48px;
  height: 46px;
  // color: rgba(255, 255, 255, 0.6);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  margin-top: 8px;
  border-radius: 10px;
  transition: all 100ms;
  background: var(--sidebar-item-background);
  .text {
    font-size: 10px;
    margin-top: 6px;
  }
}

.sidebar-item:hover {
  background: var(--sidebar-item-background-hover);
}

.sidebar-item:active {
  opacity: 0.6;
  transform: scale(0.97);
}
</style>
