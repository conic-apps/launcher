<template>
  <div class="input input-text input-slider">
    <span class="name">{{ name }}</span>
    <div style="display: flex; line-height: 1.7;width: 100%;justify-content: flex-end;">
      <div class="slider">
        <div :style="orbit"></div>
        <input ref="element" type="range" :max="max" :min="min" :step="step" v-model="value" @blur="onBlur">
      </div>
      <div class="input-data mini">
        <input type="number" :title="name" required v-model="value" placeholder="默认" @blur="onBlur" />
        <div class="underline"></div>
      </div>
      <span class="text">{{ text }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { load, update } from '@/utils/ConfigLoader'
import { computed, ref, type ComputedRef, type Ref } from 'vue'

const props = defineProps<{
  name: string,
  placeholder: string,
  config: string,
  max: string,
  min: string,
  step: string,
  text: string,
  AllowExceeding: string
}>()
let value: Ref<number> = ref(await load(props.config))

let orbit: ComputedRef<string> = computed((): string => {
  const min = Number(props.min)
  const max = Number(props.max)
  if (value.value > max - min) {
    if (props.AllowExceeding != "allow") {
      value.value = max
    }
    return 'width: 240px';
  }
  if (value.value - 1 - min < 0) {
    return 'width: 0px';
  } else if ((value.value - 1 - min) / (max - min) * 240 > 240) {
    return 'width: 240px'
  } else {
    return `width: ${(value.value - 1 - min) / (max - min) * 230 + 10}px`
  }
})

function onBlur(): void {
  if (!/^\d+$/.test(String(value.value))) {
    value.value = Number(props.min)
  }
  if (value.value - 1 - Number(props.min) < 0) {
    value.value = Number(props.min)
  }
  update(props.config, value.value)
}
</script>

<style lang="less" scoped>
.slider {
  width: 250px;
  display: flex;
  align-items: center;
  float: right;
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
</style>