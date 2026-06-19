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
    <p v-if="title" style="margin-bottom: 10px; padding-left: 8px">{{ title }}</p>
    <div style="overflow: hidden; border-radius: 8px; margin-bottom: 16px">
      <slot></slot>
    </div>
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{
  title?: string;
  danger?: boolean;
  disabled?: boolean;
}>();
</script>

<style lang="less" scoped>
.setting-group {
  margin: 0 auto;
  width: calc(100% - 16px);
  opacity: 1;
  > div {
    border: var(--setting-group-border);
  }
}

.danger-setting-group {
  > div {
    border: 1px solid rgba(210, 15, 57, 0.6);
  }
}

.setting-group-disabled {
  opacity: 0.6;

  * {
    pointer-events: none;
  }
}
</style>
