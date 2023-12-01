<template>
  <button class="button" v-bind="$attrs">{{ text }}</button>
</template>

<script setup lang="ts">
defineProps<{
  text: string
}>()
</script>

<style lang="less" scoped>
button.button {
  width: 100%;
  height: 30px;
  max-width: 130px;
  background: #f7f7f7;
  box-shadow: 0 0 0 1px rgba(var(--theme-color), 0.2);
  border: none;
  border-radius: var(--border-radius-small);
  transition: all .1s ease;
  pointer-events: all;
  margin-left: auto;
}

button.button:hover {
  background: #ffffffce;
}

button.button:active {
  transform: scale(0.98);
  background-color: rgba(var(--theme-color), 0.1);
}
</style>