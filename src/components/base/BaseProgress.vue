<!-- Conic Launcher -->
<!-- Copyright 2022-2026 OakChaser and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="progress">
    <div class="progress-track"></div>
    <div class="progress-loading" v-if="props.loading"></div>
    <div
      class="progress-progress"
      :style="{
        width: `${(value / max) * 100}%`,
      }"
      v-else></div>
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{
  loading?: boolean;
  value: number;
  max: number;
}>();
</script>

<style lang="less" scoped>
.progress {
  height: 2px;
  overflow: hidden;
  border-radius: var(--border-radius-large);
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;

  .progress-track {
    height: 1px;
    width: 100%;
    position: absolute;
    left: 0;
    background: var(--ctp-surface2);
  }

  .progress-loading {
    width: 50%;
    height: 3px;
    background-color: #f0f0f0;
    border-radius: var(--border-radius-large);
    position: relative;
    animation: progress-loading 2.5s cubic-bezier(0.66, 0.01, 0.5, 0.97) 0s infinite;
  }

  .progress-progress {
    position: absolute;
    left: 0;
    height: 3px;
    background-color: #f0f0f0;
    transition: width 300ms ease;
  }
}

@keyframes progress-loading {
  0% {
    left: -85%;
  }

  50% {
    left: 85%;
  }

  50.01% {
    left: -85%;
  }

  100% {
    left: 85%;
  }
}
</style>
