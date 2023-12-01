<template>
  <div class="select" :style="`width: ${width}px;`">
    <div class="selected" @click="toggleOpened()">{{ selected }}</div>
    <div>
      <Transition @before-enter="beforeEnter" @enter="enter" @after-enter="afterEnter" @before-leave="beforeLeave"
        @leave="leave" @after-leave="afterLeave">
        <ul class="options" :style="`width: ${width}px;`" v-if="opened" @click="opened = false">
          <Transition name="fade">
            <div v-if="opened">
              <select-option v-for="(option, index) in options" :key="index" @click="changeSelection(index)"
                :text="option"></select-option>
            </div>
          </Transition>
          <div style="width: 100vw; height: 100vh; position: fixed; top: 0; left: 0; z-index: 10000;"></div>
        </ul>
      </Transition>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import SelectOption from './SelectOption.vue';
import $ from 'jquery'
const props = withDefaults(defineProps<{
  options: string[],
  default?: number | null,
  width?: string
}>(), {
  default: null
})
let selected = ref(
  typeof props.default == "number" ?
    props.options[props.default] : "")
function beforeEnter(element: HTMLElement) {
  $(element.firstElementChild).removeClass('hidden')
  element.style.transition = transitionStyle
  element.style.height = '0px'
}

const transitionStyle = 'all 200ms ease';
function enter(element: HTMLElement) {
  const height = $(element.firstElementChild).outerHeight(true)
  element.style.height = `${height}px`
  element.style.overflow = 'hidden'
}
function afterEnter(element: HTMLElement) {
  element.style.transition = ''
  element.style.height = ''
  element.style.overflow = ''
}
function beforeLeave(element: HTMLElement) {
  element.style.transition = transitionStyle
  const height = $(element.firstElementChild).outerHeight(true)
  element.style.height = `${height}px`
  element.style.overflow = 'hidden'
}
function leave(element: HTMLElement) {
  element.style.height = '0px'
}
function afterLeave(element: HTMLElement) {
  element.style.transition = ''
  element.style.height = ''
}
function changeSelection(index: number) {
  selected.value = props.options[index]
}
// onMounted(async () => {
//   selected.value = await load(props.config)
// })
let opened = ref(false)
function toggleOpened() {
  opened.value = !opened.value
}
</script>

<style lang="less" scoped>
.select {
  width: 200px;
  height: 26px;
  display: flex;
  flex-direction: column;
  justify-content: flex-start;
  font-size: calc(14px - var(--font-size-error));
}

.selected {
  width: 100%;
  border-radius: var(--border-radius-small);
  border: 1px solid rgba(var(--theme-color), 0.2);
  padding: 8px 12px;
  transition: opacity 100ms ease;
  display: flex;
  justify-content: space-between;
  align-items: center;
  transition: all 70ms ease;
  background: rgba(255, 255, 255, 0.2);
}

.selected:hover {
  background: #ffffffce;
}

.selected::after {
  content: "\f107";
  font-family: 'fa-pro';
  width: fit-content;
  height: fit-content;
  margin-right: 2px;
  transition: transform 100ms ease;
}

.selected:hover::after {
  transform: translate(0px, 1px);
}

.selected:active {
  opacity: 0.8;
}

.options {
  width: 200px;
  margin-top: 4px;
  border-radius: var(--border-radius-medium);
  border: 1px solid rgba(var(--theme-color), 0.2);
  background: #ffffff;
  box-shadow: 0px 0px 10px #4500611d;
  transform: scale3d(1, 1, 192.7);
  font-size: calc(14px - var(--font-size-error));

}

.options>div:first-child {
  margin: 10px 12px;
}
</style>