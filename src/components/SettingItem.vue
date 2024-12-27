<!-- Amethyst Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="setting-item" :class="className">
    <div class="title">
      <div class="icon" v-if="icon"><i :class="icon"></i></div>
      <div>
        <h4 id="text">{{ title }}</h4>
        <p v-if="description" id="text" style="max-width: 560px" v-html="description"></p>
      </div>
    </div>
    <div style="display: flex; align-items: center">
      <slot></slot>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    title: string;
    description?: string;
    icon?: string;
    boxShadow?: boolean;
    clickAble?: boolean;
    disabled?: boolean;
  }>(),
  {
    boxShadow: false,
    last: false,
    clickAble: false,
    disabled: false,
  },
);

const className = computed(() => {
  let result = "";
  if (props.clickAble) {
    result += "setting-item-click-able";
  }
  if (props.disabled) {
    result += " setting-item-disabled";
  }
  return result;
});
</script>

<style lang="less" scoped>
.setting-item {
  padding: 16px 18px 16px 14px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  transition: all 0.1s ease;
  // border-bottom: 1px solid #00000079;
  margin: 0;
  margin-bottom: 1px;
  background: var(--setting-item-background);

  transition: all 50ms ease;

  > * {
    transition: all 100ms ease;
  }

  .title {
    display: flex;
  }

  .icon {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    width: 40px;
    height: inherit;
    margin-right: 8px;
  }

  .icon i {
    font-family: "fa-pro";
    font-style: normal;
    font-size: 22px;
    font-weight: 500;
    margin: 0;
  }

  .title > div {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
  }

  .title > div:last-child {
    align-items: flex-start;
  }

  .title h4 {
    font-weight: normal;
    height: 20.5px;
    font-size: 15px;
    line-height: 20.5px;
  }

  .title p {
    font-size: 12.5px;
    color: rgba(var(--default-text-color), 0.849);
    opacity: 0.6;
    margin-top: 4px;
    line-height: 1.1;
  }
}

.setting-item-click-able:hover {
  background: var(--setting-item-background-hover);
}

.setting-item-click-able:active {
  background-color: var(--setting-item-background);
}

.setting-item-disabled {
  > * {
    opacity: 0.6;
  }

  pointer-events: none;
}
</style>
