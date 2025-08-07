<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="progress" :style="widthStyle">
    <p v-if="text">{{ text }}</p>
    <div class="progress-bar" :style="loadingStyle"></div>
    <div class="progress-loading" :style="loadingStyle"></div>
    <progress :max="total" :value="value" :style="progressStyle"></progress>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    loading: boolean;
    value: string;
    total: string;
    width: string;
    text?: string;
  }>(),
  {
    width: () => "100",
  },
);

const widthStyle = computed(() => {
  return `width: ${props.width}px`;
});
const loadingStyle = computed(() => {
  return props.loading ? "" : "display: none";
});
const progressStyle = computed(() => {
  return props.loading ? "display: none" : "";
});
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
  margin: 4px 0;
}

.progress-bar {
  width: 50%;
  height: 100%;
  background-color: #f0f0f0;
  border-radius: var(--border-radius-large);
  position: relative;
  animation: progress-loading 2.5s cubic-bezier(0.66, 0.01, 0.5, 0.97) 0s infinite;
}

.progress-loading {
  height: 1px;
  width: inherit;
  position: absolute;
  z-index: -1;
  background-color: rgba(255, 255, 255, 0.242);
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

.progress progress {
  appearance: none;
  height: inherit;
  width: inherit;
  margin-top: 2px;
}

.progress progress::-webkit-progress-bar {
  appearance: none;
  height: 1px;
  background-color: rgba(255, 255, 255, 0.242);
  display: flex;
  align-items: center;
  border-radius: 100px;
}

.progress progress::-webkit-progress-value {
  appearance: none;
  height: 3px;
  background: #f0f0f0;
  border-radius: 100px;
  transition: all 0.2s cubic-bezier(0, 0.62, 0.36, 1);
}
</style>
