<template>
  <Transition name="dialog">
    <div v-if="visible" class="dialog" data-tauri-drag-region>
      <div class="content" :style="contentStyle">
        <slot></slot>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    visible?: boolean
    width?: string
    height?: string
  }>(),
  {
    visible: false,
    width: "",
    height: "",
  },
)
const contentStyle = `width: ${props.width}px; height: ${props.height}px;`
</script>

<style lang="less" scoped>
.dialog {
  display: flex;
  justify-content: center;
  align-items: center;
  position: fixed;
  top: 0;
  left: 0;
  z-index: 11451419;
  width: 100%;
  height: 100%;
  background-color: #00000042;
  border-radius: 16px;
}

.dialog-enter-from,
.dialog-leave-to {
  opacity: 0;
}

.dialog-enter-to,
.dialog-leave-from {
  opacity: 1;
}

.dialog-enter-active,
.dialog-leave-active {
  transition: all 200ms ease;
}

.dialog-leave-active {
  transition-delay: 100ms;
}

.dialog-enter-from .content,
.dialog-leave-to .content {
  transform: scale(1.1);
  opacity: 0;
}

.dialog-enter-active .content {
  transition-delay: 200ms;
  transition: all 300ms cubic-bezier(0, 0.47, 0.25, 1);
}

.dialog-leave-active .content {
  transition: all 180ms cubic-bezier(0.47, 0, 1, 0.75);
}

.dialog-enter-to,
.dialog-leave-from {
  transform: scale(1);
  opacity: 1;
}

.content {
  padding: 16px;
  background-color: #2c2c2c;
  border: 1px solid #ffffff0f;
  box-shadow: 0 0 50px 0px #00000071;
  width: fit-content;
  height: fit-content;
  border-radius: var(--border-radius-large);
  max-width: calc(100vw - 20px);
  max-height: calc(100vh - 20px);
  overflow-x: visible;
  overflow-y: overlay;
}
</style>
