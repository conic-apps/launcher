<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <button
    class="base-button"
    :class="{ disabled: props.disabled }"
    :style="{ color: props.color }"
    v-bind="$attrs">
    <slot></slot>
  </button>
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  disabled?: boolean;
  color?: string;
}>();
</script>

<style lang="less" scoped>
button.base-button {
  width: 100%;
  height: 30px;
  background: var(--controllers-background);
  border: var(--controllers-border);
  border-radius: var(--controllers-border-radius);
  margin-left: auto;
  padding: 10px 16px;
  display: flex;
  align-items: center;
  justify-content: center;
}

button.base-button.disabled {
  opacity: 0.5;
  pointer-events: none;
}

button.base-button:hover {
  background: rgba(255, 255, 255, 0.1);
}

button.base-button:active {
  opacity: 0.8;
}
</style>
