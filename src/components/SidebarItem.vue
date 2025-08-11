<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <li class="sidebar-item">
    <AppIcon :name="icon" :size="24"></AppIcon>
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
  width: 58px;
  height: 56px;
  // color: rgba(255, 255, 255, 0.6);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  margin-top: 8px;
  border-radius: 10px;
  transition: all 100ms;
  background: var(--sidebar-item-background);
}

.sidebar-item:hover {
  background: var(--sidebar-item-background-hover);
  // border: 1px solid rgba(0, 0, 0, 0.06);
}

.sidebar-item:active {
  opacity: 0.6;
  transform: scale(0.97);
}

.sidebar-item .text {
  font-size: 12px;
  margin-top: 6px;
}
</style>
