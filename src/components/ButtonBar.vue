<template>
  <div class="controller">
    <span class="name">{{ name }}</span>
    <button class="command-button" v-bind="$attrs">{{ text }}</button>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  name: string,
  text: string,
}>()
</script>

<style lang="less" scoped>
.controller {
  display: flex;
  margin-bottom: 10px;
}


</style>