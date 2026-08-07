<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="account-list-dropdown" ref="dropdownRef">
    <slot name="trigger" :opened="opened" :toggle="toggle"></slot>
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
        <ul class="dropdown" v-if="opened">
          <slot name="content" :opened="opened" :toggle="toggle"></slot>
        </ul>
      </Transition>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from "vue";

const props = withDefaults(
  defineProps<{
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

function toggle() {
  opened.value = !opened.value;
}

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

watch(opened, (value) => {
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
.account-list-dropdown {
  position: relative;
  margin-left: -16px;
  margin-bottom: 32px;
  background: var(--ctp-surface0);
  height: 36px;
  border-radius: 8px;
  font-size: 14px;
  display: flex;
  align-items: center;
  padding: 0 0 0 32px;
  box-shadow: 0 0 10px 0px rgba(0, 0, 0, 0.2);

  :deep(.account-switch) {
    appearance: none;
    background: var(--ctp-surface1);
    margin-left: 16px;
    height: 100%;
    width: 36px;
    border: none;
    border-radius: 0 8px 8px 0;

    &:hover {
      background: var(--ctp-surface2);
    }

    &:active {
      background: var(--ctp-overlay0);
    }
  }

  .dropdown {
    position: absolute;
    right: 0;
    bottom: calc(100% + 4px);
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
      gap: 8px;
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

      .player-name {
        overflow: hidden;
        text-overflow: ellipsis;
      }
    }
  }
}
</style>
