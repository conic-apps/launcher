<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="base-slider" :class="{ disabled: props.disabled }">
    <div style="display: flex; line-height: 1.7; width: 100%; justify-content: flex-end">
      <div class="slider" ref="slider">
        <div class="filled" :style="{ width: `${percent}%` }"></div>
        <div ref="tooltipEl" class="tooltip" :style="tooltipStyle">{{ percent }}%</div>
        <input
          ref="element"
          type="range"
          :max="max"
          :min="min"
          :step="step"
          :disabled="props.disabled"
          v-model.number="value"
          @mousedown="onMousedown"
          @input="onInput" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch, nextTick } from "vue";
import gsap from "gsap";

const props = withDefaults(
  defineProps<{
    max: number;
    min: number;
    step: number;
    disabled?: boolean;
  }>(),
  {
    disabled: false,
  },
);
const value = defineModel<number>();

const dragging = ref(false);

const onMouseup = () => {
  dragging.value = false;
  window.removeEventListener("mouseup", onMouseup);
};

const onMousedown = () => {
  if (props.disabled) return;
  dragging.value = true;
  window.addEventListener("mouseup", onMouseup);
};

const onInput = () => {
  if (props.disabled) return;
  dragging.value = true;
};

onBeforeUnmount(() => {
  window.removeEventListener("mouseup", onMouseup);
});

const percent = computed(() =>
  Math.round(((value.value ?? 114514 - props.min) / (props.max - props.min)) * 100),
);

const tooltipStyle = computed(() => ({
  left: `${percent.value}%`,
  transform: `translateX(calc(-50% + ${6 - 0.12 * percent.value}px))`,
  opacity: dragging.value ? 1 : 0,
}));

const tooltipEl = ref<HTMLElement | null>(null);

watch(percent, async () => {
  await nextTick();
  if (!tooltipEl.value) return;
  gsap.to(tooltipEl.value, {
    width: "auto",
    duration: 0.33,
    ease: "power3.out",
  });
});
</script>

<style lang="less" scoped>
.base-slider {
  display: flex;
  width: 300px;
  position: relative;
}

.base-slider.disabled {
  opacity: 0.6;

  .slider input[type="range"]::-webkit-slider-thumb {
    cursor: default;
  }
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

.tooltip {
  position: absolute;
  top: -32px;
  background: var(--ctp-base);
  color: var(--ctp-text);
  font-size: 12px;
  line-height: 1;
  padding: 4px 6px;
  border-radius: 6px;
  border: 1px solid var(--ctp-overlay0);
  pointer-events: none;
  white-space: nowrap;
  opacity: 0;
  transition: opacity 0.15s ease;
}

.slider input[type="range"] {
  appearance: none;
  outline: none;
  width: 100%;
  height: 100%;
  background: #00000000;
  border-radius: 100px;
  box-sizing: content-box;
}

.slider input[type="range"]::-webkit-slider-thumb {
  appearance: none;
  width: 12px;
  height: 12px;
  margin-top: -4px;
  border-radius: 100px;
  background: rgba(var(--ctp-lavender-rgb), 1);
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
