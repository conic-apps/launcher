<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="instances-list-dropdown" ref="dropdownRef">
    <div class="head" @click="opened = !opened">
      <div class="label">{{ label }}</div>
      <div class="selected" :style="selectedWidth !== undefined ? `width: ${selectedWidth}px` : ''">
        {{ selected }}
        <span class="chevron" ref="chevronRef">
          <AppIcon name="chevron-down" :size="14"></AppIcon>
        </span>
      </div>
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
        <ul class="dropdown" v-if="opened" @click="opened = false">
          <slot></slot>
        </ul>
      </Transition>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "./AppIcon.vue";
import { onMounted, onUnmounted, ref, watch } from "vue";

const props = withDefaults(
  defineProps<{
    label: string;
    selected: string;
    selectedWidth?: number;
    expandDuration?: number;
    collapseDuration?: number;
    expandEasing?: string;
    collapseEasing?: string;
  }>(),
  {
    expandDuration: 200,
    collapseDuration: 200,
    expandEasing: "ease",
    collapseEasing: "ease",
  },
);

const opened = ref(false);

const dropdownRef = ref<HTMLElement | null>(null);
const chevronRef = ref<HTMLElement | null>(null);

function onPointerDownOutside(event: PointerEvent) {
  const target = event.target as HTMLElement;
  if (dropdownRef.value && !dropdownRef.value.contains(target)) {
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
      { duration: props.expandDuration, easing: props.expandEasing },
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
    { duration: props.expandDuration, easing: props.expandEasing },
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
    { duration: props.collapseDuration, easing: props.collapseEasing },
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
</script>

<style lang="less" scoped>
.instances-list-dropdown {
  position: relative;
  width: 100%;

  .head {
    display: flex;
    width: 100%;
    align-items: center;
    font-size: 13px;
    background: rgba(var(--ctp-surface0-rgb), 1);
    border-radius: 4px;

    .label {
      background: rgba(var(--ctp-surface1-rgb), 1);
      padding: 6px 12px;
      border-radius: 4px;
      flex-shrink: 0;
    }

    .selected {
      padding: 0 12px;
      width: 100%;
      height: 100%;
      display: flex;
      align-items: center;

      .chevron {
        margin-left: auto;
        display: inline-flex;
        align-items: center;
      }
    }
  }

  .dropdown {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    min-width: 100%;
    padding: 8px 10px;
    border-radius: var(--dialog-border-radius);
    border: var(--controllers-border);
    background: var(--ctp-base);
    box-shadow: 0px 0px 10px #4500611d;
    z-index: 100000;
    list-style: none;

    :deep(.dropdown-option) {
      height: 26px;
      padding: 0 8px;
      display: flex;
      align-items: center;
      margin: 4px 0;
      border-radius: var(--controllers-border-radius);
      font-size: 12px;
      list-style: none;
      white-space: nowrap;
      transition: all 30ms ease;

      &:hover {
        background: #ffffff1f;
      }

      &:active {
        background: #ffffff15;
      }

      &.selected {
        background: #ffffff17;
      }
    }
  }
}
</style>
