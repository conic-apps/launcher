<template>
  <div class="tip-bar" :style="style">
    <slot></slot>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
const props = defineProps<{
  type?: 'seccess' | 'fail' | 'warn' | 'accent'
}>()
let style = computed(() => {
  let borderStyle
  let backgroundStyle
  if (props.type === 'seccess') {
    borderStyle = 'border: 1px solid var(--color-success-muted);'
    backgroundStyle = 'background: var(--color-success-subtle)'
  } else if (props.type === 'fail') {
    borderStyle = 'border: 1px solid var(--color-fail-muted);'
    backgroundStyle = 'background: var(--color-fail-subtle)'
  } else if (props.type === 'warn') {
    borderStyle = 'border: 1px solid var(--color-warn-muted);'
    backgroundStyle = 'background: var(--color-warn-subtle)'
  } else {
    borderStyle = 'border: 1px solid var(--color-accent-muted);'
    backgroundStyle = 'background: var(--color-accent-subtle)'
  }
  return `
    border-radius: var(--border-radius-small);
    width: 100%;
    padding: 12px 16px;
    ${borderStyle}
    ${backgroundStyle}
  `
})

</script>

<style lang="less" scoped>
.tip-bar {
  width: 100%
}
</style>