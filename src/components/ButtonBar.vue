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

button.command-button {
  width: 100%;
  height: 30px;
  max-width: 130px;
  background: #f7f7f7;
  box-shadow: 0 0 0 1px rgba(var(--theme-color), 0.2);
  border: none;
  border-radius: var(--border-radius-medium);
  transition: all .1s ease;
  pointer-events: all;
  margin-left: auto;
}
button.command-button:hover {
  background: rgba(var(--theme-color), 0.05);
}
button.command-button:active {
  transform: scale(0.98);
  background-color: rgba(var(--theme-color), 0.1);
}
</style>