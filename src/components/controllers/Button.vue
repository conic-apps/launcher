<template>
  <button
    class="button"
    :style="disabled ? 'opacity: 0.5; pointer-events: none;' : ''"
    v-bind="$attrs">
    {{ text }}
  </button>
</template>

<script setup lang="ts">
defineProps<{
  text: string;
  disabled?: boolean;
}>();
</script>

<style lang="less" scoped>
button.button {
  width: 100%;
  height: 30px;
  max-width: 130px;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: var(--border-radius-small);
  margin-left: auto;
}

button.button:hover {
  background: rgba(255, 255, 255, 0.1);
}

button.button:active {
  opacity: 0.8;
}
</style>
