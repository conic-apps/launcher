<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="instances-drawer" ref="drawerRef">
    <div
      class="instance"
      :class="{ active: instance.id === instanceStore.currentInstance.id }"
      v-for="instance in visibleInstances"
      :key="instance.id"
      @click="instanceStore.currentInstance = instance">
      <div class="tooltip">{{ instance.config.name }}</div>
      <img :src="instance.config.icon ?? instanceIconFallback" />
    </div>
    <button class="show-all" v-if="hasMore">
      <AppIcon name="apps-outline" :size="24" />
      <p class="name">{{ $t("home.showAll") }}</p>
    </button>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import { useInstanceStore } from "@/store/instance";
import instanceIconFallback from "@/assets/images/Unknown_server.webp";
import { ref, computed, onMounted, onBeforeUnmount } from "vue";

const instanceStore = useInstanceStore();
const drawerRef = ref<HTMLDivElement>();

const TILE_WIDTH = 52;
const BUTTON_WIDTH = 72;
const PADDING = 40;
const SIDE_MARGIN = 32;

const parentWidth = ref(0);

const maxVisible = computed(() => {
  if (parentWidth.value === 0) return instanceStore.instances.length;
  const available = parentWidth.value - SIDE_MARGIN - PADDING - BUTTON_WIDTH;
  return Math.max(1, Math.floor(available / TILE_WIDTH));
});

const visibleInstances = computed(() => {
  return instanceStore.instances.slice(0, maxVisible.value);
});

const hasMore = computed(() => {
  return instanceStore.instances.length > maxVisible.value;
});

let resizeObserver: ResizeObserver | null = null;

onMounted(() => {
  const parent = drawerRef.value?.parentElement;
  if (parent) {
    parentWidth.value = parent.clientWidth;
    resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        parentWidth.value = entry.contentRect.width;
      }
    });
    resizeObserver.observe(parent);
  }
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
});
</script>

<style lang="less" scoped>
.instances-drawer {
  position: absolute;
  bottom: 16px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  gap: 8px;
  padding: 12px 20px;
  background: var(--card-background);
  border: var(--card-border);
  border-radius: var(--card-border-radius);

  .instance {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 44px;
    height: 44px;
    justify-content: center;
    border-radius: var(--card-border-radius);
    border: 2px solid transparent;
    cursor: pointer;
    transition: border-color 0.2s ease;
    flex-shrink: 0;

    &:active {
      opacity: 0.8;
    }

    &::after {
      content: "";
      position: absolute;
      bottom: -6px;
      left: 50%;
      transform: translateX(-50%) scaleX(0);
      width: 16px;
      height: 3px;
      background: var(--instance-drawer-active);
      border-radius: 2px;
      transition: transform 0.2s ease;
    }

    &.active::after {
      transform: translateX(-50%) scaleX(1);
    }

    img {
      width: 100%;
      height: 100%;
      border-radius: var(--card-border-radius);
    }

    .tooltip {
      position: absolute;
      bottom: calc(100% + 8px);
      left: 50%;
      transform: translateX(-50%) translateY(4px);
      padding: 4px 10px;
      background: var(--card-background);
      color: rgb(var(--default-text-color));
      border: 1px solid rgba(var(--ctp-mocha-overlay2-rgb), 0.5);
      border-radius: var(--tag-border-radius);
      font-size: 12px;
      white-space: nowrap;
      pointer-events: none;
      opacity: 0;
      transition: opacity 0.1s ease, transform 0.1s ease;
    }

    &:hover .tooltip {
      opacity: 1;
      transform: translateX(-50%) translateY(0);
    }
  }

  .show-all {
    appearance: none;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    width: 64px;
    padding: 8px 4px;
    border-radius: var(--card-border-radius);
    border: 2px solid transparent;
    background: none;
    cursor: pointer;
    flex-shrink: 0;
    transition: background-color 0.2s ease;

    &:hover {
      background-color: var(--instance-drawer-hover);
    }

    &:active {
      opacity: 0.8;
    }

    .name {
      margin-top: 6px;
      font-size: 11px;
      max-width: 64px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      text-align: center;
    }
  }
}
</style>
