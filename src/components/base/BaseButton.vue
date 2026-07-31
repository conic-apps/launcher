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
  transition: all 150ms ease;
}

button.base-button.disabled {
  opacity: 0.5;
  pointer-events: none;
}

button.base-button:hover {
  background: rgba(var(--ctp-surface2-rgb), 0.85);
}

button.base-button:active {
  background: var(--ctp-surface2);
}
button.base-button:hover {
  transform: scale(1.02);
}
button.base-button:active {
  transform: scale(0.97);
}
</style>
