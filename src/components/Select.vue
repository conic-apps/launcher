<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="select" :style="`width: ${width}px;`" tabindex="0" @blur="opened = false">
    <div class="selected" @click="toggleOpened()">{{ selected }}</div>
    <div>
      <Transition
        @before-enter="beforeEnter"
        @enter="enter"
        @after-enter="afterEnter"
        @before-leave="beforeLeave"
        @leave="leave"
        @after-leave="afterLeave">
        <ul class="options" :style="`width: ${width}px;`" v-if="opened" @click="opened = false">
          <div v-if="opened">
            <li
              class="select-option"
              v-for="(_, index) in options"
              :key="index"
              @click="changeSelection(index)">
              {{ displayName[index] }}
            </li>
          </div>
          <!-- <div style=" -->
          <!--     width: 100vw; -->
          <!--     height: 100vh; -->
          <!--     position: fixed; -->
          <!--     top: 0; -->
          <!--     left: 0; -->
          <!--     z-index: 10000; -->
          <!--   "></div> -->
        </ul>
      </Transition>
    </div>
  </div>
</template>

<script setup lang="ts">
function a() {
  alert(1);
}

import { ref } from "vue";
import $ from "jquery";
const props = defineProps<{
  options: string[];
  width?: string;
  displayName: string[];
}>();
const model = defineModel();
let selected = ref("");
props.options.map((value, index) => {
  if (value == model.value) {
    selected.value = props.displayName[index];
  }
});
function beforeEnter(element: HTMLElement) {
  $(element.firstElementChild!).removeClass("hidden");
  element.style.transition = transitionStyle;
  element.style.height = "0px";
}

const transitionStyle = "all 200ms ease";
function enter(element: HTMLElement) {
  const height = $(element.firstElementChild!).outerHeight(true);
  element.style.height = `${height}px`;
  element.style.overflow = "hidden";
}
function afterEnter(element: HTMLElement) {
  element.style.transition = "";
  element.style.height = "";
  element.style.overflow = "";
}
function beforeLeave(element: HTMLElement) {
  element.style.transition = transitionStyle;
  const height = $(element.firstElementChild!).outerHeight(true);
  element.style.height = `${height}px`;
  element.style.overflow = "hidden";
}
function leave(element: HTMLElement) {
  element.style.height = "0px";
}
function afterLeave(element: HTMLElement) {
  element.style.transition = "";
  element.style.height = "";
}
function changeSelection(index: number) {
  selected.value = props.displayName[index];
  model.value = props.options[index];
}
// onMounted(async () => {
//   selected.value = await load(props.config)
// })
let opened = ref(false);
function toggleOpened() {
  opened.value = !opened.value;
}
</script>

<style lang="less" scoped>
.select {
  width: 240px;
  height: 26px;
  display: flex;
  flex-direction: column;
  justify-content: flex-start;
  font-size: 14px;
}

.select li {
  list-style: none;
}

.selected {
  width: 100%;
  border-radius: var(--controllers-border-radius);
  border: var(--controllers-border);
  padding: 8px 12px;
  transition: opacity 100ms ease;
  display: flex;
  justify-content: space-between;
  align-items: center;
  transition: all 70ms ease;
  background: var(--controllers-background);
}

.selected:hover {
  background: var(--controllers-background-hover);
}

.selected::after {
  content: "\f107";
  font-family: "fa-pro";
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
  width: 240px;
  margin-top: 4px;
  border-radius: var(--dialog-border-radius);
  border: var(--controllers-border);
  background: var(--dialog-background);
  box-shadow: 0px 0px 10px #4500611d;
  transform: scale3d(1, 1, 192.7);
  font-size: 14px;
  z-index: 100000;
  display: flex;
  align-items: flex-end;
}

.options > div:first-child {
  margin: 10px 12px;
  width: 100%;
}

.select-option {
  padding: 10px 16px;
  border-radius: var(--controllers-border-radius);
  // position: relative;
  z-index: 10001;
  transition: all 30ms ease;
}

.select-option:hover {
  background: #ffffff1f;
}

.select-option:active {
  background: #ffffff15;
}
</style>
