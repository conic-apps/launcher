<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div
    class="setting-group"
    :class="{
      'danger-setting-group': props.danger,
      'setting-group-disabled': props.disabled,
    }">
    <p v-if="title" class="title">
      {{ title }}
      <button v-if="props.resetable" class="reset-button">
        <AppIcon name="refresh" :size="16"></AppIcon>
      </button>
    </p>
    <div class="setting-items">
      <slot></slot>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "./AppIcon.vue";

const props = defineProps<{
  title?: string;
  danger?: boolean;
  disabled?: boolean;
  resetable?: boolean;
}>();

const emits = defineEmits(["reset"]);
</script>

<style lang="less">
.setting-group {
  margin: 0 auto;
  width: calc(100% - 16px);
  opacity: 1;

  > p.title {
    margin-bottom: 10px;
    padding-left: 8px;
    font-size: 13.5px;
    display: flex;
    align-items: center;
  }

  div.setting-items {
    border: var(--setting-group-border);
    margin-bottom: 16px;
    border-radius: 8px;

    > div:first-child {
      border-top-left-radius: 8px;
      border-top-right-radius: 8px;
    }

    > div:last-child {
      border-bottom-left-radius: 8px;
      border-bottom-right-radius: 8px;
    }
  }
}

button.reset-button {
  display: inline-flex;
  justify-content: center;
  align-items: center;
  width: 20px;
  height: 20px;
  border-radius: 3px;
  margin-left: 3px;
  appearance: none;
  border: none;
  background: none;
}

button.reset-button:hover {
  background: #ffffff1c;
}

button.reset-button:active {
  opacity: 0.8;
}

.danger-setting-group > div {
  border: var(--setting-group-danger-border);
}

.setting-group-disabled {
  opacity: 0.6;

  * {
    pointer-events: none;
  }
}
</style>
