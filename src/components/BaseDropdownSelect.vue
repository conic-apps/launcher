<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="select" :style="`width: ${width}px;`" ref="selectRef">
    <div class="value-box" @click="opened = !opened">
      <span v-if="displayName[selected]">
        {{ displayName[selected] }}
      </span>
      <span v-else> {{ placeholder }}</span>
      <span class="chevron" ref="chevronRef">
        <AppIcon name="chevron-down" :size="14"> </AppIcon>
      </span>
    </div>
    <div>
      <Transition
        :css="false"
        @before-enter="onBeforeEnter"
        @enter="onEnter"
        @after-enter="onAfterEnter"
        @enter-cancelled="onEnterCancelled"
        @before-leave="onBeforeLeave"
        @leave="onLeave"
        @after-leave="onAfterLeave"
        @leave-cancelled="onLeaveCancelled">
        <ul
          ref="options"
          class="options"
          :style="{
            width: '${width}px',
            top: `-px`,
          }"
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
import AppIcon from "./AppIcon.vue";
import { onMounted, onUnmounted, ref, watch } from "vue";
const props = defineProps<{
  options: string[];
  width?: string;
  displayName: string[];
  placeholder?: string;
}>();
const model = defineModel();
const selected = ref(props.options.findIndex((value) => value == model.value));
const opened = ref(false);

const selectRef = ref<HTMLElement | null>(null);
const chevronRef = ref<HTMLElement | null>(null);

function onPointerDownOutside(event: PointerEvent) {
  const target = event.target as HTMLElement;
  if (selectRef.value && !selectRef.value.contains(target)) {
    opened.value = false;
  }
}

onMounted(() => {
  document.addEventListener("pointerdown", onPointerDownOutside);
});

onUnmounted(() => {
  document.removeEventListener("pointerdown", onPointerDownOutside);
  cancelAnimationFrame(collapseRaf);
  collapseRaf = 0;
});

const CHEVRON_FLIP_DURATION = 200;

function flipChevron(targetDeg: number) {
  const el = chevronRef.value;
  if (!el) return;
  el.getAnimations().forEach((animation) => animation.cancel());
  const fromDeg = targetDeg === 180 ? 0 : 180;
  el.animate(
    [
      { transform: `rotateX(${fromDeg}deg)`, opacity: "1" },
      { transform: "rotateX(90deg)", opacity: "0.7" },
      { transform: `rotateX(${targetDeg}deg)`, opacity: "1" },
    ],
    { duration: CHEVRON_FLIP_DURATION, easing: "ease-in-out" },
  ).onfinish = () => {
    el.style.transform = `rotateX(${targetDeg}deg)`;
  };
}

watch(opened, (value) => {
  flipChevron(value ? 180 : 0);
  if (value && leavingEl && leavingEl.isConnected) {
    pendingHeight = Math.round(leavingEl.offsetHeight);
    pendingOpacity = getComputedStyle(leavingEl).opacity;
    leavingEl = null;
  } else if (!value) {
    pendingHeight = null;
    pendingOpacity = "1";
  }
});

const DROPDOWN_ANIMATION_DURATION = 200;
const DROPDOWN_ANIMATION_EASING_EXPAND = "ease";
const DROPDOWN_ANIMATION_EASING_COLLAPSE = "ease";

let pendingHeight: number | null = null;
let pendingOpacity = "1";
let leavingEl: HTMLElement | null = null;
let lastCollapseHeight: number | null = null;
let lastCollapseOpacity = "1";
let collapseRaf = 0;

function captureInterruptedCollapse() {
  cancelAnimationFrame(collapseRaf);
  collapseRaf = 0;
  leavingEl = null;
  if (lastCollapseHeight !== null) {
    if (pendingHeight === null) {
      pendingHeight = Math.round(lastCollapseHeight);
      pendingOpacity = lastCollapseOpacity;
    }
    lastCollapseHeight = null;
  }
}

function resetTransitionStyles(el: HTMLElement) {
  el.style.height = "";
  el.style.opacity = "";
  el.style.overflow = "";
}

function onBeforeEnter(el: Element) {
  const target = el as HTMLElement;
  if (pendingHeight !== null) {
    target.style.height = `${pendingHeight}px`;
    target.style.overflow = "hidden";
  } else {
    target.style.opacity = "0";
  }
}

function onEnter(el: Element, done: () => void) {
  const target = el as HTMLElement;
  if (pendingHeight !== null) {
    const startHeight = pendingHeight;
    const startOpacity = pendingOpacity;
    pendingHeight = null;
    pendingOpacity = "1";
    target.style.height = "";
    target.style.overflow = "";
    const targetHeight = target.offsetHeight;
    target.style.height = `${startHeight}px`;
    target.style.overflow = "hidden";
    void target.offsetHeight;
    const animation = target.animate(
      [
        { height: `${startHeight}px`, opacity: startOpacity },
        { height: `${targetHeight}px`, opacity: "1" },
      ],
      { duration: DROPDOWN_ANIMATION_DURATION, easing: DROPDOWN_ANIMATION_EASING_EXPAND },
    );
    animation.onfinish = () => {
      resetTransitionStyles(target);
      done();
    };
    return;
  }
  const targetHeight = target.offsetHeight;
  target.style.height = "0px";
  target.style.overflow = "hidden";
  void target.offsetHeight;
  const animation = target.animate(
    [
      { height: "0px", opacity: "0" },
      { height: `${targetHeight}px`, opacity: "1" },
    ],
    { duration: DROPDOWN_ANIMATION_DURATION, easing: DROPDOWN_ANIMATION_EASING_EXPAND },
  );
  animation.onfinish = () => {
    resetTransitionStyles(target);
    done();
  };
}

function onAfterEnter(el: Element) {
  resetTransitionStyles(el as HTMLElement);
}

function onEnterCancelled(el: Element) {
  const target = el as HTMLElement;
  const currentHeight = target.offsetHeight;
  const currentOpacity = getComputedStyle(target).opacity;
  target.getAnimations().forEach((animation) => animation.cancel());
  target.style.height = `${currentHeight}px`;
  target.style.opacity = currentOpacity;
  target.style.overflow = "hidden";
}

function onBeforeLeave(el: Element) {
  const target = el as HTMLElement;
  target.style.height = `${target.offsetHeight}px`;
  target.style.overflow = "hidden";
}

function onLeave(el: Element, done: () => void) {
  const target = el as HTMLElement;
  target.getAnimations().forEach((animation) => animation.cancel());
  const startHeight = target.offsetHeight;
  const startOpacity = getComputedStyle(target).opacity;
  leavingEl = target;
  lastCollapseHeight = startHeight;
  lastCollapseOpacity = startOpacity;
  const animation = target.animate(
    [
      { height: `${startHeight}px`, opacity: startOpacity },
      { height: "0px", opacity: "0" },
    ],
    { duration: DROPDOWN_ANIMATION_DURATION, easing: DROPDOWN_ANIMATION_EASING_COLLAPSE },
  );
  const track = () => {
    lastCollapseHeight = target.offsetHeight;
    lastCollapseOpacity = getComputedStyle(target).opacity;
    collapseRaf = requestAnimationFrame(track);
  };
  collapseRaf = requestAnimationFrame(track);
  animation.onfinish = () => {
    cancelAnimationFrame(collapseRaf);
    collapseRaf = 0;
    leavingEl = null;
    lastCollapseHeight = null;
    target.style.height = "0px";
    target.style.opacity = "0";
    done();
  };
}

function onAfterLeave() {
  captureInterruptedCollapse();
}

function onLeaveCancelled(el: Element) {
  const target = el as HTMLElement;
  target.getAnimations().forEach((animation) => animation.cancel());
  captureInterruptedCollapse();
  resetTransitionStyles(target);
}

function changeSelection(index: number) {
  selected.value = index;
  model.value = props.options[index];
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
  &:hover {
    background: var(--controllers-background-hover);
  }
  &:hover::after {
    transform: translate(0px, 1px);
  }
  &:active {
    opacity: 0.8;
  }
}

.chevron {
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.options {
  width: 240px;
  margin-top: 4px;
  border-radius: var(--dialog-border-radius);
  border: var(--controllers-border);
  background: var(--ctp-base);
  box-shadow: 0px 0px 10px #4500611d;
  position: relative;
  font-size: 14px;
  z-index: 100000;
  display: flex;
  align-items: flex-start;

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
  &:hover {
    background: #ffffff1f;
  }
  &:active {
    background: #ffffff15;
  }
}

li.selected {
  background: #ffffff17;
}
</style>
