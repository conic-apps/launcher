<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <button
    v-if="updateStore.updating"
    class="update-indicator"
    :title="'正在下载更新...'"
    @click="dialogStore.updateApp.visible = true">
    <svg viewBox="0 0 512 512" :class="{ indeterminate }">
      <path class="ring-track" :d="RING_PATH" pathLength="100" stroke-dasharray="100"></path>
      <path
        class="ring-progress"
        :d="RING_PATH"
        pathLength="100"
        stroke-dasharray="100"
        :stroke-dashoffset="indeterminate ? undefined : dashOffset"></path>
      <path class="arrow" :d="ARROW_PATH"></path>
    </svg>
  </button>
</template>

<script setup lang="ts">
import { useDialogStore } from "@/store/dialog";
import { useUpdateStore } from "@/store/update";
import { computed } from "vue";

const RING_PATH = "M448 256c0-106-86-192-192-192S64 150 64 256s86 192 192 192 192-86 192-192z";
const ARROW_PATH = "M176 249.38L256 170l80 79.38M256 181.03V342";

const updateStore = useUpdateStore();
const dialogStore = useDialogStore();

const indeterminate = computed(() => {
  const progress = updateStore.progress;
  return !(progress.phase === "downloading" && progress.total && progress.total > 0);
});

const dashOffset = computed(() => {
  const progress = updateStore.progress;
  if (progress.phase !== "downloading" || !progress.total || progress.total === 0) {
    return 100;
  }
  const percent = Math.min((progress.downloaded / progress.total) * 100, 100);
  return 100 - percent;
});
</script>

<style lang="less" scoped>
.update-indicator {
  appearance: none;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--window-btn-icon-color);
  cursor: pointer;
  transition: background 120ms ease;

  &:hover {
    background: rgba(var(--ctp-surface2-rgb), 0.7);
  }

  svg {
    width: 18px;
    height: 18px;
  }

  .ring-track {
    fill: none;
    stroke: var(--window-btn-icon-color);
    stroke-width: 28;
    stroke-linecap: round;
    opacity: 0.25;
  }

  .ring-progress {
    fill: none;
    stroke: var(--ctp-blue);
    stroke-width: 28;
    stroke-linecap: round;
    transition: stroke-dashoffset 150ms ease;
  }

  .arrow {
    fill: none;
    stroke: var(--window-btn-icon-color);
    stroke-width: 32;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  svg.indeterminate .ring-progress {
    animation: progress-sweep 1.2s linear infinite;
  }
}

@keyframes progress-sweep {
  from {
    stroke-dashoffset: 100;
  }
  to {
    stroke-dashoffset: -100;
  }
}
</style>
