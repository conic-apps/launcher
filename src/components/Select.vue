<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="select" :style="`width: ${width}px;`" tabindex="0" @blur="opened = false">
    <div class="selected" @click="toggleOpened()">
      {{ displayName[selected] }}
      <AppIcon name="chevron-down"> </AppIcon>
    </div>
    <div>
      <Transition
        @before-enter="beforeEnter"
        @enter="enter"
        @after-enter="afterEnter"
        @before-leave="beforeLeave"
        @leave="leave"
        @after-leave="afterLeave">
        <ul
          ref="options"
          class="options"
          :style="`width: ${width}px;`"
          v-if="opened"
          @click="opened = false">
          <div v-if="opened">
            <li
              class="select-option"
              v-for="(_, index) in options"
              :key="index"
              @click="changeSelection(index)">
              {{ displayName[index] }}
            </li>
          </div>
        </ul>
      </Transition>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "./AppIcon.vue";
import { ref, useTemplateRef } from "vue";
const props = defineProps<{
  options: string[];
  width?: string;
  displayName: string[];
}>();
const model = defineModel();
const selected = ref(0);

const optionsList = useTemplateRef("options");
props.options.forEach((value, index) => {
  if (value == model.value) {
    selected.value = index;
  }
});
function beforeEnter(element: Element) {
  const htmlElement = element as HTMLElement;
  optionsList.value?.classList.remove("hidden");
  htmlElement.style.transition = transitionStyle;
  htmlElement.style.height = "0px";
}

const transitionStyle = "all 200ms ease";
function enter(element: Element) {
  const htmlElement = element as HTMLElement;
  const height = outerHeight(optionsList.value!);
  htmlElement.style.height = `${height}px`;
  htmlElement.style.overflow = "hidden";
}
function afterEnter(element: Element) {
  const htmlElement = element as HTMLElement;
  htmlElement.style.transition = "";
  htmlElement.style.height = "";
  htmlElement.style.overflow = "";
}
function beforeLeave(element: Element) {
  const htmlElement = element as HTMLElement;
  htmlElement.style.transition = transitionStyle;
  const height = outerHeight(optionsList.value!);
  htmlElement.style.height = `${height}px`;
  htmlElement.style.overflow = "hidden";
}
function leave(element: Element) {
  (element as HTMLElement).style.height = "0px";
}
function afterLeave(element: Element) {
  (element as HTMLElement).style.transition = "";
  (element as HTMLElement).style.height = "";
}
function changeSelection(index: number) {
  selected.value = index;
  model.value = props.options[index];
}
const opened = ref(false);
function toggleOpened() {
  opened.value = !opened.value;
}
function outerHeight(el: HTMLElement) {
  if (!el) return 0;
  let height = el.getBoundingClientRect().height;
  const style = getComputedStyle(el);
  height += parseFloat(style.marginTop) + parseFloat(style.marginBottom);
  return height;
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
