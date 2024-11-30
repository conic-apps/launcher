<template>
  <div class="setting-group" :class="danger ? 'danger-setting-group' : ''">
    <p v-if="title" style="margin-bottom: 10px; padding-left: 8px">{{ title }}</p>
    <div style="overflow: hidden; border-radius: 8px; margin-bottom: 16px">
      <slot></slot>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  title?: string;
  danger?: boolean;
}>();
</script>

<style lang="less">
.setting-group {
  margin: 0 auto;
  width: calc(100% - 16px);
}

.danger-setting-group {
  > div {
    border: 1px solid rgba(210, 15, 57, 0.6);
  }
}
</style>
