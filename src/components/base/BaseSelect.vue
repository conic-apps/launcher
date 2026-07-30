<!-- Conic Launcher -->
<!-- Copyright 2022-2026 OakChaser and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="base-select">
    <div
      class="option"
      v-for="(option, index) in props.options"
      :class="{
        activated: option === model,
        disabled: props.disabled.find((v) => v == option),
      }"
      :key="option"
      @click="changeValue(option)">
      {{ displayName[index] }}
    </div>
  </div>
</template>

<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    options: string[];
    disabled?: string[];
    displayName: string[];
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

<style lang="less" scoped>
.base-select {
  display: flex;

  border: var(--controllers-border);
  background: var(--controllers-background);
  border-radius: var(--controllers-border-radius);
  overflow: hidden;

  .option {
    border-right: var(--controllers-border);
    height: 22px;
    padding: 0 8px;
    display: flex;
    justify-content: center;
    align-items: center;
    transition: background-color 0.1s ease;
    font-size: 12px;
  }

  .disabled {
    opacity: 0.4;
    pointer-events: none;
  }

  > div.option:last-child {
    border-right: none;
  }

  .activated {
    background-color: var(--ctp-lavender);
    color: var(--ctp-text-inverse);
  }

  .option:active {
    opacity: 0.8;
  }
}
</style>
