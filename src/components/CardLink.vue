<template>
  <div class="card-link" :style="cardStyle">
    <div class="title">
      <div class="icon"><i :class="icon"></i></div>
      <div>
        <h4 id="text">{{ title }}</h4>
        <p id="text">{{ description }}</p>
      </div>
    </div>
    <div style="width: 36px;height: 36px;display: flex;align-items: center;justify-content: center;"><i
        class="chevron-right"></i></div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const props = withDefaults(defineProps<{
  title: string,
  description: string,
  icon: string,
  margin?: string,
  boxShadow?: boolean,
  padding?: string
}>(), {
  margin: '',
  boxShadow: true,
  padding: '16,18,16,18'
})
let margin = props.margin.split(',')
let padding = props.padding.split(',')
let cardStyle = `${props.boxShadow ? ' box-shadow: 0 0 10px #00000015;' : ''}margin: ${margin[0]}px ${margin[1]}px ${margin[2]}px ${margin[3]}px; padding: ${padding[0]}px ${padding[1]}px ${padding[2]}px ${padding[3]}px;`
</script>

<style lang="less" scoped>
.card-link {
  border-radius: var(--border-radius-large);
  background-color: var(--expander-background-color);
  border: 1px solid var(--expander-border-color);
  margin: 15px 0 15px 0;
  display: flex;
  justify-content: space-between;
  align-items: center;
  transition: all 0.1s ease;
  border-radius: var(--border-radius-large);
  background-color: #ffffff77;
  border: 1px solid #ffffff6c;
  margin: 15px 0 15px 0;
  transition: all 0.1s ease;
}


.dialog .card-link {
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
  width: 2rem;
  height: 2em;
  margin-right: 6px;
}

.icon i {
  font-family: 'fa-pro';
  font-style: normal;
  font-size: calc(23px - var(--font-size-error));
  font-weight: 500;
}

.title h4 {
  font-weight: 100;
  height: 20.5px;

}

.title p {
  font-size: calc(12.5px - var(--font-size-error));
  color: var(--text-color);
  opacity: 0.6;
  margin: 0;
}

i.chevron-right {
  transition: transform .2s ease;
}

.card-link:hover i.chevron-right {
  transform: translate(4px, 0px);
}
</style>