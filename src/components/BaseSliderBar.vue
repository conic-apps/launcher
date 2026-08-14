<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="base-slider">
    <div style="display: flex; line-height: 1.7; width: 100%; justify-content: flex-end">
      <div class="slider" ref="slider">
        <div
          class="filled"
          :style="{ width: `${((value ?? 114514 - min) / (max - min)) * 100}%` }"></div>
        <input
          ref="element"
          type="range"
          :max="max"
          :min="min"
          :step="step"
          v-model.number="value" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  max: number;
  min: number;
  step: number;
}>();
const value = defineModel<number>();
</script>

<style lang="less" scoped>
.base-slider {
  display: flex;
  width: 300px;
  position: relative;
}

.slider {
  width: 100%;
  display: flex;
  align-items: center;
  float: right;
  position: relative;
}

.slider > * {
  position: absolute;
}

.slider > div.filled {
  background: rgba(var(--ctp-lavender-rgb), 1);
  height: 3.5px;
  width: 4px;
  /* min-width: 4px; */
  border-radius: 10px;
  position: absolute;
  pointer-events: none;
}

.slider input[type="range"] {
  appearance: none;
  outline: none;
  width: 100%;
  height: 100%;
  background: #00000000;
  border-radius: 100px;
  box-sizing: content-box;
  pointer-events: all;
}

.slider input[type="range"]::-webkit-slider-thumb {
  appearance: none;
  width: 12px;
  height: 12px;
  margin-top: -4px;
  border-radius: 100px;
  background: rgba(var(--ctp-lavender-rgb), 1);
  pointer-events: all;
  transition: all 0.2s ease;
}

.slider input[type="range"]::-webkit-slider-thumb:hover {
  transform: scale(1.1);
}

.slider input[type="range"]:active::-webkit-slider-thumb {
  transform: scale(0.9);
}

.slider input[type="range"]::-webkit-slider-runnable-track {
  appearance: none;
  height: 3.5px;
  border-radius: 10px;
  background-color: #00000045;
}

input[type="number"]::-webkit-inner-spin-button,
input[type="number"]::-webkit-outer-spin-button {
  appearance: none;
  margin: 0;
}

.input-data {
  border-radius: var(--border-radius-small);
  width: 400px;
  overflow: hidden;
  box-shadow: 0 0 0 1px rgba(var(--ctp-lavender-rgb), 0.2);
  height: 30px;
  flex-shrink: 0;
  padding: 0 8px 2px 8px;
  font-size: 16px;
  transition: all 0.1s ease;
  pointer-events: all;
  background: rgba(255, 255, 255, 0.2);
}
</style>
