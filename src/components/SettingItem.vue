<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div
    class="setting-item"
    :class="{ 'setting-item-navigable': props.navigable, 'setting-item-disabled': props.disabled }">
    <div style="display: flex">
      <div class="icon" v-if="icon">
        <AppIcon :name="icon" :size="iconSize" :fill="iconFill"></AppIcon>
      </div>
      <div class="text">
        <p class="title">{{ title }}</p>
        <p
          v-if="description"
          class="description"
          style="max-width: 560px; line-height: 1.5"
          v-html="description"></p>
      </div>
    </div>
    <div style="display: flex; align-items: center">
      <AppIcon
        name="chevron-forward"
        style="margin-right: 4px"
        v-if="props.navigable"
        :size="17"></AppIcon>
      <slot v-else></slot>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "./AppIcon.vue";

const props = withDefaults(
  defineProps<{
    title: string;
    description?: string;
    icon?: string;
    iconSize?: number | string;
    iconFill?: string;
    navigable?: boolean;
    disabled?: boolean;
  }>(),
  {
    iconSize: 26,
  },
);
</script>

<style lang="less" scoped>
.setting-item {
  padding: 10px 16px 10px 14px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin: 0;
  margin-bottom: 1px;
  background: var(--setting-item-background);
  transition: all 50ms ease;

  > * {
    transition: all 100ms ease;
  }

  .icon,
  .text {
    display: flex;
    flex-direction: column;
    justify-content: center;
  }

  .icon {
    align-items: center;
    width: 40px;
    height: inherit;
    margin-right: 8px;
    flex-shrink: 0;
  }

  .text {
    align-items: flex-start;
  }

  .text p.title {
    font-weight: normal;
    font-size: 13px;
  }

  .text p.description {
    font-size: 12px;
    color: rgba(var(--default-text-color), 0.849);
    opacity: 0.6;
    margin-top: 4px;
  }
}

.setting-item-navigable:hover {
  background: var(--setting-item-background-hover);
}

.setting-item-navigable:active {
  background-color: var(--setting-item-background-active);
}

.setting-item-disabled {
  > * {
    opacity: 0.6;
  }

  pointer-events: none;
}
</style>
