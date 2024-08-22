<template>
  <div style="margin: 0 auto; width: calc(100% - 16px)">
    <p v-if="title" style="margin-bottom: 10px; padding-left: 8px">{{ title }}</p>
    <div
      style="
        overflow: hidden;
        border-radius: 8px;
        background: rgba(255, 255, 255, 0.08);
        margin-bottom: 16px;
      ">
      <slot></slot>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  title?: string;
}>();
</script>

<style></style>
