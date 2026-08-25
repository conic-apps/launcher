<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="content-screenshots">
    <button class="close" @click="close" :title="t('content.screenshots.close')" ref="closeButton">
      <AppIcon name="xmark" :size="20"></AppIcon>
    </button>
    <div class="main">
      <template v-if="screenshotSrcs.length === 0">
        <p class="empty">
          {{
            screenshots === null ? t("content.screenshots.loading") : t("content.screenshots.empty")
          }}
        </p>
      </template>
      <template v-else>
        <button
          class="nav prev"
          @click="previous"
          :disabled="currentIndex === 0"
          :title="t('content.screenshots.prev')"
          ref="prevButton">
          <AppIcon name="chevron-forward" :size="22"></AppIcon>
        </button>
        <div class="stage" @click.self="close">
          <img :src="currentSrc" alt="screenshot" ref="stageImage" />
        </div>
        <button
          class="nav next"
          @click="next"
          :disabled="currentIndex >= screenshotSrcs.length - 1"
          :title="t('content.screenshots.next')"
          ref="nextButton">
          <AppIcon name="chevron-forward" :size="22"></AppIcon>
        </button>
      </template>
    </div>
    <div class="thumbnails" v-if="screenshotSrcs.length > 0" ref="thumbnailsStrip">
      <ScrollViewHorizontal ref="scrollView">
        <div class="thumbnails-list">
          <div
            v-for="(src, index) in screenshotSrcs"
            :key="index"
            class="thumbnail"
            :class="{ active: index === currentIndex }"
            :ref="(el) => setThumbnailRef(el, index)"
            @click="select(index)">
            <img :src="src" alt="screenshot thumbnail" />
          </div>
        </div>
      </ScrollViewHorizontal>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useGameContentStore } from "@/store/content";
import ScrollViewHorizontal from "@/components/ScrollViewHorizontal.vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import gsap from "gsap";
import { computed, onMounted, onUnmounted, ref, useTemplateRef, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useShowContent } from "./useContent";

const { t } = useI18n();

const gameContentStore = useGameContentStore();

const currentIndex = ref(0);
const closing = ref(false);
const thumbnailElements = ref([] as HTMLElement[]);
const thumbnailsStrip = useTemplateRef("thumbnailsStrip");
const scrollView = useTemplateRef("scrollView");
const stageImage = useTemplateRef("stageImage");
const prevButton = useTemplateRef("prevButton");
const nextButton = useTemplateRef("nextButton");
const closeButton = useTemplateRef("closeButton");

const screenshots = computed(() => gameContentStore.gameContent.screenshots);

const screenshotSrcs = computed(() =>
  (screenshots.value ?? []).map((path) => convertFileSrc(path)),
);

const currentSrc = computed(() => screenshotSrcs.value[currentIndex.value] ?? "");

function select(index: number) {
  if (closing.value) return;
  if (index < 0 || index >= screenshotSrcs.value.length) return;
  currentIndex.value = index;
}

function next() {
  select(currentIndex.value + 1);
}

function previous() {
  select(currentIndex.value - 1);
}

function close() {
  if (closing.value) return;
  closing.value = true;
  playOutro(() => {
    useShowContent().value.screenshots = false;
  });
}

function setThumbnailRef(el: unknown, index: number) {
  if (el instanceof HTMLElement) {
    thumbnailElements.value[index] = el;
  }
}

watch(
  () => screenshotSrcs.value,
  (srcs) => {
    if (currentIndex.value >= srcs.length) {
      currentIndex.value = Math.max(0, srcs.length - 1);
    }
  },
);

watch(currentIndex, (index) => {
  const thumbnail = thumbnailElements.value[index];
  if (!thumbnail) return;
  scrollView.value?.scrollToCenter(thumbnail, true);
});

function collectTargets() {
  const strip = thumbnailsStrip.value;
  const thumbnailImages = strip ? Array.from(strip.querySelectorAll("img")) : [];
  const mainTargets = [
    stageImage.value,
    prevButton.value,
    nextButton.value,
    closeButton.value,
  ].filter((target): target is NonNullable<typeof target> => target !== null);
  return { strip, thumbnailImages, mainTargets };
}

function playIntro() {
  const { strip, thumbnailImages, mainTargets } = collectTargets();

  const timeline = gsap.timeline();
  if (strip) {
    timeline.from(strip, {
      y: "100%",
      duration: 0.6,
      ease: "power3.out",
    });
  }
  if (thumbnailImages.length > 0) {
    timeline.fromTo(
      thumbnailImages,
      { scale: 0.6, opacity: 0 },
      {
        scale: 1,
        opacity: 1,
        duration: 0.4,
        stagger: 0.03,
        ease: "back.out",
      },
      "<",
    );
  }
  if (mainTargets.length > 0) {
    timeline.fromTo(
      mainTargets,
      { scale: 0.6, opacity: 0 },
      {
        scale: 1,
        opacity: 1,
        duration: 0.5,
        ease: "back.out",
      },
      "<",
    );
  }
}

function playOutro(onComplete: () => void) {
  const { strip, thumbnailImages, mainTargets } = collectTargets();

  const timeline = gsap.timeline({ onComplete });
  if (strip) {
    timeline.to(strip, { y: "100%", duration: 0.3, ease: "power3.in" }, 0);
  }
  if (thumbnailImages.length > 0) {
    timeline.to(thumbnailImages, { scale: 0.7, opacity: 0, duration: 0.33, ease: "power3.in" }, 0);
  }
  if (mainTargets.length > 0) {
    timeline.to(mainTargets, { scale: 0.7, opacity: 0, duration: 0.33, ease: "power3.in" }, 0);
  }
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "ArrowLeft") {
    event.preventDefault();
    previous();
  } else if (event.key === "ArrowRight") {
    event.preventDefault();
    next();
  } else if (event.key === "Escape") {
    close();
  }
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown);
  playIntro();
});
onUnmounted(() => window.removeEventListener("keydown", onKeydown));
</script>

<style lang="less" scoped>
.content-screenshots {
  width: 100%;
  height: 100%;
  position: relative;
  display: flex;
  flex-direction: column;
  overflow: hidden;

  .close {
    position: absolute;
    top: 16px;
    right: 16px;
    width: 36px;
    height: 36px;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 50%;
    background: rgba(var(--ctp-surface0-rgb), 0.8);
    color: var(--ctp-text);
    transition: background 150ms ease;

    &:hover {
      background: var(--ctp-surface1);
    }
  }

  .main {
    flex: 1;
    min-height: 0;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 32px 96px;

    .nav {
      position: absolute;
      top: 50%;
      transform: translateY(-50%);
      width: 44px;
      height: 44px;
      border-radius: 50%;
      border: 1px solid var(--ctp-surface1);
      background: rgba(var(--ctp-surface0-rgb), 0.85);
      color: var(--ctp-text);
      display: flex;
      align-items: center;
      justify-content: center;
      z-index: 5;
      transition: background 150ms ease;

      &.prev {
        left: 24px;
      }

      &.next {
        right: 24px;
      }

      &.prev svg {
        transform: rotate(180deg);
      }

      &:hover:not(:disabled) {
        background: var(--ctp-surface1);
      }

      &:active:not(:disabled) {
        background: var(--ctp-surface2);
      }

      &:disabled {
        opacity: 0.35;
      }
    }

    .stage {
      flex: 1;
      min-width: 0;
      height: 100%;
      display: flex;
      align-items: center;
      justify-content: center;
      overflow: hidden;

      img {
        max-width: 100%;
        max-height: 100%;
        object-fit: contain;
        border-radius: 8px;
        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
        user-select: none;
        -webkit-user-drag: none;
      }
    }

    .empty {
      color: var(--ctp-overlay2);
      font-size: 14px;
    }
  }

  .thumbnails {
    flex-shrink: 0;
    height: clamp(92px, 12vh, 116px);
    position: relative;
    border-top: 1px solid var(--ctp-surface1);
    background: rgba(var(--ctp-surface0-rgb), 0.8);

    .thumbnails-list {
      display: flex;
      align-items: center;
      gap: 10px;
      width: max-content;
      height: 100%;

      .thumbnail {
        flex-shrink: 0;
        width: auto;
        height: calc(100% - 20px);
        border-radius: 6px;
        border: 2px solid transparent;
        overflow: hidden;
        opacity: 0.6;
        transition:
          opacity 150ms ease,
          border-color 150ms ease;

        &:first-child {
          margin-left: 96px;
        }

        &:last-child {
          margin-right: 96px;
        }

        &:hover {
          opacity: 1;
        }

        &.active {
          opacity: 1;
          border-color: var(--ctp-blue);
        }

        img {
          width: auto;
          height: 100%;
          display: block;
          user-select: none;
          -webkit-user-drag: none;
        }
      }
    }
  }
}
</style>
