<template>
  <li class="sidebar-item">
    <i class="fa-regular nav-icon" :id="icon"></i>
    <div class="text" v-if="title">{{ title }}</div>
    <!-- todo: button active style -->
  </li>
</template>

<script setup lang="ts">
defineProps<{
  title?: string;
  icon: string;
}>();
</script>

<style lang="less" scoped>
.sidebar-item {
  width: 58px;
  height: 56px;
  // color: rgba(255, 255, 255, 0.6);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  margin-top: 8px;
  border-radius: 10px;
  transition: all 100ms;
  background: var(--sidebar-item-background);
}

.sidebar-item:hover {
  background: var(--sidebar-item-background-hover);
  // border: 1px solid rgba(0, 0, 0, 0.06);
}

.sidebar-item:active {
  opacity: 0.6;
  transform: scale(0.97);
}

.sidebar-item .text {
  font-size: 12px;
  margin-top: 6px;
}

.sidebar-item i {
  transform: scale(1.2);
  font-family: "fa-pro";
  font-style: normal;
  height: 1em;
  width: 100%;
  text-align: center;
  // transition: all 0.2s cubic-bezier(0, 0.61, 0.16, 0.98);
}

// .sidebar-item:active .tooltip {
// opacity: 0;
// transform: scale(1);
// }
</style>
