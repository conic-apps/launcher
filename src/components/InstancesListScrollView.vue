<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="instances-list-scroll-view">
    <div class="scroll-container" ref="container">
      <div class="scroll-content" ref="content">
        <slot></slot>
      </div>
    </div>
  </div>
  <Teleport to="body">
    <div
      class="instances-scrollbar"
      :class="{ hidden: !scrollbarVisible || disabled }"
      :style="{ top: scrollbarTop, bottom: scrollbarBottom }"
      ref="scrollbar"
      @pointerdown="onScrollbarPointerDown">
      <div
        class="instances-scrollbar-thumb"
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
import {
  nextTick,
  onBeforeUpdate,
  onMounted,
  onUnmounted,
  onUpdated,
  ref,
  useTemplateRef,
} from "vue";
import Lenis from "lenis";
import gsap from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import { window as appWindow } from "@tauri-apps/api";

gsap.registerPlugin(ScrollTrigger);

withDefaults(
  defineProps<{
    scrollbarTop?: string;
    scrollbarBottom?: string;
    disabled?: boolean;
  }>(),
  {
    scrollbarTop: "calc(44px + 8px + 112px + 6px)",
    scrollbarBottom: "calc(56px + 4px)",
    disabled: false,
  },
);

const containerRef = useTemplateRef("container");
const contentRef = useTemplateRef("content");
const scrollbarRef = useTemplateRef("scrollbar");
const thumbRef = useTemplateRef("thumb");

let lenis: Lenis | undefined;
let lenisTick: ((time: number) => void) | undefined;

interface CardLayout {
  top: number;
  height: number;
}

interface CardRef {
  id: string;
  wrapper: HTMLElement;
  instance: HTMLElement;
}

let cardLayouts: CardLayout[] = [];
let setters: ((value: number) => void)[] = [];
let containerHeight = 0;
const maxOffset = 128;

let cardTriggers: ScrollTrigger[] = [];
let cardTriggersKey = "";

let contentHeight = 0;
let thumbHeight = 0;
let thumbDragOffsetY = 0;
let scrollbarTrackTop = 0;
const dragging = ref(false);
const scrollbarVisible = ref(false);

function queryCards(): CardRef[] {
  const content = contentRef.value;
  if (!content) return [];
  return Array.from(content.querySelectorAll<HTMLElement>("[data-id]")).map((instance) => ({
    id: instance.dataset.id ?? "",
    instance,
    wrapper: instance.parentElement ?? instance,
  }));
}

function ensureLenis() {
  const container = containerRef.value;
  const content = contentRef.value;

  if (lenis || !container || !content) return;

  lenis = new Lenis({
    wrapper: container,
    content,
    lerp: 0.16,
    smoothWheel: true,
  });

  lenis.on("scroll", (l: Lenis) => {
    renderPositions(l.scroll);
    updateScrollbar(l.scroll);
  });

  gsap.ticker.lagSmoothing(0);
  lenisTick = (time: number) => lenis!.raf(time * 1000);
  gsap.ticker.add(lenisTick);
}

function measureLayout() {
  const container = containerRef.value;
  const cards = queryCards();

  if (!container || cards.length === 0) return;

  containerHeight = container.clientHeight;
  contentHeight = container.scrollHeight;
  const containerRect = container.getBoundingClientRect();
  const scrollTop = container.scrollTop;

  setters = cards.map(
    ({ wrapper }) => gsap.quickSetter(wrapper, "x", "px") as (value: number) => void,
  );

  cardLayouts = cards.map(({ instance }) => {
    const rect = instance.getBoundingClientRect();
    return {
      top: rect.top - containerRect.top + scrollTop,
      height: rect.height,
    };
  });
}

function syncCardTriggers() {
  const container = containerRef.value;
  const cards = queryCards();

  if (!container) return;

  const key = cards.map((card) => card.id).join(",");
  if (key === cardTriggersKey) return;
  cardTriggersKey = key;

  cardTriggers.forEach((trigger) => trigger.kill());
  cardTriggers = cards.map(({ instance }) =>
    ScrollTrigger.create({
      trigger: instance,
      scroller: container,
      start: "top bottom",
      end: "bottom top",
      toggleClass: { targets: instance, className: "visible" },
    }),
  );
}

async function updateScrollbar(scrollY: number) {
  const scrollbar = scrollbarRef.value;
  const thumb = thumbRef.value;

  if (!scrollbar || !thumb) return;

  if (contentHeight <= containerHeight) {
    scrollbarVisible.value = false;
    return;
  }

  scrollbarVisible.value = true;
  await nextTick();

  const trackHeight = scrollbar.clientHeight;
  const maxScroll = contentHeight - containerHeight;
  thumbHeight = Math.max(32, trackHeight * (containerHeight / contentHeight));
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
  const maxScroll = contentHeight - containerHeight;

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
  const maxScroll = contentHeight - containerHeight;

  if (maxThumbTop > 0 && maxScroll > 0) {
    lenis?.scrollTo((clamped / maxThumbTop) * maxScroll, { immediate: true });
  }
}

function parallaxX(centerY: number): number {
  const t = (centerY - containerHeight / 2) / containerHeight;
  const clamped = Math.max(-1, Math.min(1, t));
  return -maxOffset * (1 - clamped * clamped);
}

function renderPositions(scrollY: number) {
  for (let i = 0; i < cardLayouts.length; i++) {
    const layout = cardLayouts[i];
    const centerY = layout.top - scrollY + layout.height / 2;
    setters[i](parallaxX(centerY));
  }
}

// FLIP transitions: when the list changes (reorder, filter), cards move from their
// previous position to the new one instead of jumping instantly. They travel along
// the same curved track that the wheel-scroll parallax draws: x follows the same
// parabola as renderPositions, sampled at the eased vertical position.
let firstRects = new Map<string, number>();

function cubicBezierEase(x1: number, y1: number, x2: number, y2: number) {
  const ax = 3 * x1 - 3 * x2 + 1;
  const bx = 3 * x2 - 6 * x1;
  const cx = 3 * x1;
  const ay = 3 * y1 - 3 * y2 + 1;
  const by = 3 * y2 - 6 * y1;
  const cy = 3 * y1;
  const sampleX = (t: number) => ((ax * t + bx) * t + cx) * t;
  const sampleY = (t: number) => ((ay * t + by) * t + cy) * t;
  return (x: number) => {
    let lower = 0;
    let upper = 1;
    let t = x;
    for (let i = 0; i < 12; i++) {
      const sample = sampleX(t);
      const diff = sample - x;
      if (Math.abs(diff) < 1e-5) break;
      if (diff < 0) lower = t;
      else upper = t;
      t = (lower + upper) / 2;
    }
    return sampleY(t);
  };
}

const flipEase = cubicBezierEase(0.22, 1, 0.36, 1);

onBeforeUpdate(() => {
  firstRects = new Map(
    queryCards().map((card) => [card.id, card.wrapper.getBoundingClientRect().top]),
  );
});

onUpdated(() => {
  const cards = queryCards();
  const scrollY = lenis ? lenis.scroll : (containerRef.value?.scrollTop ?? 0);

  measureLayout();
  renderPositions(scrollY);

  for (const card of cards) {
    const from = firstRects.get(card.id);
    if (from === undefined) {
      card.instance.animate(
        [
          { transform: "translateY(24px)", opacity: "0" },
          { transform: "translateY(0)", opacity: "1" },
        ],
        { duration: 300, easing: "cubic-bezier(0.22, 1, 0.36, 1)" },
      );
      continue;
    }

    const to = card.wrapper.getBoundingClientRect().top;
    const height = card.wrapper.getBoundingClientRect().height;
    const fromCenter = from + height / 2;
    const toCenter = to + height / 2;
    const dy = from - to;
    const dx = parallaxX(fromCenter) - parallaxX(toCenter);

    if (Math.abs(dy) < 0.5 && Math.abs(dx) < 0.5) continue;

    const SAMPLES = 24;
    const keyframes: { offset: number; transform: string }[] = [];

    for (let i = 0; i <= SAMPLES; i++) {
      const p = i / SAMPLES;
      const e = flipEase(p);
      const center = fromCenter + (toCenter - fromCenter) * e;
      const x = parallaxX(center) - parallaxX(toCenter);
      const y = dy * (1 - e);
      keyframes.push({ offset: p, transform: `translate(${x}px, ${y}px)` });
    }

    card.instance.animate(keyframes, { duration: 400, easing: "linear" });
  }

  firstRects = new Map();
  reflow();
});

async function reflow() {
  await nextTick();
  ensureLenis();
  measureLayout();
  syncCardTriggers();
  lenis?.resize();
  const scrollY = lenis ? lenis.scroll : (containerRef.value?.scrollTop ?? 0);
  renderPositions(scrollY);
  updateScrollbar(scrollY);
}

function scrollTo(instanceId: string, smooth: boolean) {
  const container = containerRef.value;
  const cards = queryCards();

  if (!container || cards.length === 0) return;

  const index = cards.findIndex((card) => card.id === instanceId);
  if (index === -1) return;

  measureLayout();
  const layout = cardLayouts[index];
  if (!layout) return;

  const target = layout.top + layout.height / 2 - containerHeight / 2;

  if (lenis) {
    lenis.scrollTo(target, {
      immediate: !smooth,
      ...(smooth ? { duration: 0.4, easing: gsap.parseEase("power3.out") } : {}),
    });
    return;
  }

  container.scrollTo({
    top: target,
    behavior: smooth ? "smooth" : "auto",
  });
}

function onWindowResize() {
  measureLayout();
  const nextScrollY = lenis ? lenis.scroll : (containerRef.value?.scrollTop ?? 0);
  renderPositions(nextScrollY);
  updateScrollbar(nextScrollY);
}

let resizeCleanup: (() => void) | undefined;

onMounted(async () => {
  reflow();
  resizeCleanup = await appWindow.getCurrentWindow().onResized(onWindowResize);
});

onUnmounted(() => {
  cardTriggers.forEach((trigger) => trigger.kill());
  cardTriggers = [];
  cardTriggersKey = "";
  if (lenisTick) gsap.ticker.remove(lenisTick);
  lenis?.destroy();
  resizeCleanup?.();
});

defineExpose({ reflow, scrollTo });
</script>

<style lang="less" scoped>
.instances-list-scroll-view {
  height: 100%;

  .scroll-container {
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;

    .scroll-content {
      padding-left: 200px;
    }
  }
}

.instances-scrollbar {
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

  .instances-scrollbar-thumb {
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
