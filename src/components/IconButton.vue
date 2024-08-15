<template>
  <i :class="props.icon" class="icon-btn"></i>
</template>

<script setup lang="ts">
const props = defineProps<{
  icon: string;
}>();
let classes = `${props.icon} icon-btn`;
</script>

<style lang="less" scoped>
.icon-btn {
  font-family: "fa-pro";
  font-style: normal;
  display: flex;
  align-items: center;
  pointer-events: all;
  transition: all 0.2s ease;
  font-size: 15px;
  margin: 0 6px;
  z-index: 1;
}

.icon-btn :hover {
  color: rgba(var(--theme-color), 1);
}

.icon-btn:active {
  transform: scale(0.86);
}
</style>
