<!-- Conic Launcher -->
<!-- Copyright 2022-2026 OakChaser and contributors. All rights reserved. -->
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
        <div class="sort">
          <div class="head">
            <div class="label">排序</div>
            <div class="selected">
              {{ sortOptions.find((x) => x.key === sortMode)?.label }}
              <AppIcon name="chevron-down" :size="14" style="margin-left: 12px"></AppIcon>
            </div>
          </div>
          <div class="dropdown" v-if="sortDropdownOpen">
            <div
              class="sort-option"
              v-for="option in sortOptions"
              :key="option.key"
              :class="{ active: sortMode === option.key }"
              @click="selectSort(option.key)">
              {{ option.label }}
            </div>
          </div>
        </div>
        <div class="group">
          <div class="head">
            <div class="label">分组</div>
            <div class="selected">
              {{ sortOptions.find((x) => x.key === sortMode)?.label }}
              <AppIcon name="chevron-down" :size="14"></AppIcon>
            </div>
          </div>
          <div class="dropdown" v-if="sortDropdownOpen">
            <div
              class="sort-option"
              v-for="option in sortOptions"
              :key="option.key"
              :class="{ active: sortMode === option.key }"
              @click="selectSort(option.key)">
              {{ option.label }}
            </div>
          </div>
        </div>
      </div>
    </div>
    <div class="scroll-container" ref="container" @scroll="updatePositions">
      <div class="gap-top"></div>
      <div
        class="card-container"
        :class="{ current: instance.id === instanceStore.currentInstance.id }"
        v-for="instance in instanceStore.instances"
        :key="instance.id">
        <div
          class="instance"
          :class="{ current: instance.id === instanceStore.currentInstance.id }"
          :style="styleMap.get(instance.id)"
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
            <span class="last-play"><span class="label">上次运行：</span>昨天</span>
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
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import { useInstanceStore } from "@/store/instance";
import {
  getBackgroundPath,
  Instance,
  LATEST_RELEASE_INSTANCE_ID,
  LATEST_SNAPSHOT_INSTANCE_ID,
} from "@conic/instance";
import { nextTick, onMounted, reactive, ref, useTemplateRef } from "vue";
import { window as appWindow } from "@tauri-apps/api";
import { convertFileSrc } from "@tauri-apps/api/core";

const instanceStore = useInstanceStore();
const containerRef = useTemplateRef("container");
const items = useTemplateRef<HTMLElement[]>("instances");

const styleMap = reactive(new Map<string, { transform: string }>());

function updatePositions() {
  const container = containerRef.value;
  const elements = items.value;

  if (!container || !elements) return;

  const containerRect = container.getBoundingClientRect();

  const center = containerRect.height / 2;
  const maxOffset = 128;

  const curveRange = containerRect.height;

  for (const element of elements) {
    const rect = element.getBoundingClientRect();

    const y = rect.top - containerRect.top + rect.height / 2;

    const t = (y - center) / curveRange;

    const clamped = Math.max(-1, Math.min(1, t));

    const x = maxOffset * (1 - clamped * clamped);

    styleMap.set(element.dataset.id!, {
      transform: `translateX(${-x}px)`,
    });
  }
}

function scrollToInstance(instanceId: string, smooth: boolean) {
  const container = containerRef.value;
  const elements = items.value;

  if (!container || !elements) return;

  const element = elements.find((el) => el.dataset.id === instanceId);

  if (!element) return;

  const containerRect = container.getBoundingClientRect();
  const elementRect = element.getBoundingClientRect();

  const offset =
    elementRect.top + elementRect.height / 2 - (containerRect.top + containerRect.height / 2);

  if (smooth) {
    container.scrollTo({
      top: container.scrollTop + offset,
      behavior: "smooth",
    });
  } else {
    container.scrollTo({
      top: container.scrollTop + offset,
    });
  }
}

async function selectInstance(instance: Instance) {
  scrollToInstance(instance.id, true);
  await nextTick();
  instanceStore.currentInstance = instance;
}

onMounted(async () => {
  init();
});

async function init() {
  await nextTick();
  updatePositions();
  scrollToInstance(instanceStore.currentInstance.id, false);
  appWindow.getCurrentWindow().onResized(() => {
    updatePositions();
  });
  Object.values(instanceStore.instances).forEach(async (instance) => {
    backgroundImagesSrc.value[instance.id] = await getBackgroundSrc(instance.id);
  });
}

const sortDropdownOpen = ref(false);

type SortMode = "name" | "version" | "playtime" | "lastplay";
const sortMode = ref<SortMode>("playtime");
const sortOptions: { key: SortMode; label: string }[] = [
  { key: "name", label: "名称" },
  { key: "version", label: "版本" },
  { key: "playtime", label: "游玩时间" },
  { key: "lastplay", label: "最后运行日期" },
];

function selectSort(mode: SortMode) {
  sortMode.value = mode;
  sortDropdownOpen.value = false;
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
  }
  .scroll-container {
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    padding-left: 200px;
    .gap-top {
      height: 132px;
    }
    .gap-bottom {
      height: 100px;
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
      margin 200ms ease;
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
      .last-play {
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
  }

  .card-container {
    transition: all 100ms linear;
    margin-top: 0px;
    margin-bottom: 0px;

    &:active {
      transform: scale(0.99);
    }
  }
  .card-container.current {
    transform: scale(1.03);
    margin-top: 4px;
    margin-bottom: 4px;
    pointer-events: none;
  }
}
</style>
