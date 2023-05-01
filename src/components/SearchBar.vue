<template>
  <div class="search-bar ">
    <i class="magnifying-glass"></i>
    <input type="text" v-model="value" @input="$emit('search', value)" placeholder="按名称搜索"/>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
let value = ref("")
</script>

<style lang="less" scoped>
.search-bar {
  border-radius: var(--border-radius-small);
  margin-left: auto;
  overflow: hidden;
  box-shadow: 0 0 0 1px rgba(var(--theme-color), 0.2);
  height: 30px;
  padding: 0 8px 2px 8px;
  font-size: 1rem;
  transition: all 0.1s ease;
  pointer-events: all;
  background: rgba(255, 255, 255, 0.658);
  display: flex;
  align-items: center;
  margin-bottom: 4px;
}

.search-bar i {
  font-family: 'fa-pro';
  font-style: normal;
  font-size: 14px;
  margin-right: 6px;
}

.search-bar:hover i {
  color: rgba(var(--theme-color), 1);
}

.search-bar div.input {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.6rem;
  position: relative;
  height: 32px;
  pointer-events: none;
}

.search-bar input {
  border: none;
  outline: none;
  border-bottom: 2px #000000;
  background: none;
  padding: 0;
  height: 100%;
  width: 100%;
  text-align: inherit;
  z-index: 10;
}

.search-bar:hover {
  background: #fff;
}

.underline {
  background: rgba(var(--theme-color), 1);
  margin-left: -8px;
  height: 2px;
  width: calc(100% + 16px);
  transform: scale(0, 1);
  opacity: 0;
  border-radius: var(--border-radius-medium);
  transition: transform 0.3s ease, opacity 0.3s ease;
}

input:focus~.underline {
  transform: scale(1);
  opacity: 1;
  transition: transform 0.3s ease, opacity 0.3s ease;
}

input:focus::-webkit-input-placeholder {
  color: #00000000;
}
</style>