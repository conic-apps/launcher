<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="instances-list">
    <div class="tool-bar">
      <div class="search">
        <div class="search-input"><input type="text" placeholder="搜索..." /></div>
        <button class="search-button">
          <AppIcon name="search"></AppIcon>
        </button>
      </div>
      <div class="other">
        <div class="sort" ref="sortRef">
          <div class="head" @click="sortDropdownOpen = !sortDropdownOpen">
            <div class="label">排序</div>
            <div class="selected" style="width: 102px">
              {{ sortOptions.find((x) => x.key === sortMode)?.label }}
              <AppIcon name="chevron-down" :size="14" style="margin-left: auto"></AppIcon>
            </div>
          </div>
          <Transition name="instances-list-dropdown-fade">
            <ul class="dropdown" v-if="sortDropdownOpen">
              <li
                class="dropdown-option"
                v-for="option in sortOptions"
                :key="option.key"
                :class="{ selected: sortMode === option.key }"
                @click="selectSort(option.key)">
                {{ option.label }}
              </li>
            </ul>
          </Transition>
        </div>
        <div class="group" ref="groupRef">
          <div class="head" @click="groupDropdownOpen = !groupDropdownOpen">
            <div class="label">分组</div>
            <div class="selected">
              {{ groupOptions.find((x) => x.key === groupMode)?.label }}
              <AppIcon name="chevron-down" :size="14"></AppIcon>
            </div>
          </div>
          <Transition name="instances-list-dropdown-fade">
            <ul class="dropdown" v-if="groupDropdownOpen">
              <li
                class="dropdown-option"
                v-for="option in groupOptions"
                :key="option.key"
                :class="{ selected: groupMode === option.key }"
                @click="selectGroup(option.key)">
                {{ option.label }}
              </li>
            </ul>
          </Transition>
        </div>
      </div>
    </div>
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
            :class="{ current: instance.id === instanceStore.currentInstance.id }"
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
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import { useInstanceStore } from "@/store/instance";
import {
  formatLastPlayed,
  getBackgroundPath,
  Instance,
  LATEST_RELEASE_INSTANCE_ID,
  LATEST_SNAPSHOT_INSTANCE_ID,
  zhCN,
} from "@conic/instance";
import { nextTick, onMounted, onUnmounted, ref, useTemplateRef } from "vue";
import Lenis from "lenis";
import gsap from "gsap";
import { window as appWindow } from "@tauri-apps/api";
import { convertFileSrc } from "@tauri-apps/api/core";

const instanceStore = useInstanceStore();
const containerRef = useTemplateRef("container");
const contentRef = useTemplateRef("content");
const items = useTemplateRef<HTMLElement[]>("instances");
const wrappers = useTemplateRef<HTMLElement[]>("wrappers");

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
});

let resizeCleanup: (() => void) | undefined;

async function init() {
  await nextTick();
  ensureLenis();
  measureLayout();
  lenis?.resize();
  renderPositions(lenis ? lenis.scroll : (containerRef.value?.scrollTop ?? 0));
  resizeCleanup?.();
  resizeCleanup = await appWindow.getCurrentWindow().onResized(() => {
    measureLayout();
    renderPositions(lenis ? lenis.scroll : (containerRef.value?.scrollTop ?? 0));
  });
  Object.values(instanceStore.instances).forEach(async (instance) => {
    backgroundImagesSrc.value[instance.id] = await getBackgroundSrc(instance.id);
  });
}

const sortDropdownOpen = ref(false);
const groupDropdownOpen = ref(false);

const sortRef = useTemplateRef<HTMLElement>("sortRef");
const groupRef = useTemplateRef<HTMLElement>("groupRef");

function onPointerDownOutside(event: PointerEvent) {
  const target = event.target as HTMLElement;
  if (sortRef.value && !sortRef.value.contains(target)) {
    sortDropdownOpen.value = false;
  }
  if (groupRef.value && !groupRef.value.contains(target)) {
    groupDropdownOpen.value = false;
  }
}

onMounted(() => {
  document.addEventListener("pointerdown", onPointerDownOutside);
});

onUnmounted(() => {
  if (lenisTick) gsap.ticker.remove(lenisTick);
  lenis?.destroy();
  resizeCleanup?.();
  document.removeEventListener("pointerdown", onPointerDownOutside);
});

type SortMode = "name" | "version" | "playtime" | "lastplay";
const sortMode = ref<SortMode>("playtime");
const sortOptions: { key: SortMode; label: string }[] = [
  { key: "name", label: "名称" },
  { key: "version", label: "版本" },
  { key: "playtime", label: "游玩时间" },
  { key: "lastplay", label: "最后运行" },
];

function selectSort(mode: SortMode) {
  sortMode.value = mode;
  sortDropdownOpen.value = false;
}

type GroupMode = "all" | "none" | "loader";
const groupMode = ref<GroupMode>("all");
const groupOptions: { key: GroupMode; label: string }[] = [
  { key: "all", label: "全部实例" },
  { key: "none", label: "未分组" },
  { key: "loader", label: "模组加载器" },
];

function selectGroup(mode: GroupMode) {
  groupMode.value = mode;
  groupDropdownOpen.value = false;
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

  .tool-bar {
    height: 112px;
    width: 352px;
    position: absolute;
    top: 8px;
    right: 280px;
    border-radius: 16px 0 0 16px;
    background: rgba(var(--ctp-surface0-rgb), 0.4);
    backdrop-filter: blur(4px);
    z-index: 114;

    .search {
      display: flex;
      width: 320px;
      height: 40px;
      margin-top: 16px;
      margin-left: 16px;
    }

    .search .search-input {
      background: rgba(var(--ctp-surface0-rgb), 1);
      border-radius: 8px 0 0 8px;
      width: 100%;

      input {
        appearance: none;
        border: none;
        background: none;
        font-size: 14px;
        height: 100%;
        padding-left: 16px;
      }
    }

    .search button.search-button {
      width: 40px;
      flex-shrink: 0;
      appearance: none;
      border: none;
      background: rgba(var(--ctp-surface1-rgb), 1);
      border-radius: 0 8px 8px 0;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: all 0.1s ease;

      svg {
        transition: inherit;
      }

      &:hover {
        background: rgba(var(--ctp-surface2-rgb), 0.8);
      }

      &:active {
        background: rgba(var(--ctp-surface2-rgb), 1);

        svg {
          transform: scale(0.97);
        }
      }
    }

    .other {
      display: flex;
      width: 320px;
      margin-left: 16px;
      margin-top: 12px;

      > div {
        display: flex;
      }

      .sort {
        margin-right: 8px;
        flex-shrink: 0;
      }

      .group {
        width: 100%;
      }

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
          svg {
            margin-left: auto;
          }
        }
      }
    }

    .sort,
    .group {
      position: relative;
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

      .dropdown-option {
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

    .instances-list-dropdown-fade-leave-active,
    .instances-list-dropdown-fade-enter-active {
      transition: all 120ms ease;
    }

    .instances-list-dropdown-fade-leave-from,
    .instances-list-dropdown-fade-enter-to {
      opacity: 1;
    }

    .instances-list-dropdown-fade-leave-to,
    .instances-list-dropdown-fade-enter-from {
      opacity: 0;
    }
  }
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
    transition:
      border-left 200ms ease,
      margin 200ms ease,
      transform 100ms linear;

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
  .instance.current {
    border-left: 16px solid rgba(var(--ctp-lavender-rgb), 0.8);
    margin-left: -20px;
    transform: scale(1.03);
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
  }
}
</style>
