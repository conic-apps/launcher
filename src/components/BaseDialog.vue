<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <Transition :css="false" @enter="onEnter" @leave="onLeave">
    <div v-if="visible" class="dialog" data-tauri-drag-region>
      <div class="content" :style="contentStyle" ref="contentRef">
        <slot></slot>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import gsap from "gsap";

const props = withDefaults(
  defineProps<{
    visible?: boolean;
    width?: number;
    height?: number;
    animateHeight?: boolean;
  }>(),
  {
    visible: false,
    animateHeight: false,
  },
);

const HEIGHT_ANIMATION_DURATION = 0.3;

const contentRef = ref<HTMLElement | null>(null);

const contentStyle = computed(() => ({
  width: props.width != null ? `${props.width}px` : undefined,
  height: props.height != null ? `${props.height}px` : undefined,
}));

let resizeObserver: ResizeObserver | null = null;
let mutationObserver: MutationObserver | null = null;
let skipNextResize = false;

function autoHeightEnabled() {
  return props.animateHeight && props.height == null;
}

// Tracks slot children so the dialog height can tween whenever the rendered
// content changes size. An explicit pixel height is kept on .content between
// tweens so removing old content (e.g. out-in swaps) never collapses it.
function stopHeightTracking() {
  resizeObserver?.disconnect();
  resizeObserver = null;
  mutationObserver?.disconnect();
  mutationObserver = null;
}

function onContentResize() {
  const content = contentRef.value;
  if (!content || !content.isConnected || content.childElementCount === 0) return;
  if (skipNextResize) {
    skipNextResize = false;
    return;
  }
  gsap.to(content, {
    height: "auto",
    duration: HEIGHT_ANIMATION_DURATION,
    ease: "power2.out",
    overwrite: "auto",
    onComplete: () => {
      if (contentRef.value === content && content.isConnected) {
        content.style.height = `${content.offsetHeight}px`;
      }
    },
  });
}

function observeChildren(target: HTMLElement, skipFirst: boolean) {
  resizeObserver?.disconnect();
  skipNextResize = skipFirst;
  resizeObserver = new ResizeObserver(onContentResize);
  for (const child of Array.from(target.children)) {
    resizeObserver.observe(child);
  }
}

function startHeightTracking(target: HTMLElement) {
  stopHeightTracking();
  target.style.height = `${target.offsetHeight}px`;
  observeChildren(target, true);
  mutationObserver = new MutationObserver(() => observeChildren(target, false));
  mutationObserver.observe(target, { childList: true });
}

watch(
  () => props.height,
  (value, previous) => {
    if (!props.animateHeight || value == null || previous == null) return;
    const content = contentRef.value;
    if (!content || !content.isConnected) return;
    gsap.to(content, {
      height: value,
      duration: HEIGHT_ANIMATION_DURATION,
      ease: "power2.out",
      overwrite: "auto",
    });
  },
);

function onEnter(el: Element, done: () => void) {
  const content = el.querySelector(".content");
  if (content instanceof HTMLElement && autoHeightEnabled()) {
    startHeightTracking(content);
  }
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
  stopHeightTracking();
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

onMounted(async () => {
  if (props.visible && autoHeightEnabled()) {
    await nextTick();
    if (contentRef.value) startHeightTracking(contentRef.value);
  }
});

onUnmounted(() => {
  stopHeightTracking();
});
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
