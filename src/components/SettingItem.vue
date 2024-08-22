<template>
  <div class="setting-item" :style="clickAble ? '' : 'background-color: #08080800 !important;'">
    <div class="title">
      <div class="icon" v-if="icon"><i :class="icon"></i></div>
      <div>
        <h4 id="text">{{ title }}</h4>
        <p v-if="description" id="text" style="max-width: 560px" v-html="description"></p>
      </div>
    </div>
    <div>
      <slot></slot>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";

const props = withDefaults(
  defineProps<{
    title: string;
    description?: string;
    icon?: string;
    boxShadow?: boolean;
    clickAble?: boolean;
  }>(),
  {
    boxShadow: false,
    last: false,
    clickAble: false,
  },
);
</script>

<style lang="less" scoped>
.setting-item {
  padding: 16px 18px 16px 14px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  transition: all 0.1s ease;
  // border-radius: var(--border-radius-large);
  background-color: #08080800;
  border-bottom: 1px solid #00000079;
  margin: 0;
  transition: all 50ms ease;
}

.setting-item:hover {
  background-color: #ffffff0c;
}

.setting-item:active {
  background-color: #ffffff1f;
}

.dialog .setting-item {
  background-color: #000000b9;
  border: 1px solid #0000002e;
}

.title {
  display: flex;
}

.icon {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  width: 40px;
  height: inherit;
  margin-right: 8px;
}

.icon i {
  font-family: "fa-pro";
  font-style: normal;
  font-size: 22px;
  font-weight: 500;
  margin: 0;
}

.title > div {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
}

.title > div:last-child {
  align-items: flex-start;
}

.title h4 {
  font-weight: normal;
  height: 20.5px;
  font-size: 15px;
  line-height: 20.5px;
}

.title p {
  font-size: 12.5px;
  color: rgba(255, 255, 255, 0.849);
  opacity: 0.6;
  margin-top: 4px;
  line-height: 1.1;
}
</style>
