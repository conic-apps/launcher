<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="wrapper" ref="wrapper">
    <div class="content" ref="content">
      <slot></slot>
    </div>
  </div>
  <div
    class="scrollbar"
    :class="{ hidden: !scrollbarVisible || disabled }"
    :style="{ left: scrollbarLeft, right: scrollbarRight }"
    ref="scrollbar"
    @pointerdown="onScrollbarPointerDown">
    <div
      class="scrollbar-thumb"
      :class="{ dragging }"
      ref="thumb"
      @pointerdown.stop="onThumbPointerDown"
      @pointermove="onThumbPointerMove"
      @pointerup="onThumbPointerUp"
      @pointercancel="onThumbPointerUp"></div>
  </div>
</template>

<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, useTemplateRef } from "vue";
import Lenis from "lenis";

withDefaults(
  defineProps<{
    scrollbarLeft?: string;
    scrollbarRight?: string;
    disabled?: boolean;
  }>(),
  {
    scrollbarLeft: "8px",
    scrollbarRight: "8px",
    disabled: false,
  },
);

const wrapperRef = useTemplateRef("wrapper");
const contentRef = useTemplateRef("content");
const scrollbarRef = useTemplateRef("scrollbar");
const thumbRef = useTemplateRef("thumb");

const scrollbarVisible = ref(false);
const dragging = ref(false);

let lenis: Lenis | undefined;
let wrapperWidth = 0;
let contentWidth = 0;
let thumbWidth = 0;
let thumbDragOffsetX = 0;
let scrollbarTrackLeft = 0;
let resizeObserver: ResizeObserver | undefined;

function ensureLenis() {
  const wrapper = wrapperRef.value;
  const content = contentRef.value;

  if (lenis || !wrapper || !content) return;

  lenis = new Lenis({
    wrapper,
    content,
    orientation: "horizontal",
    lerp: 0.12,
    smoothWheel: true,
    autoRaf: true,
  });

  lenis.on("scroll", (l: Lenis) => {
    updateScrollbar(l.scroll);
  });
}

function measure() {
  const wrapper = wrapperRef.value;
  if (!wrapper) return;
  wrapperWidth = wrapper.clientWidth;
  contentWidth = wrapper.scrollWidth;
}

async function updateScrollbar(scrollX: number) {
  const scrollbar = scrollbarRef.value;
  const thumb = thumbRef.value;

  if (!scrollbar || !thumb) return;

  measure();

  if (contentWidth <= wrapperWidth) {
    scrollbarVisible.value = false;
    return;
  }

  scrollbarVisible.value = true;
  await nextTick();

  const trackWidth = scrollbar.clientWidth;
  const maxScroll = contentWidth - wrapperWidth;
  thumbWidth = Math.max(32, trackWidth * (wrapperWidth / contentWidth));
  thumb.style.width = `${thumbWidth}px`;

  const maxThumbLeft = trackWidth - thumbWidth;
  const clamped = Math.max(0, Math.min(maxScroll, scrollX));
  thumb.style.left = `${maxThumbLeft <= 0 ? 0 : (clamped / maxScroll) * maxThumbLeft}px`;
}

function onThumbPointerDown(event: PointerEvent) {
  const thumb = thumbRef.value;
  if (!thumb) return;
  dragging.value = true;
  scrollbarTrackLeft = scrollbarRef.value?.getBoundingClientRect().left ?? 0;
  thumbDragOffsetX = event.clientX - thumb.getBoundingClientRect().left;
  thumb.setPointerCapture(event.pointerId);
}

function onThumbPointerMove(event: PointerEvent) {
  if (!dragging.value) return;
  const scrollbar = scrollbarRef.value;
  if (!scrollbar) return;

  const trackWidth = scrollbar.clientWidth;
  const maxThumbLeft = trackWidth - thumbWidth;
  const left = Math.max(
    0,
    Math.min(maxThumbLeft, event.clientX - scrollbarTrackLeft - thumbDragOffsetX),
  );
  const maxScroll = contentWidth - wrapperWidth;

  if (maxThumbLeft > 0 && maxScroll > 0) {
    lenis?.scrollTo((left / maxThumbLeft) * maxScroll, { immediate: true });
  }
}

function onThumbPointerUp(event: PointerEvent) {
  dragging.value = false;
  thumbRef.value?.releasePointerCapture(event.pointerId);
}

function onScrollbarPointerDown(event: PointerEvent) {
  const scrollbar = scrollbarRef.value;
  if (!scrollbar) return;

  const trackWidth = scrollbar.clientWidth;
  const maxThumbLeft = trackWidth - thumbWidth;
  const left = event.clientX - scrollbar.getBoundingClientRect().left - thumbWidth / 2;
  const clamped = Math.max(0, Math.min(maxThumbLeft, left));
  const maxScroll = contentWidth - wrapperWidth;

  if (maxThumbLeft > 0 && maxScroll > 0) {
    lenis?.scrollTo((clamped / maxThumbLeft) * maxScroll, { immediate: true });
  }
}

function onScroll() {
  updateScrollbar(lenis?.scroll ?? wrapperRef.value?.scrollLeft ?? 0);
}

async function reflow() {
  await nextTick();
  ensureLenis();
  lenis?.resize();
  onScroll();
}

function scrollTo(target: number, smooth: boolean) {
  const wrapper = wrapperRef.value;
  if (!wrapper) return;

  measure();
  const maxScroll = contentWidth - wrapperWidth;
  const clamped = Math.max(0, Math.min(maxScroll, target));

  if (lenis) {
    lenis.scrollTo(clamped, {
      immediate: !smooth,
      ...(smooth ? { duration: 0.4 } : {}),
    });
    return;
  }

  wrapper.scrollTo({
    left: clamped,
    behavior: smooth ? "smooth" : "auto",
  });
}

function scrollToCenter(target: HTMLElement, smooth: boolean) {
  const wrapper = wrapperRef.value;
  if (!wrapper) return;

  const targetRect = target.getBoundingClientRect();
  const wrapperRect = wrapper.getBoundingClientRect();
  const targetLeft =
    wrapper.scrollLeft +
    (targetRect.left - wrapperRect.left) -
    (wrapper.clientWidth - targetRect.width) / 2;
  scrollTo(targetLeft, smooth);
}

onMounted(async () => {
  await reflow();

  resizeObserver = new ResizeObserver(() => {
    lenis?.resize();
    onScroll();
  });
  if (wrapperRef.value) resizeObserver.observe(wrapperRef.value);
  if (contentRef.value) resizeObserver.observe(contentRef.value);
});

onUnmounted(() => {
  resizeObserver?.disconnect();
  lenis?.destroy();
});

defineExpose({ reflow, scrollTo, scrollToCenter });
</script>

<style lang="less" scoped>
.wrapper {
  height: 100%;
  width: 100%;
  overflow-x: auto;
  overflow-y: hidden;
}

.content {
  height: 100%;
}

.scrollbar {
  position: absolute;
  bottom: 4px;
  height: 6px;
  z-index: 500;
  user-select: none;
  -webkit-user-select: none;
  touch-action: none;

  &.hidden {
    display: none;
  }

  .scrollbar-thumb {
    position: absolute;
    left: 0;
    top: 0;
    height: 8px;
    width: 30%;
    border-radius: 999px;
    background: var(--ctp-text);
    opacity: 0.35;
    transition:
      opacity 160ms ease,
      height 160ms ease,
      top 160ms ease,
      transform 120ms ease;

    &:hover,
    &.dragging {
      opacity: 0.55;
      height: 10px;
      top: -2px;
    }

    &.dragging {
      transform: scale(0.9);
    }
  }
}
</style>
