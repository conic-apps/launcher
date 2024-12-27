<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="tip-bar" :style="style">
    <slot></slot>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
const props = defineProps<{
  type?: "seccess" | "danger" | "attention" | "accent";
}>();
let style = computed(() => {
  let borderStyle;
  let backgroundStyle;
  let textStyle;
  if (props.type === "seccess") {
    borderStyle = "border: 1px solid var(--color-success-muted);";
    backgroundStyle = "background: var(--color-success-subtle)";
    textStyle = `color: var(--color-seccess-emphasis);`;
  } else if (props.type === "danger") {
    borderStyle = "border: 1px solid var(--color-danger-muted);";
    backgroundStyle = "background: var(--color-danger-subtle)";
    textStyle = `color: var(--color-danger-emphasis);`;
  } else if (props.type === "attention") {
    borderStyle = "border: 1px solid var(--color-attention-muted);";
    backgroundStyle = "background: var(--color-attention-subtle)";
    textStyle = `color: var(--color-attention-emphasis);`;
  } else {
    borderStyle = "border: 1px solid var(--color-accent-muted);";
    backgroundStyle = "background: var(--color-accent-subtle)";
    textStyle = `color: var(--color-accent-emphasis);`;
  }
  return `
    border-radius: var(--border-radius-small);
    width: 100%;
    padding: 16px 20px;
    ${borderStyle}
    ${backgroundStyle}
  `;
});
</script>

<style lang="less" scoped>
.tip-bar {
  width: 100%;
}
</style>
