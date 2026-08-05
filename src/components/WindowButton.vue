<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div
    class="win-btn win-btn-minimize"
    v-if="buttonType === 'minimize'"
    @mouseenter="windowButtonHover = true"
    @mouseleave="windowButtonHover = false"
    :style="{
      background: computedLit ? 'rgb(254,188,46)' : 'var(--window-btn-unfocused-color)',
      border: 'var(--window-btn-border)',
    }"
    @click="$emit(buttonType)">
    <AppIcon v-if="showIcon" name="minus" :size="10" />
  </div>
  <div
    class="win-btn win-btn-maximize"
    v-else-if="buttonType === 'maximize'"
    @mouseenter="windowButtonHover = true"
    @mouseleave="windowButtonHover = false"
    :style="{
      background: computedLit ? 'rgb(97,197,84)' : 'var(--window-btn-unfocused-color)',
      border: 'var(--window-btn-border)',
    }"
    @click="$emit(buttonType)">
    <AppIcon v-if="showIcon" name="expand-2" :size="10" />
  </div>
  <div
    class="win-btn win-btn-close"
    v-else-if="buttonType === 'close'"
    @mouseenter="windowButtonHover = true"
    @mouseleave="windowButtonHover = false"
    :style="{
      background: computedLit ? 'rgb(255,95,87)' : 'var(--window-btn-unfocused-color)',
      border: 'var(--window-btn-border)',
    }"
    @click="$emit(buttonType)">
    <AppIcon v-if="showIcon" name="xmark" :size="10" />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { window as appWindow } from "@tauri-apps/api";
import { Event } from "@tauri-apps/api/event";

const props = defineProps<{
  buttonType: "minimize" | "maximize" | "close";
  lit?: boolean;
  hover?: boolean;
}>();
defineEmits(["minimize", "maximize", "close"]);
const windowButtonHover = ref(false);
const appWindowFocused = ref(true);

appWindow
  .getCurrentWindow()
  .isFocused()
  .then((focused) => {
    appWindowFocused.value = focused;
  });

appWindow.getCurrentWindow().onFocusChanged((event: Event<boolean>) => {
  appWindowFocused.value = event.payload;
});
const hovered = computed(() =>
  typeof props.hover === "boolean" ? props.hover : windowButtonHover.value,
);
const computedLit = computed(() => {
  if (typeof props.lit === "boolean") {
    return props.lit;
  } else {
    return hovered.value ? true : appWindowFocused.value;
  }
});
const showIcon = computed(() => hovered.value);
</script>

<style lang="less" scoped>
div.win-btn {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  margin-left: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform 100ms;

  &:active {
    opacity: 0.8;
  }
}
</style>
