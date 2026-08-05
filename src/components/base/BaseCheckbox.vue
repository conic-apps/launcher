<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <label class="base-checkbox" :class="{ disabled: props.disabled }">
    <input type="checkbox" v-model="model" :disabled="props.disabled" />
    <span class="checkmark"></span>
  </label>
</template>

<script setup lang="ts">
const model = defineModel<boolean>({ default: false });

const props = defineProps<{
  disabled?: boolean;
}>();
</script>

<style lang="less" scoped>
.base-checkbox {
  display: inline-flex;
  align-items: center;
  cursor: pointer;
  position: relative;
  user-select: none;

  input {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
  }

  .checkmark {
    width: 18px;
    height: 18px;
    border-radius: 4px;
    border: 1.5px solid var(--ctp-overlay0);
    background: transparent;
    transition: all 0.15s ease;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .checkmark::after {
    content: "";
    width: 5px;
    height: 9px;
    border: solid #fff;
    border-width: 0 2px 2px 0;
    transform: rotate(45deg);
    opacity: 0;
    transition: opacity 0.1s ease;
    margin-bottom: 1px;
  }

  input:checked ~ .checkmark {
    background: var(--ctp-latte-lavender);
    border-color: var(--ctp-latte-lavender);
  }

  input:checked ~ .checkmark::after {
    opacity: 1;
  }

  &:hover .checkmark {
    border-color: var(--ctp-subtext0);
  }

  &.disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }
}
</style>
