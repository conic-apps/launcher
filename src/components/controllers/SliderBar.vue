<template>
  <div class="input input-text input-slider">
    <span class="name">{{ name }}</span>
    <div style="display: flex; line-height: 1.7;width: 100%;justify-content: flex-end;">
      <div class="slider" ref="slider">
        <div :style="orbit"></div>
        <input ref="element" type="range" :max="max" :min="min" :step="step" v-model="value" @blur="onBlur">
      </div>
      <div class="input-data mini" style="margin-right: 0;">
        <input type="number" :title="name" required v-model="value" placeholder="默认" @blur="onBlur" />
        <div class="underline"></div>
      </div>
      <span class="text">{{ text }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, type ComputedRef, type Ref, onMounted } from 'vue'
import TextInputBox from './TextInputBox.vue'
import $ from 'jquery'
const props = withDefaults(defineProps<{
  name: string,
  config?: string,
  max: string,
  min: string,
  step: string,
  text?: string,
  AllowExceeding?: boolean,
}>(), {
  AllowExceeding: false
})
const slider = ref<any>(null)
onMounted(() => {
  console.log()
})
// let value: Ref<number> = ref(await load(props.config))
let value = ref(Number(props.min))
function setValue(newValue: number) {
  value.value = newValue
}
let orbit: ComputedRef<string> = computed((): string => {
  const min = Number(props.min)
  const max = Number(props.max)
  const sliderOuterWidth = $(slider.value).outerWidth(true)
  let exceeded = value.value > max - min + 1
  if (exceeded && props.AllowExceeding == false) {
    setValue(max)
  }
  let lessThanMinimum = value.value - 1 - min < 0
  if (lessThanMinimum) {
    return 'width: 0px;';
  } else {
    return `width: ${(value.value - 1 - min) / (max - min) * (sliderOuterWidth! - 20) + 10}px;`
  }
})

function onBlur(): void {
  if (!/^\d+$/.test(String(value.value))) {
    value.value = Number(props.min)
  }
  if (value.value - 1 - Number(props.min) < 0) {
    value.value = Number(props.min)
  }
  // update(props.config, value.value)
}
</script>

<style lang="less" scoped>
.input-slider {
  display: flex;
  margin-bottom: 10px;
}

.slider {
  width: 100%;
  display: flex;
  align-items: center;
  float: right;
  position: relative;
}

.slider>* {
  position: absolute;
}

.slider>div {
  background: rgba(var(--theme-color), 1);
  height: 3.5px;
  width: 4px;
  /* min-width: 4px; */
  border-radius: 10px;
  position: absolute;
  pointer-events: none;
}

.slider>div.slider_btn>div {
  width: 53%;
  height: 53%;
  background: rgba(var(--theme-color), 1);
  border-radius: 100%;
  transition: all 0.2s ease;
}

.slider input[type=range] {
  appearance: none;
  outline: none;
  width: 100%;
  height: 100%;
  background: #00000000;
  border-radius: 100px;
  box-sizing: content-box;
  pointer-events: all;
}

.slider input[type=range]::-webkit-slider-thumb {
  appearance: none;
  width: 20px;
  height: 20px;
  margin-top: -8px;
  border-radius: 100px;
  background: rgba(var(--theme-color), 1);
  box-shadow: inset 0 0 0 5px #ffffff;
  pointer-events: all;
  transition: all .2s ease;
}

.slider input[type=range]::-webkit-slider-thumb:hover {
  box-shadow: inset 0 0 0 3.7px #ffffff;
}

.slider input[type=range]:active::-webkit-slider-thumb {
  box-shadow: inset 0 0 0 6px #ffffff;
}

.slider input[type=range]::-webkit-slider-runnable-track {
  appearance: none;
  height: 3.5px;
  border-radius: 10px;
  background-color: #00000045
}

input[type=number]::-webkit-inner-spin-button,
input[type=number]::-webkit-outer-spin-button {
  appearance: none;
  margin: 0;
}


.input-data {
  border-radius: var(--border-radius-small);
  width: 400px;
  overflow: hidden;
  box-shadow: 0 0 0 1px rgba(var(--theme-color), 0.2);
  height: 30px;
  flex-shrink: 0;
  padding: 0 8px 2px 8px;
  font-size: calc(16px - var(--font-size-error));
  transition: all 0.1s ease;
  pointer-events: all;
  background: rgba(255, 255, 255, 0.2);
}

.input-data div.input {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.6rem;
  position: relative;
  height: 32px;
  pointer-events: none;
}

.input-data input {
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

.input-data:hover {
  background: rgba(var(--theme-color), 0.01);
}

.underline {
  background: rgba(var(--theme-color), 1);
  margin-left: -8px;
  height: 2px;
  width: calc(100% + 16px);
  transform: scale(0, 1);
  opacity: 0;
  border-radius: var(--border-radius-small);
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