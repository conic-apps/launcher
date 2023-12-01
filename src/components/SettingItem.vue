<template>
  <div class="setting-item" :style="cardStyle">
    <div class="title">
      <div class="icon"><i :class="icon"></i></div>
      <div>
        <h4 id="text">{{ title }}</h4>
        <p v-if="description" id="text">{{ description }}</p>
      </div>
    </div>
    <div>
      <slot></slot>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const props = withDefaults(defineProps<{
  title: string,
  description?: string,
  icon?: string,
  margin?: string,
  boxShadow?: boolean,
  padding?: string
  last?: boolean
}>(), {
  margin: '0,0,0,0',
  boxShadow: false,
  padding: '12,14,12,14',
  last: false
})
let margin = props.margin.split(',')
let padding = props.padding.split(',')
let cardStyle = `${props.boxShadow ? ' box-shadow: 0 0 10px #00000015;' : ''}margin: ${margin[0]}px ${margin[1]}px ${margin[2]}px ${margin[3]}px; padding: ${padding[0]}px ${padding[1]}px ${padding[2]}px ${padding[3]}px; ${props.last ? 'border-bottom: none; border-radius: 0 0 var(--border-radius-large) var(--border-radius-large)' : ''}`
</script>

<style lang="less" scoped>
.setting-item {
  background-color: var(--expander-background-color);
  border: 1px solid var(--expander-border-color);
  margin: 15px 0 15px 0;
  display: flex;
  justify-content: space-between;
  align-items: center;
  transition: all 0.1s ease;
  // border-radius: var(--border-radius-large);
  background-color: #f8f8f8;
  border-bottom: 1px solid #5000851d;
  margin: 15px 0 15px 0;
  transition: all 0.1s ease;
}


.dialog .setting-item {
  background-color: #ffffffb9;
  border: 1px solid #0000002e;
}

:active {
  opacity: 0.8;
}

.title {
  display: flex;
}

.icon {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  width: 2.3rem;
  height: 2.3rem;
  margin-right: 4px;
}

.icon i {
  font-family: 'fa-pro';
  font-style: normal;
  font-size: calc(24px - var(--font-size-error));
  font-weight: 500;
  margin: 0;
}

.title>div {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
}

.title>div:last-child {
  align-items: flex-start;
}

.title h4 {
  font-weight: normal;
  height: 20.5px;
  font-size: calc(15px - var(--font-size-error));
  line-height: 20.5px;
}

.title p {
  font-size: calc(12.5px - var(--font-size-error));
  color: var(--text-color);
  opacity: 0.6;
  margin-top: 4px;
}
</style>