<template>
  <li class="select-option">{{ text }}</li>
</template>

<script setup lang="ts">
import { ref } from "vue";
defineProps<{
  text: string;
}>();
</script>

<style lang="less" scoped>
.select-option {
  padding: 6px 8px;
  border-radius: var(--border-radius-small);
  position: relative;
  z-index: 10001;
  transition: all 100ms ease;
}

.select-option:hover {
  background: #0101010b;
}

.select-option:active {
  background: #0000001f;
}
</style>
