<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="setting-collapse" :class="{ 'setting-collapse-disabled': props.disabled }">
    <div class="setting-collapse-head" :class="{ opened: expanded }" @click="toggle">
      <SettingItem
        :title="props.title"
        :description="props.description"
        :icon="props.icon"
        :resetable="props.resetable"
        @reset="$emit('reset')">
        <span class="chevron" ref="chevronRef">
          <AppIcon name="chevron-down" :size="17"></AppIcon>
        </span>
      </SettingItem>
    </div>
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
      <div v-if="opened" class="setting-collapse-content">
        <slot></slot>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "./AppIcon.vue";
import SettingItem from "./SettingItem.vue";
import { onUnmounted, ref, watch } from "vue";

const props = withDefaults(
  defineProps<{
    title: string;
    description?: string;
    icon?: string;
    resetable?: boolean;
    disabled?: boolean;
  }>(),
  {
    resetable: false,
    disabled: false,
  },
);

defineEmits(["reset"]);

const opened = ref(false);
const expanded = ref(false);
const chevronRef = ref<HTMLElement | null>(null);

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

function toggle() {
  if (props.disabled) return;
  opened.value = !opened.value;
}

watch(opened, (value) => {
  flipChevron(value ? 180 : 0);
  if (value) {
    expanded.value = true;
    if (leavingEl && leavingEl.isConnected) {
      pendingHeight = Math.round(leavingEl.offsetHeight);
      pendingOpacity = getComputedStyle(leavingEl).opacity;
      leavingEl = null;
    }
  } else {
    pendingHeight = null;
    pendingOpacity = "1";
  }
});

const COLLAPSE_ANIMATION_DURATION = 200;
const COLLAPSE_ANIMATION_EASING = "cubic-bezier(0.215, 0.61, 0.355, 1)";

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
      { duration: COLLAPSE_ANIMATION_DURATION, easing: COLLAPSE_ANIMATION_EASING },
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
    { duration: COLLAPSE_ANIMATION_DURATION, easing: COLLAPSE_ANIMATION_EASING },
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
    { duration: COLLAPSE_ANIMATION_DURATION, easing: COLLAPSE_ANIMATION_EASING },
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
  expanded.value = false;
  captureInterruptedCollapse();
}

function onLeaveCancelled(el: Element) {
  const target = el as HTMLElement;
  target.getAnimations().forEach((animation) => animation.cancel());
  captureInterruptedCollapse();
  resetTransitionStyles(target);
}

onUnmounted(() => {
  cancelAnimationFrame(collapseRaf);
  collapseRaf = 0;
});
</script>

<style lang="less" scoped>
.setting-collapse {
  margin: 0 auto;
  width: calc(100% - 16px);
  margin-bottom: 16px;

  .setting-collapse-head {
    border: var(--setting-group-border);
    border-radius: 8px;
    overflow: hidden;
    transition: border-radius 200ms ease;

    &.opened {
      border-radius: 8px 8px 0 0;
      border-bottom: none;
    }

    :deep(.setting-item) {
      margin-bottom: 0;
    }

    &:hover :deep(.setting-item) {
      background: var(--setting-item-background-hover);
    }

    &:active :deep(.setting-item) {
      background-color: var(--setting-item-background-active);
    }
  }

  .setting-collapse-content {
    border: var(--setting-group-border);
    border-top: none;
    border-radius: 0 0 8px 8px;
    overflow: hidden;

    > div:last-child {
      border-bottom-left-radius: 8px;
      border-bottom-right-radius: 8px;
    }
  }

  .chevron {
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
}

.setting-collapse-disabled {
  opacity: 0.6;

  * {
    pointer-events: none;
  }
}
</style>
