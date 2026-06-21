<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="select" :style="`width: ${width}px;`" tabindex="0" @blur="opened = false">
    <div class="value-box" @click="opened = !opened">
      {{ displayName[selected] }}
      <AppIcon name="chevron-down" :size="14"> </AppIcon>
    </div>
    <div>
      <Transition>
        <ul
          ref="options"
          class="options"
          :style="{ width: '${width}px', top: `-${selected * 30 + 45}px` }"
          v-if="opened"
          @click="opened = false">
          <div>
            <li
              class="select-option"
              v-for="(_, index) in options"
              :class="{ selected: selected === index }"
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
import AppIcon from "../AppIcon.vue";
import { ref, useTemplateRef } from "vue";
const props = defineProps<{
  options: string[];
  width?: string;
  displayName: string[];
}>();
const model = defineModel();
const selected = ref(props.options.findIndex((value) => value == model.value));
const opened = ref(false);

const optionsList = useTemplateRef("options");

function changeSelection(index: number) {
  selected.value = index;
  model.value = props.options[index];
}

function outerHeight(el: HTMLElement) {
  if (!el) return 0;
  let height = el.getBoundingClientRect().height;
  console.log(el);
  console.log(height);
  const style = getComputedStyle(el);
  height += parseFloat(style.marginTop) + parseFloat(style.marginBottom);
  return height;
}
</script>

<style lang="less" scoped>
.select {
  width: 240px;
  height: 28px;
  display: flex;
  flex-direction: column;
  justify-content: flex-start;
  font-size: 12px;
}

.value-box {
  width: 100%;
  height: 100%;
  border-radius: var(--controllers-border-radius);
  border: var(--controllers-border);
  padding: 4px 8px;
  transition: opacity 100ms ease;
  display: flex;
  justify-content: space-between;
  align-items: center;
  transition: all 70ms ease;
  flex-shrink: 0;
  background: var(--controllers-background);
}

.value-box:hover {
  background: var(--controllers-background-hover);
}

.value-box:hover::after {
  transform: translate(0px, 1px);
}

.value-box:active {
  opacity: 0.8;
}

.options {
  width: 240px;
  margin-top: 4px;
  border-radius: var(--dialog-border-radius);
  border: var(--controllers-border);
  background: var(--dialog-background);
  box-shadow: 0px 0px 10px #4500611d;
  position: relative;
  font-size: 14px;
  z-index: 100000;
  display: flex;
  align-items: flex-end;
  > div:first-child {
    margin: 8px 10px;
    width: 100%;
  }
}

li.select-option {
  height: 26px;
  padding: 0 8px;
  display: flex;
  align-items: center;
  margin: 4px 0;
  border-radius: var(--controllers-border-radius);
  font-size: 12px;
  list-style: none;
  z-index: 10001;
  transition: all 30ms ease;
}

li.select-option:hover {
  background: #ffffff1f;
}

li.select-option:active {
  background: #ffffff15;
}
li.selected {
  background: #ffffff17;
}
</style>
