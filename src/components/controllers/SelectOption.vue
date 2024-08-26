<template>
  <li class="select-option">{{ text }}</li>
</template>

<script setup lang="ts">
defineProps<{
  text: string;
}>();
</script>

<style lang="less" scoped>
.select-option {
  padding: 10px 16px;
  border-radius: var(--controllers-border-radius);
  position: relative;
  z-index: 10001;
  transition: all 30ms ease;
}

.select-option:hover {
  background: #ffffff1f;
}

.select-option:active {
  background: #ffffff15;
}
</style>
