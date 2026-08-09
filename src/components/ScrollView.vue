<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="wrapper" ref="wrapper">
    <div class="content" ref="content">
      <slot></slot>
    </div>
  </div>
  <Teleport to="body">
    <div
      class="scrollbar"
      :class="{ hidden: !scrollbarVisible || disabled }"
      :style="{ top: scrollbarTop, bottom: scrollbarBottom }"
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
  </Teleport>
</template>

<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, useTemplateRef } from "vue";
import Lenis from "lenis";

withDefaults(
  defineProps<{
    scrollbarTop?: string;
    scrollbarBottom?: string;
    disabled?: boolean;
  }>(),
  {
    scrollbarTop: "44px",
    scrollbarBottom: "0px",
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
let wrapperHeight = 0;
let contentHeight = 0;
let thumbHeight = 0;
let thumbDragOffsetY = 0;
let scrollbarTrackTop = 0;
let resizeObserver: ResizeObserver | undefined;

function ensureLenis() {
  const wrapper = wrapperRef.value;
  const content = contentRef.value;

  if (lenis || !wrapper || !content) return;

  lenis = new Lenis({
    wrapper,
    content,
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
  wrapperHeight = wrapper.clientHeight;
  contentHeight = wrapper.scrollHeight;
}

async function updateScrollbar(scrollY: number) {
  const scrollbar = scrollbarRef.value;
  const thumb = thumbRef.value;

  if (!scrollbar || !thumb) return;

  measure();

  if (contentHeight <= wrapperHeight) {
    scrollbarVisible.value = false;
    return;
  }

  scrollbarVisible.value = true;
  await nextTick();

  const trackHeight = scrollbar.clientHeight;
  const maxScroll = contentHeight - wrapperHeight;
  thumbHeight = Math.max(32, trackHeight * (wrapperHeight / contentHeight));
  thumb.style.height = `${thumbHeight}px`;

  const maxThumbTop = trackHeight - thumbHeight;
  const clamped = Math.max(0, Math.min(maxScroll, scrollY));
  thumb.style.top = `${maxThumbTop <= 0 ? 0 : (clamped / maxScroll) * maxThumbTop}px`;
}

function onThumbPointerDown(event: PointerEvent) {
  const thumb = thumbRef.value;
  if (!thumb) return;
  dragging.value = true;
  scrollbarTrackTop = scrollbarRef.value?.getBoundingClientRect().top ?? 0;
  thumbDragOffsetY = event.clientY - thumb.getBoundingClientRect().top;
  thumb.setPointerCapture(event.pointerId);
}

function onThumbPointerMove(event: PointerEvent) {
  if (!dragging.value) return;
  const scrollbar = scrollbarRef.value;
  if (!scrollbar) return;

  const trackHeight = scrollbar.clientHeight;
  const maxThumbTop = trackHeight - thumbHeight;
  const top = Math.max(
    0,
    Math.min(maxThumbTop, event.clientY - scrollbarTrackTop - thumbDragOffsetY),
  );
  const maxScroll = contentHeight - wrapperHeight;

  if (maxThumbTop > 0 && maxScroll > 0) {
    lenis?.scrollTo((top / maxThumbTop) * maxScroll, { immediate: true });
  }
}

function onThumbPointerUp(event: PointerEvent) {
  dragging.value = false;
  thumbRef.value?.releasePointerCapture(event.pointerId);
}

function onScrollbarPointerDown(event: PointerEvent) {
  const scrollbar = scrollbarRef.value;
  if (!scrollbar) return;

  const trackHeight = scrollbar.clientHeight;
  const maxThumbTop = trackHeight - thumbHeight;
  const top = event.clientY - scrollbar.getBoundingClientRect().top - thumbHeight / 2;
  const clamped = Math.max(0, Math.min(maxThumbTop, top));
  const maxScroll = contentHeight - wrapperHeight;

  if (maxThumbTop > 0 && maxScroll > 0) {
    lenis?.scrollTo((clamped / maxThumbTop) * maxScroll, { immediate: true });
  }
}

function onScroll() {
  updateScrollbar(lenis?.scroll ?? wrapperRef.value?.scrollTop ?? 0);
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
  const maxScroll = contentHeight - wrapperHeight;
  const clamped = Math.max(0, Math.min(maxScroll, target));

  if (lenis) {
    lenis.scrollTo(clamped, {
      immediate: !smooth,
      ...(smooth ? { duration: 0.4 } : {}),
    });
    return;
  }

  wrapper.scrollTo({
    top: clamped,
    behavior: smooth ? "smooth" : "auto",
  });
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

defineExpose({ reflow, scrollTo });
</script>

<style lang="less" scoped>
.wrapper {
  height: 100%;
  overflow-y: auto;
  overflow-x: hidden;
}

.scrollbar {
  position: fixed;
  right: 8px;
  width: 6px;
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
    width: 8px;
    height: 30%;
    border-radius: 999px;
    background: var(--ctp-text);
    opacity: 0.35;
    transition:
      opacity 160ms ease,
      width 160ms ease,
      left 160ms ease,
      transform 120ms ease;

    &:hover,
    &.dragging {
      opacity: 0.55;
      width: 10px;
      left: -2px;
    }

    &.dragging {
      transform: scale(0.9);
    }
  }
}
</style>
