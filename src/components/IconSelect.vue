<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="icon-select">
    <div
      class="option"
      v-for="(option, index) in props.options"
      :class="
        (option === model ? 'activated' : '') +
        (props.disabled.find((v) => v == option) ? ' disabled' : '')
      "
      :key="option"
      @click="changeValue(option)">
      <i :class="props.icons[index]"></i>
    </div>
  </div>
</template>

<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    options: string[];
    icons: string[];
    disabled?: string[];
  }>(),
  {
    disabled: () => {
      return [];
    },
  },
);

type ModelType = (typeof props.options)[number];
const model = defineModel<ModelType>();
const changeValue = (option: string) => {
  model.value = option;
};
</script>

<style lang="less">
.icon-select {
  display: flex;

  border: var(--controllers-border);
  background: var(--controllers-background);
  border-radius: var(--controllers-border-radius);
  overflow: hidden;

  .option {
    border-right: var(--controllers-border);
    width: 36px;
    height: 32px;
    display: flex;
    justify-content: center;
    align-items: center;
    transition: background-color 0.1s ease;

    i {
      width: 20px;
      height: 20px;
      font-style: normal;
      display: flex;
      justify-content: center;
      align-items: center;
    }
  }

  .disabled {
    opacity: 0.4;
    pointer-events: none;
  }

  > div.option:last-child {
    border-right: none;
  }

  .activated {
    background-color: rgba(var(--theme-color), 0.7);
  }

  .option:active {
    opacity: 0.8;
  }
}
</style>
