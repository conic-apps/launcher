<template>
  <div class="search-bar" :style="style">
    <i class="magnifying-glass"></i>
    <input type="text" v-model="value" @input="$emit('search', value)" :placeholder="props.placeholder" />
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
let value = ref("")
const props = withDefaults(defineProps<{
  width?: string,
  height?: string,
  placeholder?: string
}>(), {
  width: '300px',
  height: '34px',
  placeholder: '搜索'
});
let style = `width: ${props.width}; height: ${props.height};`
// todo: 添加search in 属性，搜索用的算法全部在这个组件中进行
</script>

<style lang="less" scoped>
.search-bar {
  display: flex;
  align-items: center;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  padding: 0px 10px;
}

.search-bar i {
  font-family: 'fa-pro';
  font-style: normal;
  font-size: calc(12px - var(--font-size-error));
  margin-right: 6px;
  color: #808080;
  transform: scale(0.95);
}


.search-bar input {
  border: none;
  background: none;
  outline: none;
  font-size: 14px;
  color: rgba(255, 255, 255, 0.8);
  width: 100%;
  margin-left: 6px;
}

.search-bar input::placeholder {
  display: none;
}
</style>