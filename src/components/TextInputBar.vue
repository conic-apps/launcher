<template>
  <div class="input input-text">
    <span class="name">{{ name }}</span>
    <div class="input-box">
      <input type="text" :title="name" :placeholder="placeholder" @blur="updateData" required v-model="text" />
      <div class="underline"></div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { load, update } from '@/utils/ConfigLoader';
import { ref, type Ref } from 'vue'

const props = defineProps<{
  name: string,
  placeholder?: string,
  // config: string
}>()
let text: Ref<string> = ref("")
function updateData(): void {
  // update(props.config, text)
}
</script>

<style lang="less" scoped>
div.input {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.6rem;
  position: relative;
  height: 32px;
  pointer-events: none;
}

.input-text input {
  border: none;
  outline: none;
  border-bottom: 2px #000000;
  background: none;
  padding: 0;
  height: 100%;
  width: 100%;
  text-align: inherit;
  z-index: 10;
  font-size: calc(13px - var(--font-size-error));
}

.input-text .underline {
  background: rgba(var(--theme-color), 1);
  margin-left: -8px;
  height: 2px;
  width: calc(100% + 16px);
  transform: scale(0, 1);
  opacity: 0;
  border-radius: var(--border-radius-small);
  transition: transform 0.3s ease, opacity 0.3s ease;
}

.input-text input:focus~.underline {
  transform: scale(1);
  opacity: 1;
  transition: transform 0.3s ease, opacity 0.3s ease;
}

.input-box {
  border-radius: var(--border-radius-small);
  margin-left: auto;
  width: 400px;
  overflow: hidden;
  box-shadow: 0 0 0 1px rgba(var(--theme-color), 0.2);
  height: 30px;
  flex-shrink: 0;
  padding: 0 8px 2px 8px;
  font-size: calc(16px - var(--font-size-error));
  transition: all 0.2s ease;
  pointer-events: all;
  background: rgba(255, 255, 255, 0.2);
}

.input-box:hover {
  background: #ffffffce;
}

input:focus::-webkit-input-placeholder {
  color: #00000000;
}
</style>