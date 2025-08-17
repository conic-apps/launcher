<template>
  <div class="info" :class="clickAble ? 'click-able' : ''">
    <slot></slot>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  clickAble?: boolean;
}>();
</script>

<style lang="less" scoped>
.info {
  width: calc(100% - 16px);
  margin: 0 auto;
  margin-bottom: 16px;
  padding: 20px;
  border-radius: 10px;
  line-height: 1.6;
  background: var(--card-background);
  font-size: 14px;
  transition: all 150ms ease;
  border: var(--card-border);
}
.click-able:hover {
  transform: scale(1.01);
}

.click-able:active {
  transform: scale(0.98);
  opacity: 0.8;
}
</style>
