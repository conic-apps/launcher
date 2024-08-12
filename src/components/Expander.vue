<template>
  <div :class="expanderClass" :style="expanderStyle">
    <div class="expander-header" v-if="expanderHeader" @click="expander()" :style="headerStyle" ref="head">
      <div class="expander-title">
        <div class="expander-icon"><i :class="icon" :style="iconStyle"></i></div>
        <div>
          <h4>{{ title }}<div class="sub-title">
              <slot name="subtitle"></slot>
            </div>
          </h4>
          <p>{{ description }}</p>
        </div>
      </div>
      <div class="expander-button" :style="expanderButton"><i></i></div>
    </div>
    <Transition @before-enter="beforeEnter" @enter="enter" @after-enter="afterEnter" @before-leave="beforeLeave"
      @leave="leave" @after-leave="afterLeave">
      <div :class="bodyClassName" v-show="!isSwapping" :style="bodyStyle">
        <div :style="contentStyle" :class="contentClassName">
          <slot></slot>
        </div>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, type Ref } from 'vue'
import $ from 'jquery'

const props = withDefaults(defineProps<{
  title?: string,
  description?: string,
  canSwap?: boolean,
  isSwaped?: boolean,
  width?: string,
  height?: string,
  margin?: string,
  flex?: string,
  flexStart?: boolean,
  icon?: string,
  logoPixelated?: boolean,
  expanderHeader?: boolean,
  padding?: Array<number>,
}>(), {
  canSwap: true,
  isSwaped: false,
  expanderHeader: true,
  logoPixelated: false,
  margin: '',
})
let margin = props.margin.split(",")
let isSwapping = ref(!!props.isSwaped)
const expanderStyle = `margin: ${margin[0]}px ${margin[1]}px ${margin[2]}px ${margin[3]}px`
const bodyClassName = props.flexStart ? 'expander-body flex-start' : 'expander-body'
const bodyStyle = `width: ${props.width}px;height: ${props.height}px;${props.isSwaped ? "height: 0;overflow: hidden;" : ""};`
const contentStyle = props.padding ? `margin: ${props.padding[0]}px ${props.padding[1]}px ${props.padding[2]}px ${props.padding[3]}px;` : ''
const contentClassName = props.isSwaped ? "hidden" : ""
const expanderButton = props.canSwap ? "" : `display: none;`
const headerStyle = props.canSwap ? '' : 'pointer-events: none;'
const head = ref<any>(null)
let expanderClass = computed(() => {
  return isSwapping.value ? 'expander expander-not-expanded' : 'expander'
})
let iconStyle = computed(() => {
  return props.logoPixelated ? "image-rendering: pixelated;" : ""
})

const transitionStyle = 'all 250ms ease';
function beforeEnter(element: HTMLElement) {
  $(element.firstElementChild!).removeClass('hidden')
  element.style.transition = transitionStyle
  element.style.height = '0px'
}
function enter(element: HTMLElement) {
  const height = $(element.firstElementChild!).outerHeight(true)
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
  const height = $(element.firstElementChild!).outerHeight(true)
  element.style.height = `${height}px`
  element.style.overflow = 'hidden'
}
function leave(element: HTMLElement) {
  element.style.height = '0px'
  head.value.style['border-bottom'] = ''
}
function afterLeave(element: HTMLElement) {
  element.style.transition = ''
  element.style.height = ''
}
function expander() {
  isSwapping.value = !isSwapping.value
}
</script>

<style lang="less" scoped>
.expander {
  border-radius: var(--border-radius-large);
  background-color: #00000000;
  border: 1px solid #5000851d;
  box-shadow: 0 0 10px #ffffff15;
  margin: 15px 0 15px 0;
  transition: all 0.1s ease;
}

.expander-icon {
  width: 2rem;
  height: 2em;
  margin-right: 6px;
  display: flex;
  justify-content: center;
  align-items: center;
}

.expander-icon i {
  font-family: 'fa-pro';
  font-style: normal;
  font-size: calc(23px - var(--));
  font-weight: 500;
}

.expander-header,
.expander-body,
.expander-footer {
  padding: 16px 18px;
}

.expander-header {
  border-bottom: 1px solid #5000851d;
  display: flex;
  justify-content: space-between;
  align-items: center;
  transition: all 0.1s ease;
}

.expander-header:active {
  opacity: 0.6;
}

.expander-header>.expander-button {
  width: 36px;
  height: 36px;
  display: flex;
  align-content: center;
  justify-content: center;
  border-radius: var(--border-radius-medium);
}

.expander-button>i {
  font-family: "fa-pro";
  font-style: normal;
  width: 100%;
  height: 100%;
  font-size: 0.9em;
  display: flex;
  justify-content: center;
  align-items: center;
  transition: transform 0.25s ease;
  transform: rotate(180deg);
}

.expander-header:hover>.expander-button {
  background-color: #00000020;
}

.expander-header:active>.expander-button {
  background-color: #00000018;
}

.expander-header:active>.expander-button>i {
  transform: scale(0.85, 0.85) rotate(180deg);
}

.expander-header>.expander-title {
  display: flex;
  align-items: center;
}

.expander-header>.expander-title>div {
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.expander-header h4 {
  font-weight: 100;
  height: 20.5px;
  display: flex;
  align-items: center;
}

.expander-header h4>div {
  color: #0000006f;
  display: inline-block;
  font-size: 14px;
  margin-left: 4px;
}

.expander-header p {
  font-size: 12.3px;
  color: var(--text-color);
  opacity: 0.6;
  margin: 0;
}

.expander-body {
  padding: 0;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  border-radius: 0 0 var(--border-radius-large) var(--border-radius-large);
  background-color: rgba(0, 0, 0, 0.011);
}

.expander-body>div {
  margin: 24px 34px;
  font-size: 14px;
}

.expander-not-expanded .expander-header {
  border-bottom: 1px solid rgb(0 0 0 / 0%);
}

.expander-not-expanded .expander-header>.expander-button>i {
  transform: rotate(0deg);
}

.expander-not-expanded .expander-header:active>.expander-button>i {
  transform: scale(0.85, 0.85) rotate(0deg);
}

// .expander-body {
//   display:flex !important ;
// }
</style>