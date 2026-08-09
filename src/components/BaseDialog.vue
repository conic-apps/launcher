<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <Transition :css="false" @enter="onEnter" @leave="onLeave">
    <div v-if="visible" class="dialog" data-tauri-drag-region>
      <div class="content" :style="contentStyle">
        <slot></slot>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { computed } from "vue";
import gsap from "gsap";

const props = withDefaults(
  defineProps<{
    visible?: boolean;
    width?: number;
    height?: number;
  }>(),
  {
    visible: false,
  },
);
const contentStyle = computed(() => {
  return `width: ${props.width}px; height: ${props.height}px;`;
});

function onEnter(el: Element, done: () => void) {
  const content = el.querySelector(".content");
  const tl = gsap.timeline({ onComplete: done });
  tl.fromTo(el, { opacity: 0 }, { opacity: 1, duration: 0.2 }, 0);
  if (content) {
    tl.fromTo(
      content,
      { opacity: 0, scale: 0.9 },
      { opacity: 1, scale: 1, duration: 0.3, ease: "back.out" },
      0.2,
    );
  }
}

function onLeave(el: Element, done: () => void) {
  const content = el.querySelector(".content");
  const tl = gsap.timeline({ onComplete: done });
  tl.fromTo(el, { opacity: 1 }, { opacity: 0, duration: 0.2 }, 0.1);
  if (content) {
    tl.fromTo(
      content,
      { opacity: 1, scale: 1 },
      { opacity: 0, scale: 0.9, duration: 0.18, ease: "power1.out" },
      0,
    );
  }
}
</script>

<style lang="less" scoped>
.dialog {
  display: flex;
  justify-content: center;
  align-items: center;
  position: fixed;
  top: 0;
  left: 0;
  z-index: 11451419;
  width: 100%;
  height: 100%;
  background: #00000042;
  border-radius: 16px;

  .content {
    padding: 16px;
    background: var(--dialog-background);
    border: var(--dialog-border);
    box-shadow: 0 0 50px 0px #00000071;
    width: fit-content;
    height: fit-content;
    border-radius: var(--dialog-border-radius);
    max-width: calc(100vw - 20px);
    max-height: calc(100vh - 20px);
    overflow-x: visible;
    overflow-y: overlay;
  }
}
</style>
