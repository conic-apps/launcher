<template>
  <div class="tag" :style="`${border ? `border: 1px solid rgb(${color[0]}, ${color[1]}, ${color[2]}); ` : ''}
    ${background ? `background: rgba(${color[0]}, ${color[1]}, ${color[2]}, 0.4);` : ''} 
    color: ${textColor}; font-size: ${fontSize}px;
    ${round ? 'border-radius: 10000px' : ''}`">
    {{ text }}
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const props = defineProps<{
  text: string,
  color: string[],
  border?: boolean,
  background?: boolean,
  textColor?: string,
  fontSize?: string,
  round?: boolean
}>()
</script>

<style lang="less" scoped>
.tag {
  width: fit-content;
  height: fit-content;
  font-size: 11px;
  padding: 2px 4px;
  border-radius: var(--border-radius-small);
}
</style>
