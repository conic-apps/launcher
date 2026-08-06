<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="instances-list">
    <InstancesListToolBar
      :sortLabel="sortLabel"
      :sortOptions="sortOptions"
      :selectSort="selectSort"
      :groupLabel="groupLabel"
      :groupOptions="groupOptions"
      :selectGroup="selectGroup"
      v-model:sortMode="sortMode"
      v-model:groupMode="groupMode" />
    <div class="scroll-container" ref="container">
      <div class="scroll-content" ref="content">
        <div class="gap-top"></div>
        <div
          class="card-container"
          :class="{ current: instance.id === instanceStore.currentInstance.id }"
          v-for="instance in instanceStore.instances"
          :key="instance.id"
          ref="wrappers">
          <div
            class="instance"
            @click="selectInstance(instance)"
            :data-id="instance.id"
            ref="instances">
            <p v-if="instance.id === LATEST_RELEASE_INSTANCE_ID">
              {{ $t("game.latestRelease") }}
            </p>
            <p v-else-if="instance.id === LATEST_SNAPSHOT_INSTANCE_ID">
              {{ $t("game.latestSnapshot") }}
            </p>
            <p v-else>{{ instance.config.name }}</p>
            <div class="details">
              <span
                :class="`tag ${instance.config.runtime.mod_loader_type.toLowerCase()}`"
                v-if="instance.config.runtime.mod_loader_type"
                >{{ instance.config.runtime.mod_loader_type }}</span
              >
              <span class="tag vanilla" v-else>Vanilla</span>
              <span class="last-play"
                ><span class="label">上次运行：</span>
                <span v-if="instance.last_played">{{
                  formatLastPlayed(instance.last_played, zhCN)
                }}</span>
                <span v-else>从未运行</span>
              </span>
            </div>
            <img
              class="instance-background"
              v-if="backgroundImagesSrc[instance.id]"
              v-show="backgroundImagesShow[instance.id]"
              :src="backgroundImagesSrc[instance.id]"
              alt=""
              @load="backgroundImagesShow[instance.id] = true"
              @error="backgroundImagesShow[instance.id] = false" />
          </div>
        </div>
        <div class="gap-bottom"></div>
      </div>
    </div>
  </div>
  <Teleport to="body">
    <div
      class="instances-scrollbar"
      :class="{ hidden: !scrollbarVisible }"
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
import { useInstanceStore } from "@/store/instance";
import {
  formatLastPlayed,
  getBackgroundPath,
  Instance,
  LATEST_RELEASE_INSTANCE_ID,
  LATEST_SNAPSHOT_INSTANCE_ID,
  zhCN,
} from "@conic/instance";
import { computed, nextTick, onMounted, onUnmounted, ref, useTemplateRef } from "vue";
import Lenis from "lenis";
import gsap from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import { window as appWindow } from "@tauri-apps/api";
import { convertFileSrc } from "@tauri-apps/api/core";
import InstancesListToolBar from "./InstancesListToolBar.vue";

gsap.registerPlugin(ScrollTrigger);

const instanceStore = useInstanceStore();
const containerRef = useTemplateRef("container");
const contentRef = useTemplateRef("content");
const items = useTemplateRef<HTMLElement[]>("instances");
const wrappers = useTemplateRef<HTMLElement[]>("wrappers");
const scrollbarRef = useTemplateRef("scrollbar");
const thumbRef = useTemplateRef("thumb");

let lenis: Lenis | undefined;
let lenisTick: ((time: number) => void) | undefined;

interface CardLayout {
  top: number;
  height: number;
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
let scrollbarTop = 0;
const dragging = ref(false);
const scrollbarVisible = ref(false);

function ensureLenis() {
  const container = containerRef.value;
  const content = contentRef.value;

  if (lenis || !container || !content) return;

  lenis = new Lenis({
    wrapper: container,
    content,
    lerp: 1,
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
  const elements = items.value;
  const wrapperElements = wrappers.value;

  if (!container || !elements || !wrapperElements) return;

  containerHeight = container.clientHeight;
  contentHeight = container.scrollHeight;
  const containerRect = container.getBoundingClientRect();

  const count = Math.min(elements.length, wrapperElements.length);

  setters = wrapperElements
    .slice(0, count)
    .map((element) => gsap.quickSetter(element, "x", "px") as (value: number) => void);

  cardLayouts = elements.slice(0, count).map((element) => {
    const rect = element.getBoundingClientRect();
    return {
      top: rect.top - containerRect.top + container.scrollTop,
      height: rect.height,
    };
  });
}

function syncCardTriggers() {
  const container = containerRef.value;
  const elements = items.value;

  if (!container || !elements) return;

  const key = elements.map((el) => el.dataset.id ?? "").join(",");
  if (key === cardTriggersKey) return;
  cardTriggersKey = key;

  cardTriggers.forEach((trigger) => trigger.kill());
  cardTriggers = elements.map((el) =>
    ScrollTrigger.create({
      trigger: el,
      scroller: container,
      start: "top bottom",
      end: "bottom top",
      toggleClass: { targets: el, className: "visible" },
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
  scrollbarTop = scrollbarRef.value?.getBoundingClientRect().top ?? 0;
  thumbDragOffsetY = event.clientY - thumb.getBoundingClientRect().top;
  thumb.setPointerCapture(event.pointerId);
}

function onThumbPointerMove(event: PointerEvent) {
  if (!dragging.value) return;
  const scrollbar = scrollbarRef.value;
  if (!scrollbar) return;

  const trackHeight = scrollbar.clientHeight;
  const maxThumbTop = trackHeight - thumbHeight;
  const top = Math.max(0, Math.min(maxThumbTop, event.clientY - scrollbarTop - thumbDragOffsetY));
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

function renderPositions(scrollY: number) {
  const center = containerHeight / 2;
  const curveRange = containerHeight;

  for (let i = 0; i < cardLayouts.length; i++) {
    const layout = cardLayouts[i];
    const y = layout.top - scrollY + layout.height / 2;
    const t = (y - center) / curveRange;
    const clamped = Math.max(-1, Math.min(1, t));
    const x = maxOffset * (1 - clamped * clamped);
    setters[i](-x);
  }
}

function scrollToInstance(instanceId: string, smooth: boolean) {
  const elements = items.value;
  const container = containerRef.value;

  if (!elements || !container) return;

  const index = elements.findIndex((el) => el.dataset.id === instanceId);

  if (index === -1) return;

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

async function selectInstance(instance: Instance) {
  instanceStore.currentInstance = instance;
  await nextTick();
  measureLayout();
  scrollToInstance(instance.id, true);
}

// function instanceDblclick(instance: Instance) {
//   if (instanceStore.currentInstance === instance) {
//     navigationStore.navigate("launch");
//   }
// }

onMounted(async () => {
  await init();
  scrollToInstance(instanceStore.currentInstance.id, false);
  requestAnimationFrame(() => {
    measureLayout();
    const scrollY = lenis ? lenis.scroll : (containerRef.value?.scrollTop ?? 0);
    updateScrollbar(scrollY);
  });
});

let resizeCleanup: (() => void) | undefined;

async function init() {
  await nextTick();
  ensureLenis();
  measureLayout();
  syncCardTriggers();
  lenis?.resize();
  const scrollY = lenis ? lenis.scroll : (containerRef.value?.scrollTop ?? 0);
  renderPositions(scrollY);
  updateScrollbar(scrollY);
  resizeCleanup?.();
  resizeCleanup = await appWindow.getCurrentWindow().onResized(() => {
    measureLayout();
    const nextScrollY = lenis ? lenis.scroll : (containerRef.value?.scrollTop ?? 0);
    renderPositions(nextScrollY);
    updateScrollbar(nextScrollY);
  });
  Object.values(instanceStore.instances).forEach(async (instance) => {
    backgroundImagesSrc.value[instance.id] = await getBackgroundSrc(instance.id);
  });
}

const sortLabel = computed(() => sortOptions.find((x) => x.key === sortMode.value)?.label ?? "");
const groupLabel = computed(() => groupOptions.find((x) => x.key === groupMode.value)?.label ?? "");

onUnmounted(() => {
  cardTriggers.forEach((trigger) => trigger.kill());
  cardTriggers = [];
  cardTriggersKey = "";
  if (lenisTick) gsap.ticker.remove(lenisTick);
  lenis?.destroy();
  resizeCleanup?.();
});

export type SortMode = "name" | "version" | "playtime" | "lastplay";
const sortMode = ref<SortMode>("playtime");
const sortOptions: { key: SortMode; label: string }[] = [
  { key: "name", label: "名称" },
  { key: "version", label: "版本" },
  { key: "playtime", label: "游玩时间" },
  { key: "lastplay", label: "最后运行" },
];

function selectSort(mode: SortMode) {
  sortMode.value = mode;
}

export type GroupMode = "all" | "none" | "loader";
const groupMode = ref<GroupMode>("all");
const groupOptions: { key: GroupMode; label: string }[] = [
  { key: "all", label: "全部实例" },
  { key: "none", label: "未分组" },
  { key: "loader", label: "模组加载器" },
];

function selectGroup(mode: GroupMode) {
  groupMode.value = mode;
}

instanceStore.$subscribe(async () => {
  await nextTick();
  init();
});

const backgroundImagesSrc = ref<Record<string, string>>({});

const backgroundImagesShow = ref<Record<string, boolean>>({});

async function getBackgroundSrc(id: string) {
  const backgroundPath = await getBackgroundPath(id);
  return convertFileSrc(backgroundPath);
}
</script>

<style lang="less" scoped>
.instances-list {
  height: 100%;
  width: fit-content;
  margin-left: auto;
  transform: translateX(280px);
  overflow: visible;

  .scroll-container {
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    .scroll-content {
      padding-left: 200px;
      .gap-top {
        height: 132px;
      }
      .gap-bottom {
        height: 100px;
      }
    }
  }
  .instance {
    position: relative;
    border: 1px solid rgba(var(--ctp-surface1-rgb), 0.8);
    border-left: 16px solid rgba(var(--ctp-surface1-rgb), 0.8);
    background: rgba(var(--ctp-surface0-rgb), 0.4);
    padding: 8px 16px;
    border-radius: 8px;
    margin-top: 2px;
    width: 480px;
    height: 60px;
    opacity: 0.6;
    transition:
      border-left 200ms ease,
      margin 200ms ease,
      transform 100ms linear,
      opacity 300ms ease;

    &:active {
      transform: scale(0.99);
    }

    &:hover {
      background: rgba(var(--ctp-surface0-rgb), 0.8);
    }
    img.instance-background {
      mask-image: linear-gradient(to left, black 0%, transparent 100%);
      width: calc(100% - 200px);
      height: 100%;
      object-fit: cover;
      position: absolute;
      top: 0;
      left: 100px;
      border-radius: 0 8px 8px 0;
    }
    p {
      font-size: 16px;
    }
    .details {
      margin-top: 6px;
      .tag {
        font-size: 11px;
        border-radius: 100px;
        padding: 1px 6px;
        font-weight: 500;
        display: inline-flex;
        align-items: center;
        width: fit-content;
      }
      .tag.quilt {
        background: var(--ctp-mauve);
        color: var(--ctp-text-inverse);
      }
      .tag.fabric {
        background: var(--ctp-yellow);
        color: var(--ctp-text-inverse);
      }
      .tag.forge {
        background: var(--ctp-blue);
        color: var(--ctp-text-inverse);
      }
      .tag.neoforge {
        background: var(--ctp-peach);
        color: var(--ctp-text-inverse);
      }
      .tag.vanilla {
        background: var(--ctp-green);
        color: var(--ctp-text-inverse);
      }
      .last-play,
      .minecraft-version {
        font-size: 11px;
        margin-left: 8px;
        font-weight: 500;
        .label {
          opacity: 0.8;
          font-weight: 300;
        }
      }
    }
  }
  .instance.visible {
    opacity: 1;
  }

  .card-container {
    transition: margin 100ms linear;
    margin-top: 0px;
    margin-bottom: 0px;
    will-change: transform;
  }
  .card-container.current {
    margin-top: 4px;
    margin-bottom: 4px;

    .instance {
      border-left: 16px solid rgba(var(--ctp-lavender-rgb), 0.8);
      margin-left: -20px;
      transform: scale(1.03);
    }
  }
}

.instances-scrollbar {
  position: fixed;
  top: calc(44px + 8px + 112px + 6px);
  bottom: calc(56px + 4px);
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
    background: rgba(255, 255, 255, 1);
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
