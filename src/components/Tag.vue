<!-- Amethyst Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div
    class="tag"
    :style="`${border ? `border: 1px solid rgb(${color[0]}, ${color[1]}, ${color[2]}); ` : ''}
      ${background ? `background: rgba(${color[0]}, ${color[1]}, ${color[2]}, 0.3);` : ''} 
      color: ${textColor}; font-size: ${fontSize}px;
      ${round ? 'border-radius: 10000px' : ''}`">
    {{ text }}
  </div>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    text: string;
    color: string[];
    border?: boolean;
    background?: boolean;
    textColor?: string;
    fontSize?: string;
    round?: boolean;
  }>(),
  {
    fontSize: "11",
  },
);
</script>

<style lang="less" scoped>
.tag {
  margin-right: 2px;
  width: fit-content;
  height: fit-content;
  font-size: 11px;
  padding: 2px 4px;
  border-radius: var(--tag-border-radius);
}
</style>
