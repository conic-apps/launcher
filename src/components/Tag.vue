<template>
  <div class="tag" :style="`${border ? `border: 1px solid rgb(${color[0]}, ${color[1]}, ${color[2]}); ` : ''}
      ${background ? `background: rgba(${color[0]}, ${color[1]}, ${color[2]}, 0.2);` : ''} 
      color: ${textColor}; font-size: calc(${fontSize}px - var(--font-size-error));
      ${round ? 'border-radius: 10000px' : ''}`">
    {{ text }}
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const props = withDefaults(defineProps<{
  text: string,
  color: string[],
  border?: boolean,
  background?: boolean,
  textColor?: string,
  fontSize?: string,
  round?: boolean
}>(), {
  fontSize: '11'
})
</script>

<style lang="less" scoped>
.tag {
  margin-right: 2px;
  width: fit-content;
  height: fit-content;
  font-size: calc(11px - var(--font-size-error));
  padding: 2px 4px;
  border-radius: var(--border-radius-small);
}
</style>
