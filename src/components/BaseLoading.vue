<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <svg
    :width="props.size"
    :height="props.size"
    viewBox="0 0 48 48"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    role="img"
    aria-label="loading">
    <circle
      cx="24"
      cy="24"
      r="20"
      stroke="currentColor"
      class="base-loading-circle"
      :stroke-width="props.strokeWidth"
      stroke-linecap="round"
      pathLength="100"
      :stroke-dasharray="dasharray"></circle>
  </svg>
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    size?: number;
    strokeWidth?: number;
    gap?: number;
  }>(),
  { size: 32, strokeWidth: 4, gap: 6 },
);

const dasharray = computed(() => {
  const gapUnits = props.gap + props.strokeWidth;
  return `${100 - gapUnits} ${gapUnits}`;
});
</script>

<style lang="less" scoped>
.base-loading-circle {
  transform-origin: 24px 24px;
  will-change: transform;
  animation: base-loading-spin 0.9s linear infinite;
}

@keyframes base-loading-spin {
  100% {
    transform: rotate(360deg);
  }
}
</style>
