<template>
  <div class="setting-group">
    <p v-if="title" style="margin-bottom: 10px; padding-left: 8px">{{ title }}</p>
    <div style="overflow: hidden; border-radius: 8px; margin-bottom: 16px">
      <slot></slot>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  title?: string;
}>();
</script>

<style>
.setting-group {
  margin: 0 auto;
  width: calc(100% - 16px);
}
</style>
