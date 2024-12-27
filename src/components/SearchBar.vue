<!-- Amethyst Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="search-bar" :style="style">
    <i class="magnifying-glass"></i>
    <input
      type="text"
      v-model="value"
      @input="$emit('search', value)"
      :placeholder="props.placeholder" />
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
let value = ref("");
const props = withDefaults(
  defineProps<{
    width?: string;
    height?: string;
    placeholder?: string;
  }>(),
  {
    width: "",
    height: "",
    placeholder: "搜索",
  },
);
let style = `width: ${props.width}; height: ${props.height};`;
// todo: 添加search in 属性，搜索用的算法全部在这个组件中进行
</script>

<style lang="less" scoped>
.search-bar {
  display: flex;
  align-items: center;
  background: var(--controllers-background);
  border: var(--controllers-border);
  border-radius: var(--controllers-border-radius);
  padding: 0px 10px;
  width: 472px;
  height: 34px;
  position: relative;
  margin: 0 auto;
  top: 0px;
  bottom: 0px;
  left: 0px;
  right: 0px;
  z-index: 10;
  transition: all 0.3s ease;
}

.search-bar i {
  font-family: "fa-pro";
  font-style: normal;
  font-size: 12px;
  margin-right: 6px;
  color: rgba(var(--default-text-color), 0.8);
  transform: scale(0.95);
}

.search-bar input {
  border: none;
  background: none;
  outline: none;
  font-size: 14px;
  color: rgba(var(--default-text-color), 0.8);
  width: 100%;
  margin-left: 6px;
}

.search-bar input::placeholder {
  color: rgba(var(--default-text-color), 0.8);
  display: none;
}
</style>
