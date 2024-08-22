<template>
  <div class="input-box" :style="`width: ${width};`">
    <input
      type="text"
      :title="name"
      :placeholder="placeholder"
      required
      v-model="model"
      :style="error ? 'outline: rgb(127,0,0)' : ''" />
  </div>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    name?: string;
    placeholder?: string;
    type?: string;
    error?: boolean;
    width?: string;
  }>(),
  {
    type: "text",
    width: "400px",
  },
);

const model = defineModel();
</script>

<style lang="less" scoped>
.input-box {
  border-radius: var(--border-radius-small);
  margin-left: auto;
  overflow: hidden;
  height: 30px;
  flex-shrink: 0;
  padding: 0;
  font-size: 15px;
  transition: all 0.1s ease;
  pointer-events: all;
  background-color: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.input-box input {
  border: none;
  background-color: #00000000;
  padding: 0;
  height: 100%;
  width: 100%;
  margin: 0;
  padding: 2px 8px;
  text-align: inherit;
}

.input-box:hover {
  background: rgba(255, 255, 255, 0.08);
}

.input-box:focus-within {
  outline: 1px solid #4493f8;
  background-color: rgba(255, 255, 255, 0.05);
}
</style>
